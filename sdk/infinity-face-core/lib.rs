#![no_std]
//! Bounded enrollment/verification policy for an isolated recognition worker.
//! Includes fixed-model detection/embedding operators, but no liveness test or authentication API.
//! Inputs must come from the native worker's decoded, detected and aligned frame.
//! Verified records can be exported only to the private OS persistence boundary; no result authorizes login.

pub mod inference;
pub mod vision;
pub mod recognition;
pub mod protocol;
#[cfg(feature="native-jpeg")]
pub mod jpeg;

pub const DIMENSIONS: usize = 128;
pub const PHOTO_COUNT: usize = 3;
pub const CROP_SIDE: usize = 112;
const TIMEOUT_MS: u64 = 240_000;
const FRAME_SPACING_MS: u64 = 500;
const MAX_FRAME_AGE_MS: u64 = 30_000;
const MAX_PROCESSING_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose { Front, Left, Right }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State { Photos(u8), ReadyToVerify, Verifying(u8), Verified, Cancelled, Expired, Failed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPolicy, InvalidName, WrongState, WrongCapture, StaleFrame, TooSoon,
    NoFace, MultipleFaces, TooSmall, InvalidPixels, TooDark, TooBright, Clipped,
    LowContrast, Blurred, InvalidEmbedding, WrongPose, DifferentPerson,
    NoMatch, Cancelled, Expired, AttemptsExhausted, UnusableLandmarks,
}

/// Experimental comparison thresholds must be supplied by the evaluated model
/// adapter. They are not published false-accept or security guarantees.
#[derive(Clone, Copy)]
pub struct Policy { pub cosine: f32, pub min_laplacian_variance: u32 }

/// Binding is generated and checked by the trusted orchestrator, never a website
/// or arbitrary app. A separate capture lease is required for verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding { pub account: u64, pub worker: u64, pub lease: u64, pub model: [u8; 32] }

pub struct Observation<'a> {
    pub binding: Binding,
    pub sequence: u64,
    pub captured_ms: u64,
    pub face_count: u8,
    pub face_width: u16,
    pub face_height: u16,
    pub pose: Pose,
    /// Aligned grayscale face pixels, not a downsample of the entire scene.
    pub crop: &'a [u8],
    pub embedding: &'a [f32; DIMENSIONS],
}

/// Private-field receipt issued before inference by the fixed camera worker.
/// Capture freshness and bounded processing time are separate checks.
pub struct FrameAdmission {
    binding: Binding,
    sequence: u64,
    captured_ms: u64,
    admitted_ms: u64,
}

pub struct Enrollment {
    policy: Policy,
    binding: Binding,
    name: [u8; 64],
    name_len: usize,
    state: State,
    deadline: u64,
    start: u64,
    last_sequence: u64,
    last_capture: Option<u64>,
    samples: [[f32; DIMENSIONS]; PHOTO_COUNT],
    portraits: [[u8; protocol::PORTRAIT_SIDE*protocol::PORTRAIT_SIDE]; PHOTO_COUNT],
    failures: u8,
}

