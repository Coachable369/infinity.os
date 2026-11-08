extern crate std;
use super::*;
use client::{Connection, State};
use core::cell::Cell;
struct PasswordAuthority { active: Cell<bool>, attempts: Cell<u32> }
impl IdentityAuthority for PasswordAuthority {
    // ------------------------=
    // FUNC: authorized_key
    // DESC: Ensures this test cannot accidentally succeed through public-key authentication.
    // ------------------=
    fn authorized_key(&self, _: &[u8], _: &[u8; 32]) -> Option<Principal> { None }
    // ------------------------=
    // FUNC: authorized_password
    // DESC: Supplies a test-only credential boundary for real encrypted SSH authentication.
    // ------------------=
    fn authorized_password(&self, name: &[u8], password: &[u8], _: u64) -> Option<Principal> {
        self.attempts.set(self.attempts.get() + 1);
        (self.active.get() && name == b"alice" && password == b"native password")
            .then_some(Principal { user: [7; 16], public_key: [0; 32], password_credential: Some([9; 16]) })
    }
    // ------------------------=
    // FUNC: still_authorized
    // DESC: Models credential revocation after a successful password login.
    // ------------------=
    fn still_authorized(&self, p: Principal) -> bool { self.active.get() && p.user == [7; 16] && p.password_credential == Some([9; 16]) }
}
// ------------------------=
// FUNC: password_client_round_trip_and_revocation
// DESC: Exercises host verification, wrong-password retry, encrypted shell I/O and live credential revocation.
// ------------------=
#[test]
fn password_client_round_trip_and_revocation() {
    let host = SignKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(&[23; 32]));
    let expected = match host.pubkey() { PubKey::Ed25519(k) => k.key.0, _ => panic!() };
    let authority = PasswordAuthority { active: Cell::new(true), attempts: Cell::new(0) };
    let (mut si, mut so, mut ci, mut co) = ([0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER]);
    let mut server = Transport::new(&mut si, &mut so, 0).unwrap();
    let mut client = Connection::new(&mut ci, &mut co, "alice", 0).unwrap();
    assert_eq!(client.password("native password", 0), Err(Failure::PermissionDenied));
    let mut wire = [std::vec::Vec::new(), std::vec::Vec::new()];
    let (mut host_checked, mut supplied, mut shell, mut sent, mut replied) = (false, 0, false, false, false);
    let mut received = std::vec::Vec::new();
    let mut pty=None;client.resize(91,37).unwrap();
    for now in 0..2000 {
        wire[0].extend_from_slice(client.output()); let n = client.output().len(); client.consume_output(n).unwrap();
        wire[1].extend_from_slice(server.output(&authority, now).unwrap()); let n = server.output(&authority, now).unwrap().len(); server.consume_output(n).unwrap();
        let n = server.input(&wire[0][..wire[0].len().min(37)], &authority, now).unwrap(); wire[0].drain(..n);
        let n = client.input(&wire[1][..wire[1].len().min(41)]).unwrap(); wire[1].drain(..n);
        if server.progress(&authority, &host, now).unwrap() == Diagnostic::Authenticated { server.authorize_console().unwrap(); }
        match server.take_console_request(){Some(ConsoleRequest::Shell)=>shell=true,Some(ConsoleRequest::Pty{columns,rows})=>pty=Some((columns,rows)),_=>()}
        match client.progress(now).unwrap() {
            State::HostKey(key) => {
                assert_eq!(key, expected); assert_eq!(authority.attempts.get(), 0);
                assert_eq!(client.confirm_host([0; 32], now), Err(Failure::PermissionDenied));
                // A prompt may remain pending across arbitrary service passes.
                assert_eq!(client.progress(now + 1).unwrap(), State::HostKey(key));
                client.confirm_host(key, now).unwrap(); host_checked = true;
            }
            State::Password => {
                assert!(host_checked); client.password(if supplied == 0 { "wrong password" } else { "native password" }, now).unwrap(); supplied += 1;
            }
            State::Connected if !sent => { assert_eq!(client.write(b"ping\n").unwrap(), 5); sent = true; }
            _ => (),
        }
        if shell && !replied {
            let mut input = [0; 32];
            let n = server.read_console(&mut input, &authority, now).unwrap();
            if n > 0 { assert_eq!(&input[..n], b"ping\n"); assert_eq!(server.write_console(b"pong\n", false, &authority, now).unwrap(), 5); replied = true; }
        }
        let mut bytes = [0; 16]; let n = client.read(&mut bytes, false).unwrap(); received.extend_from_slice(&bytes[..n]);
        if received.len() == 5 { break; }
    }
    assert_eq!(pty,Some((91,37)));assert!(shell && sent && replied); assert_eq!(received, b"pong\n"); assert_eq!(supplied, 2);
    assert_eq!(authority.attempts.get(), 2);
    authority.active.set(false);
    assert_eq!(server.progress(&authority, &host, 2100), Err(Failure::Revoked));
}

