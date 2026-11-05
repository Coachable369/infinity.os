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