// ------------------------=
// FUNC: quality
// DESC: Measures exposure, clipping, contrast and sharpness on a bounded aligned face crop; does not detect faces or spoofing.
// ------------------=
pub fn quality(pixels: &[u8], min_laplacian_variance: u32) -> Result<(), Error> {
    if pixels.len() != CROP_SIDE * CROP_SIDE { return Err(Error::InvalidPixels); }
    // SFace alignment includes hair and background around the facial landmarks.
    // Evaluate the landmark region, not a bright window or dark wall beside a
    // turned face. The same region and thresholds apply to enrollment and login.
    const LEFT: usize = 24;
    const RIGHT: usize = 88;
    const TOP: usize = 32;
    const BOTTOM: usize = 104;
    let n = ((RIGHT-LEFT)*(BOTTOM-TOP)) as i64;
    let mut sum = 0i64;
    let mut squares = 0i64;
    let mut clipped = 0;
    for y in TOP..BOTTOM { for x in LEFT..RIGHT {
        let p=pixels[y*CROP_SIDE+x];
        let v = i64::from(p); sum += v; squares += v*v; clipped += usize::from(p < 8 || p > 247);
    }}
    if sum < 40*n { return Err(Error::TooDark); }
    if sum > 215*n { return Err(Error::TooBright); }
    if clipped * 5 > n as usize { return Err(Error::Clipped); }
    if squares*n - sum*sum < 144*n*n { return Err(Error::LowContrast); }
    let mut lap_sum = 0i64;
    let mut lap_squares = 0i64;
    for y in TOP+1..BOTTOM-1 { for x in LEFT+1..RIGHT-1 {
        let at = y*CROP_SIDE+x;
        let l = i64::from(pixels[at-1])+i64::from(pixels[at+1])
            +i64::from(pixels[at-CROP_SIDE])+i64::from(pixels[at+CROP_SIDE])-4*i64::from(pixels[at]);
        lap_sum += l; lap_squares += l*l;
    }}
    let count = ((RIGHT-LEFT-2)*(BOTTOM-TOP-2)) as i64;
    if lap_squares*count-lap_sum*lap_sum < i64::from(min_laplacian_variance)*count*count {
        return Err(Error::Blurred);
    }
    Ok(())
}

// ------------------------=
// FUNC: quality_excludes_background_but_rejects_unusable_faces
// DESC: Checks that background clipping cannot reject detailed facial pixels and that actual face defects remain rejected.
// ------------------=
#[test]
fn quality_excludes_background_but_rejects_unusable_faces() {
    for background in [0,255] {
        let mut crop=[background;CROP_SIDE*CROP_SIDE];
        for y in 32..104 {for x in 24..88 {crop[y*CROP_SIDE+x]=if (x+y)%2==0 {80}else{180};}}
        assert_eq!(quality(&crop,100),Ok(()));
    }
    assert_eq!(quality(&[0;CROP_SIDE*CROP_SIDE],100),Err(Error::TooDark));
    assert_eq!(quality(&[255;CROP_SIDE*CROP_SIDE],100),Err(Error::TooBright));
    assert_eq!(quality(&[120;CROP_SIDE*CROP_SIDE],100),Err(Error::LowContrast));
    let mut smooth=[0;CROP_SIDE*CROP_SIDE];
    for y in 0..CROP_SIDE {for x in 0..CROP_SIDE {smooth[y*CROP_SIDE+x]=(50+x) as u8;}}
    assert_eq!(quality(&smooth,100),Err(Error::Blurred));
}

// ------------------------=
// FUNC: norm
// DESC: Rejects non-finite, zero or unreasonably scaled model embeddings before comparison.
// ------------------=
fn norm(values: &[f32; DIMENSIONS]) -> Result<f64, Error> {
    let mut sum = 0.0f64;
    for &value in values {
        if !value.is_finite() || value.abs() > 1_000_000.0 { return Err(Error::InvalidEmbedding); }
        sum += f64::from(value)*f64::from(value);
    }
    if sum < 1e-12 { Err(Error::InvalidEmbedding) } else { Ok(sum) }
}

// ------------------------=
// FUNC: matches
// DESC: Compares positive cosine similarity without a platform-dependent square-root or allocation.
// ------------------=
fn matches(a: &[f32; DIMENSIONS], b: &[f32; DIMENSIONS], threshold: f32) -> Result<bool, Error> {
    let na = norm(a)?;
    let nb = norm(b)?;
    let dot: f64 = a.iter().zip(b).map(|(&a,&b)| f64::from(a)*f64::from(b)).sum();
    Ok(dot > 0.0 && dot*dot >= f64::from(threshold)*f64::from(threshold)*na*nb)
}

