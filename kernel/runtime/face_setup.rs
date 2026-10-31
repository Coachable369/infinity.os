//! Setup-only coordinator with bounded verified enrollment transfer. A saved
//! enrollment cannot create a session, enable login, or unlock a window.
#[path = "../../sdk/infinity-face-core/protocol.rs"]
pub mod wire;

pub const PREVIEW_WIDTH: usize = 160;
pub const PREVIEW_HEIGHT: usize = 120;
pub const PREVIEW_BYTES: usize = PREVIEW_WIDTH * PREVIEW_HEIGHT * 3;
pub const PHOTO_WIDTH: usize = 80;
pub const PHOTO_HEIGHT: usize = 60;
const SESSION_MS: u64 = 240_000;
const COMMAND_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Idle, Loading, Photos(u8), Review, Verifying(u8), Passed, Failed(u32), Cancelled }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Invalid, Busy, WrongStage, Expired, Stale, Protocol }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotoStatus { Pending, Checking, Accepted }

/// Owned by the trusted setup controller, not a user-selected executable.
pub struct Setup {
    control: wire::Control,
    stage: Stage,
    pending: bool,
    started: u64,
    deadline: u64,
    command_deadline: u64,
    preview: [u8; PREVIEW_BYTES],
    staging: [u8; PREVIEW_BYTES],
    preview_revision: u64,
    preview_written: usize,
    preview_ready: bool,
    enrollment: [u8;wire::ENROLLMENT_BYTES],
    enrollment_written: usize,
    saved: bool,
    rejection:Option<u32>,
    photos: [[u8; PHOTO_WIDTH * PHOTO_HEIGHT * 3]; 3],
    photo_ready: [bool; 3],
    capture_preview: bool,
}
impl Setup {
    // ------------------------=
    // FUNC: begin_login
    // DESC: Starts a fresh bounded login comparison using an authenticated vault payload; stored data alone never passes verification.
    // ------------------=
    pub(crate) fn begin_login(&mut self,worker:u64,model:[u8;32],payload:&[u8],now:u64)->Result<(),Error> {
        if payload.len()!=wire::ENROLLMENT_BYTES {return Err(Error::Invalid);}
        self.begin(worker,worker,model,"Login",now)?;
        self.control.command=wire::LOGIN;self.stage=Stage::Verifying(0);
        self.enrollment.copy_from_slice(payload);
        Ok(())
    }
    // ------------------------=
    // FUNC: login_template
    // DESC: Exposes stored templates only to the bound login worker while its comparison is pending.
    // ------------------=
    pub(crate) fn login_template(&self)->Option<&[u8]> {
        (self.control.command==wire::LOGIN && self.pending).then_some(&self.enrollment)
    }
    // ------------------------=
    // FUNC: new
    // DESC: Initializes a setup session without granting camera or authentication authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {control:wire::Control::empty(),stage:Stage::Idle,pending:false,started:0,
            deadline:0,command_deadline:0,preview:[0;PREVIEW_BYTES],staging:[0;PREVIEW_BYTES],preview_revision:0,preview_written:0,preview_ready:false,
            enrollment:[0;wire::ENROLLMENT_BYTES],enrollment_written:0,saved:false,rejection:None,
            photos:[[0; PHOTO_WIDTH * PHOTO_HEIGHT * 3];3],photo_ready:[false;3],capture_preview:false}
    }
    // ------------------------=
    // FUNC: begin
    // DESC: Binds a password-authorized setup attempt to an OS-selected worker and immutable model identity.
    // ------------------=
    pub fn begin(&mut self, worker:u64, account:u64, model:[u8;32], name:&str, now:u64) -> Result<(),Error> {
        if self.pending || !matches!(self.stage,Stage::Idle|Stage::Cancelled|Stage::Failed(_)|Stage::Passed) {return Err(Error::Busy);}
        if worker==0 || account==0 || model==[0;32] || name.trim().is_empty() || name.len()>64
            || name.chars().any(char::is_control) {return Err(Error::Invalid);}
        self.erase_preview();
        self.erase_photos();
        self.erase_enrollment();self.saved=false;self.rejection=None;
        self.control=wire::Control::empty();
        self.control.worker=worker;self.control.account=account;self.control.model=model;
        self.control.lease=1;self.control.name_len=name.len() as u32;
        self.control.name[..name.len()].copy_from_slice(name.as_bytes());
        self.started=now;self.deadline=now.saturating_add(SESSION_MS);
        self.stage=Stage::Loading;self.issue(wire::BEGIN,now)
    }
    // ------------------------=
    // FUNC: capture
    // DESC: Requests exactly one photo only when the previous photo has completed; double clicks cannot queue captures.
    // ------------------=
    pub fn capture(&mut self, now:u64) -> Result<(),Error> {
        self.tick(now)?;
        if self.pending {return Err(Error::Busy);}
        if !matches!(self.stage,Stage::Photos(0..=2)) {return Err(Error::WrongStage);}
        self.preview_written=0;self.capture_preview=false;self.rejection=None;self.issue(wire::CAPTURE,now)
    }
    // ------------------------=
    // FUNC: verify
    // DESC: Starts a distinct fresh-camera phase only after all three enrollment photos have been accepted.
    // ------------------=
    pub fn verify(&mut self, now:u64) -> Result<(),Error> {
        self.tick(now)?;
        if self.pending {return Err(Error::Busy);}
        if self.stage!=Stage::Review {return Err(Error::WrongStage);}
        self.rejection=None;
        self.control.lease=self.control.lease.checked_add(1).ok_or(Error::Invalid)?;
        self.stage=Stage::Verifying(0);self.erase_preview();self.issue(wire::VERIFY,now)
    }
    // ------------------------=
    // FUNC: issue
    // DESC: Advances a monotonic command generation and bounds its execution time.
    // ------------------=
    fn issue(&mut self, command:u32, now:u64) -> Result<(),Error> {
        self.control.command_id=self.control.command_id.checked_add(1).ok_or(Error::Invalid)?;
        self.control.command=command;self.pending=true;
        self.command_deadline=now.saturating_add(COMMAND_MS).min(self.deadline);Ok(())
    }
    // ------------------------=
    // FUNC: accept
    // DESC: Validates worker progress against the pending command; a forged or out-of-order report never completes verification.
    // ------------------=
    pub fn accept(&mut self, report:wire::Report, now:u64) -> Result<(),Error> {
        self.tick(now)?;
        if report.command_id!=self.control.command_id {return Err(Error::Stale);}
        if !self.pending && !(matches!(self.stage,Stage::Photos(_)) && matches!(report.phase,wire::FAILED|wire::PREVIEW_PAUSED)) {return Err(Error::Stale);}
        if report.version!=wire::VERSION || report.reserved!=0 || report.photos>3 || report.comparisons>3 {return Err(Error::Protocol);}
        if report.phase==wire::PREVIEW_PAUSED {
            if self.pending || self.stage!=Stage::Photos(report.photos as u8)
                || report.comparisons!=0 || report.error!=105 {return Err(Error::Protocol);}
            self.rejection=Some(report.error);self.erase_preview();return Ok(());
        }
        if report.phase==wire::REJECTED {
            if self.control.command==wire::VERIFY && matches!(self.stage,Stage::Verifying(_))
                && report.photos==3 && report.comparisons<=2
                && matches!(report.error,105|204..=214|216..=218|222) {
                self.stage=Stage::Review;self.rejection=Some(report.error);self.pending=false;
                self.erase_enrollment();return Ok(());
            }
            if self.control.command!=wire::CAPTURE || self.stage!=Stage::Photos(report.photos as u8)
                || report.comparisons!=0 || !matches!(report.error,105|204..=214|216..=218|222) {return Err(Error::Protocol);}
            for byte in &mut self.photos[report.photos as usize] {unsafe {core::ptr::write_volatile(byte,0);}}
            self.capture_preview=false;
            if report.error==105 {self.erase_preview();}
            self.rejection=Some(report.error);self.pending=false;return Ok(());
        }
        if report.phase==wire::FAILED {
            if report.error==0 {return Err(Error::Protocol);}
            self.stage=Stage::Failed(report.error);self.pending=false;self.erase_preview();self.erase_enrollment();self.erase_photos();return Ok(());
        }
        if report.error!=0 {return Err(Error::Protocol);}
        let terminal=match (self.control.command,report.phase,self.stage) {
            (wire::LOGIN,wire::PROCESSING,Stage::Verifying(n)) if report.photos==3 && report.comparisons==n as u32 => false,
            (wire::LOGIN,wire::VERIFYING,Stage::Verifying(n)) if n<2 && report.photos==3 && report.comparisons==n as u32+1 => {
                self.stage=Stage::Verifying(n+1);
                self.command_deadline=now.saturating_add(COMMAND_MS).min(self.deadline);false
            }
            (wire::LOGIN,wire::PASSED,Stage::Verifying(2)) if report.photos==3 && report.comparisons==3 => {
                self.stage=Stage::Passed;self.erase_enrollment();true
            }
            (wire::BEGIN,wire::READY,Stage::Loading) if report.photos==0 && report.comparisons==0 => {
                self.stage=Stage::Photos(0);true
            }
            (wire::CAPTURE,wire::PROCESSING,Stage::Photos(n)) if report.photos==n as u32 && report.comparisons==0 => false,
            (wire::CAPTURE,wire::READY,Stage::Photos(n)) if n<2 && report.photos==n as u32+1 && report.comparisons==0 => {
                if !self.capture_preview {return Err(Error::Protocol);}
                self.photo_ready[n as usize]=true;self.capture_preview=false;
                self.stage=Stage::Photos(n+1);true
            }
            (wire::CAPTURE,wire::REVIEW,Stage::Photos(2)) if report.photos==3 && report.comparisons==0 => {
                if !self.capture_preview {return Err(Error::Protocol);}
                self.photo_ready[2]=true;self.capture_preview=false;
                self.stage=Stage::Review;true
            }
            (wire::VERIFY,wire::PROCESSING,Stage::Verifying(n)) if report.photos==3 && report.comparisons==n as u32 => false,
            (wire::VERIFY,wire::VERIFYING,Stage::Verifying(n)) if n<2 && report.photos==3 && report.comparisons==n as u32+1 => {
                self.stage=Stage::Verifying(n+1);
                self.command_deadline=now.saturating_add(COMMAND_MS).min(self.deadline);false
            }
            (wire::VERIFY,wire::PASSED,Stage::Verifying(2)) if report.photos==3 && report.comparisons==3
                && self.enrollment_written==wire::ENROLLMENT_BYTES => {
                self.stage=Stage::Passed;true
            }
            _=>return Err(Error::Protocol),
        };
        if terminal {self.pending=false;}
        Ok(())
    }
    // ------------------------=
    // FUNC: write_preview
    // DESC: Atomically publishes sequential RGB preview frames without counting them as enrollment photographs.
    // ------------------=
    pub fn write_preview(&mut self, command:u64, offset:usize, bytes:&[u8], now:u64) -> Result<(),Error> {
        self.tick(now)?;
        if command!=self.control.command_id || !matches!(self.stage,Stage::Loading|Stage::Photos(_))
            || !matches!(self.control.command,wire::BEGIN|wire::CAPTURE) {return Err(Error::Stale);}
        if self.checking_photo() {return Err(Error::Stale);}
        if bytes.is_empty() || bytes.len()>4096 || offset!=self.preview_written
            || offset.checked_add(bytes.len()).is_none_or(|end|end>PREVIEW_BYTES) {return Err(Error::Invalid);}
        self.staging[offset..offset+bytes.len()].copy_from_slice(bytes);
        self.preview_written+=bytes.len();
        if self.preview_written==PREVIEW_BYTES {
            self.preview.copy_from_slice(&self.staging);self.preview_ready=true;
            self.preview_written=0;self.preview_revision=self.preview_revision.wrapping_add(1);
            if self.pending && self.control.command==wire::CAPTURE {
                if let Stage::Photos(index)=self.stage {
                    self.capture_preview=true;self.retain_photo(index as usize);
                }
            }
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: tick
    // DESC: Expires setup and blocked commands independently of camera input or worker cooperation.
    // ------------------=
    pub fn tick(&mut self, now:u64) -> Result<(),Error> {
        if matches!(self.stage,Stage::Idle|Stage::Cancelled|Stage::Failed(_)|Stage::Passed) {return Ok(());}
        if now<self.started || now>=self.deadline || (self.pending && now>=self.command_deadline) {
            let code=if now>=self.deadline || now<self.started {111}
                else if self.stage==Stage::Loading {114}
                else if self.checking_photo() || matches!(self.stage,Stage::Verifying(_)) {115}
                else {105};
            self.cancel();self.stage=Stage::Failed(code);return Err(Error::Expired);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Invalidates pending reports and erases temporary photos; the caller must revoke capture and terminate the worker.
    // ------------------=
    pub fn cancel(&mut self) {
        self.control.command_id=self.control.command_id.saturating_add(1);
        self.control.command=wire::CANCEL;self.pending=false;self.stage=Stage::Cancelled;self.erase_preview();
        self.erase_enrollment();
        self.erase_photos();
    }
    // ------------------------=
    // FUNC: retain_photo
    // DESC: Copies the explicit captured frame into its checking slot without accepting it as an enrollment sample.
    // ------------------=
    fn retain_photo(&mut self,index:usize) {
        if !self.capture_preview {return;}
        for y in 0..PHOTO_HEIGHT {for x in 0..PHOTO_WIDTH {
            let src=(y*2*PREVIEW_WIDTH+x*2)*3;
            let dst=(y*PHOTO_WIDTH+x)*3;
            self.photos[index][dst..dst+3].copy_from_slice(&self.preview[src..src+3]);
        }}
    }
    // ------------------------=
    // FUNC: checking_photo
    // DESC: Distinguishes waiting for camera pixels from analysis of a complete explicit capture.
    // ------------------=
    pub fn checking_photo(&self)->bool {
        self.pending && self.control.command==wire::CAPTURE && self.capture_preview
            && matches!(self.stage,Stage::Photos(0..=2))
    }
    // ------------------------=
    // FUNC: photo_status
    // DESC: Exposes candidate and accepted thumbnail states without treating a captured frame as a recognized face.
    // ------------------=
    pub fn photo_status(&self,index:usize)->PhotoStatus {
        if self.photo_ready.get(index).copied().unwrap_or(false) {PhotoStatus::Accepted}
        else if index<3 && self.checking_photo() && self.stage==Stage::Photos(index as u8) {PhotoStatus::Checking}
        else {PhotoStatus::Pending}
    }
    // ------------------------=
    // FUNC: photo
    // DESC: Exposes a complete explicit-capture thumbnail, with acceptance distinguished by photo_status.
    // ------------------=
    pub fn photo(&self,index:usize)->Option<&[u8]> {
        (self.photo_status(index)!=PhotoStatus::Pending).then(|| &self.photos[index][..])
    }
    // ------------------------=
    // FUNC: erase_photos
    // DESC: Scrubs transient review photographs on cancellation, restart and teardown.
    // ------------------=
    fn erase_photos(&mut self) {
        for photo in &mut self.photos {for byte in photo {unsafe {core::ptr::write_volatile(byte,0);}}}
        self.photo_ready=[false;3];self.capture_preview=false;
    }
    // ------------------------=
    // FUNC: write_enrollment
    // DESC: Accepts sequential private biometric payload chunks only during the final verification command.
    // ------------------=
    pub fn write_enrollment(&mut self,command:u64,offset:usize,bytes:&[u8],now:u64)->Result<(),Error> {
        self.tick(now)?;
        if command!=self.control.command_id || self.control.command!=wire::VERIFY
            || self.stage!=Stage::Verifying(2) || !self.pending {return Err(Error::Stale);}
        if bytes.is_empty() || bytes.len()>4096 || offset!=self.enrollment_written
            || offset.checked_add(bytes.len()).is_none_or(|n|n>wire::ENROLLMENT_BYTES) {return Err(Error::Invalid);}
        self.enrollment[offset..offset+bytes.len()].copy_from_slice(bytes);
        self.enrollment_written+=bytes.len();Ok(())
    }
    // ------------------------=
    // FUNC: enrollment
    // DESC: Borrows a complete verified payload exclusively for the trusted persistence boundary.
    // ------------------=
    pub(crate) fn enrollment(&self)->Option<&[u8]> {
        (self.stage==Stage::Passed && !self.saved && self.enrollment_written==wire::ENROLLMENT_BYTES).then_some(&self.enrollment)
    }
    // ------------------------=
    // FUNC: finish_save
    // DESC: Publishes durable success only after vault commit and readback, then scrubs unencrypted enrollment.
    // ------------------=
    pub(crate) fn finish_save(&mut self,success:bool) {
        if self.stage!=Stage::Passed || self.saved {return;}
        self.saved=success;
        if !success {self.stage=Stage::Failed(112);}
        self.erase_enrollment();
    }
    // ------------------------=
    // FUNC: saved
    // DESC: Distinguishes a persisted enrollment from a worker comparison result.
    // ------------------=
    pub fn saved(&self)->bool {self.saved}
    // ------------------------=
    // FUNC: rejection
    // DESC: Exposes recoverable capture-quality guidance without discarding previously accepted views.
    // ------------------=
    pub fn rejection(&self)->Option<u32> {self.rejection}
    // ------------------------=
    // FUNC: erase_enrollment
    // DESC: Removes all plaintext templates and portraits on completion, cancellation and failure.
    // ------------------=
    fn erase_enrollment(&mut self) {
        for byte in &mut self.enrollment {unsafe {core::ptr::write_volatile(byte,0);}}
        self.enrollment_written=0;
    }
    // ------------------------=
    // FUNC: erase_preview
    // DESC: Clears transient camera pixels without relying on an optimizable dead-store memset.
    // ------------------=
    fn erase_preview(&mut self) {
        for byte in self.preview.iter_mut().chain(self.staging.iter_mut()) {unsafe {core::ptr::write_volatile(byte,0);}}
        self.preview_written=0;self.preview_ready=false;
    }
    // ------------------------=
    // FUNC: control
    // DESC: Returns the current immutable command for the bound worker only.
    // ------------------=
    pub fn control(&self)->wire::Control {self.control}
    // ------------------------=
    // FUNC: stage
    // DESC: Exposes typed progress to the setup renderer without exposing a biometric score or credential.
    // ------------------=
    pub fn stage(&self)->Stage {self.stage}
    // ------------------------=
    // FUNC: pending
    // DESC: Allows the UI to disable duplicate actions while bounded capture or inference is active.
    // ------------------=
    pub fn pending(&self)->bool {self.pending}
    // ------------------------=
    // FUNC: preview
    // DESC: Exposes only a complete real thumbnail, never uninitialized or partially copied pixels.
    // ------------------=
    pub fn preview(&self)->Option<&[u8]> {if self.preview_ready {Some(&self.preview)}else{None}}
    // ------------------------=
    // FUNC: preview_revision
    // DESC: Signals every complete frame to the renderer even when preview availability remains unchanged.
    // ------------------=
    pub fn preview_revision(&self)->u64 {self.preview_revision}
}
impl Drop for Setup {
    // ------------------------=
    // FUNC: drop
    // DESC: Removes captured pixels on every coordinator teardown path.
    // ------------------=
    fn drop(&mut self) {self.erase_preview();self.erase_enrollment();self.erase_photos();}
}
