//! Native polling HDA backend. One controller, fixed coherent DMA, negotiated S16 stereo.
//! Caller must exclusively own the mapped controller and resident DMA storage.
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};
use core::cell::Cell;

pub const FRAMES: usize = 4800;
pub const SAMPLES: usize = FRAMES * 2;
pub const CAPTURE_FRAMES: usize = 96000;
pub const CAPTURE_SAMPLES: usize = CAPTURE_FRAMES * 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Timeout, Unsupported, Busy, Invalid, Dma }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureRoute {
    pub nodes: [u32; 8],
    pub selectors: [u32; 8],
    pub length: usize,
}
// ------------------------=
// FUNC: capture_route
// DESC: Searches a bounded codec graph without enabling any microphone or mutating codec state.
// ------------------=
pub fn capture_route<F: FnMut(u32, u32) -> Result<u32, Error>>(
    adc: u32, first: u32, end: u32, command: &mut F,
) -> Result<Option<CaptureRoute>, Error> {
    if first >= end || end > 128 || adc < first || adc >= end { return Err(Error::Invalid); }
    let mut route = CaptureRoute { nodes: [0; 8], selectors: [0; 8], length: 0 };
    if route_visit(adc, first, end, 0, 0, &mut route, command)? { Ok(Some(route)) } else { Ok(None) }
}
// ------------------------=
// FUNC: route_visit
// DESC: Follows ADC, mixer and selector connections with cycle, depth and connection-count bounds.
// ------------------=
fn route_visit<F: FnMut(u32, u32) -> Result<u32, Error>>(
    node: u32, first: u32, end: u32, visited: u128, depth: usize,
    route: &mut CaptureRoute, command: &mut F,
) -> Result<bool, Error> {
    if node < first || node >= end || depth == route.nodes.len() || visited & (1u128 << node) != 0 { return Ok(false); }
    let caps = command(node, 0xf0009)?;
    let kind = (caps >> 20) & 15;
    route.nodes[depth] = node;
    route.selectors[depth] = 0;
    if kind == 4 {
        if command(node, 0xf000c)? & 0x20 == 0 { return Ok(false); }
        route.length = depth + 1;
        return Ok(true);
    }
    if !matches!(kind, 1 | 2 | 3) { return Ok(false); }
    let connections = command(node, 0xf000e)?;
    // This backend supports short, non-range lists; unsupported routes fail closed.
    if connections & 0x80 != 0 || connections & 127 > 16 { return Ok(false); }
    for index in 0..(connections & 127) {
        let packed = command(node, 0xf0200 | (index & !3))?;
        let next = (packed >> ((index & 3) * 8)) & 255;
        if next & 0x80 != 0 { continue; }
        if route_visit(next, first, end, visited | (1u128 << node), depth + 1, route, command)? {
            route.selectors[depth] = index;
            return Ok(true);
        }
    }
    Ok(false)
}
#[repr(C, align(128))]
pub struct Dma {
    descriptors: [[u64; 2]; 16],
    playback: [i16; SAMPLES],
    capture: [i16; CAPTURE_SAMPLES],
    commands: [u32; 256],
    responses: [u64; 256],
}
impl Dma {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves resident zeroed DMA storage without allocation.
    // ------------------=
    pub const fn new() -> Self {
        Self { descriptors: [[0; 2]; 16], playback: [0; SAMPLES], capture: [0; CAPTURE_SAMPLES], commands: [0; 256], responses: [0; 256] }
    }
}
pub struct Hda {
    base: usize,
    dma: *mut Dma,
    output: usize,
    codec: u32,
    dac: u32,
    adc: u32,
    input_route: Option<CaptureRoute>,
    input_group: u32,
    command_write: Cell<u16>,
    response_read: Cell<u16>,
    capture_position: usize,
    pub capturing: bool,
    pub codec_id: u32,
    pub sample_rate: u32,
    pub playing: bool,
}
impl Hda {
    // ------------------------=
    // FUNC: r32
    // DESC: Reads one controller register.
    // ------------------=
    unsafe fn r32(&self, r: usize) -> u32 { read_volatile((self.base + r) as *const u32) }
    // ------------------------=
    // FUNC: w32
    // DESC: Writes one controller register.
    // ------------------=
    unsafe fn w32(&self, r: usize, v: u32) { write_volatile((self.base + r) as *mut u32, v); }
    // ------------------------=
    // FUNC: w16
    // DESC: Writes a register without modifying its neighboring W1C status bits.
    // ------------------=
    unsafe fn w16(&self, r: usize, v: u16) { write_volatile((self.base + r) as *mut u16, v); }
    // ------------------------=
    // FUNC: w8
    // DESC: Writes an eight-bit control register.
    // ------------------=
    unsafe fn w8(&self, r: usize, v: u8) { write_volatile((self.base + r) as *mut u8, v); }
    // ------------------------=
    // FUNC: wait
    // DESC: Bounds hardware initialization waits to a finite number of register reads.
    // ------------------=
    unsafe fn wait(&self, r: usize, mask: u32, expected: u32) -> Result<(), Error> {
        for _ in 0..100_000 {
            if self.r32(r) & mask == expected { return Ok(()); }
            core::hint::spin_loop();
        }
        Err(Error::Timeout)
    }
    // ------------------------=
    // FUNC: verb
    // DESC: Executes one codec verb through coherent CORB/RIRB DMA, including VirtualBox where immediate responses are masked.
    // ------------------=
    unsafe fn verb(&self, node: u32, command: u32) -> Result<u32, Error> {
        let next = (self.command_write.get() + 1) & 255;
        write_volatile(core::ptr::addr_of_mut!((*self.dma).commands[next as usize]), self.codec << 28 | node << 20 | command);
        fence(Ordering::Release);
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("dsb sy", options(nostack));
        self.command_write.set(next);
        self.w16(0x48, next);
        let response = (self.response_read.get() + 1) & 255;
        for _ in 0..100_000 {
            if read_volatile((self.base + 0x58) as *const u16) & 255 != self.response_read.get() {
                fence(Ordering::Acquire);
                #[cfg(target_arch = "aarch64")]
                core::arch::asm!("dsb sy", options(nostack));
                let value = read_volatile(core::ptr::addr_of!((*self.dma).responses[response as usize]));
                self.response_read.set(response);
                self.w8(0x5d, 1); // Retire the response-count event even when controller interrupts are disabled.
                if ((value >> 32) as u32 & 31) != self.codec { return Err(Error::Dma); }
                return Ok(value as u32);
            }
            core::hint::spin_loop();
        }
        Err(Error::Timeout)
    }
    // ------------------------=
    // FUNC: initialize_commands
    // DESC: Configures resident 256-entry codec command and response DMA rings with interrupts disabled.
    // ------------------=
    unsafe fn initialize_commands(&self) -> Result<(), Error> {
        if read_volatile((self.base + 0x4e) as *const u8) & 0x40 == 0
            || read_volatile((self.base + 0x5e) as *const u8) & 0x40 == 0 { return Err(Error::Unsupported); }
        let commands = core::ptr::addr_of_mut!((*self.dma).commands) as u64;
        let responses = core::ptr::addr_of_mut!((*self.dma).responses) as u64;
        if commands & 127 != 0 || responses & 127 != 0 { return Err(Error::Invalid); }
        for index in 0..256 {
            write_volatile(core::ptr::addr_of_mut!((*self.dma).commands[index]), 0);
            write_volatile(core::ptr::addr_of_mut!((*self.dma).responses[index]), 0);
        }
        self.w32(0x40, commands as u32); self.w32(0x44, (commands >> 32) as u32);
        self.w32(0x50, responses as u32); self.w32(0x54, (responses >> 32) as u32);
        self.w8(0x4e, 2); self.w8(0x5e, 2);
        self.w16(0x4a, 0x8000); self.w16(0x4a, 0);
        self.w16(0x48, 0); self.w16(0x58, 0x8000); self.w16(0x5a, 1);
        self.w8(0x4d, 1); self.w8(0x5d, 5);
        fence(Ordering::SeqCst);
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("dsb sy", options(nostack));
        // Enable response events for acknowledgement; INTCTL remains zero so no CPU IRQ is delivered.
        self.w8(0x5c, 3); self.w8(0x4c, 2);
        Ok(())
    }
    // ------------------------=
    // FUNC: parameter
    // DESC: Reads codec topology metadata.
    // ------------------=
    unsafe fn parameter(&self, node: u32, parameter: u32) -> Result<u32, Error> {
        self.verb(node, 0xf0000 | parameter)
    }
    // ------------------------=
    // FUNC: initialize
    // DESC: Resets a uniquely owned coherent HDA controller and discovers a direct pin-to-DAC output route.
    // ------------------=
    pub unsafe fn initialize(base: usize, dma: *mut Dma) -> Result<Self, Error> {
        if base == 0 || base & 0x3fff != 0 || dma.is_null() || dma as usize & 127 != 0 {
            return Err(Error::Invalid);
        }
        let mut h = Self { base, dma, output: 0, codec: 0, dac: 0, adc: 0,
            input_route: None, input_group: 0,
            command_write: Cell::new(0), response_read: Cell::new(0),
            capture_position: 0, capturing: false, codec_id: 0, sample_rate: 48000, playing: false };
        h.w32(0x20, 0); // No interrupts until an interrupt service exists.
        h.w8(0x4c, 0); h.w8(0x5c, 0);
        h.w32(8, 0); h.wait(8, 1, 0)?;
        h.w32(8, 1); h.wait(8, 1, 1)?;
        h.initialize_commands()?;
        let caps = h.r32(0) & 0xffff;
        let inputs = (caps >> 8) & 15;
        if (caps >> 12) & 15 == 0 { return Err(Error::Unsupported); }
        h.output = 0x80 + inputs as usize * 0x20;
        let mut presence = 0;
        for _ in 0..100_000 {
            presence = read_volatile((base + 0x0e) as *const u16);
            if presence != 0 { break; }
        }
        if presence == 0 { return Err(Error::Unsupported); }
        for codec in 0..15 {
            if presence & (1 << codec) == 0 { continue; }
            h.codec = codec;
            h.codec_id = h.parameter(0, 0)?;
            let groups = h.parameter(0, 4)?;
            for group in (groups >> 16)..(groups >> 16) + (groups & 255) {
                if h.parameter(group, 5)? & 255 != 1 { continue; }
                h.verb(group, 0x70500)?; // D0.
                let nodes = h.parameter(group, 4)?;
                let first = nodes >> 16;
                let end = (first + (nodes & 255)).min(128);
                for pin in first..end {
                    let pin_caps = h.parameter(pin, 9)?;
                    if (pin_caps >> 20) & 15 != 4 { continue; }
                    if h.parameter(pin, 0x0c)? & 0x10 == 0 { continue; }
                    let connections = h.parameter(pin, 0x0e)?;
                    // Narrow first backend: short-form direct DAC paths only.
                    if connections & 0x80 != 0 { continue; }
                    for index in 0..(connections & 127).min(16) {
                        let packed = h.verb(pin, 0xf0200 | (index & !3))?;
                        let dac = (packed >> ((index & 3) * 8)) & 255;
                        if dac < first || dac >= end { continue; }
                        let wcaps = h.parameter(dac, 9)?;
                        if (wcaps >> 20) & 15 != 0 { continue; }
                        let pcm = h.parameter(if wcaps & 0x10 != 0 { dac } else { group }, 0x0a)?;
                        if pcm & (1 << 17) == 0 { continue; }
                        h.sample_rate = if pcm & (1 << 6) != 0 { 48000 }
                            else if pcm & (1 << 5) != 0 { 44100 } else { continue; };
                        h.verb(dac, 0x70500)?;
                        h.verb(pin, 0x70500)?;
                        h.verb(pin, 0x70100 | index)?;
                        h.verb(pin, 0x70740)?;
                        h.verb(pin, 0x70c02)?; // EAPD output enabled where present.
                        if wcaps & 4 != 0 {
                            let amp = h.parameter(if wcaps & 8 != 0 { dac } else { group }, 0x12)?;
                            h.verb(dac, 0x3b000 | (amp & 127))?;
                        }
                        if pin_caps & 4 != 0 {
                            let amp = h.parameter(if pin_caps & 8 != 0 { pin } else { group }, 0x12)?;
                            h.verb(pin, 0x3b000 | (amp & 127))?;
                        }
                        h.dac = dac;
                        if inputs != 0 { let _ = h.discover_capture(first, end, group); }
                        return Ok(h);
                    }
                }
            }
        }
        Err(Error::Unsupported)
    }
    // ------------------------=
    // FUNC: discover_capture
    // DESC: Discovers direct or selector/amplifier ADC input paths without activating the microphone.
    // ------------------=
    unsafe fn discover_capture(&mut self, first: u32, end: u32, group: u32) -> Result<(), Error> {
        for adc in first..end {
            let caps = self.parameter(adc, 9)?;
            if (caps >> 20) & 15 != 1 { continue; }
            let pcm = self.parameter(if caps & 0x10 != 0 { adc } else { group }, 0x0a)?;
            let rate = if self.sample_rate == 48000 { 1 << 6 } else { 1 << 5 };
            if pcm & ((1 << 17) | rate) != ((1 << 17) | rate) { continue; }
            if let Some(route) = capture_route(adc, first, end, &mut |n, c| self.verb(n, c))? {
                self.input_route = Some(route);
                self.input_group = group;
                self.adc = adc;
                return Ok(());
            }
        }
        Ok(()) // Lack of an input route must not remove a working speaker.
    }
    // ------------------------=
    // FUNC: capture_available
    // DESC: Reports a discovered ADC route, not an invented microphone.
    // ------------------=
    pub fn capture_available(&self) -> bool { self.adc != 0 }
    // ------------------------=
    // FUNC: start_capture
    // DESC: Activates input DMA only after the service has validated microphone authority.
    // ------------------=
    pub unsafe fn start_capture(&mut self) -> Result<(), Error> {
        if self.capturing { return Err(Error::Busy); }
        let route = self.input_route.ok_or(Error::Unsupported)?;
        for i in 0..route.length {
            let node = route.nodes[i];
            let caps = self.parameter(node, 9)?;
            if caps & (1 << 10) != 0 { self.verb(node, 0x70500)?; }
            if i + 1 < route.length { self.verb(node, 0x70100 | route.selectors[i])?; }
            if caps & 2 != 0 {
                let amp = self.parameter(if caps & 8 != 0 { node } else { self.input_group }, 0x0d)?;
                self.verb(node, 0x37000 | (route.selectors[i] << 8) | (amp & 127))?;
            }
            if caps & 4 != 0 {
                let amp = self.parameter(if caps & 8 != 0 { node } else { self.input_group }, 0x12)?;
                self.verb(node, 0x3b000 | (amp & 127))?;
            }
        }
        let r = 0x80;
        self.w8(r, 1); self.wait(r, 1, 1)?;
        self.w8(r, 0); self.wait(r, 1, 0)?;
        for i in 0..CAPTURE_SAMPLES { write_volatile(core::ptr::addr_of_mut!((*self.dma).capture[i]), 0); }
        let address = core::ptr::addr_of_mut!((*self.dma).capture) as u64;
        let bdl = core::ptr::addr_of_mut!((*self.dma).descriptors[8]) as u64;
        write_volatile(core::ptr::addr_of_mut!((*self.dma).descriptors[8]), [address, CAPTURE_SAMPLES as u64]);
        write_volatile(core::ptr::addr_of_mut!((*self.dma).descriptors[9]), [address + CAPTURE_SAMPLES as u64, CAPTURE_SAMPLES as u64]);
        fence(Ordering::SeqCst);
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("dsb sy", options(nostack));
        self.w32(r + 8, (CAPTURE_SAMPLES * 2) as u32); self.w16(r + 0x0c, 1);
        self.w16(r + 0x12, if self.sample_rate == 48000 { 0x11 } else { 0x4011 });
        self.w32(r + 0x18, bdl as u32); self.w32(r + 0x1c, (bdl >> 32) as u32);
        self.verb(self.adc, if self.sample_rate == 48000 { 0x20011 } else { 0x24011 })?; self.verb(self.adc, 0x70620)?;
        let pin = route.nodes[route.length - 1];
        self.verb(pin, 0x70720)?;
        self.capture_position = 0;
        self.w8(r + 3, 0x1c); self.w32(r, (2 << 20) | 2);
        self.capturing = true;
        Ok(())
    }
    // ------------------------=
    // FUNC: read_capture
    // DESC: Copies only hardware-completed stereo frames; caller polls faster than one ring period.
    // ------------------=
    pub unsafe fn read_capture(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        if !self.capturing { return Err(Error::Invalid); }
        if self.r32(0x80) & (0x18 << 24) != 0 { return Err(Error::Dma); }
        let bytes = self.r32(0x84) as usize;
        if bytes >= CAPTURE_SAMPLES * 2 { return Err(Error::Dma); }
        let position = bytes / 4 * 2;
        fence(Ordering::Acquire);
        let count = ((position + CAPTURE_SAMPLES - self.capture_position) % CAPTURE_SAMPLES).min(out.len() & !1);
        for (i, sample) in out[..count].iter_mut().enumerate() {
            *sample = read_volatile(core::ptr::addr_of!((*self.dma).capture[(self.capture_position + i) % CAPTURE_SAMPLES]));
        }
        self.capture_position = (self.capture_position + count) % CAPTURE_SAMPLES;
        Ok(count)
    }
    // ------------------------=
    // FUNC: stop_capture
    // DESC: Stops capture DMA and unroutes its ADC immediately.
    // ------------------=
    pub unsafe fn stop_capture(&mut self) {
        self.w8(0x80, 0);
        if self.adc != 0 { let _ = self.verb(self.adc, 0x70600); }
        if let Some(route) = self.input_route {
            let _ = self.verb(route.nodes[route.length - 1], 0x70700);
        }
        self.capturing = false;
        for i in 0..CAPTURE_SAMPLES { write_volatile(core::ptr::addr_of_mut!((*self.dma).capture[i]), 0); }
    }
    // ------------------------=
    // FUNC: start
    // DESC: Copies exactly one 100-ms PCM loop into resident storage and starts hardware playback.
    // ------------------=
    pub unsafe fn start(&mut self, samples: &[i16]) -> Result<(), Error> {
        if self.playing { return Err(Error::Busy); }
        let count = self.sample_rate as usize / 10 * 2;
        if samples.len() != count { return Err(Error::Invalid); }
        self.stop();
        let r = self.output;
        self.w8(r, 1); self.wait(r, 1, 1)?;
        self.w8(r, 0); self.wait(r, 1, 0)?;
        for (i, &sample) in samples.iter().enumerate() {
            write_volatile(core::ptr::addr_of_mut!((*self.dma).playback[i]), sample);
        }
        let address = core::ptr::addr_of_mut!((*self.dma).playback) as u64;
        let bdl = core::ptr::addr_of_mut!((*self.dma).descriptors) as u64;
        write_volatile(core::ptr::addr_of_mut!((*self.dma).descriptors[0]), [address, count as u64]);
        write_volatile(core::ptr::addr_of_mut!((*self.dma).descriptors[1]), [address + count as u64, count as u64]);
        fence(Ordering::SeqCst);
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("dsb sy", options(nostack));
        self.w32(r + 8, (count * 2) as u32);
        self.w16(r + 0x0c, 1);
        self.w16(r + 0x12, if self.sample_rate == 48000 { 0x11 } else { 0x4011 });
        self.w32(r + 0x18, bdl as u32); self.w32(r + 0x1c, (bdl >> 32) as u32);
        self.verb(self.dac, if self.sample_rate == 48000 { 0x20011 } else { 0x24011 })?;
        self.verb(self.dac, 0x70610)?;
        self.w8(r + 3, 0x1c);
        self.w32(r, (1 << 20) | 2);
        self.playing = true;
        Ok(())
    }
    // ------------------------=
    // FUNC: position
    // DESC: Returns the DMA byte position or a stream error, never simulated progress.
    // ------------------=
    pub unsafe fn position(&self) -> Result<u32, Error> {
        if self.r32(self.output) & (0x18 << 24) != 0 { return Err(Error::Dma); }
        Ok(self.r32(self.output + 4))
    }
    // ------------------------=
    // FUNC: start_resident
    // DESC: Plays a kernel-owned prefilled phrase and silence tail without scheduling-sensitive refills.
    // ------------------=
    /// Caller keeps coherent, DMA-addressable samples resident and immutable until stop.
    pub unsafe fn start_resident(&mut self, samples: &'static [i16]) -> Result<(), Error> {
        if self.playing { return Err(Error::Busy); }
        if samples.is_empty() || samples.len() % 2 != 0 || samples.len() > 48_000 * 2 * 32 { return Err(Error::Invalid); }
        let r = self.output;
        self.stop(); self.w8(r, 1); self.wait(r, 1, 1)?;
        self.w8(r, 0); self.wait(r, 1, 0)?;
        let address = samples.as_ptr() as u64;
        let bytes = samples.len() as u64 * 2;
        let bdl = core::ptr::addr_of_mut!((*self.dma).descriptors) as u64;
        // VirtualBox requires nonzero LVI; use two resident extents, neither needs refilling.
        write_volatile(core::ptr::addr_of_mut!((*self.dma).descriptors[0]), [address, bytes / 2]);
        write_volatile(core::ptr::addr_of_mut!((*self.dma).descriptors[1]), [address + bytes / 2, bytes / 2]);
        fence(Ordering::SeqCst);
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("dsb sy", options(nostack));
        self.w32(r + 8, bytes as u32); self.w16(r + 0x0c, 1);
        self.w16(r + 0x12, if self.sample_rate == 48000 { 0x11 } else { 0x4011 });
        self.w32(r + 0x18, bdl as u32); self.w32(r + 0x1c, (bdl >> 32) as u32);
        self.verb(self.dac, if self.sample_rate == 48000 { 0x20011 } else { 0x24011 })?;
        self.verb(self.dac, 0x70610)?;
        self.w8(r + 3, 0x1c); self.w32(r, (1 << 20) | 2); self.playing = true;
        Ok(())
    }
    // ------------------------=
    // FUNC: refill_playback_half
    // DESC: Replaces only a consumed DMA half with the next 50 ms of interleaved PCM; never writes the active half.
    // ------------------=
    pub unsafe fn refill_playback_half(&mut self, half: usize, samples: &[i16]) -> Result<(), Error> {
        let count = self.sample_rate as usize / 10;
        if !self.playing || half > 1 || samples.len() != count { return Err(Error::Invalid); }
        let position = self.position()? as usize;
        if position >= count * 4 { return Err(Error::Dma); }
        if position / (count * 2) == half { return Err(Error::Busy); }
        for (index, &sample) in samples.iter().enumerate() {
            write_volatile(core::ptr::addr_of_mut!((*self.dma).playback[half * count + index]), sample);
        }
        fence(Ordering::SeqCst);
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("dsb sy", options(nostack));
        // A missed deadline is reported, not concealed as a successful refill.
        if self.position()? as usize / (count * 2) == half { return Err(Error::Dma); }
        Ok(())
    }
    // ------------------------=
    // FUNC: stop
    // DESC: Stops output DMA without enabling capture.
    // ------------------=
    pub unsafe fn stop(&mut self) { self.w8(self.output, 0); self.playing = false; }
}

// ------------------------=
// FUNC: tone
// DESC: Generates 44 complete 440-Hz cycles in a 100-ms stereo buffer using bounded integer sine approximation.
// ------------------=
pub fn tone(out: &mut [i16; SAMPLES]) {
    let _ = tone_at_rate(out, 48000);
}
// ------------------------=
// FUNC: tone_at_rate
// DESC: Generates a whole-cycle tone at the codec's negotiated 48 or 44.1-kHz rate.
// ------------------=
pub fn tone_at_rate(out: &mut [i16], rate: u32) -> Result<(), Error> {
    if !matches!(rate, 48000 | 44100) || out.len() != rate as usize / 10 * 2 { return Err(Error::Invalid); }
    let half = rate as i64 / 2;
    for frame in 0..out.len() / 2 {
        let phase = (frame as i64 * 440) % rate as i64;
        let negative = phase >= half;
        let x = phase % half;
        // Bhaskara sine approximation; peak is one quarter of full scale.
        let product = x * (half - x);
        let sample = (16 * product * 8191 / (5 * half * half - 4 * product)) as i16;
        let sample = if negative { -sample } else { sample };
        out[frame * 2] = sample; out[frame * 2 + 1] = sample;
    }
    Ok(())
}