impl Enrollment {
    // ------------------------=
    // FUNC: from_verified
    // DESC: Imports authenticated stored vectors for a new three-frame comparison, never as an already verified result.
    // ------------------=
    pub fn from_verified(binding:Binding, payload:&[u8], policy:Policy, now:u64)->Result<Self,Error> {
        if payload.len()!=protocol::ENROLLMENT_BYTES {return Err(Error::InvalidEmbedding);}
        let mut result=Self::new(binding,"Login",policy,now)?;
        for (sample,bytes) in result.samples.iter_mut().zip(payload[..PHOTO_COUNT*DIMENSIONS*4].chunks_exact(DIMENSIONS*4)) {
            for (value,raw) in sample.iter_mut().zip(bytes.chunks_exact(4)) {*value=f32::from_le_bytes(raw.try_into().map_err(|_|Error::InvalidEmbedding)?);}
            norm(sample)?;
        }
        result.state=State::Verifying(0);
        Ok(result)
    }
    // ------------------------=
    // FUNC: new
    // DESC: Starts a named, time-bounded enrollment without changing any account authentication policy.
    // ------------------=
    pub fn new(binding: Binding, name: &str, policy: Policy, now_ms: u64) -> Result<Self, Error> {
        if !policy.cosine.is_finite() || policy.cosine <= 0.0 || policy.cosine > 1.0
            || policy.min_laplacian_variance == 0 || policy.min_laplacian_variance > 1_000_000
            || binding.account == 0 || binding.worker == 0 || binding.lease == 0 || binding.model == [0;32] {
            return Err(Error::InvalidPolicy);
        }
        if name.trim().is_empty() || name.len() > 64 || name.chars().any(char::is_control) { return Err(Error::InvalidName); }
        let mut owned_name = [0;64];
        owned_name[..name.len()].copy_from_slice(name.as_bytes());
        Ok(Self { policy, binding, name: owned_name, name_len: name.len(), state: State::Photos(0),
            deadline: now_ms.checked_add(TIMEOUT_MS).ok_or(Error::InvalidPolicy)?, start: now_ms,
            last_sequence: 0, last_capture: None, samples: [[0.0;DIMENSIONS];PHOTO_COUNT],
            portraits:[[0;protocol::PORTRAIT_SIDE*protocol::PORTRAIT_SIDE];PHOTO_COUNT], failures: 0 })
    }
    // ------------------------=
    // FUNC: state
    // DESC: Exposes workflow state only, with no login authority or authentication token.
    // ------------------=
    pub fn state(&self) -> State { self.state }
    // ------------------------=
    // FUNC: name
    // DESC: Returns the bounded user-supplied enrollment label.
    // ------------------=
    pub fn name(&self) -> &str { core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("") }
    // ------------------------=
    // FUNC: expected_pose
    // DESC: Supplies guidance for three enrollment views followed by a separate frontal verification capture.
    // ------------------=
    pub fn expected_pose(&self) -> Option<Pose> {
        match self.state { State::Photos(0) | State::Verifying(_) => Some(Pose::Front),
            State::Photos(1) => Some(Pose::Left), State::Photos(2) => Some(Pose::Right), _ => None }
    }
    // ------------------------=
    // FUNC: tick
    // DESC: Expires incomplete enrollment even when no frames arrive, wiping temporary biometric vectors.
    // ------------------=
    pub fn tick(&mut self, now_ms: u64) -> Result<(), Error> {
        match self.state {
            State::Cancelled => return Err(Error::Cancelled), State::Expired => return Err(Error::Expired),
            State::Failed => return Err(Error::AttemptsExhausted), State::Verified => return Ok(()), _ => (),
        }
        if now_ms < self.start || now_ms >= self.deadline { self.wipe(); self.state = State::Expired; return Err(Error::Expired); }
        Ok(())
    }
    // ------------------------=
    // FUNC: begin_verification
    // DESC: Requires a newly authorized capture lease from the same account, worker and model before verifying enrollment.
    // ------------------=
    pub fn begin_verification(&mut self, binding: Binding, now_ms: u64) -> Result<(), Error> {
        self.tick(now_ms)?;
        if !matches!(self.state,State::ReadyToVerify|State::Verifying(_)) { return Err(Error::WrongState); }
        if binding.account != self.binding.account || binding.worker != self.binding.worker
            || binding.model != self.binding.model || binding.lease == 0 || binding.lease == self.binding.lease {
            return Err(Error::WrongCapture);
        }
        self.binding = binding; self.start = now_ms; self.last_sequence = 0; self.last_capture = None;
        self.state = State::Verifying(0);
        Ok(())
    }
    // ------------------------=
    // FUNC: observe
    // DESC: Accepts quality-checked, sequence-consistent worker observations; a new capture must match all three enrolled views.
    // ------------------=
    pub fn observe(&mut self, frame: &Observation<'_>, now_ms: u64) -> Result<State, Error> {
        let result = self.observe_checked(frame, now_ms);
        if matches!(result, Err(error) if !matches!(error, Error::TooSoon | Error::StaleFrame | Error::WrongCapture))
            && matches!(self.state, State::Verifying(_)) {
            self.state = State::Verifying(0);
        }
        result
    }
    // ------------------------=
    // FUNC: admit_frame
    // DESC: Checks actual capture freshness before inference without granting recognition or login success.
    // ------------------=
    pub fn admit_frame(&mut self, binding: Binding, sequence: u64, captured_ms: u64, now_ms: u64) -> Result<FrameAdmission, Error> {
        self.tick(now_ms)?;
        self.expected_pose().ok_or(Error::WrongState)?;
        if binding != self.binding { return Err(Error::WrongCapture); }
        if sequence == 0 || sequence <= self.last_sequence || captured_ms < self.start
            || captured_ms > now_ms || now_ms - captured_ms > MAX_FRAME_AGE_MS { return Err(Error::StaleFrame); }
        if self.last_capture.is_some_and(|last| captured_ms < last.saturating_add(FRAME_SPACING_MS)) { return Err(Error::TooSoon); }
        Ok(FrameAdmission { binding, sequence, captured_ms, admitted_ms: now_ms })
    }
    // ------------------------=
    // FUNC: observe_admitted
    // DESC: Consumes a capture-bound receipt after bounded inference, retaining expiry, replay, pose, quality and identity checks.
    // ------------------=
    pub fn observe_admitted(&mut self, frame: &Observation<'_>, receipt: FrameAdmission, now_ms: u64) -> Result<State, Error> {
        self.tick(now_ms)?;
        if frame.binding != receipt.binding || frame.sequence != receipt.sequence || frame.captured_ms != receipt.captured_ms {
            return Err(Error::WrongCapture);
        }
        if now_ms < receipt.admitted_ms || now_ms - receipt.admitted_ms >= MAX_PROCESSING_MS { return Err(Error::StaleFrame); }
        self.observe(frame, receipt.admitted_ms)
    }
    // ------------------------=
    // FUNC: observe_checked
    // DESC: Applies capture and comparison checks; fresh quality or match failures reset progress, while polling duplicates do not.
    // ------------------=
    fn observe_checked(&mut self, frame: &Observation<'_>, now_ms: u64) -> Result<State, Error> {
        self.tick(now_ms)?;
        let expected = self.expected_pose().ok_or(Error::WrongState)?;
        if frame.binding != self.binding { return Err(Error::WrongCapture); }
        if frame.sequence == 0 || frame.sequence <= self.last_sequence || frame.captured_ms < self.start
            || frame.captured_ms > now_ms || now_ms-frame.captured_ms > MAX_FRAME_AGE_MS { return Err(Error::StaleFrame); }
        self.last_sequence = frame.sequence;
        if self.last_capture.is_some_and(|last| frame.captured_ms < last.saturating_add(FRAME_SPACING_MS)) { return Err(Error::TooSoon); }
        if frame.face_count == 0 { return Err(Error::NoFace); }
        if frame.face_count != 1 { return Err(Error::MultipleFaces); }
        if frame.face_width < 100 || frame.face_height < 100 { return Err(Error::TooSmall); }
        quality(frame.crop, self.policy.min_laplacian_variance)?;
        norm(frame.embedding)?;
        if frame.pose != expected { return Err(Error::WrongPose); }
        match self.state {
            State::Photos(count) => {
                for prior in &self.samples[..usize::from(count)] {
                    if !matches(prior, frame.embedding, self.policy.cosine)? { return Err(Error::DifferentPerson); }
                }
                self.samples[usize::from(count)].copy_from_slice(frame.embedding);
                for y in 0..protocol::PORTRAIT_SIDE {for x in 0..protocol::PORTRAIT_SIDE {
                    let at=y*2*CROP_SIDE+x*2;
                    self.portraits[usize::from(count)][y*protocol::PORTRAIT_SIDE+x]=
                        ((u16::from(frame.crop[at])+u16::from(frame.crop[at+1])
                            +u16::from(frame.crop[at+CROP_SIDE])+u16::from(frame.crop[at+CROP_SIDE+1]))/4) as u8;
                }}
                self.state = if usize::from(count)+1 == PHOTO_COUNT { State::ReadyToVerify } else { State::Photos(count+1) };
            }
            State::Verifying(count) => {
                for prior in &self.samples {
                    if !matches(prior, frame.embedding, self.policy.cosine)? {
                        self.failures += 1;
                        self.state = State::Verifying(0);
                        if self.failures >= 3 { self.wipe(); self.state = State::Failed; return Err(Error::AttemptsExhausted); }
                        return Err(Error::NoMatch);
                    }
                }
                self.state = if count == 2 { State::Verified } else { State::Verifying(count+1) };
            }
            _ => return Err(Error::WrongState),
        }
        self.last_capture = Some(frame.captured_ms);
        Ok(self.state)
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Clears temporary enrollment on user cancellation, permission loss, disconnect or worker termination.
    // ------------------=
    pub fn cancel(&mut self) { self.wipe(); self.state = State::Cancelled; }
    // ------------------------=
    // FUNC: export_verified
    // DESC: Exports only independently verified enrollment in bounded slices to the private OS vault; never grants login authority.
    // ------------------=
    pub fn export_verified(&self, offset:usize, out:&mut [u8])->Result<(),Error> {
        if self.state!=State::Verified {return Err(Error::WrongState);}
        if out.is_empty() || out.len()>4096 || offset.checked_add(out.len()).is_none_or(|end|end>protocol::ENROLLMENT_BYTES) {
            return Err(Error::InvalidPixels);
        }
        for (index,byte) in out.iter_mut().enumerate() {
            let at=offset+index;
            *byte=if at<PHOTO_COUNT*DIMENSIONS*4 {
                self.samples[at/(DIMENSIONS*4)][at/4%DIMENSIONS].to_le_bytes()[at%4]
            }else {
                let at=at-PHOTO_COUNT*DIMENSIONS*4;
                let size=protocol::PORTRAIT_SIDE*protocol::PORTRAIT_SIDE;
                self.portraits[at/size][at%size]
            };
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: wipe
    // DESC: Overwrites private temporary vectors and enrollment labels without an optimizable-away ordinary memset.
    // ------------------=
    fn wipe(&mut self) {
        for sample in &mut self.samples { for value in sample { unsafe { core::ptr::write_volatile(value, 0.0); } } }
        for portrait in &mut self.portraits {for value in portrait {unsafe {core::ptr::write_volatile(value,0);}}}
        for value in &mut self.name { unsafe { core::ptr::write_volatile(value, 0); } }
        self.name_len = 0;
    }
}
impl Drop for Enrollment {
    // ------------------------=
    // FUNC: drop
    // DESC: Clears bounded temporary biometrics when the isolated workflow is discarded, including after verified export.
    // ------------------=
    fn drop(&mut self) { self.wipe(); }
}

#[cfg(test)]
mod tests;
