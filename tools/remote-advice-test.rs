#![allow(dead_code)]
//! Real identity, model-context and output ownership acceptance for isolated SSH advice.
#[path="../kernel/runtime/mod.rs"]mod runtime;
#[path="../kernel/ui/mod.rs"]mod ui;
use runtime::ai::{AiRuntime,chat::{GenerationState,HERMES_MODEL_ID},text_request::{AdviceMode,AdviceError},remote_advice::OUTPUT_BYTES};
use runtime::identity::{StableId,AccountRequest,AccountResult};
struct Users{a:StableId,b:StableId,local:StableId,remote_a:StableId,remote_b:StableId,key_a:[u8;32]}
// ------------------------=
// FUNC: output_text
// DESC: Keeps optional native diagnostic prose outside behavioral assertions.
// ------------------=
fn output_text(_:&[u8]){}
// ------------------------=
// FUNC: identities
// DESC: Creates two real identities and separately authenticated SSH credentials plus a local administrator session.
// ------------------=
fn identities()->Users {
    runtime::initialize(false);runtime::with_runtime(|r|{
        r.identity.begin_onboarding().unwrap();r.identity.create_machine(b"Remote advice",0xaa64,1,1).unwrap();
        let a=r.identity.create_user(b"remote-a",b"Remote A",2).unwrap().id;r.identity.create_password(a,b"local password",3).unwrap();r.identity.complete_onboarding().unwrap();let local=r.identity.create_session(a,b"local password",4).unwrap().id;
        let b=match r.identity.manage_accounts(local,AccountRequest::CreateUser{handle:b"remote-b",display_name:b"Remote B",password:b"member password",now:5}).unwrap(){AccountResult::User(user)=>user.id,_=>panic!()};
        let key_a=ed25519_dalek::SigningKey::from_bytes(&[41;32]).verifying_key().to_bytes();let key_b=ed25519_dalek::SigningKey::from_bytes(&[42;32]).verifying_key().to_bytes();
        r.identity.manage_ssh_key_durable(local,a,key_a,false,6,|_|Ok(())).unwrap();r.identity.manage_ssh_key_durable(local,b,key_b,false,7,|_|Ok(())).unwrap();
        let remote_a=r.identity.create_ssh_session(a,key_a,8).unwrap().id;let remote_b=r.identity.create_ssh_session(b,key_b,9).unwrap().id;
        Users{a,b,local,remote_a,remote_b,key_a}
    }).unwrap()
}
// ------------------------=
// FUNC: contract
// DESC: Verifies identity, SSH-only admission, profile revocation and empty-session privacy before a model is installed.
// ------------------=
fn contract(users:&Users) {
    let mut ai=Box::new(AiRuntime::new());ai.bind_chat_owner(users.b.0);ai.chat.set_enabled(false);
    for(owner,session)in[([0;16],users.remote_a.0),(users.a.0,users.local.0),(users.b.0,users.remote_a.0),(users.a.0,[9;16])]{assert_eq!(ai.submit_remote_advice(owner,session,HERMES_MODEL_ID,b"hello",AdviceMode::Answer,None),Err(AdviceError::Denied));}
    assert_eq!(ai.submit_remote_advice(users.a.0,users.remote_a.0,HERMES_MODEL_ID,b"hello",AdviceMode::Answer,None),Err(AdviceError::Unavailable));
    assert_eq!(ai.remote_advice_state(users.a.0,users.remote_a.0),None);assert_eq!(ai.remote_advice_latest(users.a.0,users.remote_a.0),Err(AdviceError::NotOwned));
    runtime::with_runtime(|r|r.identity.update_ai_chat_preferences(users.a,users.a,false,3).unwrap());assert_eq!(ai.submit_remote_advice(users.a.0,users.remote_a.0,HERMES_MODEL_ID,b"hello",AdviceMode::Answer,None),Err(AdviceError::Denied));runtime::with_runtime(|r|r.identity.update_ai_chat_preferences(users.a,users.a,true,3).unwrap());
    assert!(ai.reset_remote_advice(users.a.0,users.remote_a.0).is_ok());
}
// ------------------------=
// FUNC: poll_remote
// DESC: Advances the production native lease until a typed terminal state while proving no executable control is produced.
// ------------------=
fn poll_remote(ai:&mut AiRuntime,owner:[u8;16],session:[u8;16],turn:u64)->Vec<u8> {
    let start=std::time::Instant::now();while ai.remote_advice_active(){ai.poll_qwen();assert!(!ai.control_pending());assert!(start.elapsed().as_secs()<180);}
    let mut output=[0;OUTPUT_BYTES];let(state,n)=ai.remote_advice_reply(owner,session,turn,&mut output).unwrap();assert_eq!(state,GenerationState::Complete);output[..n].to_vec()
}
// ------------------------=
// FUNC: native
// DESC: Exercises actual Hermes generation across separate SSH users, local logout, cancellation, credential revocation and exact native-context restoration.
// ------------------=
fn native(users:&Users) {
    let weights=Box::leak(std::fs::read("model-cache/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf").unwrap().into_boxed_slice());let arena=Box::leak(vec![0u128;1280*1024*1024/16].into_boxed_slice());let arena=unsafe{std::slice::from_raw_parts_mut(arena.as_mut_ptr().cast::<u8>(),arena.len()*16)};
    let mut ai=Box::new(AiRuntime::new());assert!(ai.load_hermes(weights,arena));let threads:Vec<_>=(0..4).map(|i|std::thread::spawn(move||unsafe{runtime::ai::qwen::workers::run_host_worker(i)})).collect();let start=std::time::Instant::now();while runtime::ai::qwen::workers::online()!=4{assert!(start.elapsed().as_secs()<10);std::thread::yield_now();}
    ai.bind_chat_owner(users.b.0);ai.submit_text_advice(users.b.0,b"Return the integer 7 only.",AdviceMode::Answer,None,true).unwrap();let started=std::time::Instant::now();while ai.chat.generation_state==GenerationState::Running{ai.poll_qwen();assert!(started.elapsed().as_secs()<180);}assert_eq!(ai.chat.generation_state,GenerationState::Complete);
    ai.chat.push_input(b'x');let desktop=ai.chat.state_hash();let original=ai.acceptance_native_context().unwrap().0.to_vec();assert!(!original.is_empty());
    let prompt=b"weights=20,80 scores=9,4/4,9 Reply with exactly CHOICE:1 or CHOICE:2 for the highest scoring option, and no other text.";
    let turn=ai.submit_remote_advice(users.a.0,users.remote_a.0,HERMES_MODEL_ID,prompt,AdviceMode::Decide,None).unwrap();let remote_prefix=ai.acceptance_native_context().unwrap().0.len();assert_eq!(ai.chat.state_hash(),desktop);assert_eq!(ai.remote_advice_state(users.a.0,users.remote_a.0),Some((turn,GenerationState::Running)));
    assert_eq!(ai.cancel_remote_advice(users.b.0,users.remote_b.0,turn),Err(AdviceError::NotOwned));assert_eq!(ai.remote_advice_reply(users.b.0,users.remote_b.0,turn,&mut [0;OUTPUT_BYTES]),Err(AdviceError::NotOwned));assert_eq!(ai.reset_remote_advice(users.a.0,users.remote_a.0),Err(AdviceError::Busy));
    ai.cancel_app_turn();assert!(ai.remote_advice_active());let output=poll_remote(&mut ai,users.a.0,users.remote_a.0,turn);let choice=core::str::from_utf8(&output).unwrap().trim().strip_prefix("CHOICE:").and_then(|n|n.parse::<u8>().ok());assert_eq!(choice,Some(2),"native reply protocol: {}",String::from_utf8_lossy(&output));assert_eq!(ai.chat.state_hash(),desktop);assert_eq!(ai.acceptance_native_context().unwrap().0,original);assert_eq!(ai.remote_advice_reply(users.a.0,users.remote_a.0,turn,&mut []),Err(AdviceError::ContextFull));
    let continued=ai.submit_remote_advice(users.a.0,users.remote_a.0,HERMES_MODEL_ID,prompt,AdviceMode::Decide,None).unwrap();assert!(ai.acceptance_native_context().unwrap().0.len()>remote_prefix);ai.cancel_remote_advice(users.a.0,users.remote_a.0,continued).unwrap();assert_eq!(ai.remote_advice_state(users.a.0,users.remote_a.0),Some((continued,GenerationState::Cancelled)));assert_eq!(ai.acceptance_native_context().unwrap().0,original);ai.reset_remote_advice(users.a.0,users.remote_a.0).unwrap();assert_eq!(ai.remote_advice_latest(users.a.0,users.remote_a.0),Err(AdviceError::NotOwned));
    // A second SSH user starts with a fresh model context and survives local logout.
    let second=ai.submit_remote_advice(users.b.0,users.remote_b.0,HERMES_MODEL_ID,prompt,AdviceMode::Decide,None).unwrap();assert_eq!(ai.acceptance_native_context().unwrap().0.len(),remote_prefix);ai.bind_chat_owner([0;16]);ai.chat.set_enabled(false);assert!(ai.remote_advice_active());let logged_out=ai.chat.state_hash();let output=poll_remote(&mut ai,users.b.0,users.remote_b.0,second);let choice=core::str::from_utf8(&output).unwrap().trim().strip_prefix("CHOICE:").and_then(|n|n.parse::<u8>().ok());assert_eq!(choice,Some(2));assert_eq!(ai.chat.state_hash(),logged_out);assert!(ai.acceptance_native_context().unwrap().0.is_empty());
    assert_eq!(ai.remote_advice_reply(users.a.0,users.remote_a.0,second,&mut [0;OUTPUT_BYTES]),Err(AdviceError::NotOwned));
    // Revoking the actual SSH credential cancels admission and prevents retained output access.
    let revoked=ai.submit_remote_advice(users.a.0,users.remote_a.0,HERMES_MODEL_ID,prompt,AdviceMode::Decide,None).unwrap();runtime::with_runtime(|r|r.identity.manage_ssh_key_durable(users.local,users.a,users.key_a,true,20,|_|Ok(())).unwrap());assert!(!ai.remote_advice_authorized());ai.poll_qwen();assert!(!ai.remote_advice_active());assert_eq!(ai.remote_advice_reply(users.a.0,users.remote_a.0,revoked,&mut [0;OUTPUT_BYTES]),Err(AdviceError::Denied));assert!(ai.acceptance_native_context().unwrap().0.is_empty());
    unsafe{runtime::ai::qwen::workers::stop_host_workers();}for thread in threads{thread.join().unwrap();}
}
// ------------------------=
// FUNC: main
// DESC: Runs typed access checks and optional real model privacy acceptance without claiming installed-VM verification.
// ------------------=
fn main(){let users=identities();contract(&users);if std::env::args().any(|a|a=="--native"){native(&users);}println!("PASS isolated SSH AI ownership and lease acceptance");}
