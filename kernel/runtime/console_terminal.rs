//! Per-channel terminal state. No framebuffer, global input, interpreter or process API.
use super::console_execution::{Outcome, Diagnostic};
use infinity_enterprise_core::{History, INPUT};
pub const QUEUE: usize = 2048;
pub struct Terminal {
    pub input: [u8; INPUT], pub length: usize, cursor: usize, echo_cursor:usize, history: History,
    escape: [u8; 16], escape_length: usize, after_cr: bool,
    echo: [u8; QUEUE], echo_length: usize,
    pub columns: u16, pub rows: u16, pub pty: bool, pub interactive: bool,
    pub pending: bool, pub started: bool, pub closing: bool, pub finished: bool, pub status: u32,
    pub waiting:bool,pub secret:bool,pub private:bool,pub raw:bool,pub read_input:bool,more:bool,notification:bool,
    pub outcome: Option<Outcome>, pub stdout_offset: usize, pub stderr_offset: usize,
    pub deadline: u64,
    pub wire: [u8; 512], pub wire_length: usize, pub wire_offset: usize, pub wire_stderr: bool,
}
impl Terminal {
    // ------------------------=
    // FUNC: new
    // DESC: Creates independent bounded input, output, history and cancellation ownership for one channel.
    // ------------------=
    pub const fn new() -> Self { Self { input: [0; INPUT], length: 0, cursor: 0, echo_cursor:0, history: History::new(),
        escape: [0; 16], escape_length: 0, after_cr: false, echo: [0; QUEUE], echo_length: 0,
        columns: 80, rows: 24, pty: false, interactive: false, pending: false, started: false,
        waiting:false,secret:false,private:false,raw:false,read_input:false,more:false,notification:false,
        closing: false, finished: false, status: 0, outcome: None, stdout_offset: 0, stderr_offset: 0, deadline: 0, wire: [0;512], wire_length: 0, wire_offset: 0, wire_stderr: false } }
    // ------------------------=
    // FUNC: history_count
    // DESC: Reports bounded history occupancy without exposing its text in diagnostics.
    // ------------------=
    pub fn history_count(&self) -> usize { self.history.count() }
    // ------------------------=
    // FUNC: resize
    // DESC: Bounds terminal geometry independently of desktop windows and rendering.
    // ------------------=
    pub fn resize(&mut self, columns: u32, rows: u32) -> bool {
        if columns > 1000 || rows > 1000 { return false; }
        if columns != 0 { self.columns = columns as u16; }
        if rows != 0 { self.rows = rows as u16; } true
    }
    // ------------------------=
    // FUNC: start
    // DESC: Starts either interactive input or a single exec request without introducing another command parser.
    // ------------------=
    pub fn start(&mut self, command: Option<&[u8]>, now: u64) -> bool {
        if self.started { return false; }
        self.started = true; self.interactive = command.is_none();
        if let Some(command) = command {
            if command.is_empty() || command.len() > INPUT { self.closing = true; self.status = 2; return false; }
            self.input[..command.len()].copy_from_slice(command); self.length = command.len();
            self.pending = true; self.deadline = now.saturating_add(10_000);
        } else { self.emit(b"InfinityOS native Console. Ctrl-D closes this session.\n"); self.prompt(); }
        true
    }
    // ------------------------=
    // FUNC: emit
    // DESC: Queues terminal bytes with PTY newline translation; input admission reserves enough room for redraw.
    // ------------------=
    fn emit(&mut self, bytes: &[u8]) {
        for byte in bytes {
            if self.pty && *byte == b'\n' { self.push(b'\r'); }
            self.push(*byte);
        }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Fails the channel on impossible output overflow rather than silently dropping terminal bytes.
    // ------------------=
    fn push(&mut self, byte: u8) {
        if self.echo_length == QUEUE { self.closing = true; self.status = 75; return; }
        self.echo[self.echo_length] = byte; self.echo_length += 1;
    }
    // ------------------------=
    // FUNC: prompt
    // DESC: Emits the same isolated native prompt after each completed command.
    // ------------------=
    fn prompt(&mut self) { self.emit(b"Infinity> "); }
    // ------------------------=
    // FUNC: echo
    // DESC: Borrows unsent terminal data until SSH acknowledges its consumed prefix.
    // ------------------=
    pub fn echo(&self) -> &[u8] { &self.echo[..self.echo_length] }
    // ------------------------=
    // FUNC: consume_echo
    // DESC: Retains unsent bytes under network backpressure.
    // ------------------=
    pub fn consume_echo(&mut self, amount: usize) {
        let n = amount.min(self.echo_length); self.echo.copy_within(n..self.echo_length, 0); self.echo_length -= n;
    }
    // ------------------------=
    // FUNC: can_input
    // DESC: Stops channel reads before input or redraw capacity is exhausted.
    // ------------------=
    pub fn can_input(&self) -> bool { self.started && (self.interactive||self.raw||self.secret||self.read_input) && !self.closing && !self.pending && !self.waiting
        && self.outcome.is_none() && self.echo_length < QUEUE - 1200 }
    // ------------------------=
    // FUNC: redraw
    // DESC: Uses standard ANSI line editing only for allocated terminals; a non-PTY shell has no echo.
    // ------------------=
    fn redraw(&mut self) {
        if !self.pty || self.secret || self.raw { return; }
        if self.read_input {self.move_left(self.echo_cursor);self.emit(b"\x1b[K");}
        else {self.emit(b"\r\x1b[2K");self.prompt();}
        let bytes = self.input; self.emit(&bytes[..self.length]);
        let distance = self.input[self.cursor..self.length].iter().filter(|byte| **byte & 0xc0 != 0x80).count();
        self.move_left(distance);
        self.echo_cursor=self.input[..self.cursor].iter().filter(|byte| **byte & 0xc0 != 0x80).count();
    }
    // ------------------------=
    // FUNC: move_left
    // DESC: Moves the PTY cursor within its current input draft while preserving an application's own prompt.
    // ------------------=
    fn move_left(&mut self,mut distance:usize){
        if distance==0{return;}
        let mut digits=[0;4];let mut at=digits.len();
        while distance!=0{at-=1;digits[at]=b'0'+(distance%10)as u8;distance/=10;}
        self.emit(b"\x1b[");self.emit(&digits[at..]);self.emit(b"D");
    }
    // ------------------------=
    // FUNC: input_byte
    // DESC: Edits one admitted byte, handles CRLF, history and terminal controls, and queues complete text for the native parser.
    // ------------------=
    pub fn input_byte(&mut self, byte: u8, now: u64) {
        if !self.can_input() { return; }
        if byte == 3 { self.interrupt(); return; }
        if self.escape_length > 0 {
            if self.escape_length == self.escape.len() { self.escape_length = 0; return; }
            self.escape[self.escape_length] = byte; self.escape_length += 1;
            if byte.is_ascii_alphabetic() || byte == b'~' {
                match &self.escape[..self.escape_length] {
                    b"\x1b[A" | b"\x1b[B" if !self.secret&&!self.private&&!self.read_input => {
                        if let Some(entry) = self.history.recall(byte == b'A', &self.input[..self.length]) {
                            self.input = entry.bytes; self.length = entry.length; self.cursor = self.length;
                        }
                    }
                    b"\x1b[D" => self.cursor = self.cursor.saturating_sub(1),
                    b"\x1b[C" => self.cursor = (self.cursor + 1).min(self.length),
                    b"\x1b[H" | b"\x1b[1~" => self.cursor = 0,
                    b"\x1b[F" | b"\x1b[4~" => self.cursor = self.length,
                    b"\x1b[3~" if self.cursor < self.length => { self.input.copy_within(self.cursor+1..self.length, self.cursor); self.length -= 1; }
                    _ => (),
                }
                self.escape_length = 0; self.redraw();
            }
            return;
        }
        if byte == 27 { self.escape[0] = byte; self.escape_length = 1; return; }
        if byte == b'\n' && self.after_cr { self.after_cr = false; return; }
        self.after_cr = byte == b'\r';
        match byte {
            b'\r' | b'\n' => {
                if self.pty { self.emit(b"\n"); }
                if self.length == 0 && !self.secret && !self.read_input { self.prompt(); } else {
                    if !self.secret&&!self.private&&!self.read_input {self.history.record(&self.input[..self.length]);} self.pending = true;
                    self.deadline = now.saturating_add(10_000);
                }
            }
            4 if self.length == 0 => self.closing = true,
            4 if self.cursor < self.length => { self.input.copy_within(self.cursor+1..self.length, self.cursor); self.length -= 1; self.redraw(); }
            1 => { self.cursor = 0; self.redraw(); }
            5 => { self.cursor = self.length; self.redraw(); }
            21 => { self.length = 0; self.cursor = 0; self.echo_cursor=0; self.input.fill(0); self.redraw(); }
            8 | 127 if self.cursor > 0 => {
                self.input.copy_within(self.cursor..self.length, self.cursor-1); self.cursor -= 1; self.length -= 1; self.redraw();
            }
            32..=126 | 128..=255 if self.length < INPUT => {
                self.input.copy_within(self.cursor..self.length, self.cursor+1);
                self.input[self.cursor] = byte; self.length += 1; self.cursor += 1; self.redraw();
            }
            32..=126 | 128..=255 => self.complete(Outcome::failure(Diagnostic::Capacity)),
            _ => (),
        }
    }
    // ------------------------=
    // FUNC: interrupt
    // DESC: Cancels only this terminal's pending command and output; it never touches desktop or another connection.
    // ------------------=
    pub fn interrupt(&mut self) {
        self.pending = false; self.length = 0; self.cursor = 0; self.echo_cursor=0; self.escape_length = 0; self.input.fill(0);
        self.waiting=false;self.secret=false;self.raw=false;self.read_input=false;self.more=false;self.notification=false;
        self.outcome = Some(Outcome::failure(Diagnostic::Cancelled)); self.stdout_offset = 0; self.stderr_offset = 0;
        self.status = 130; self.deadline = 0; self.wire_length = 0; self.wire_offset = 0;
    }
    // ------------------------=
    // FUNC: complete
    // DESC: Retains the structured execution result until both SSH output streams are consumed.
    // ------------------=
    pub fn complete(&mut self, outcome: Outcome) {
        self.pending = false; self.length = 0; self.cursor = 0; self.echo_cursor=0; self.input.fill(0);
        self.waiting=false;self.more=false;self.notification=false;
        self.status = outcome.status; self.outcome = Some(outcome); self.stdout_offset = 0; self.stderr_offset = 0;
    }
    // ------------------------=
    // FUNC: submitted
    // DESC: Clears transferred input and waits for initial Console dispatch without blocking the network actor.
    // ------------------=
    pub fn submitted(&mut self,now:u64){self.pending=false;self.waiting=true;self.input.fill(0);self.length=0;self.cursor=0;self.echo_cursor=0;self.deadline=now.saturating_add(180_000);}
    // ------------------------=
    // FUNC: input_raw
    // DESC: Retains a whole raw burst without echo, history, CRLF handling or interception of application control keys.
    // ------------------=
    pub fn input_raw(&mut self,bytes:&[u8],now:u64)->bool {
        if !self.raw||!self.can_input()||bytes.len()>INPUT{return false;}
        self.input[..bytes.len()].copy_from_slice(bytes);self.length=bytes.len();self.pending=true;
        self.deadline=now.saturating_add(10_000);true
    }
    // ------------------------=
    // FUNC: deliver
    // DESC: Applies a command receipt or asynchronous notification while protecting newer drafts and password-entry state.
    // ------------------=
    pub fn deliver(&mut self,reply:super::console_remote::Reply,notification:bool) {
        if notification {
            self.outcome=Some(reply.outcome);self.stdout_offset=0;self.stderr_offset=0;self.notification=true;
            if self.interactive&&self.pty&&!self.secret&&!self.raw&&!self.read_input {self.emit(b"\r\x1b[2K");}
        }else{
            self.complete(reply.outcome);self.secret=reply.secret;self.private=reply.private;self.raw=reply.raw;self.read_input=reply.read_input;
            if reply.clear_history {self.history.clear();}
            self.closing|=reply.close;self.more=reply.more&&!reply.close;
            self.waiting=self.more&&!self.interactive&&!self.raw&&!self.secret&&!self.read_input;
        }
    }
    // ------------------------=
    // FUNC: update
    // DESC: Applies metadata for the latest owned request without erasing input typed since its first receipt.
    // ------------------=
    pub fn update(&mut self,reply:super::console_remote::Reply){
        let input=self.input;let length=self.length;let cursor=self.cursor;let echo_cursor=self.echo_cursor;let pending=self.pending;
        self.deliver(reply,false);
        self.input=input;self.length=length;self.cursor=cursor;self.echo_cursor=echo_cursor;self.pending=pending;self.notification=true;
        if self.interactive&&self.pty&&!self.secret&&!self.raw&&!self.read_input {self.emit(b"\r\x1b[2K");}
    }
    // ------------------------=
    // FUNC: stage_output
    // DESC: Preserves partially transmitted CRLF expansions and stderr identity across SSH backpressure.
    // ------------------=
    pub fn stage_output(&mut self) {
        if self.wire_offset < self.wire_length { return; }
        self.wire_offset = 0; self.wire_length = 0;
        let Some(outcome) = self.outcome.as_ref() else { return; };
        let (bytes, offset, stderr) = if self.stdout_offset < outcome.stdout.bytes().len() {
            (outcome.stdout.bytes(), &mut self.stdout_offset, false)
        } else { (outcome.stderr.bytes(), &mut self.stderr_offset, true) };
        self.wire_stderr = stderr && !self.pty;
        while *offset < bytes.len() && self.wire_length <= self.wire.len()-2 {
            let byte = bytes[*offset]; *offset += 1;
            if self.pty && byte == b'\n' && (*offset<2||bytes[*offset-2]!=b'\r') { self.wire[self.wire_length] = b'\r'; self.wire_length += 1; }
            self.wire[self.wire_length] = byte; self.wire_length += 1;
        }
    }
    // ------------------------=
    // FUNC: output_complete
    // DESC: Ends exec or prompts interactive input only after all response bytes have been accepted by SSH.
    // ------------------=
    pub fn output_complete(&mut self) {
        self.outcome = None;
        if self.interactive||self.raw||self.secret||self.read_input||!self.more {self.deadline=0;}
        if self.interactive && !self.closing {
            if !self.secret&&!self.raw&&!self.read_input&&!self.waiting {if self.notification&&self.pty {self.redraw();}else{self.prompt();}}
        } else if !self.more {self.closing = true;}
        self.notification=false;
    }
    // ------------------------=
    // FUNC: ready_to_close
    // DESC: Defers stdin-EOF channel closure until the already accepted command and all output have completed.
    // ------------------=
    pub fn ready_to_close(&self)->bool {
        self.closing&&!self.more&&!self.pending&&!self.waiting&&self.outcome.is_none()
            &&self.echo_length==0&&self.wire_offset>=self.wire_length
    }
}
