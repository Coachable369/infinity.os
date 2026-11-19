//! Actual hardware-user-mode speaker worker; no microphone or installed-login claim.
use super::super::{native_memory::*,native_app_task::Task,native_process::State,cpu,execution::SecurityIdentity,IMAGE,emit,finish};
static MODEL:&[u8]=include_bytes!(concat!(env!("INFINITY_PROJECT_ROOT"),"/build/speaker/models/resnet34.iface"));
static HEAD:&[u8]=include_bytes!(concat!(env!("INFINITY_PROJECT_ROOT"),"/build/speaker/models/resnet34.head"));
static PCM:&[u8]=include_bytes!(env!("INFINITY_SPEAKER_PCM"));
// ------------------------=
// FUNC: run
// DESC: Executes the packaged native worker, reads its binary embedding and verifies complete memory reclamation after each of four successive runs in one VM.
// ------------------=
pub unsafe fn run(memory:&mut NativeMemory,controller:cpu::Controller)->!{
 let baseline=memory.available_pages();
 for cycle in 0..4 {
 assert_eq!(baseline,memory.available_pages());
 let started=cpu::clock_ns().unwrap();
 let before=memory.available_pages();let owner=SecurityIdentity(*b"VOICEIDENTITY\0\0\0");
 let mut task=Task::start_camera_worker(IMAGE,owner,9,memory,controller,u64::MAX,32768).unwrap();
 let prepared=cpu::clock_ns().unwrap();
 let mut vector=[0u8;1024];let mut received=false;let mut visits=0u64;
 loop{
  visits+=1;assert!(visits<2_000_000);
  let state=task.poll(cpu::clock_ns().unwrap_or(0),true,|caller,tables,op,a|{
   assert!(caller==owner);
   match op{
    0x1280=>{let data=match a[0]{1=>MODEL,2=>HEAD,3=>PCM,_=>return -2};
     if a[3]==0{return data.len()as i64;}
     let Some(end)=a[1].checked_add(a[3])else{return -22;};
     let Some(bytes)=data.get(a[1]as usize..end as usize)else{return -22;};
     if copy_to_user(tables,a[2],bytes).is_ok(){bytes.len()as i64}else{-14}
    },
    0x1281=>1,
    0x1282=>{if a[1]!=1024||received{return -22;}if copy_from_user(tables,a[0],&mut vector).is_err(){return -14;}received=true;1024},
    _=>-38
   }
  });
  if state!=State::Runnable{assert!(state==State::Exited(0));break;}
 }
 let inferred=cpu::clock_ns().unwrap();
 assert!(received);let mut offset=0;while !task.scrub_private(&mut offset){}task.release(memory).unwrap();
 assert_eq!(before,memory.available_pages());
 if cycle==3 {emit(0x535045414b455231);emit(visits);emit(before as u64);emit(memory.available_pages() as u64);
 emit(started);emit(prepared);emit(inferred);emit(cpu::clock_ns().unwrap());
 for b in vector.chunks_exact(8){emit(u64::from_le_bytes(b.try_into().unwrap()));}
 emit(4);}
 }
 assert_eq!(baseline,memory.available_pages());
 finish()
}
