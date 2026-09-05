//! Allocation-free fatal failure capture and emergency-screen dispatch.

use crate::boot_info::BootInfo;
use core::fmt::{self, Write};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, Ordering};

const CRASH_SUMMARY_CAPACITY: usize = 192;
static BOOT_INFO: AtomicPtr<BootInfo> = AtomicPtr::new(core::ptr::null_mut());
static ACTIVE_PHASE: AtomicU8 = AtomicU8::new(CrashPhase::Bootstrap as u8);
static CRASH_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CrashPhase {
    Bootstrap = 0,
    Drivers = 1,
    Runtime = 2,
    Storage = 3,
    Services = 4,
    UserInterface = 5,
    Input = 6,
    Unknown = 255,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashReason {
    KernelPanic,
    InvalidBootInformation,
    ProcessorFault,
    InvariantViolation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrashReport {
    pub reason: CrashReason,
    pub phase: CrashPhase,
    pub code: u32,
    pub line: u32,
    pub column: u32,
    pub fingerprint: u64,
    summary: [u8; CRASH_SUMMARY_CAPACITY],
    summary_len: usize,
}

pub struct CrashCapture {
    report: Option<CrashReport>,
}

struct FixedText {
    bytes: [u8; CRASH_SUMMARY_CAPACITY],
    len: usize,
}

impl CrashPhase {
    // ------------------------=
    // FUNC: from_u8
    // DESC: Converts the stored phase byte into a bounded crash phase.
    // ------------------=
    const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Bootstrap,
            1 => Self::Drivers,
            2 => Self::Runtime,
            3 => Self::Storage,
            4 => Self::Services,
            5 => Self::UserInterface,
            6 => Self::Input,
            _ => Self::Unknown,
        }
    }

    // ------------------------=
    // FUNC: label
    // DESC: Returns the stable operator-facing name of a boot or runtime phase.
    // ------------------=
    pub const fn label(self) -> &'static [u8] {
        match self {
            Self::Bootstrap => b"BOOTSTRAP",
            Self::Drivers => b"HARDWARE AND DRIVERS",
            Self::Runtime => b"EXECUTION RUNTIME",
            Self::Storage => b"OBJECT STORAGE",
            Self::Services => b"SYSTEM SERVICES",
            Self::UserInterface => b"USER INTERFACE",
            Self::Input => b"INPUT EVENT LOOP",
            Self::Unknown => b"UNKNOWN PHASE",
        }
    }
}

impl CrashReason {
    // ------------------------=
    // FUNC: code
    // DESC: Returns a stable numeric reason code suitable for field diagnostics.
    // ------------------=
    pub const fn code(self) -> u32 {
        match self {
            Self::KernelPanic => 0x1001,
            Self::InvalidBootInformation => 0x1002,
            Self::ProcessorFault => 0x1003,
            Self::InvariantViolation => 0x1004,
        }
    }

    // ------------------------=
    // FUNC: label
    // DESC: Returns a concise classification for the fatal condition.
    // ------------------=
    pub const fn label(self) -> &'static [u8] {
        match self {
            Self::KernelPanic => b"KERNEL PANIC",
            Self::InvalidBootInformation => b"INVALID BOOT INFORMATION",
            Self::ProcessorFault => b"PROCESSOR FAULT",
            Self::InvariantViolation => b"SYSTEM INVARIANT VIOLATION",
        }
    }

    // ------------------------=
    // FUNC: description
    // DESC: Describes the class of failure without exposing secrets or raw memory.
    // ------------------=
    pub const fn description(self) -> &'static [u8] {
        match self {
            Self::KernelPanic => b"THE KERNEL STOPPED AFTER DETECTING AN UNRECOVERABLE SOFTWARE CONDITION.",
            Self::InvalidBootInformation => b"THE BOOT CONTRACT WAS INVALID, SO STARTUP COULD NOT CONTINUE SAFELY.",
            Self::ProcessorFault => b"THE PROCESSOR REPORTED A FAULT THAT COULD NOT BE CONTAINED.",
            Self::InvariantViolation => b"A REQUIRED SYSTEM SAFETY INVARIANT WAS VIOLATED.",
        }
    }
}

impl CrashReport {
    // ------------------------=
    // FUNC: new
    // DESC: Captures a bounded structured fatal report without heap allocation.
    // ------------------=
    pub fn new(
        reason: CrashReason,
        phase: CrashPhase,
        line: u32,
        column: u32,
        summary: &[u8],
    ) -> Self {
        let mut stored = [0u8; CRASH_SUMMARY_CAPACITY];
        let summary_len = summary.len().min(CRASH_SUMMARY_CAPACITY);
        stored[..summary_len].copy_from_slice(&summary[..summary_len]);
        let code = reason.code();
        let fingerprint = crash_fingerprint(code, phase, line, column, &stored[..summary_len]);
        Self {
            reason,
            phase,
            code,
            line,
            column,
            fingerprint,
            summary: stored,
            summary_len,
        }
    }

