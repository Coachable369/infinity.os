//! Exact older metadata geometry for behavioral migration fixtures; never used on guest or user disks.
use crate::storage::{BlockDevice,object::{crc32,BANK_A,BANK_B}};
// ------------------------=
// FUNC: number
// DESC: Reads one fixed little-endian fixture field.
// ------------------=
fn number(bytes:&[u8],at:usize)->u32{u32::from_le_bytes(bytes[at..at+4].try_into().unwrap())}
// ------------------------=
// FUNC: read
// DESC: Requires a complete fixture sector without modifying its underlying disk.
// ------------------=
fn read<D:BlockDevice>(disk:&mut D,lba:u64)->[u8;512]{let mut out=[0;512];assert!(disk.read_sector(lba,&mut out));out}
// ------------------------=
// FUNC: checksum
// DESC: Recomputes the native sector checksum after an intentional fixture-format conversion.
// ------------------=
fn checksum(bytes:&mut[u8;512]){bytes[508..].fill(0);let value=crc32(bytes);bytes[508..].copy_from_slice(&value.to_le_bytes());}
// ------------------------=
// FUNC: table_sector
// DESC: Maps a v10 logical table sector into the current fixture's real on-disk geometry.
// ------------------=
pub fn table_sector(header:&[u8;512],store:u64,bank:u64,logical:u64)->u64{
    let (objects,versions,names)=if number(header,8)>=11{(u64::from(number(header,56))/4,u64::from(number(header,60))/8,u64::from(number(header,64))/4)}else{(32,16,32)};
    let offset=if logical<32{logical}else if logical<48{objects+logical-32}else{objects+versions+logical-48};
    store+80+u64::from(number(header,40))*8+if bank==BANK_A{0}else{objects+versions+names}+offset
}
// ------------------------=
// FUNC: downgrade_v10
// DESC: Materializes genuine 128-record v10 banks and allocation reservations from a bounded fresh fixture, preserving both roots and all live records.
// ------------------=
pub fn downgrade_v10<D:BlockDevice>(disk:&mut D,store:u64){
    let mut writes=Vec::new();
    for bank in [BANK_A,BANK_B]{
        let mut header=read(disk,store+bank);if &header[..8]!=b"INFOSTAT"{continue;}
        assert_eq!(number(&header,8),11);assert_eq!(number(&header,48),u32::MAX);
        let extent=number(&header,40);let blocks=number(&header,44);
        for(kind,count,offset,per,stride)in [(0,number(&header,56),0,4,120),(1,number(&header,60),number(&header,56)/4,8,60),(2,number(&header,64),number(&header,56)/4+number(&header,60)/8,4,120)]{
            let base=store+80+u64::from(extent)*8+if bank==BANK_A{0}else{u64::from(number(&header,56)/4+number(&header,60)/8+number(&header,64)/4)};
            for sector in 128/per..count/per{let bytes=read(disk,base+u64::from(offset+sector));for row in 0..per{assert_eq!(bytes[16+(row*stride)as usize],0,"v10 fixture must fit all tables: {kind}");}}
        }
        for logical in 0..80{let mut bytes=read(disk,table_sector(&header,store,bank,logical));bytes[8..12].copy_from_slice(&10u32.to_le_bytes());checksum(&mut bytes);writes.push((store+80+u64::from(extent)*8+if bank==BANK_A{0}else{80}+logical,bytes));}
        let allocation_bytes=number(&header,36)as usize;
        for sector in 0..allocation_bytes.div_ceil(492){
            let lba=if sector<4{store+bank+1+sector as u64}else{store+80+15744*8+if bank==BANK_A{0}else{130}+(sector-4)as u64};
            let mut bytes=read(disk,lba);assert_eq!(&bytes[..8],b"INFOALC2");
            for block in extent+20..extent+blocks{let byte=block as usize/8;if byte/492==sector{bytes[16+byte%492]&=!(1<<(block%8));}}
            bytes[8..12].copy_from_slice(&10u32.to_le_bytes());checksum(&mut bytes);writes.push((lba,bytes));
        }
        header[8..12].copy_from_slice(&10u32.to_le_bytes());header[44..68].fill(0);checksum(&mut header);writes.push((store+bank,header));
        for sector in 23..25{let mut bytes=read(disk,store+bank+sector);bytes[8..12].copy_from_slice(&10u32.to_le_bytes());checksum(&mut bytes);writes.push((store+bank+sector,bytes));}
    }
    for root in 0..2{let mut bytes=read(disk,store+root);if &bytes[..8]!=b"INFOROOT"{continue;}bytes[8..12].copy_from_slice(&10u32.to_le_bytes());checksum(&mut bytes);writes.push((store+root,bytes));}
    for(lba,bytes)in writes{assert!(disk.write_sector(lba,&bytes));}assert!(disk.flush());
}