// ------------------------=
// FUNC: queued_terminal_bytes_survive_partial_writes
// DESC: Verifies atomic bounded admission and byte preservation for split UTF-8, terminal escapes and pasted control bytes.
// ------------------=
#[test]
fn queued_terminal_bytes_survive_partial_writes(){
    let mut bytes=[0;16];let(mut length,mut sent)=(0,0);
    assert!(client::queue_terminal_input(&mut bytes,&mut length,&mut sent,b"\x1bOA\xe2\x82"));
    sent=3;assert_eq!(client::pending_input_capacity(bytes.len(),length,sent),14);
    assert!(client::queue_terminal_input(&mut bytes,&mut length,&mut sent,b"\xac\r\n\0\x03"));
    assert_eq!(&bytes[..length],b"\xe2\x82\xac\r\n\0\x03");assert_eq!(sent,0);
    let before=bytes;let before_length=length;
    assert!(!client::queue_terminal_input(&mut bytes,&mut length,&mut sent,&[9;10]));
    assert_eq!(bytes,before);assert_eq!(length,before_length);assert_eq!(sent,0);
    sent=length;assert!(client::queue_terminal_input(&mut bytes,&mut length,&mut sent,&[5;16]));
    assert_eq!(&bytes[..length],&[5;16]);assert_eq!(client::pending_input_capacity(bytes.len(),length,sent),0);
}
// ------------------------=
// FUNC: client_stdin_half_close_keeps_reverse_streams
// DESC: Uses real encrypted peers to prove stdin EOF preserves all preceding bytes, later stdout/stderr, and the remote exit status.
// ------------------=
#[test]
fn client_stdin_half_close_keeps_reverse_streams(){
    let host=SignKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(&[29;32]));
    let authority=PasswordAuthority{active:Cell::new(true),attempts:Cell::new(0)};
    let(mut si,mut so,mut ci,mut co)=([0;PACKET_BUFFER],[0;PACKET_BUFFER],[0;PACKET_BUFFER],[0;PACKET_BUFFER]);
    let mut server=Transport::new(&mut si,&mut so,0).unwrap();let mut client=Connection::new(&mut ci,&mut co,"alice",0).unwrap();
    assert_eq!(client.finish_input(),Err(Failure::PermissionDenied));
    let mut wire=[std::vec::Vec::new(),std::vec::Vec::new()];
    let payload=b"\x1bOA\x1b[200~\xe2\x82\xac\r\n\0\x03\x1b[201~";
    let(mut sent,mut half_closed,mut shell,mut output_sent,mut error_sent,mut finished)=(0,false,false,0,0,false);
    let(mut received,mut output,mut errors)=(std::vec::Vec::new(),std::vec::Vec::new(),std::vec::Vec::new());
    for now in 0..6000{
        wire[0].extend_from_slice(client.output());let n=client.output().len();client.consume_output(n).unwrap();
        wire[1].extend_from_slice(server.output(&authority,now).unwrap());let n=server.output(&authority,now).unwrap().len();server.consume_output(n).unwrap();
        let n=server.input(&wire[0][..wire[0].len().min(37)],&authority,now).unwrap();wire[0].drain(..n);
        let n=client.input(&wire[1][..wire[1].len().min(41)]).unwrap();wire[1].drain(..n);
        if server.progress(&authority,&host,now).unwrap()==Diagnostic::Authenticated{server.authorize_console().unwrap();}
        if matches!(server.take_console_request(),Some(ConsoleRequest::Shell)){shell=true;}
        match client.progress(now).unwrap(){
            State::HostKey(key)=>client.confirm_host(key,now).unwrap(),
            State::Password=>client.password("native password",now).unwrap(),
            State::Connected if !half_closed=>{
                if sent<payload.len(){sent+=client.write(&payload[sent..]).unwrap();assert_eq!(client.finish_input(),Ok(false));}
                else{half_closed=client.finish_input().unwrap();if half_closed{assert!(client.write(&[255]).is_err());}}
            }
            _=>(),
        }
        if shell&&!finished{
            let mut bytes=[0;2];let n=server.read_console(&mut bytes,&authority,now).unwrap();received.extend_from_slice(&bytes[..n]);
            if server.console_eof(){
                assert_eq!(received,payload);assert!(!server.runner.is_channel_closed(server.channel.as_ref().unwrap()));
                if output_sent<4{output_sent+=server.write_console(&[8,0,13,10][output_sent..],false,&authority,now).unwrap();}
                else if error_sent<3{error_sent+=server.write_console(&[7,2,9][error_sent..],true,&authority,now).unwrap();}
                else{finished=server.finish_console(41).unwrap();}
            }
        }
        let mut bytes=[0;2];let n=client.read(&mut bytes,false).unwrap();output.extend_from_slice(&bytes[..n]);
        let n=client.read(&mut bytes,true).unwrap();errors.extend_from_slice(&bytes[..n]);
        if finished&&client.state()==State::Exited(41)&&output.len()==4&&errors.len()==3{break;}
    }
    assert!(shell&&half_closed&&finished);assert_eq!(received,payload);assert_eq!(output,[8,0,13,10]);assert_eq!(errors,[7,2,9]);assert_eq!(client.state(),State::Exited(41));
}
