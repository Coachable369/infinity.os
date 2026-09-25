//! Native polling HDA backend. One controller, fixed coherent DMA, negotiated S16 stereo.
//! Caller must exclusively own the mapped controller and resident DMA storage.
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};

pub const FRAMES: usize = 4800;
pub const SAMPLES: usize = FRAMES * 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Timeout, Unsupported, Busy, Invalid, Dma }
#[repr(C, align(128))]
pub struct Dma {
    descriptors: [[u64; 2]; 16],
    playback: [i16; SAMPLES],
    capture: [i16; SAMPLES],
}
impl Dma {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves resident zeroed DMA storage without allocation.
    // ------------------=
    pub const fn new() -> Self {
        Self { descriptors: [[0; 2]; 16], playback: [0; SAMPLES], capture: [0; SAMPLES] }
    }
}
pub struct Hda {
    base: usize,
    dma: *mut Dma,
    output: usize,
    codec: u32,
    dac: u32,
    adc: u32,
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
    // DESC: Executes one immediate codec command with bounded timeout and no CORB dependency.
    // ------------------=
    unsafe fn verb(&self, node: u32, command: u32) -> Result<u32, Error> {
        self.wait(0x68, 1, 0)?;
        self.w16(0x68, 2);
        self.w32(0x60, self.codec << 28 | node << 20 | command);
        self.w16(0x68, 1);
        self.wait(0x68, 3, 2)?;
        let value = self.r32(0x64);
        self.w16(0x68, 2);
        Ok(value)
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
            capture_position: 0, capturing: false, codec_id: 0, sample_rate: 48000, playing: false };
        h.w32(0x20, 0); // No interrupts until an interrupt service exists.
        h.w8(0x4c, 0); h.w8(0x5c, 0);
        h.w32(8, 0); h.wait(8, 1, 0)?;
        h.w32(8, 1); h.wait(8, 1, 1)?;
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
                        if inputs != 0 { h.discover_capture(first, end, group)?; }
                        return Ok(h);
                    }
                }
            }
        }
        Err(Error::Unsupported)
    }
    // ------------------------=
    // FUNC: discover_capture
    // DESC: Finds a direct input-pin to ADC route without activating capture or its pin.
    // ------------------=
    unsafe fn discover_capture(&mut self, first: u32, end: u32, group: u32) -> Result<(), Error> {
        for adc in first..end {
            let caps = self.parameter(adc, 9)?;
            if (caps >> 20) & 15 != 1 { continue; }
            let pcm = self.parameter(if caps & 0x10 != 0 { adc } else { group }, 0x0a)?;
            let rate = if self.sample_rate == 48000 { 1 << 6 } else { 1 << 5 };
            if pcm & ((1 << 17) | rate) != ((1 << 17) | rate) { continue; }
            let connections = self.parameter(adc, 0x0e)?;
            if connections & 0x80 != 0 { continue; }
            for index in 0..(connections & 127).min(16) {
                let packed = self.verb(adc, 0xf0200 | (index & !3))?;
                let pin = (packed >> ((index & 3) * 8)) & 255;
                if pin < first || pin >= end || (self.parameter(pin, 9)? >> 20) & 15 != 4 { continue; }
                if self.parameter(pin, 0x0c)? & 0x20 == 0 { continue; }
                self.verb(adc, 0x70500)?;
                self.verb(adc, 0x70100 | index)?;
                if caps & 2 != 0 {
                    let amp = self.parameter(if caps & 8 != 0 { adc } else { group }, 0x0d)?;
                    self.verb(adc, 0x37000 | (index << 8) | (amp & 127))?;
                }
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
        if self.adc == 0 { return Err(Error::Unsupported); }
        let r = 0x80;
        self.w8(r, 1); self.wait(r, 1, 1)?;
        self.w8(r, 0); self.wait(r, 1, 0)?;
        for i in 0..SAMPLES { write_volatile(core::ptr::addr_of_mut!((*self.dma).capture[i]), 0); }
        let address = core::ptr::addr_of_mut!((*self.dma).capture) as u64;
        let bdl = core::ptr::addr_of_mut!((*self.dma).descriptors[8]) as u64;
        write_volatile(core::ptr::addr_of_mut!((*self.dma).descriptors[8]), [address, SAMPLES as u64]);
        write_volatile(core::ptr::addr_of_mut!((*self.dma).descriptors[9]), [address + SAMPLES as u64, SAMPLES as u64]);
        fence(Ordering::SeqCst);
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("dsb sy", options(nostack));
        self.w32(r + 8, (SAMPLES * 2) as u32); self.w16(r + 0x0c, 1);
        self.w16(r + 0x12, if self.sample_rate == 48000 { 0x11 } else { 0x4011 });
        self.w32(r + 0x18, bdl as u32); self.w32(r + 0x1c, (bdl >> 32) as u32);
        self.verb(self.adc, if self.sample_rate == 48000 { 0x20011 } else { 0x24011 })?; self.verb(self.adc, 0x70620)?;
        // Connection selection is fixed during discovery; enable only this pin.
        let index = self.verb(self.adc, 0xf0100)? & 127;
        let connections = self.verb(self.adc, 0xf0200 | (index & !3))?;
        let pin = (connections >> ((index & 3) * 8)) & 127;
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
        let position = (self.r32(0x84) as usize / 4 * 2) % SAMPLES;
        fence(Ordering::Acquire);
        let count = ((position + SAMPLES - self.capture_position) % SAMPLES).min(out.len() & !1);
        for (i, sample) in out[..count].iter_mut().enumerate() {
            *sample = read_volatile(core::ptr::addr_of!((*self.dma).capture[(self.capture_position + i) % SAMPLES]));
        }
        self.capture_position = (self.capture_position + count) % SAMPLES;
        Ok(count)
    }
    // ------------------------=
    // FUNC: stop_capture
    // DESC: Stops capture DMA and unroutes its ADC immediately.
    // ------------------=
    pub unsafe fn stop_capture(&mut self) {
        self.w8(0x80, 0);
        if self.adc != 0 { let _ = self.verb(self.adc, 0x70600); }
        self.capturing = false;
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
