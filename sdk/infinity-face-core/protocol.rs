//! Private ABI for the OS-selected isolated verification worker. Not a public
//! authentication API; these calls transfer verified enrollment but never create sessions or unlock the desktop.
pub const VERSION:u32=1;
pub const MODEL_READ:u64=0x1270;
pub const CONTROL_READ:u64=0x1271;
pub const REPORT_WRITE:u64=0x1272;
pub const PREVIEW_WRITE:u64=0x1273;
pub const CLOCK_MS:u64=0x1274;
pub const ENROLLMENT_WRITE:u64=0x1275;
pub const TEMPLATE_READ:u64=0x1276;
pub const LOGIN:u32=5;
/// Three little-endian 128-float vectors followed by three aligned 56x56 grayscale portraits.
pub const PORTRAIT_SIDE:usize=56;
pub const ENROLLMENT_BYTES:usize=3*128*4+3*PORTRAIT_SIDE*PORTRAIT_SIDE;
pub const BEGIN:u32=1;
pub const CAPTURE:u32=2;
pub const VERIFY:u32=3;
pub const CANCEL:u32=4;
pub const READY:u32=1;
pub const PROCESSING:u32=2;
pub const REVIEW:u32=3;
pub const VERIFYING:u32=4;
pub const PASSED:u32=5;
pub const FAILED:u32=6;
pub const CANCELLED:u32=7;
pub const REJECTED:u32=8;
pub const PREVIEW_PAUSED:u32=9;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Control {
    pub version:u32,pub command:u32,pub command_id:u64,pub worker:u64,pub lease:u64,pub account:u64,
    pub name_len:u32,pub reserved:u32,pub name:[u8;64],pub model:[u8;32],
}
impl Control {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates a fully initialized control record with no command or capture authority.
    // ------------------=
    pub const fn empty()->Self {Self {version:VERSION,command:0,command_id:0,worker:0,lease:0,account:0,
        name_len:0,reserved:0,name:[0;64],model:[0;32]}}
}
#[repr(C)]
#[derive(Clone,Copy)]
pub struct Report {
    pub version:u32,pub phase:u32,pub error:u32,pub photos:u32,pub comparisons:u32,pub reserved:u32,pub command_id:u64,
}
impl Report {
    // ------------------------=
    // FUNC: new
    // DESC: Constructs a non-authenticating UI progress report with no image, template, score or unlock authority.
    // ------------------=
    pub const fn new(command_id:u64,phase:u32,error:u32,photos:u32,comparisons:u32)->Self {
        Self {version:VERSION,phase,error,photos,comparisons,reserved:0,command_id}
    }
}
