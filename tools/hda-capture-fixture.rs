//! Host-only MMIO fixture for the actual native capture cursor; never part of the OS image.
use super::*;

// ------------------------=
// FUNC: verify_discard_boundary
// DESC: Exercises actual HDA register reads and DMA PCM across discard, wrap, partial frames and faults without changing stream registers.
// ------------------=
pub fn verify_discard_boundary() {
    let mut registers = [0u32; 128];
    registers[0x80 / 4] = (2 << 20) | 2;
    registers[0x100 / 4] = (1 << 20) | 2;
    let mut dma = Box::new(Dma::new());
    for (index, sample) in dma.capture.iter_mut().enumerate() { *sample = (index % 32768) as i16; }
    let mut device = Hda {
        base: registers.as_mut_ptr() as usize, dma: &mut *dma,
        output: 0x100, codec: 0, dac: 0, adc: 0, input_route: None, input_group: 0,
        command_write: Cell::new(0), response_read: Cell::new(0),
        capture_position: 0, capturing: true, codec_id: 0, sample_rate: 48000, playing: true,
        resident: Cell::new(None), resident_failed: Cell::new(false),
    };
    unsafe {
        // Twenty-three bytes contain five complete stereo frames, not six.
        registers[0x84 / 4] = 23;
        let before = registers;
        assert_eq!(device.discard_capture_pending(), Ok(10));
        assert_eq!(registers, before);
        assert_eq!(device.capture_position, 10);
        let mut next = [0i16; 4];
        assert_eq!(device.read_capture(&mut next), Ok(0));
        registers[0x84 / 4] = 31;
        assert_eq!(device.read_capture(&mut next), Ok(4));
        assert_eq!(next, [10, 11, 12, 13]);

        device.capture_position = CAPTURE_SAMPLES - 4;
        registers[0x84 / 4] = 8;
        let before = registers;
        assert_eq!(device.discard_capture_pending(), Ok(8));
        assert_eq!(device.capture_position, 4);
        assert_eq!(registers, before);
        registers[0x84 / 4] = 16;
        assert_eq!(device.read_capture(&mut next), Ok(4));
        assert_eq!(next, [4, 5, 6, 7]);

        registers[0x80 / 4] |= 0x18 << 24;
        assert_eq!(device.discard_capture_pending(), Err(Error::Dma));
        assert_eq!(device.capture_position, 8);
        registers[0x80 / 4] &= !(0x18 << 24);
        registers[0x84 / 4] = (CAPTURE_SAMPLES * 2) as u32;
        assert_eq!(device.discard_capture_pending(), Err(Error::Dma));
        assert_eq!(device.capture_position, 8);
        assert!(device.capturing && device.playing);
        assert_eq!(registers[0x80 / 4], (2 << 20) | 2);
        assert_eq!(registers[0x100 / 4], (1 << 20) | 2);
        device.capturing = false;
        assert_eq!(device.discard_capture_pending(), Err(Error::Invalid));
    }
}
