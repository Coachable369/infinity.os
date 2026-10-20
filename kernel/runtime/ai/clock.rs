//! Typed, allocation-free wall-clock commands. No model output is execution evidence.
use core::sync::atomic::{AtomicU64, Ordering};

pub static SYSTEM: SystemClock = SystemClock::new();

// Newlib's clockid_t ABI, forwarded unchanged by sdk/compiler/isolated.c.
// These are not Linux clock IDs; the native probe checks the actual C headers.
pub const NATIVE_CLOCK_REALTIME: u64 = 1;
pub const NATIVE_CLOCK_MONOTONIC: u64 = 4;

pub struct SystemClock { settings: AtomicU64, sequence: AtomicU64, hardware: AtomicU64, monotonic: AtomicU64 }

impl SystemClock {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an unavailable clock until trusted firmware publishes an epoch sample.
    // ------------------=
    pub const fn new() -> Self { Self { settings: AtomicU64::new(0), sequence: AtomicU64::new(0), hardware: AtomicU64::new(u64::MAX), monotonic: AtomicU64::new(0) } }
    // ------------------------=
    // FUNC: settings
    // DESC: Reads timezone and correction as one atomic generation to avoid mixed projections.
    // ------------------=
    pub fn settings(&self) -> Settings {
        let value = self.settings.load(Ordering::Acquire);
        Settings { offset_minutes: value as u16 as i16, correction_seconds: (value as i64) >> 16 }
    }
    // ------------------------=
    // FUNC: configure
    // DESC: Publishes a validated durable configuration without resetting the monotonic clock.
    // ------------------=
    pub fn configure(&self, settings: Settings) -> bool {
        if !settings.valid() { return false; }
        self.settings.store(((settings.correction_seconds as u64) << 16) | u64::from(settings.offset_minutes as u16), Ordering::Release); true
    }
    // ------------------------=
    // FUNC: observe
    // DESC: Publishes a BSP-owned firmware sample for worker-side clock reads without firmware calls.
    // ------------------=
    pub fn observe(&self, utc: u64, monotonic_ns: u64) {
        self.sequence.fetch_add(1, Ordering::AcqRel);
        self.hardware.store(utc, Ordering::Relaxed); self.monotonic.store(monotonic_ns, Ordering::Relaxed);
        self.sequence.fetch_add(1, Ordering::Release);
    }
    // ------------------------=
    // FUNC: utc_nanoseconds
    // DESC: Advances trusted UTC using monotonic elapsed time and the currently committed manual correction.
    // ------------------=
    pub fn utc_nanoseconds(&self, now: u64) -> Option<u64> {
        for _ in 0..4 {
            let sequence = self.sequence.load(Ordering::Acquire);
            if sequence & 1 != 0 { continue; }
            let hardware = self.hardware.load(Ordering::Relaxed);
            let monotonic = self.monotonic.load(Ordering::Relaxed);
            if self.sequence.load(Ordering::Acquire) != sequence { continue; }
            if hardware == u64::MAX || now < monotonic { return None; }
            return self.settings().utc(hardware)?.checked_mul(1_000_000_000)?.checked_add(now - monotonic);
        }
        None
    }
    // ------------------------=
    // FUNC: native_nanoseconds
    // DESC: Adapts the shared Newlib clock ABI without confusing UTC epoch time with monotonic uptime.
    // ------------------=
    pub fn native_nanoseconds(&self, clock: u64, now: u64) -> Result<u64, i64> {
        match clock {
            NATIVE_CLOCK_REALTIME => self.utc_nanoseconds(now).ok_or(-38),
            NATIVE_CLOCK_MONOTONIC => Ok(now),
            _ => Err(-22),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Read,
    SetTime { hour: u8, minute: u8, second: u8 },
    SetZone(i16),
    InvalidTime,
    InvalidZone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub offset_minutes: i16,
    pub correction_seconds: i64,
}

impl Settings {
    pub const UTC: Self = Self { offset_minutes: 0, correction_seconds: 0 };
    // ------------------------=
    // FUNC: valid
    // DESC: Bounds persistent offsets and corrections before arithmetic or clock publication.
    // ------------------=
    pub fn valid(self) -> bool {
        (-840..=840).contains(&self.offset_minutes)
            && (-6_311_520_000..=6_311_520_000).contains(&self.correction_seconds)
    }
    // ------------------------=
    // FUNC: utc
    // DESC: Applies the persistent OS wall-clock adjustment without changing monotonic timers.
    // ------------------=
    pub fn utc(self, hardware_utc: u64) -> Option<u64> {
        if !self.valid() { return None; }
        let utc = i64::try_from(hardware_utc).ok()?.checked_add(self.correction_seconds)?;
        (utc >= 0).then_some(utc as u64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Read, Updated, Denied, ClockUnavailable, InvalidTime, InvalidZone, SaveFailed }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub request: Request,
    pub outcome: Outcome,
    pub local: Option<LocalTime>,
    pub settings: Settings,
}

impl Request {
    // ------------------------=
    // FUNC: parse
    // DESC: Recognizes complete direct time requests, not quoted examples, negations or document instructions.
    // ------------------=
    pub fn parse(input: &[u8]) -> Option<Self> {
        if input.len() > 160 || input.iter().any(|c| matches!(c, b'\n'|b'\r'|b'"'|b'`')) { return None; }
        let mut buffer = [0; 160];
        let mut n = 0;
        for byte in input.trim_ascii() {
            let c = byte.to_ascii_lowercase();
            if c == b' ' && (n == 0 || buffer[n - 1] == b' ') { continue; }
            buffer[n] = c; n += 1;
        }
        let mut text = buffer[..n].trim_ascii().trim_ascii_end();
        while text.last().is_some_and(|b| matches!(b, b'?'|b'.'|b'!')) { text = text[..text.len()-1].trim_ascii_end(); }
        for wake in [b"infinity, ".as_slice(), b"computer, ", b"infinity ", b"computer ", b"please "] {
            text = text.strip_prefix(wake).unwrap_or(text);
        }
        if [b"what time is it".as_slice(), b"what is the time", b"what's the time", b"what is the current time",
            b"tell me the time", b"tell me what time it is", b"read the time", b"current time", b"clock"].contains(&text) {
            return Some(Self::Read);
        }
        for prefix in [b"set time zone to ".as_slice(), b"set the time zone to ", b"change time zone to ",
            b"change the time zone to ", b"set timezone to ", b"change timezone to ",
            b"set my time zone to ", b"set my timezone to ", b"clock zone "] {
            if let Some(value) = text.strip_prefix(prefix) { return Some(parse_zone(value).map(Self::SetZone).unwrap_or(Self::InvalidZone)); }
        }
        for prefix in [b"set time to ".as_slice(), b"set the time to ", b"change time to ", b"change the time to ", b"clock set "] {
            if let Some(value) = text.strip_prefix(prefix) { return Some(parse_time(value).unwrap_or(Self::InvalidTime)); }
        }
        None
    }
    // ------------------------=
    // FUNC: protocol
    // DESC: Decodes the bounded native AIKit clock protocol without arbitrary commands.
    // ------------------=
    pub fn protocol(input: &[u8]) -> Option<Self> {
        if input == b"read" { return Some(Self::Read); }
        if let Some(zone) = input.strip_prefix(b"zone:") { return Some(parse_zone(zone).map(Self::SetZone).unwrap_or(Self::InvalidZone)); }
        input.strip_prefix(b"set:").map(|time| parse_time(time).unwrap_or(Self::InvalidTime))
    }
}

// ------------------------=
// FUNC: number
// DESC: Parses a bounded unsigned decimal component with no signs, suffixes or wrapping.
// ------------------=
fn number(bytes: &[u8]) -> Option<u16> {
    if bytes.is_empty() || bytes.len() > 2 || !bytes.iter().all(u8::is_ascii_digit) { return None; }
    Some(bytes.iter().fold(0, |n, b| n * 10 + u16::from(b - b'0')))
}

// ------------------------=
// FUNC: parse_time
// DESC: Accepts explicit 24-hour time or an unambiguous AM/PM time with optional seconds.
// ------------------=
fn parse_time(input: &[u8]) -> Option<Request> {
    let mut lower = [0; 32];
    if input.len() > lower.len() { return None; }
    for (out, byte) in lower.iter_mut().zip(input) { *out = byte.to_ascii_lowercase(); }
    let mut value = lower[..input.len()].trim_ascii();
    let meridiem = if value.ends_with(b"am") { value = value[..value.len()-2].trim_ascii_end(); Some(false) }
        else if value.ends_with(b"pm") { value = value[..value.len()-2].trim_ascii_end(); Some(true) } else { None };
    let mut parts = value.split(|b| *b == b':');
    let mut hour = number(parts.next()?)?;
    let minute = if let Some(part) = parts.next() { number(part)? } else if meridiem.is_some() { 0 } else { return None; };
    let second = if let Some(part) = parts.next() { number(part)? } else { 0 };
    if parts.next().is_some() || minute > 59 || second > 59 { return None; }
    if let Some(pm) = meridiem { if !(1..=12).contains(&hour) { return None; } hour = hour % 12 + if pm {12} else {0}; }
    if hour > 23 { return None; }
    Some(Request::SetTime { hour: hour as u8, minute: minute as u8, second: second as u8 })
}

// ------------------------=
// FUNC: parse_zone
// DESC: Resolves explicitly fixed zones and UTC/GMT offsets; geographic DST rules are never guessed.
// ------------------=
pub fn parse_zone(input: &[u8]) -> Option<i16> {
    let mut upper = [0; 48];
    if input.len() > upper.len() { return None; }
    for (out, byte) in upper.iter_mut().zip(input) { *out = byte.to_ascii_uppercase(); }
    let value = upper[..input.len()].trim_ascii();
    let named = match value {
        b"UTC"|b"GMT"|b"Z" => Some(0), b"CST"|b"CENTRAL STANDARD TIME" => Some(-360),
        b"CDT"|b"CENTRAL DAYLIGHT TIME" => Some(-300), b"EST" => Some(-300), b"EDT" => Some(-240),
        b"MST" => Some(-420), b"MDT" => Some(-360), b"PST" => Some(-480), b"PDT" => Some(-420), _ => None,
    };
    if named.is_some() { return named; }
    let value = value.strip_prefix(b"UTC").or_else(|| value.strip_prefix(b"GMT")).unwrap_or(value).trim_ascii();
    let (sign, digits) = if let Some(rest) = value.strip_prefix(b"PLUS ") { (1, rest) }
        else if let Some(rest) = value.strip_prefix(b"MINUS ") { (-1, rest) }
        else { (match value.first()? { b'+' => 1, b'-' => -1, _ => return None }, &value[1..]) };
    let mut parts = digits.split(|b| *b == b':');
    let hour = number(parts.next()?)?;
    let minute = if let Some(part) = parts.next() { if part.len() != 2 { return None; } number(part)? } else { 0 };
    if parts.next().is_some() || hour > 14 || minute > 59 || hour == 14 && minute != 0 { return None; }
    Some(sign * (hour * 60 + minute) as i16)
}

// ------------------------=
// FUNC: leap
// DESC: Applies Gregorian leap-year rules for the shared native calendar.
// ------------------=
fn leap(year: u16) -> bool { year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) }

// ------------------------=
// FUNC: local_time
// DESC: Converts corrected UTC seconds to local calendar fields with proper day and year rollover.
// ------------------=
pub fn local_time(utc: u64, offset_minutes: i16) -> Option<LocalTime> {
    let local = i64::try_from(utc).ok()?.checked_add(i64::from(offset_minutes) * 60)?;
    if local < 0 { return None; }
    let mut days = local / 86400;
    let mut year = 1970u16;
    while days >= if leap(year) { 366 } else { 365 } {
        days -= if leap(year) { 366 } else { 365 }; year += 1;
        if year > 2199 { return None; }
    }
    let months = [31, if leap(year) {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0;
    while days >= months[month] { days -= months[month]; month += 1; }
    Some(LocalTime { year, month: month as u8 + 1, day: days as u8 + 1,
        hour: (local % 86400 / 3600) as u8, minute: (local % 3600 / 60) as u8, second: (local % 60) as u8 })
}

// ------------------------=
// FUNC: execute
// DESC: Verifies native clock input and authority, then commits before publishing a changed system clock.
// ------------------=
pub fn execute(request: Request, hardware_utc: Option<u64>, settings: Settings,
    can_read: bool, can_update: bool, mut commit: impl FnMut(Settings, LocalTime) -> bool) -> Receipt {
    let mut receipt = Receipt { request, outcome: Outcome::Denied, local: None, settings };
    if !can_read || !matches!(request, Request::Read) && !can_update { return receipt; }
    if matches!(request, Request::InvalidTime|Request::InvalidZone) {
        receipt.outcome = if request == Request::InvalidTime { Outcome::InvalidTime } else { Outcome::InvalidZone }; return receipt;
    }
    receipt.outcome = Outcome::ClockUnavailable;
    let Some(raw) = hardware_utc else { return receipt; };
    let Some(utc) = settings.utc(raw) else { return receipt; };
    let Some(local) = local_time(utc, settings.offset_minutes) else { return receipt; };
    receipt.local = Some(local);
    if request == Request::Read { receipt.outcome = Outcome::Read; return receipt; }
    let mut next = settings;
    match request {
        Request::SetZone(offset) => next.offset_minutes = offset,
        Request::SetTime { hour, minute, second } => {
            if hour > 23 || minute > 59 || second > 59 { receipt.outcome = Outcome::InvalidTime; return receipt; }
            let old_seconds = i64::from(local.hour) * 3600 + i64::from(local.minute) * 60 + i64::from(local.second);
            let new_seconds = i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
            let Some(correction) = next.correction_seconds.checked_add(new_seconds - old_seconds) else { return receipt; };
            next.correction_seconds = correction;
        },
        _ => return receipt,
    }
    let Some(next_local) = next.utc(raw).and_then(|time| local_time(time, next.offset_minutes)) else { return receipt; };
    if next != settings && !commit(next, next_local) { receipt.outcome = Outcome::SaveFailed; return receipt; }
    receipt.settings = next; receipt.local = Some(next_local); receipt.outcome = Outcome::Updated; receipt
}

// ------------------------=
// FUNC: format
// DESC: Produces bounded speech-friendly text only from a verified native clock receipt.
// ------------------=
pub fn format(receipt: Receipt, out: &mut [u8]) -> usize {
    struct Writer<'a> { out: &'a mut [u8], n: usize }
    impl core::fmt::Write for Writer<'_> {
        // ------------------------=
        // FUNC: write_str
        // DESC: Writes a complete formatted segment or fails before overflowing the native receipt buffer.
        // ------------------=
        fn write_str(&mut self, text: &str) -> core::fmt::Result {
            let end = self.n.checked_add(text.len()).ok_or(core::fmt::Error)?;
            let target = self.out.get_mut(self.n..end).ok_or(core::fmt::Error)?;
            target.copy_from_slice(text.as_bytes()); self.n = end; Ok(())
        }
    }
    use core::fmt::Write;
    let mut writer = Writer { out, n: 0 };
    match receipt.outcome {
        Outcome::Read|Outcome::Updated => if let Some(time) = receipt.local {
            let hour = if time.hour % 12 == 0 { 12 } else { time.hour % 12 };
            let meridiem = if time.hour < 12 { "AM" } else { "PM" };
            let prefix = match receipt.request { Request::SetZone(_) => "Time zone saved. The local system time is", Request::SetTime{..} => "System time saved. It is", _ => "The current time is" };
            let offset = receipt.settings.offset_minutes;
            let sign = if offset < 0 { "minus" } else { "plus" };
            let _ = write!(writer, "{} {}:{:02} {}", prefix, hour, time.minute, meridiem);
            if offset == 0 { let _ = write!(writer, ", UTC."); }
            else { let _ = write!(writer, ", UTC {} {} hours", sign, offset.unsigned_abs()/60);
                if offset % 60 != 0 { let _ = write!(writer, " {} minutes", offset.unsigned_abs()%60); }
                let _ = write!(writer, "."); }
            if matches!(receipt.request, Request::SetZone(_)) { let _ = write!(writer, " This is a fixed offset; daylight saving changes are not automatic."); }
        },
        Outcome::Denied => { let _ = writer.write_str("Changing or reading the system clock requires an active session with the appropriate Settings permission."); },
        Outcome::ClockUnavailable => { let _ = writer.write_str("The system clock is unavailable. I cannot report a reliable time."); },
        Outcome::InvalidTime => { let _ = writer.write_str("Use an explicit time such as 3:30 PM or 15:30. The system clock has not changed."); },
        Outcome::InvalidZone => { let _ = writer.write_str("Use UTC, CST, CDT, EST, EDT, MST, MDT, PST, PDT, or an explicit offset such as UTC plus 05:30. The system clock has not changed."); },
        Outcome::SaveFailed => { let _ = writer.write_str("The clock setting could not be saved. The system clock has not changed."); },
    }
    writer.n
}
