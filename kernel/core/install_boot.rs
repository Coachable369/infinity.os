//! Registers the exact installed ESP with the UEFI boot manager.

const GLOBAL: [u8; 16] = [
    0x61, 0xdf, 0xe4, 0x8b, 0xca, 0x93, 0xd2, 0x11, 0xaa, 0x0d, 0, 0xe0, 0x98, 3, 0x2b, 0x8c,
];
const ORDER: &[u16] = &[66, 111, 111, 116, 79, 114, 100, 101, 114, 0];
const NEXT: &[u16] = &[66, 111, 111, 116, 78, 101, 120, 116, 0];
const NOT_FOUND: usize = (1usize << (usize::BITS - 1)) | 14;

// ------------------------=
// FUNC: option
// DESC: Encodes an active load option with a GPT hard-drive short path and architecture-specific fallback loader.
// ------------------=
fn option(uuid: [u8; 16], first: u64, last: u64, arm: bool) -> Option<([u8; 256], usize)> {
    let sectors = last.checked_sub(first)?.checked_add(1)?;
    let mut bytes = [0; 256];
    bytes[0] = 1;
    let mut at = 6;
    for c in b"InfinityOS" {
        bytes[at] = *c;
        at += 2;
    }
    at += 2;
    let path_start = at;
    bytes[at..at + 4].copy_from_slice(&[4, 1, 42, 0]);
    bytes[at + 4..at + 8].copy_from_slice(&1u32.to_le_bytes());
    bytes[at + 8..at + 16].copy_from_slice(&first.to_le_bytes());
    bytes[at + 16..at + 24].copy_from_slice(&sectors.to_le_bytes());
    bytes[at + 24..at + 40].copy_from_slice(&uuid);
    bytes[at + 40] = 2; // GPT
    bytes[at + 41] = 2; // GUID signature
    at += 42;
    let path: &[u8] = if arm {
        b"\\EFI\\BOOT\\BOOTAA64.EFI"
    } else {
        b"\\EFI\\BOOT\\BOOTX64.EFI"
    };
    let length = 4 + (path.len() + 1) * 2;
    bytes[at..at + 2].copy_from_slice(&[4, 4]);
    bytes[at + 2..at + 4].copy_from_slice(&(length as u16).to_le_bytes());
    for (i, c) in path.iter().enumerate() {
        bytes[at + 4 + i * 2] = *c;
    }
    at += length;
    bytes[at..at + 4].copy_from_slice(&[0x7f, 0xff, 4, 0]);
    at += 4;
    bytes[4..6].copy_from_slice(&((at - path_start) as u16).to_le_bytes());
    Some((bytes, at))
}

// ------------------------=
// FUNC: register
// DESC: Adds a free boot option, preserves existing order, and requests the installed target for the next reboot.
// ------------------=
fn register(
    mut get: impl FnMut(&[u16], &mut [u8]) -> Result<usize, usize>,
    mut set: impl FnMut(&[u16], &[u8]) -> bool,
    uuid: [u8; 16],
    first: u64,
    last: u64,
    arm: bool,
) -> bool {
    let Some((data, length)) = option(uuid, first, last, arm) else {
        return false;
    };
    let mut order = [0u8; 514];
    let count = match get(ORDER, &mut order[2..]) {
        Ok(n) if n <= 512 && n % 2 == 0 => n,
        Err(NOT_FOUND) => 0,
        _ => return false,
    };
    let mut name = [66, 111, 111, 116, 48, 48, 48, 48, 0];
    for id in 0..256u16 {
        let hex = b"0123456789ABCDEF";
        name[6] = hex[(id >> 4) as usize] as u16;
        name[7] = hex[(id & 15) as usize] as u16;
        let mut existing = [0; 256];
        match get(&name, &mut existing) {
            Err(NOT_FOUND) => {
                if order[2..2 + count]
                    .chunks_exact(2)
                    .any(|v| u16::from_le_bytes([v[0], v[1]]) == id)
                {
                    continue;
                }
                order[..2].copy_from_slice(&id.to_le_bytes());
                return set(&name, &data[..length])
                    && set(ORDER, &order[..count + 2])
                    && set(NEXT, &id.to_le_bytes());
            }
            Ok(_) => {}
            // A larger occupied entry is still occupied; other firmware failures stop registration.
            Err(e) if e == ((1usize << (usize::BITS - 1)) | 5) => {}
            _ => return false,
        }
    }
    false
}

