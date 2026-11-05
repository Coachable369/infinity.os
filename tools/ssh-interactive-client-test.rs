//! Production SSH client against an independent SSH server and real remote shell PTY.
use std::{io::{Read,Write,ErrorKind},net::TcpStream,time::{Instant,Duration},path::PathBuf};
use infinity_ssh::{client::{Connection,State},PACKET_BUFFER};
// ------------------------=
// FUNC: main
// DESC: Verifies exact host trust, password auth, PTY shell side effects, raw editing, interrupt and resize over real TCP.
// ------------------=
fn main(){
    let args:Vec<_>=std::env::args().collect();let work=PathBuf::from(&args[1]);let port:u16=args[2].parse().unwrap();
    let expected:[u8;32]=std::fs::read(work.join("host-key.raw")).unwrap().try_into().unwrap();
    let mut tcp=TcpStream::connect(("127.0.0.1",port)).unwrap();tcp.set_nonblocking(true).unwrap();
    let(mut input,mut output)=([0;PACKET_BUFFER],[0;PACKET_BUFFER]);let mut c=Connection::new(&mut input,&mut output,"fixture",0).unwrap();c.resize(91,37).unwrap();
    let start=Instant::now();let mut pending=Vec::new();let mut script:Vec<u8>=Vec::new();let mut stage=0;let mut received=0;let mut timer=Instant::now();
    loop {
        assert!(start.elapsed()<Duration::from_secs(45),"interactive SSH deadline at stage {stage}");
        if !c.output().is_empty(){match tcp.write(c.output()){Ok(n)=>c.consume_output(n).unwrap(),Err(e)if e.kind()==ErrorKind::WouldBlock=>(),Err(e)=>panic!("{e}")}}
        let mut bytes=[0;4096];match tcp.read(&mut bytes){Ok(n)if n>0=>pending.extend_from_slice(&bytes[..n]),Ok(_)=>(),Err(e)if e.kind()==ErrorKind::WouldBlock=>(),Err(e)=>panic!("{e}")}
        if !pending.is_empty(){let n=c.input(&pending).unwrap();pending.drain(..n);}
        let state=c.progress(start.elapsed().as_millis()as u64).unwrap();
        match state {
            State::HostKey(key)=>{assert_eq!(key,expected);c.confirm_host(expected,start.elapsed().as_millis()as u64).unwrap();},
            State::Password=>c.password("TerminalProof901",start.elapsed().as_millis()as u64).unwrap(),
            State::Connected=>{
                let n=c.read(&mut bytes,false).unwrap();received+=n;
                if !script.is_empty(){let n=c.write(&script).unwrap();script.drain(..n);}
                if script.is_empty(){match stage{
                    0 if received>0=>{script.extend_from_slice(b"printf proof > accepted\r");stage=1;},
                    1 if work.join("accepted").exists()=>{assert_eq!(std::fs::read(work.join("accepted")).unwrap(),b"proof");c.resize(103,41).unwrap();script.extend_from_slice(b"printf arrow > arrow-proox\x7ff\r");stage=2;},
                    2 if work.join("arrow-proof").exists()=>{assert_eq!(std::fs::read(work.join("arrow-proof")).unwrap(),b"arrow");script.extend_from_slice(b"sleep 30\r");timer=Instant::now();stage=3;},
                    3 if timer.elapsed()>Duration::from_millis(500)=>{script.extend_from_slice(b"\x03printf resumed > interrupt-proof\r");stage=4;},
                    4 if work.join("interrupt-proof").exists()=>{assert_eq!(std::fs::read(work.join("interrupt-proof")).unwrap(),b"resumed");script.extend_from_slice(b"exit\r");stage=5;},_=>()
                }}
            },State::Exited(code)=>{assert_eq!(stage,5);assert_eq!(code,0);break;},State::Failed(e)=>panic!("{e:?}"),_=>()
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(received>30);std::fs::write(work.join("client-result.json"),format!("{{\"passed\":true,\"remote_bytes\":{received},\"stages\":{stage}}}\n")).unwrap();
}
