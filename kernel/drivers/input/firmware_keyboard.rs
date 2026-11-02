//! SimpleTextInputEx current-state polling, including modifier-only and idle reads.
pub const NOT_READY: u64 = 0x8000_0000_0000_0006;
pub const UNSUPPORTED: u64 = 0x8000_0000_0000_0003;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct InputKey { pub scan_code: u16, pub unicode_character: u16 }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct KeyData { pub key: InputKey, pub shift_state: u32, pub toggle_state: u8 }

#[repr(C)]
pub struct SimpleTextInputEx {
    pub reset: usize,
    pub read_key_stroke: unsafe extern "efiapi" fn(*mut SimpleTextInputEx,*mut KeyData)->u64,
    pub wait_for_key: usize,
    pub set_state: Option<unsafe extern "efiapi" fn(*mut SimpleTextInputEx,*mut u8)->u64>,
    pub register_notify: usize,
    pub unregister_notify: usize,
}

pub struct Exposure { source: usize, attempted: bool, pub enabled: bool }
impl Exposure {
    // ------------------------=
    // FUNC: new
    // DESC: Starts negotiation without assuming firmware supports partial-key reporting.
    // ------------------=
    pub const fn new() -> Self {Self {source:0,attempted:false,enabled:false}}
}

pub struct Sample { pub status: u64, pub data: KeyData, pub modifiers: u8 }

// ------------------------=
// FUNC: poll
// DESC: Samples valid current state on success or NOT_READY and enables partial reporting without changing lock toggles.
// ------------------=
pub unsafe fn poll(protocol:*mut SimpleTextInputEx,exposure:&mut Exposure)->Sample {
    if exposure.source!=protocol as usize { *exposure=Exposure::new();exposure.source=protocol as usize; }
    let mut data=KeyData::default();
    let status=if protocol.is_null() {UNSUPPORTED}else{((*protocol).read_key_stroke)(protocol,&mut data)};
    let current=status==0 || status==NOT_READY;
    if current && data.toggle_state&0x80!=0 && !exposure.attempted {
        exposure.attempted=true;
        exposure.enabled=data.toggle_state&0x40!=0;
        if !exposure.enabled {
            if let Some(set)=(*protocol).set_state {
                let mut toggles=data.toggle_state|0x40;
                exposure.enabled=set(protocol,&mut toggles)==0;
            }
        }
    }
    let modifiers=if current && data.shift_state&0x8000_0000!=0 {
        u8::from(data.shift_state&3!=0) | (u8::from(data.shift_state&12!=0)<<1)
            | (u8::from(data.shift_state&0xc0!=0)<<2) | (u8::from(data.shift_state&0x30!=0)<<3)
    }else{0};
    Sample {status,data,modifiers}
}