// ------------------------=
// FUNC: prepare
// DESC: Calls UEFI variable services only after successful installation, using runtime-table ABI offsets.
// ------------------=
pub fn prepare(runtime: u64, uuid: [u8; 16], first: u64, last: u64) -> bool {
    if runtime == 0 {
        return false;
    }
    type Get =
        unsafe extern "efiapi" fn(*const u16, *const u8, *mut u32, *mut usize, *mut u8) -> usize;
    type Set = unsafe extern "efiapi" fn(*const u16, *const u8, u32, usize, *const u8) -> usize;
    // SAFETY: the validated boot ABI supplies the live UEFI runtime table.
    // EFI_TABLE_HEADER is 24 bytes; GetVariable and SetVariable are slots 6 and 8.
    unsafe {
        let table = (runtime as usize + 24) as *const usize;
        let get: Get = core::mem::transmute(*table.add(6));
        let set: Set = core::mem::transmute(*table.add(8));
        register(
            |name, data| {
                let mut size = data.len();
                let status = get(
                    name.as_ptr(),
                    GLOBAL.as_ptr(),
                    core::ptr::null_mut(),
                    &mut size,
                    data.as_mut_ptr(),
                );
                if status == 0 {
                    Ok(size)
                } else {
                    Err(status)
                }
            },
            |name, data| set(name.as_ptr(), GLOBAL.as_ptr(), 7, data.len(), data.as_ptr()) == 0,
            uuid,
            first,
            last,
            cfg!(target_arch = "aarch64"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: exact_partition_and_preserved_order
    // DESC: Verifies binary device paths, existing-option preservation and one-shot selection.
    // ------------------=
    #[test]
    fn exact_partition_and_preserved_order() {
        for arm in [false, true] {
            let mut writes = Vec::new();
            assert!(register(
                |name, data| {
                    if name == ORDER {
                        data[..2].copy_from_slice(&7u16.to_le_bytes());
                        Ok(2)
                    } else {
                        Err(NOT_FOUND)
                    }
                },
                |name, data| {
                    writes.push((name.to_vec(), data.to_vec()));
                    true
                },
                [9; 16],
                2048,
                4095,
                arm
            ));
            assert_eq!(writes.len(), 3);
            let data = &writes[0].1;
            assert_eq!(&data[28..32], &[4, 1, 42, 0]);
            assert_eq!(&data[52..68], &[9; 16]);
            assert_eq!(u64::from_le_bytes(data[44..52].try_into().unwrap()), 2048);
            assert_eq!(writes[1].1, [0, 0, 7, 0]);
            assert_eq!(writes[2].0, NEXT);
            assert_eq!(writes[2].1, [0, 0]);
        }
    }
    // ------------------------=
    // FUNC: failures_do_not_request_reboot
    // DESC: Ensures malformed geometry and firmware write failure cannot report a successful handoff.
    // ------------------=
    #[test]
    fn failures_do_not_request_reboot() {
        assert!(!register(
            |_, _| Err(NOT_FOUND),
            |_, _| panic!(),
            [0; 16],
            9,
            8,
            true
        ));
        assert!(!register(
            |_, _| Err(NOT_FOUND),
            |_, _| false,
            [0; 16],
            1,
            8,
            true
        ));
    }
}

/// Detected GOP dimensions and the user's persistent next-boot selection.
#[derive(Clone,Copy)]
pub struct DisplayModes {
    modes:[[u32;2];32], count:usize, selected:[u32;2], active:[u32;2], failed:bool,
}
impl DisplayModes {
    const GUID:[u8;16]=[0x44,0x53,0x4f,0x49,0x50,0x47,0x4d,0x4f,0x97,0x41,0x62,0x11,0x60,0x24,0x85,0x19];
    const MODES:[u16;9]=[73,79,83,77,111,100,101,115,0];
    const PREF:[u16;11]=[73,79,83,68,105,115,112,108,97,121,0];
    // ------------------------=
    // FUNC: empty
    // DESC: Starts with no firmware claims or pending changes.
    // ------------------=
    pub const fn empty()->Self{Self{modes:[[0;2];32],count:0,selected:[0;2],active:[0;2],failed:false}}
    // ------------------------=
    // FUNC: load
    // DESC: Reads only the validated mode list exported by the current bootloader.
    // ------------------=
    pub fn load(runtime:u64,active:[u32;2])->Self {
        let mut s=Self::empty();s.active=active;s.selected=active;
        if runtime==0{return s;}
        type Get=unsafe extern "efiapi" fn(*const u16,*const u8,*mut u32,*mut usize,*mut u8)->usize;
        unsafe{
            let ptr=*((runtime as usize+24)as *const usize).add(6);
            if ptr==0{return s;}let get:Get=core::mem::transmute(ptr);
            let mut size=core::mem::size_of_val(&s.modes);
            if get(Self::MODES.as_ptr(),Self::GUID.as_ptr(),core::ptr::null_mut(),&mut size,s.modes.as_mut_ptr().cast())!=0 || size>256 || size%8!=0{return s;}
            s.count=size/8;s.modes[..s.count].sort_unstable();
            let mut pref=[0u32;2];size=8;
            if get(Self::PREF.as_ptr(),Self::GUID.as_ptr(),core::ptr::null_mut(),&mut size,pref.as_mut_ptr().cast())==0 && size==8 && s.modes[..s.count].contains(&pref){s.selected=pref;}
        }s
    }
    // ------------------------=
    // FUNC: cycle
    // DESC: Saves a detected mode, updating the displayed preference only on a successful firmware write.
    // ------------------=
    pub fn cycle(&mut self,runtime:u64) {
        if runtime==0||self.count<2{return;}
        let i=self.modes[..self.count].iter().position(|m|*m==self.selected).map(|n|(n+1)%self.count).unwrap_or(0);
        self.select(runtime, i);
    }
    // ------------------------=
    // FUNC: detected
    // DESC: Exposes only firmware-reported resolutions for the Settings list.
    // ------------------=
    pub fn detected(&self) -> &[[u32; 2]] { &self.modes[..self.count] }
    // ------------------------=
    // FUNC: applied
    // DESC: Records confirmed live scanout dimensions independently of firmware preference storage.
    // ------------------=
    pub fn applied(&mut self,w:u32,h:u32){self.active=[w,h];self.failed=false;}
    // ------------------------=
    // FUNC: rejected
    // DESC: Keeps the active scanout visible when the adapter rejects a live request.
    // ------------------=
    pub fn rejected(&mut self){self.failed=true;}
    // ------------------------=
    // FUNC: selected
    // DESC: Returns the dimensions currently driving the desktop.
    // ------------------=
    pub fn selected(&self) -> [u32; 2] { self.active }
    // ------------------------=
    // FUNC: select
    // DESC: Commits the explicitly chosen mode without cycling through intermediate resolutions.
    // ------------------=
    pub fn select(&mut self, runtime: u64, i: usize) {
        if runtime == 0 || i >= self.count || self.modes[i] == self.selected {return;}
        type Set=unsafe extern "efiapi" fn(*const u16,*const u8,u32,usize,*const u8)->usize;
        let ok=unsafe {let p=*((runtime as usize+24)as *const usize).add(8);p!=0 && core::mem::transmute::<usize,Set>(p)(Self::PREF.as_ptr(),Self::GUID.as_ptr(),7,8,self.modes[i].as_ptr().cast())==0};
        self.failed=!ok;if ok{self.selected=self.modes[i];}
    }
    // ------------------------=
    // FUNC: label
    // DESC: Reports the actual scanout and whether the last change succeeded.
    // ------------------=
    pub fn label(&self,out:&mut[u8;48])->usize {
        let mut n=0;
        for v in self.active{
            if n!=0{for b in b" x "{out[n]=*b;n+=1;}}
            let mut d=[0u8;10];let mut at=10;let mut x=v;loop{at-=1;d[at]=b'0'+(x%10)as u8;x/=10;if x==0{break;}}
            out[n..n+10-at].copy_from_slice(&d[at..]);n+=10-at;
        }
        let suffix:&[u8]=if self.failed{b" (change failed)"}else if self.count<2{b" (only mode)"}else{b""};
        out[n..n+suffix.len()].copy_from_slice(suffix);n+suffix.len()
    }
}

#[cfg(test)]
mod display_mode_tests {
    use super::DisplayModes as Modes;
    use std::sync::Mutex;
    static PREFERENCE: Mutex<[u32;2]> = Mutex::new([0;2]);
    // ------------------------=
    // FUNC: get
    // DESC: Supplies detected dimensions and the persistent preference through the firmware ABI.
    // ------------------=
    unsafe extern "efiapi" fn get(name:*const u16,guid:*const u8,_:*mut u32,size:*mut usize,out:*mut u8)->usize {
        if core::slice::from_raw_parts(guid,16)!=Modes::GUID {return 1;}
        let pref=*PREFERENCE.lock().unwrap();
        let data:&[u32]=if *name.add(3)==77 {&[1920,1080,1280,720]}else{&pref};
        let n=core::mem::size_of_val(data);if *size<n{return 1;}
        core::ptr::copy_nonoverlapping(data.as_ptr().cast(),out,n);*size=n;0
    }
    // ------------------------=
    // FUNC: set
    // DESC: Emulates a firmware that accepts one saved preference and rejects its replacement.
    // ------------------=
    unsafe extern "efiapi" fn set(name:*const u16,guid:*const u8,attr:u32,size:usize,data:*const u8)->usize {
        if core::slice::from_raw_parts(name,11)!=Modes::PREF || core::slice::from_raw_parts(guid,16)!=Modes::GUID || attr!=7 || size!=8{return 1;}
        let mut pref=PREFERENCE.lock().unwrap();if *pref!=[0;2]{return 1;}
        *pref=(data as *const [u32;2]).read_unaligned();0
    }
    // ------------------------=
    // FUNC: selection_survives_reload_and_failed_write
    // DESC: Exercises discovery, preference persistence, restart activation and rejection without changing the active scanout.
    // ------------------=
    #[test]
    fn selection_survives_reload_and_failed_write(){
        let mut table=[0usize;12];table[9]=get as *const () as usize;table[11]=set as *const () as usize;
        let rt=table.as_ptr() as u64;let mut modes=Modes::load(rt,[1920,1080]);
        assert_eq!(modes.count,2);modes.cycle(rt);
        assert_eq!(modes.selected,[1280,720]);assert_eq!(modes.active,[1920,1080]);
        assert_eq!(modes.selected(),[1920,1080]);modes.applied(1280,720);assert_eq!(modes.selected(),[1280,720]);
        modes.rejected();assert!(modes.failed);assert_eq!(modes.selected(),[1280,720]);
        let mut restarted=Modes::load(rt,[1280,720]);assert_eq!(restarted.selected,restarted.active);
        restarted.cycle(rt);assert!(restarted.failed);assert_eq!(restarted.selected,[1280,720]);
    }
}
