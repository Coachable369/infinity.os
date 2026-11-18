use infinity_speaker_core as s;
use infinity_face_core::inference::Model;
// ------------------------=
// FUNC: main
// DESC: Exercises the production native frontend and model against binary reference fixtures.
// ------------------=
fn main(){
    let args:Vec<_>=std::env::args().collect();
    let root=std::path::Path::new(&args[1]);
    let pcm=std::fs::read(root.join("pcm.bin")).unwrap();
    let pcm:Vec<i16>=pcm.chunks_exact(2).map(|b|i16::from_le_bytes(b.try_into().unwrap())).collect();
    let mut features=[0f32;s::FEATURES];s::features(&pcm,&mut features).unwrap();
    std::fs::write(root.join("native-features.bin"),features.iter().flat_map(|x|x.to_le_bytes()).collect::<Vec<_>>()).unwrap();
    let model=std::fs::read("build/speaker/models/resnet34.iface").unwrap();
    let head=std::fs::read("build/speaker/models/resnet34.head").unwrap();
    let mut arena=vec![0f32;Model::parse(&model).unwrap().arena_len];
    let mut trunk=Box::new([0f32;s::TRUNK_OUTPUT]);let mut out=[0f32;s::DIM];
    let started=std::time::Instant::now();
    s::embed(&model,&head,&features,&mut arena,&mut trunk,&mut out,||true).unwrap();
    println!("Native embedding elapsed {:?}",started.elapsed());
    std::fs::write(root.join("native-embedding.bin"),out.iter().flat_map(|x|x.to_le_bytes()).collect::<Vec<_>>()).unwrap();
    assert!(s::features(&[0;s::SAMPLES],&mut features)==Err(s::Error::Quiet));
    assert!(s::similarity(&[f32::NAN;s::DIM],&out).is_err());
}
