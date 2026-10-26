//! Behavioral equivalence and cancellation checks using packaged native graphs.
#[path="../sdk/infinity-face-core/inference.rs"] mod inference;
use inference::{Execution,Model,Error};
// ------------------------=
// FUNC: execute
// DESC: Executes a real graph with deterministic input and returns activation state and checkpoint count.
// ------------------=
fn execute(model:Model<'_>,budget:usize)->(Vec<f32>,usize) {
    let mut arena=vec![0.0;model.arena_len];
    let mut run=Execution::new(model,&mut arena).unwrap();
    for (i,v) in run.input().unwrap().iter_mut().enumerate(){*v=(i%251) as f32/251.0;}
    let mut steps=1;
    while !run.step(budget).unwrap(){steps+=1;}
    drop(run);
    (arena,steps)
}
// ------------------------=
// FUNC: main
// DESC: Verifies batch equivalence, reduced scheduling checkpoints and cancellation wiping for both shipped graphs.
// ------------------=
fn main(){
    for name in ["yunet","sface"] {
        let bytes=std::fs::read(format!("build/face/models/{name}.iface")).unwrap();
        let model=Model::parse(&bytes).unwrap();
        let start=std::time::Instant::now();
        let (small,a)=execute(model,128);
        let (large,b)=execute(model,4096);
        assert_eq!(small,large);
        assert!(b<=a.div_ceil(32)+1);
        let mut arena=vec![1.0;model.arena_len];
        let mut run=Execution::new(model,&mut arena).unwrap();
        run.step(4096).unwrap();run.cancel();
        assert_eq!(run.step(4096),Err(Error::Cancelled));
        drop(run);assert!(arena.iter().all(|v|*v==0.0));
        println!("{name}: checkpoints {a} -> {b}, elapsed {:?}",start.elapsed());
    }
}
