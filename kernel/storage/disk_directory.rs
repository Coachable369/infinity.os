//! Bounded, read-only FAT16/32 directory inspection. Never opens raw write access.
use super::{BlockDevice, ObjectError};

#[derive(Clone, Copy)]
struct Directory {
    cluster: u32,
    path: [u8; 96],
    len: usize,
}
const EMPTY: Directory = Directory {
    cluster: 0,
    path: [0; 96],
    len: 0,
};
// ------------------------=
// FUNC: word
// DESC: Reads a little-endian FAT header word from an already bounded sector.
// ------------------=
fn word(s: &[u8], at: usize) -> u32 {
    u32::from(u16::from_le_bytes([s[at], s[at + 1]]))
}
// ------------------------=
// FUNC: dword
// DESC: Reads a little-endian FAT double word from an already bounded sector.
// ------------------=
fn dword(s: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(s[at..at + 4].try_into().unwrap())
}
// ------------------------=
// FUNC: visit
// DESC: Walks bounded short-name directories and reports file extents within the selected GPT partition only.
// ------------------=
pub(super) fn visit<D: BlockDevice>(
    disk: &mut D,
    first: u64,
    bytes: u64,
    mut file: impl FnMut(&[u8], u64),
) -> Result<(), ObjectError> {
    let sectors = bytes / 512;
    let mut boot = [0; 512];
    if first
        .checked_add(sectors)
        .is_none_or(|end| end > disk.block_count())
        || !disk.read_sector(first, &mut boot)
    {
        return Err(ObjectError::CorruptMetadata);
    }
    if word(&boot, 11) != 512 || boot[510..] != [0x55, 0xaa] {
        return Err(ObjectError::UnsupportedFormat);
    }
    let cluster_size = u64::from(boot[13]);
    let reserved = u64::from(word(&boot, 14));
    let fats = u64::from(boot[16]);
    let root_sectors = (u64::from(word(&boot, 17)) * 32).div_ceil(512);
    let fat32 = root_sectors == 0;
    let fat_size = u64::from(if fat32 {
        dword(&boot, 36)
    } else {
        word(&boot, 22)
    });
    let root = reserved + fats * fat_size;
    let data = root + root_sectors;
    if !cluster_size.is_power_of_two()
        || cluster_size > 128
        || reserved == 0
        || !(1..=2).contains(&fats)
        || fat_size == 0
        || data >= sectors
    {
        return Err(ObjectError::UnsupportedFormat);
    }
    let volume = u64::from(if word(&boot, 19) != 0 {
        word(&boot, 19)
    } else {
        dword(&boot, 32)
    });
    if volume > sectors || data >= volume {
        return Err(ObjectError::CorruptMetadata);
    }
    let clusters = (volume - data) / cluster_size;
    if clusters < 4085 {
        return Err(ObjectError::UnsupportedFormat);
    }
    let mut dirs = [EMPTY; 32];
    let mut tail = 1;
    let mut head = 0;
    dirs[0].cluster = if fat32 { dword(&boot, 44) } else { 0 };
    let mut budget = 2048;
    while head < tail {
        let dir = dirs[head];
        head += 1;
        let mut cluster = dir.cluster;
        'chain: loop {
            let (offset, count) = if cluster == 0 && !fat32 && head == 1 {
                (root, root_sectors)
            } else {
                if cluster < 2 || u64::from(cluster) - 2 >= clusters {
                    return Err(ObjectError::CorruptMetadata);
                }
                (data + (u64::from(cluster) - 2) * cluster_size, cluster_size)
            };
            for n in 0..count {
                if budget == 0 {
                    return Err(ObjectError::InsufficientCapacity);
                }
                budget -= 1;
                let mut sector = [0; 512];
                if !disk.read_sector(first + offset + n, &mut sector) {
                    return Err(ObjectError::CorruptMetadata);
                }
                for entry in sector.chunks_exact(32) {
                    if entry[0] == 0 {
                        break 'chain;
                    }
                    if entry[0] == 0xe5 || entry[0] == b'.' || entry[11] & 8 != 0 {
                        continue;
                    }
                    let mut child = dir;
                    if child.len + 13 >= child.path.len() {
                        continue;
                    }
                    child.path[child.len] = b'/';
                    child.len += 1;
                    for &b in &entry[..8] {
                        if b == b' ' {
                            break;
                        }
                        child.path[child.len] = if b.is_ascii_graphic() { b } else { b'?' };
                        child.len += 1;
                    }
                    if entry[8] != b' ' {
                        child.path[child.len] = b'.';
                        child.len += 1;
                        for &b in &entry[8..11] {
                            if b == b' ' {
                                break;
                            }
                            child.path[child.len] = if b.is_ascii_graphic() { b } else { b'?' };
                            child.len += 1;
                        }
                    }
                    if entry[11] & 16 == 0 {
                        file(&child.path[..child.len], u64::from(dword(entry, 28)));
                        continue;
                    }
                    child.cluster = word(entry, 26) | if fat32 { word(entry, 20) << 16 } else { 0 };
                    if tail < dirs.len()
                        && child.cluster >= 2
                        && !dirs[..tail].iter().any(|d| d.cluster == child.cluster)
                    {
                        dirs[tail] = child;
                        tail += 1;
                    }
                }
            }
            if cluster == 0 {
                break;
            }
            let stride = if fat32 { 4 } else { 2 };
            let index = u64::from(cluster) * stride;
            if index / 512 >= fat_size
                || !disk.read_sector(first + reserved + index / 512, &mut boot)
            {
                return Err(ObjectError::CorruptMetadata);
            }
            cluster = if fat32 {
                dword(&boot, (index % 512) as usize) & 0x0fffffff
            } else {
                word(&boot, (index % 512) as usize)
            };
            if cluster >= if fat32 { 0x0ffffff8 } else { 0xfff8 } {
                break;
            }
        }
    }
    Ok(())
}