    // ------------------------=
    // FUNC: summary
    // DESC: Returns the bounded diagnostic summary stored in this report.
    // ------------------=
    pub fn summary(&self) -> &[u8] {
        &self.summary[..self.summary_len]
    }

    // ------------------------=
    // FUNC: summary_len
    // DESC: Reports the bounded diagnostic payload length without interpreting its prose.
    // ------------------=
    pub const fn summary_len(&self) -> usize {
        self.summary_len
    }
}

impl CrashCapture {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty first-failure capture slot.
    // ------------------=
    pub const fn new() -> Self {
        Self { report: None }
    }

    // ------------------------=
    // FUNC: capture
    // DESC: Stores only the first fatal report so recursive faults cannot hide the cause.
    // ------------------=
    pub fn capture(&mut self, report: CrashReport) -> bool {
        if self.report.is_some() {
            return false;
        }
        self.report = Some(report);
        true
    }

    // ------------------------=
    // FUNC: report
    // DESC: Returns the first fatal report if one has been captured.
    // ------------------=
    pub const fn report(&self) -> Option<&CrashReport> {
        self.report.as_ref()
    }
}

impl FixedText {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty allocation-free diagnostic text buffer.
    // ------------------=
    const fn new() -> Self {
        Self {
            bytes: [0; CRASH_SUMMARY_CAPACITY],
            len: 0,
        }
    }

    // ------------------------=
    // FUNC: as_bytes
    // DESC: Returns the portion of the diagnostic buffer that was written.
    // ------------------=
    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl Write for FixedText {
    // ------------------------=
    // FUNC: write_str
    // DESC: Appends UTF-8 diagnostics until the fixed emergency buffer is full.
    // ------------------=
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let remaining = self.bytes.len().saturating_sub(self.len);
        let amount = value.len().min(remaining);
        self.bytes[self.len..self.len + amount].copy_from_slice(&value.as_bytes()[..amount]);
        self.len += amount;
        Ok(())
    }
}

// ------------------------=
// FUNC: crash_fingerprint
// DESC: Computes a stable non-secret fingerprint from the structured failure record.
// ------------------=
fn crash_fingerprint(
    code: u32,
    phase: CrashPhase,
    line: u32,
    column: u32,
    summary: &[u8],
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in code
        .to_le_bytes()
        .into_iter()
        .chain([phase as u8])
        .chain(line.to_le_bytes())
        .chain(column.to_le_bytes())
        .chain(summary.iter().copied())
    {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// ------------------------=
// FUNC: register_boot_info
// DESC: Retains the validated handoff location for allocation-free fatal rendering.
// ------------------=
pub fn register_boot_info(info: *const BootInfo) {
    BOOT_INFO.store(info.cast_mut(), Ordering::Release);
}

// ------------------------=
// FUNC: set_phase
// DESC: Records the active initialization or runtime phase for later crash diagnosis.
// ------------------=
pub fn set_phase(phase: CrashPhase) {
    ACTIVE_PHASE.store(phase as u8, Ordering::Release);
}

// ------------------------=
// FUNC: fatal
// DESC: Stops the machine after rendering a structured explicit fatal condition.
// ------------------=
pub fn fatal(reason: CrashReason, summary: &[u8]) -> ! {
    let phase = CrashPhase::from_u8(ACTIVE_PHASE.load(Ordering::Acquire));
    fatal_report(CrashReport::new(reason, phase, 0, 0, summary))
}

// ------------------------=
// FUNC: fatal_panic
// DESC: Converts the first Rust panic into a bounded diagnostic report and stops safely.
// ------------------=
pub fn fatal_panic(info: &PanicInfo<'_>) -> ! {
    let phase = CrashPhase::from_u8(ACTIVE_PHASE.load(Ordering::Acquire));
    let mut summary = FixedText::new();
    let _ = write!(&mut summary, "{}", info.message());
    let (line, column) = info
        .location()
        .map(|location| (location.line(), location.column()))
        .unwrap_or((0, 0));
    fatal_report(CrashReport::new(
        CrashReason::KernelPanic,
        phase,
        line,
        column,
        summary.as_bytes(),
    ))
}

// ------------------------=
// FUNC: fatal_report
// DESC: Displays the first fatal report, emits its bounded summary, and quiesces the CPU.
// ------------------=
fn fatal_report(report: CrashReport) -> ! {
    let first_failure = CRASH_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok();
    if first_failure {
        unsafe {
            crate::output::write(b"InfinityOS fatal condition\n");
            crate::output::write(report.summary());
            crate::output::write(b"\n");
        }
        let info = BOOT_INFO.load(Ordering::Acquire);
        if !info.is_null() {
            unsafe {
                crate::bootstrap::show_fatal_crash(&*info, &report);
            }
        }
    }
    crate::output::quiesce();
    crate::output::idle()
}
