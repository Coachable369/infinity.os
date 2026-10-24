//! Policy tests use synthetic vectors/pixels. They are not face-recognition or
//! installed-camera acceptance and must never be reported as such.
use super::*;

// ------------------------=
// FUNC: binding
// DESC: Creates a synthetic capture identity for workflow tests only.
// ------------------=
fn binding(lease: u64) -> Binding { Binding { account: 1, worker: 2, lease, model: [3;32] } }
// ------------------------=
// FUNC: pixels
// DESC: Constructs a high-contrast synthetic crop for numerical policy tests, not a simulated camera backend.
// ------------------=
fn pixels() -> [u8; CROP_SIDE*CROP_SIDE] {
    let mut pixels = [0;CROP_SIDE*CROP_SIDE];
    for (i,p) in pixels.iter_mut().enumerate() { *p = if (i+i/CROP_SIDE)%2==0 {80} else {180}; }
    pixels
}
// ------------------------=
// FUNC: vector
// DESC: Constructs an orthogonal test vector; it is not a model-generated embedding.
// ------------------=
fn vector(index: usize) -> [f32;DIMENSIONS] { let mut v = [0.0;DIMENSIONS]; v[index] = 1.0; v }
// ------------------------=
// FUNC: enrollment
// DESC: Starts a workflow with explicit experimental numerical thresholds.
// ------------------=
fn enrollment() -> Enrollment {
    Enrollment::new(binding(1), "Everyday", Policy { cosine: 0.6, min_laplacian_variance: 100 }, 0).unwrap()
}
// ------------------------=
// FUNC: observation
// DESC: Provides borrowed numerical inputs to the policy without claiming camera or detector integration.
// ------------------=
fn observation<'a>(lease: u64, sequence: u64, time: u64, pose: Pose, crop: &'a [u8], embedding: &'a [f32;DIMENSIONS]) -> Observation<'a> {
    Observation { binding: binding(lease), sequence, captured_ms: time, pose, crop, embedding,
        face_count: 1, face_width: 140, face_height: 160 }
}
// ------------------------=
// FUNC: collect
// DESC: Exercises enrollment progression with three synthetic same-vector views.
// ------------------=
fn collect(e: &mut Enrollment) {
    for (i,pose) in [Pose::Front,Pose::Left,Pose::Right].into_iter().enumerate() {
        let now = i as u64*500;
        e.observe(&observation(1,i as u64+1,now,pose,&pixels(),&vector(0)),now).unwrap();
    }
    assert_eq!(e.state(),State::ReadyToVerify);
}
// ------------------------=
// FUNC: fresh_capture_and_three_comparisons_required
// DESC: Checks that enrollment alone cannot produce verification and that reused capture authority is rejected.
// ------------------=
#[test]
fn fresh_capture_and_three_comparisons_required() {
    let mut e = enrollment(); collect(&mut e);
    assert_eq!(e.begin_verification(binding(1),1500),Err(Error::WrongCapture));
    let mut foreign = binding(2); foreign.account = 99;
    assert_eq!(e.begin_verification(foreign,1500),Err(Error::WrongCapture));
    e.begin_verification(binding(2),1500).unwrap();
    assert_eq!(e.observe(&observation(1,4,1500,Pose::Front,&pixels(),&vector(0)),1500),Err(Error::WrongCapture));
    for i in 0..3 {
        let now = 1500+i*500;
        let state = e.observe(&observation(2,i+1,now,Pose::Front,&pixels(),&vector(0)),now).unwrap();
        assert_eq!(state,if i==2 {State::Verified} else {State::Verifying(i as u8+1)});
    }
    assert_eq!(e.observe(&observation(2,4,3000,Pose::Front,&pixels(),&vector(0)),3000),Err(Error::WrongState));
}
// ------------------------=
// FUNC: quality_rejection_does_not_advance
// DESC: Verifies structured actionable errors for absent, multiple, small, dark, bright and unfocused faces.
// ------------------=
#[test]
fn quality_rejection_does_not_advance() {
    let crop = pixels(); let v = vector(0); let mut e = enrollment();
    let mut f = observation(1,1,0,Pose::Front,&crop,&v);
    f.face_count=0; assert_eq!(e.observe(&f,0),Err(Error::NoFace));
    f.sequence+=1; f.face_count=2; assert_eq!(e.observe(&f,0),Err(Error::MultipleFaces));
    f.sequence+=1; f.face_count=1; f.face_width=99; assert_eq!(e.observe(&f,0),Err(Error::TooSmall));
    assert_eq!(e.state(),State::Photos(0));
    assert_eq!(quality(&[0;CROP_SIDE*CROP_SIDE],100),Err(Error::TooDark));
    assert_eq!(quality(&[255;CROP_SIDE*CROP_SIDE],100),Err(Error::TooBright));
    assert_eq!(quality(&[128;CROP_SIDE*CROP_SIDE],100),Err(Error::LowContrast));
    let mut ramp=[0;CROP_SIDE*CROP_SIDE];
    for (i,p) in ramp.iter_mut().enumerate() { *p=70+(i%CROP_SIDE) as u8; }
    assert_eq!(quality(&ramp,100),Err(Error::Blurred));
    assert_eq!(quality(&crop[..100],100),Err(Error::InvalidPixels));
}
// ------------------------=
// FUNC: invalid_vectors_and_mixed_enrollment_rejected
// DESC: Rejects invalid floats, zero vectors and a different synthetic embedding during photo collection.
// ------------------=
#[test]
fn invalid_vectors_and_mixed_enrollment_rejected() {
    let mut e=enrollment(); let crop=pixels(); let v=vector(0);
    e.observe(&observation(1,1,0,Pose::Front,&crop,&v),0).unwrap();
    assert_eq!(e.observe(&observation(1,2,500,Pose::Left,&crop,&vector(1)),500),Err(Error::DifferentPerson));
    assert_eq!(e.state(),State::Photos(1));
    assert_eq!(norm(&[0.0;DIMENSIONS]),Err(Error::InvalidEmbedding));
    assert_eq!(norm(&[f32::NAN;DIMENSIONS]),Err(Error::InvalidEmbedding));
    assert_eq!(norm(&[f32::INFINITY;DIMENSIONS]),Err(Error::InvalidEmbedding));
    assert!(!matches(&v,&vector(1),0.6).unwrap());
    let mut opposite=v; opposite[0]=-1.0;
    assert!(!matches(&v,&opposite,0.6).unwrap());
}
// ------------------------=
// FUNC: replay_spacing_timeout_and_wipe
// DESC: Exercises frame freshness, minimum capture spacing, timeout and cancellation cleanup.
// ------------------=
#[test]
fn replay_spacing_timeout_and_wipe() {
    let mut e=enrollment(); let crop=pixels(); let v=vector(0);
    e.observe(&observation(1,1,0,Pose::Front,&crop,&v),0).unwrap();
    assert_eq!(e.observe(&observation(1,1,500,Pose::Left,&crop,&v),500),Err(Error::StaleFrame));
    assert_eq!(e.observe(&observation(1,2,499,Pose::Left,&crop,&v),499),Err(Error::TooSoon));
    assert_eq!(e.observe(&observation(1,3,2000,Pose::Left,&crop,&v),1500),Err(Error::StaleFrame));
    assert_eq!(e.tick(TIMEOUT_MS),Err(Error::Expired));
    assert_eq!(e.state(),State::Expired); assert_eq!(e.name(),"");
    assert!(e.samples.iter().flatten().all(|&v|v==0.0));
    let mut e=enrollment(); collect(&mut e); e.cancel();
    assert_eq!(e.state(),State::Cancelled); assert_eq!(e.tick(2000),Err(Error::Cancelled));
    assert!(e.samples.iter().flatten().all(|&v|v==0.0));
}
// ------------------------=
// FUNC: bounded_verification_failures
// DESC: Confirms three mismatches terminate and clear the workflow, rather than granting authority or retrying indefinitely.
// ------------------=
#[test]
fn bounded_verification_failures() {
    let mut e=enrollment(); collect(&mut e); e.begin_verification(binding(2),1500).unwrap();
    for i in 0..3 {
        let now=1500+i*500;
        let result=e.observe(&observation(2,i+1,now,Pose::Front,&pixels(),&vector(1)),now);
        assert_eq!(result,Err(if i==2 {Error::AttemptsExhausted} else {Error::NoMatch}));
    }
    assert_eq!(e.state(),State::Failed); assert!(e.samples.iter().flatten().all(|&v|v==0.0));
}
// ------------------------=
// FUNC: interrupted_verification_requires_three_new_matches
// DESC: Rejects nonconsecutive verification when a second face enters the scene between successful comparisons.
// ------------------=
#[test]
fn interrupted_verification_requires_three_new_matches() {
    let mut e=enrollment(); collect(&mut e); e.begin_verification(binding(2),1500).unwrap();
    let crop=pixels(); let v=vector(0);
    assert_eq!(e.observe(&observation(2,1,1500,Pose::Front,&crop,&v),1500),Ok(State::Verifying(1)));
    let mut frame=observation(2,2,2000,Pose::Front,&crop,&v); frame.face_count=2;
    assert_eq!(e.observe(&frame,2000),Err(Error::MultipleFaces));
    assert_eq!(e.state(),State::Verifying(0));
}
