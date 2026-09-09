#![allow(dead_code)]
mod storage;
#[path = "../kernel/drivers/input/buffer.rs"] mod input_buffer;
#[path = "../kernel/storage/fabric.rs"] mod native_fabric;
#[path = "../kernel/storage/install_identity.rs"] mod install_identity;
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/runtime/mod.rs"] mod runtime;
use std::{fs::File, io::{Read, Seek, SeekFrom}};
use storage::{BlockDevice, object::{ObjectStore, crc32}};
#[path="installed-pool-lifecycle.rs"] mod lifecycle;
// ------------------------=
// FUNC: output_text
// DESC: Suppresses runtime human diagnostics from the structured verifier output.
// ------------------=
fn output_text(_: &[u8]) {}
struct FileDisk { file: File, sectors: u64 }
impl BlockDevice for FileDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Reports actual raw artifact capacity.
    // ------------------=
    fn block_count(&self) -> u64 { self.sectors }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one bounded sector through a read-only host handle.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8;512]) -> bool {
        lba < self.sectors && self.file.seek(SeekFrom::Start(lba*512)).is_ok() && self.file.read_exact(out).is_ok()
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Rejects every mutation, including accidental native catalog creation.
    // ------------------=
    fn write_sector(&mut self, _: u64, _: &[u8;512]) -> bool { false }
    // ------------------------=
    // FUNC: flush
    // DESC: Satisfies the adapter without writing any bytes.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}
// ------------------------=
// FUNC: number
// DESC: Decodes a bounded metadata integer.
// ------------------=
fn number(b: &[u8], at: usize) -> u64 { u64::from_le_bytes(b[at..at+8].try_into().unwrap()) }
// ------------------------=
// FUNC: container
// DESC: Locates the installer-native GPT partition and validates its checksummed container and Pool metadata before native mount.
// ------------------=
fn container<D: BlockDevice>(disk: &mut D) -> Result<u64, &'static str> {
    let mut h = [0;512];
    if !disk.read_sector(1,&mut h) || &h[..8] != b"EFI PART" { return Err("gpt_missing"); }
    let size = u32::from_le_bytes(h[12..16].try_into().unwrap()) as usize;
    if !(92..=512).contains(&size) { return Err("gpt_invalid"); }
    let sum = u32::from_le_bytes(h[16..20].try_into().unwrap()); h[16..20].fill(0);
    if crc32(&h[..size]) != sum { return Err("gpt_checksum"); }
    let entries = number(&h,72);
    let count = u32::from_le_bytes(h[80..84].try_into().unwrap()) as usize;
    if count > 128 || h[84..88] != 128u32.to_le_bytes() { return Err("gpt_bounds"); }
    let mut table = [0;16384];
    for i in 0..(count*128).div_ceil(512) {
        let mut s = [0;512]; if !disk.read_sector(entries+i as u64,&mut s) { return Err("read_failed"); }
        table[i*512..i*512+512].copy_from_slice(&s);
    }
    if crc32(&table[..count*128]) != u32::from_le_bytes(h[88..92].try_into().unwrap()) { return Err("partition_checksum"); }
    let mut found = None;
    for p in table[..count*128].chunks_exact(128) {
        if &p[..16] != b"ifnIinytSTORAGE1" { continue; }
        let first = number(p,32); let last = number(p,40);
        if first == 0 || last <= first || last >= disk.block_count() || found.is_some() { return Err("partition_invalid"); }
        for (offset,magic) in [(0,b"INFCONT1"),(1,b"INFPOOL1")] {
            let mut s = [0;512]; if !disk.read_sector(first+offset,&mut s) { return Err("read_failed"); }
            let sum = u32::from_le_bytes(s[508..].try_into().unwrap()); s[508..].fill(0);
            if &s[..8] != magic || crc32(&s) != sum { return Err("native_metadata_invalid"); }
        }
        found = Some(first);
    }
    found.ok_or("native_partition_missing")
}
// ------------------------=
// FUNC: hex
// DESC: Parses exact immutable identities without truncation.
// ------------------=
fn hex<const N: usize>(s: &str) -> Result<[u8;N], &'static str> {
    if s.len()!=N*2 || !s.is_ascii() { return Err("invalid_hex"); }
    let mut b=[0;N]; for (i,v) in b.iter_mut().enumerate() { *v=u8::from_str_radix(&s[i*2..i*2+2],16).map_err(|_|"invalid_hex")?; } Ok(b)
}
// ------------------------=
// FUNC: run
// DESC: Mounts stopped-VM raw disk read-only and verifies all persisted recipient bytes against caller's immutable expectation.
// ------------------=
fn run() -> Result<String,String> {
    let a: Vec<String> = std::env::args().collect();
    if a.get(1).is_some_and(|s|s=="--lifecycle") {
        if a.len()<6 || a[2]!="--vm-paused" {return Err("usage: installed-pool-verify --lifecycle --vm-paused RAW OWNER_HEX OBJECT_HEX [CONTENT_ID_HEX...]".into())}
        let owner=hex(&a[4])?;let object=hex(&a[5])?;
        let content=a[6..].iter().map(|s|hex(s)).collect::<Result<Vec<[u8;16]>,_>>()?;
        if content.len()>64{return Err("content_limit".into())}
        let file=File::open(&a[3]).map_err(|_|"open_failed")?;let length=file.metadata().map_err(|_|"metadata_failed")?.len();
        if length%512!=0{return Err("not_raw_sectors".into())}
        let mut disk=FileDisk{file,sectors:length/512};let start=container(&mut disk)?;
        let mut store=ObjectStore::mount(disk,start).map_err(|_|"native_mount_failed")?;
        return lifecycle::inspect(&mut store,owner,object,&content).map(|r|r.json());
    }
    if a.len()!=6 { return Err("usage: installed-pool-verify RAW OWNER_HEX OBJECT_HEX HASH_HEX VERSION".into()); }
    let owner=hex(&a[2])?; let object=hex(&a[3])?; let hash=hex(&a[4])?;
    let version=a[5].parse().map_err(|_|"invalid_version")?;
    let file=File::open(&a[1]).map_err(|_|"open_failed")?;
    let length=file.metadata().map_err(|_|"metadata_failed")?.len();
    if length%512!=0 { return Err("not_raw_sectors".into()); }
    let mut disk=FileDisk {file,sectors:length/512}; let start=container(&mut disk)?;
    let mut store=ObjectStore::mount(disk,start).map_err(|_|"native_mount_failed")?;
    verify_store(&mut store,owner,object,version,hash)
}
// ------------------------=
// FUNC: verify_store
// DESC: Verifies recipient or authoritative native objects without mutating either catalog.
// ------------------=
fn verify_store<D:BlockDevice>(store:&mut ObjectStore<D>,owner:[u8;32],object:[u8;16],version:u64,hash:[u8;32])->Result<String,String> {
    if let Ok(n) = native_fabric::service::verify_persisted_replica(store,owner,object,version,hash) {
        return Ok(format!("{{\"verified\":true,\"source\":\"recipient\",\"bytes\":{n}}}"));
    }
    // The explicit resolve prevents a missing catalog from being initialized even on a future writable adapter.
    store.resolve(b"/system/storage/pool-manifests").map_err(|_|"pool_catalog_missing")?;
    let m=store.pool_manifest(storage::object::ObjectId(object),runtime::node::types::NodeId(owner),0).map_err(|_|"manifest_missing")?;
    if m.version!=version || m.hash!=hash { return Err("manifest_mismatch".into()); }
    use sha2::{Digest,Sha256};
    let mut digest=Sha256::new();let mut bytes=[0;64];let mut offset=0;
    while offset<m.length {
        let n=(m.length-offset).min(64) as usize;
        store.pool_read(&m,offset,&mut bytes[..n]).map_err(|_|"content_invalid")?;
        digest.update(&bytes[..n]);offset+=n as u64;
    }
    if <[u8;32]>::from(digest.finalize())!=hash { return Err("content_hash_mismatch".into()); }
    let chunks:Vec<String>=m.chunks.iter().flatten().map(|c| {
        let id:String=c.content.iter().map(|b|format!("{b:02x}")).collect();
        format!("{{\"content\":\"{id}\",\"bytes\":{}}}",c.bytes)
    }).collect();
    Ok(format!("{{\"verified\":true,\"source\":\"authority\",\"bytes\":{},\"chunks\":[{}]}}",offset,chunks.join(",")))
}
// ------------------------=
// FUNC: main
// DESC: Emits machine-readable pass/failure and process status; never modifies the input artifact.
// ------------------=
fn main() { match run() { Ok(result)=>println!("{result}"),Err(e)=>{println!("{{\"verified\":false,\"error\":{e:?}}}");std::process::exit(1);} } }

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(std::collections::BTreeMap<u64,[u8;512]>);
    impl BlockDevice for Fixture {
        // ------------------------=
        // FUNC: block_count
        // DESC: Defines bounded raw fixture geometry.
        // ------------------=
        fn block_count(&self)->u64 { 1_000_000 }
        // ------------------------=
        // FUNC: read_sector
        // DESC: Reads fixture binary metadata.
        // ------------------=
        fn read_sector(&mut self,l:u64,o:&mut[u8;512])->bool { *o=*self.0.get(&l).unwrap_or(&[0;512]);true }
        // ------------------------=
        // FUNC: write_sector
        // DESC: Enforces verification never requests writes.
        // ------------------=
        fn write_sector(&mut self,_:u64,_:&[u8;512])->bool { panic!("read-only") }
        // ------------------------=
        // FUNC: flush
        // DESC: Completes read-only contract.
        // ------------------=
        fn flush(&mut self)->bool { true }
    }
    // ------------------------=
    // FUNC: authority_verification_checks_native_content
    // DESC: Verifies the fallback against a production native object and rejects incorrect expected immutable values.
    // ------------------=
    #[test]
    fn authority_verification_checks_native_content() {
        use sha2::{Digest,Sha256};
        struct Writable(Fixture,std::rc::Rc<std::cell::Cell<bool>>);
        impl BlockDevice for Writable {
            // ------------------------=
            // FUNC: block_count
            // DESC: Exposes fixture geometry.
            // ------------------=
            fn block_count(&self)->u64 { self.0.block_count() }
            // ------------------------=
            // FUNC: read_sector
            // DESC: Reads persisted native fixture bytes.
            // ------------------=
            fn read_sector(&mut self,l:u64,o:&mut[u8;512])->bool { self.0.read_sector(l,o) }
            // ------------------------=
            // FUNC: write_sector
            // DESC: Allows setup through the real native transaction writer only.
            // ------------------=
            fn write_sector(&mut self,l:u64,s:&[u8;512])->bool { assert!(self.1.get());self.0.0.insert(l,*s);true }
            // ------------------------=
            // FUNC: flush
            // DESC: Models successful fixture persistence.
            // ------------------=
            fn flush(&mut self)->bool { true }
        }
        let writable=std::rc::Rc::new(std::cell::Cell::new(true));
        let disk=Writable(Fixture(Default::default()),writable.clone());
        let mut store=ObjectStore::format(disk,0,1_000_000,[7;16]).unwrap();
        store.initialize_pool_catalog().unwrap();
        let owner=runtime::node::types::NodeId([1;32]);
        let bytes=[37;4097];
        let m=store.pool_create(owner,0,1,runtime::fabric::placement::StorageClass::Protected,&bytes,
            owner,runtime::fabric::resources::ResourceId([2;16]),[3;16],1).unwrap();
        let hash=Sha256::digest(bytes).into();
        let generation=store.generation();
        assert!(verify_store(&mut store,owner.0,m.object,m.version,hash).is_ok());
        assert!(verify_store(&mut store,owner.0,m.object,m.version+1,hash).is_err());
        assert!(verify_store(&mut store,owner.0,m.object,m.version,[0;32]).is_err());
        assert_eq!(store.generation(),generation);
        let ids:Vec<[u8;16]>=m.chunks.iter().flatten().map(|c|c.content).collect();
        writable.set(false);
        let before=lifecycle::inspect(&mut store,owner.0,m.object,&ids).unwrap();
        assert!(before.object_present);assert_eq!(before.audit.len(),1);assert_eq!(before.audit[0].1,2);assert!(before.pending.is_none());
        assert_eq!(store.generation(),generation);
        writable.set(true);store.pool_delete(storage::object::ObjectId(m.object),owner,0,m.generation).unwrap();writable.set(false);
        let after=lifecycle::inspect(&mut store,owner.0,m.object,&ids).unwrap();
        assert!(!after.object_present);assert!(after.outbox_present);assert!(after.pending.is_none());assert_eq!(after.audit.last().unwrap().1,6);
        writable.set(true);store.replace_named_state(b"/system/storage/pool-audit",&[0;2080]).unwrap();writable.set(false);
        assert!(lifecycle::inspect(&mut store,owner.0,m.object,&ids).is_err());
    }
    // ------------------------=
    // FUNC: partition_discovery_checks_native_metadata_and_corruption
    // DESC: Exercises exact installer-format binary GPT/container metadata and rejects corruption or absent native partitions.
    // ------------------=
    #[test]
    fn partition_discovery_checks_native_metadata_and_corruption() {
        let mut disk=Fixture(Default::default());
        assert!(container(&mut disk).is_err());
        let mut table=[0;512]; table[128..144].copy_from_slice(b"ifnIinytSTORAGE1");
        table[160..168].copy_from_slice(&4096u64.to_le_bytes());
        table[168..176].copy_from_slice(&999966u64.to_le_bytes());
        let mut h=[0;512];h[..8].copy_from_slice(b"EFI PART");h[12..16].copy_from_slice(&92u32.to_le_bytes());
        h[72..80].copy_from_slice(&2u64.to_le_bytes());h[80..84].copy_from_slice(&4u32.to_le_bytes());
        h[84..88].copy_from_slice(&128u32.to_le_bytes());h[88..92].copy_from_slice(&crc32(&table).to_le_bytes());
        let crc=crc32(&h[..92]);h[16..20].copy_from_slice(&crc.to_le_bytes());disk.0.insert(1,h);disk.0.insert(2,table);
        for (l,magic) in [(4096,b"INFCONT1"),(4097,b"INFPOOL1")] {
            let mut s=[0;512];s[..8].copy_from_slice(magic);let crc=crc32(&s);s[508..].copy_from_slice(&crc.to_le_bytes());disk.0.insert(l,s);
        }
        assert_eq!(container(&mut disk),Ok(4096));
        disk.0.get_mut(&4097).unwrap()[100]^=1;assert!(container(&mut disk).is_err());
        disk.0.get_mut(&4097).unwrap()[100]^=1;
        disk.0.get_mut(&2).unwrap()[160]^=1;assert!(container(&mut disk).is_err());
    }
}
