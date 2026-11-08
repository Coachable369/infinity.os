#[path = "../kernel/runtime/ai/qwen/mod.rs"]
mod qwen;
pub trait BlockDevice {
    fn block_count(&self) -> u64;
    fn read_blocks(&mut self, lba: u64, bytes: &mut [u8]) -> bool;
    fn write_blocks(&mut self, lba: u64, bytes: &[u8]) -> bool;
}
#[path = "../kernel/storage/payload.rs"]
mod payload;
struct Disk {
    data: Vec<u8>,
    writes: usize,
    fail: bool,
}
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Exposes mock disk capacity.
    // ------------------=
    fn block_count(&self) -> u64 {
        self.data.len() as u64 / 512
    }
    // ------------------------=
    // FUNC: read_blocks
    // DESC: Exercises payload verification against written bytes.
    // ------------------=
    fn read_blocks(&mut self, lba: u64, bytes: &mut [u8]) -> bool {
        bytes.copy_from_slice(&self.data[lba as usize * 512..lba as usize * 512 + bytes.len()]);
        true
    }
    // ------------------------=
    // FUNC: write_blocks
    // DESC: Injects a write failure or records a bounded payload transfer.
    // ------------------=
    fn write_blocks(&mut self, lba: u64, bytes: &[u8]) -> bool {
        self.writes += 1;
        if self.fail {
            return false;
        }
        self.data[lba as usize * 512..lba as usize * 512 + bytes.len()].copy_from_slice(bytes);
        true
    }
}
// ------------------------=
// FUNC: main
// DESC: Verifies numeric decoding and installation failure semantics using structured values.
// ------------------=
fn main() {
    batched_installation_progress();
    let mut decoded = [0f32; 256];
    let mut q4 = [0u8; 144];
    q4[0..2].copy_from_slice(&0x3c00u16.to_le_bytes());
    q4[4..8].fill(1);
    q4[12..16].fill(1);
    q4[16..].fill(0x21);
    qwen::quant::decode_block(12, &q4, &mut decoded).unwrap();
    for (i, v) in decoded.iter().enumerate() {
        assert_eq!(*v, if i / 32 % 2 == 0 { 1.0 } else { 2.0 });
    }
    let mut q6 = [0u8; 210];
    q6[..128].fill(0x10);
    q6[192..208].fill(1);
    q6[208..].copy_from_slice(&0x3c00u16.to_le_bytes());
    qwen::quant::decode_block(14, &q6, &mut decoded).unwrap();
    for (i, v) in decoded.iter().enumerate() {
        assert_eq!(*v, if i % 128 < 64 { -32.0 } else { -31.0 });
    }
    assert!(qwen::quant::decode_block(12, &q4[..143], &mut decoded).is_err());
    assert!(qwen::gguf::Model::parse(b"GGUF").is_err());
    static SOURCE: [u8; 1030] = [37; 1030];
    let image = payload::Image::embedded(&SOURCE);
    let mut disk = Disk {
        data: vec![0; 4096],
        writes: 0,
        fail: false,
    };
    image.transfer(&mut disk, 1, false, |_| {}).unwrap();
    assert_eq!(&disk.data[512..1542], &SOURCE);
    assert!(disk.data[1542..2048].iter().all(|v| *v == 0));
    image.transfer(&mut disk, 1, true, |_| {}).unwrap();
    disk.data[900] ^= 1;
    assert!(image.transfer(&mut disk, 1, true, |_| {}).is_err());
    disk.fail = true;
    assert!(image.transfer(&mut disk, 1, false, |_| {}).is_err());
    let writes = disk.writes;
    assert!(image.transfer(&mut disk, u64::MAX, false, |_| {}).is_err());
    assert_eq!(writes, disk.writes);
}

// ------------------------=
// FUNC: batched_installation_progress
// DESC: Exercises production transfer callbacks across 128-sector boundaries and a partial final sector, including the retained x86 ESP geometry.
// ------------------=
fn batched_installation_progress(){
    static SOURCE:[u8;65536*3+7]=[37;65536*3+7];
    let image=payload::Image::embedded(&SOURCE);
    let mut disk=Disk{data:vec![0;(SOURCE.len().div_ceil(512)+1)*512],writes:0,fail:false};
    for verify in [false,true]{
        let mut progress=payload::TransferProgress::new(SOURCE.len(),67,73);
        let mut callbacks=Vec::new();let mut reported=Vec::new();
        image.transfer(&mut disk,1,verify,|sectors|{callbacks.push(sectors);if let Some(value)=progress.advance(sectors){reported.push(value);}}).unwrap();
        assert_eq!(callbacks,[128,256,384,385]);
        assert_eq!(reported,[(68,128),(70,256),(72,384),(73,385)]);
        assert_eq!(progress.advance(385),None);assert_eq!(progress.advance(128),None);
    }
    let bytes=5_771_362_304usize;let mut progress=payload::TransferProgress::new(bytes,67,73);let mut reported=Vec::new();
    for completed in (128..=bytes/512).step_by(128){if let Some((percent,_))=progress.advance(completed){reported.push(percent);}}
    assert_eq!(reported,[68,69,70,71,72,73]);
    disk.fail=true;let mut callbacks=0;
    assert!(image.transfer(&mut disk,1,false,|_|callbacks+=1).is_err());assert_eq!(callbacks,0);
}
