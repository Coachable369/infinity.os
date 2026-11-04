//! Local administration only. The SSH transport never receives a ConsoleRuntime.
use super::ConsoleRuntime;
use infinity_ssh::management::{self, Command};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: ssh_command
    // DESC: Provides local listener configuration and public-key provisioning without adding any remote command capability.
    // ------------------=
    pub(super) fn ssh_command(&mut self, command: &[u8]) -> bool {
        if command != b"ssh" && !command.starts_with(b"ssh ") { return false; }
        match management::parse(command) {
            Ok(Command::Status) => {
                if let Some(status) = crate::drivers::ssh::status() {
                    self.output.write_line(if status.listening { b"SSH listening; restricted native Console queries only." }
                        else if status.enabled { b"SSH enabled but not listening. Check entropy, protected storage and network configuration." }
                        else { b"SSH disabled." });
                    for (label, value) in [(b"Port: ".as_slice(), status.port as u64), (b"Active: ", status.active as u64),
                        (b"Accepted: ", status.accepted), (b"Authenticated: ", status.authenticated),
                        (b"Rejected requests: ", status.denied), (b"Closed: ", status.closed)] {
                        self.output.write_number(label, value);
                    }
                    if let Some(fingerprint) = crate::drivers::ssh::host_fingerprint() { self.output.write_line(&fingerprint); }
                }
            }
            Ok(Command::Configure(config)) => match crate::drivers::ssh::configure(self.current_session, config) {
                Ok(()) => self.output.write_line(if config.enabled { b"SSH configuration saved. Public-key authentication; native Console help, system status, node list and ai status." }
                    else { b"SSH disabled; connections and network grants released." }),
                Err(_) => self.output.write_line(b"SSH configuration failed. An active local administrator, protected storage and validated entropy are required."),
            },
            Ok(Command::Key { user, key, revoke }) => match crate::drivers::ssh::manage_key(self.current_session, user, key, revoke) {
                Ok(()) => self.output.write_line(if revoke { b"SSH public key revoked." } else { b"SSH public key registered for this native account." }),
                Err(error) => { self.identity_error(error); },
            },
            Err(_) => {
                self.output.write_line(b"ssh status | ssh enable [port:1-49151] [limit:1-4] | ssh disable");
                self.output.write_line(b"ssh key add <user> ssh-ed25519 <base64-public-key>");
                self.output.write_line(b"ssh key revoke <user> ssh-ed25519 <base64-public-key>");
            }
        }
        true
    }
}
