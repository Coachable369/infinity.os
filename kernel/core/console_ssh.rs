//! Local administration only. The SSH transport never receives a ConsoleRuntime.
use super::ConsoleRuntime;
use infinity_ssh::management::{self, Command};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: ssh_command
    // DESC: Provides local listener configuration and public-key provisioning without adding any remote command capability.
    // ------------------=
    pub(super) fn ssh_command(&mut self, command: &[u8]) -> bool {
        if command.trim_ascii().split(u8::is_ascii_whitespace).next() != Some(b"ssh".as_slice()) { return false; }
        if self.ssh_connect_command(command) { return true; }
        match management::parse(command) {
            Ok(Command::Status) => {
                if let Some(status) = crate::drivers::ssh::status() {
                    match management::availability(status.enabled,status.listening,status.endpoint,status.host_ready,status.network_ready,status.last_failure) {
                        management::Availability::Listening { address, port } => {
                            self.output.write_line(b"SSH listening. Password or public key; restricted native Console queries.");
                            let (endpoint,length)=super::node_endpoint_text(&address,port);
                            self.output.write_segments(&[b"Listening on: ",&endpoint[..length]]);
                            let handle=crate::runtime::with_runtime(|runtime|runtime.identity.user(self.current_user).map(|user|user.handle)).flatten();
                            let mut number=[0;5];let digits=super::write_decimal(&mut number,port as usize);
                            let host_length=endpoint[..length].iter().position(|b|*b==b':').unwrap_or(length);
                            self.output.write_segments(&[b"Connect from another computer: ssh -p ",&number[..digits],b" ",handle.as_ref().map_or(b"USER".as_slice(),|handle|handle.as_bytes()),b"@",&endpoint[..host_length]]);
                            self.output.write_line(b"Use the forwarded host port instead when this VM is behind NAT.");
                        },
                        management::Availability::Disabled => self.output.write_line(b"SSH disabled by saved configuration. A local administrator can use ssh enable."),
                        management::Availability::WaitingForNetwork => self.output.write_line(b"SSH enabled; waiting for an enabled network interface, link and valid IPv4 address. Check ipconfig or Network settings."),
                        management::Availability::WaitingForListener => self.output.write_line(b"SSH enabled; waiting for native listener resources."),
                        management::Availability::Failed(failure) => self.ssh_failure(failure),
                    }
                    for (label, value) in [(b"Port: ".as_slice(), status.port as u64), (b"Active: ", status.active as u64),
                        (b"Accepted: ", status.accepted), (b"Authenticated: ", status.authenticated),
                        (b"Rejected requests: ", status.denied), (b"Closed: ", status.closed)] {
                        self.output.write_number(label, value);
                    }
                    if let Some(fingerprint) = crate::drivers::ssh::host_fingerprint() { self.output.write_line(&fingerprint); }
                    if let Some(failure)=status.last_failure { self.output.write_line(b"Last recorded listener or connection failure:");self.ssh_failure(failure); }
                } else { self.output.write_line(b"SSH status is temporarily busy; retry ssh status."); }
            }
            Ok(Command::Configure(config)) => match crate::drivers::ssh::configure(self.current_session, config) {
                Ok(()) => self.output.write_line(if config.enabled { b"SSH configuration saved. Password or public-key authentication; native Console help, system status, node list and ai status." }
                    else { b"SSH disabled; connections and network grants released." }),
                Err(error) => self.ssh_failure(error),
            },
            Ok(Command::Key { user, key, revoke }) => match crate::drivers::ssh::manage_key(self.current_session, user, key, revoke) {
                Ok(()) => self.output.write_line(if revoke { b"SSH public key revoked." } else { b"SSH public key registered for this native account." }),
                Err(error) => { self.identity_error(error); },
            },
            Err(_) => {
                self.output.write_line(b"ssh status | ssh enable [PORT] [LIMIT] | ssh disable");
                self.output.write_line(b"PORT: 1-49151 (default 22). LIMIT: 1-4 (default 2). Example: ssh enable 2222 2");
                self.output.write_line(b"ssh key add <user> ssh-ed25519 <base64-public-key>");
                self.output.write_line(b"ssh key revoke <user> ssh-ed25519 <base64-public-key>");
            }
        }
        true
    }

    // ------------------------=
    // FUNC: ssh_failure
    // DESC: Explains typed listener and connection failures without exposing keys or packet contents.
    // ------------------=
    fn ssh_failure(&mut self,error:infinity_ssh::Failure) {
        use infinity_ssh::Failure;
        self.output.write_line(match error {
            Failure::Configuration=>b"Invalid SSH configuration. Use ssh enable [PORT 1-49151] [LIMIT 1-4].",
            Failure::EntropyUnavailable=>b"Validated system entropy is unavailable; SSH cannot create secure keys or connections.",
            Failure::HostKeyUnavailable=>b"Protected SSH configuration or host-key storage is unavailable. Check installed storage before retrying.",
            Failure::PermissionDenied=>b"SSH authority was denied. Configuration requires an active local administrator; remote sessions permit supported queries only.",
            Failure::NetworkUnavailable=>b"SSH network authority or interface is unavailable. Check Network settings and ipconfig.",
            Failure::Capacity=>b"SSH resources or connection slots are busy. Close an unused connection and retry.",
            Failure::Deadline=>b"The connection timed out.",Failure::AuthenticationLimit=>b"The connection exceeded its authentication attempt limit.",
            Failure::Revoked=>b"The account, key or session is no longer authorized.",Failure::Disconnected=>b"The client disconnected.",
            Failure::UnsupportedAlgorithm=>b"The client offered no supported secure algorithm.",Failure::InvalidSignature=>b"The client's public-key signature was invalid.",
            Failure::Integrity=>b"Encrypted packet integrity failed; the connection was closed.",Failure::Protocol=>b"The client sent an invalid or unsupported SSH protocol request.",
        });
    }
}
