use infinity_speaker_core::{self as s,policy};
// ------------------------=
// FUNC: read
// DESC: Reads binary embeddings produced by the native inference probe for real recorded speech.
// ------------------=
fn read(path:&str)->[f32;s::DIM]{let b=std::fs::read(path).unwrap();assert_eq!(b.len(),s::DIM*4);core::array::from_fn(|i|f32::from_le_bytes(b[i*4..i*4+4].try_into().unwrap()))}
// ------------------------=
// FUNC: main
// DESC: Verifies held-out genuine speech and different-speaker rejection through the production enrollment and matching policy.
// ------------------=
fn main(){
 let args:Vec<_>=std::env::args().skip(1).collect();assert!(args.len()>=5);
 let mut e=policy::Enrollment::new();for path in &args[..3]{e.add(read(path)).unwrap();}
 let mut bytes=[0;policy::BYTES];e.encode(&mut bytes).unwrap();
 assert_eq!(policy::matches(&bytes,&read(&args[3])),Ok(true));
 for path in &args[4..]{assert_eq!(policy::matches(&bytes,&read(path)),Ok(false));}
 println!("Held-out genuine sample accepted; {} distinct-speaker samples rejected",args.len()-4);
}
