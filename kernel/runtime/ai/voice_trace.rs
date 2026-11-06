//! Bounded, prompt-free timing records shared by native conversation workers.

// ------------------------=
// FUNC: capture
// DESC: Records calibration and VAD transitions as timestamp, state, threshold, mean-square energy and peak; never retains PCM.
// ------------------=
pub fn capture(now: u64, state: u64, threshold: u16, pcm: &[i16]) {
    let energy = pcm
        .iter()
        .map(|&v| (i64::from(v) * i64::from(v)) as u64)
        .sum::<u64>()
        / pcm.len().max(1) as u64;
    let peak = pcm.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
    #[cfg(target_os="none")]
    super::super::pipeline_log::emit("FINE", "capture", format_args!(
        "event=vad sample_ns={now} state={state} threshold_rms={threshold} mean_square={energy} peak={peak} samples={}",pcm.len()));
    const PREFIX: &[u8] = b"[VOICE CAPTURE] ";
    const HEX: &[u8] = b"0123456789abcdef";
    let mut record = [0u8; PREFIX.len() + 5 * 17];
    record[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut at = PREFIX.len();
    for value in [now, state, u64::from(threshold), energy, u64::from(peak)] {
        for shift in (0..16).rev() {
            record[at] = HEX[((value >> (shift * 4)) & 15) as usize];
            at += 1;
        }
        record[at] = b' ';
        at += 1;
    }
    record[at - 1] = b'\n';
    unsafe {
        crate::output::write(&record);
    }
}
// ------------------------=
// FUNC: write
// DESC: Adds the same monotonic nanosecond clock to each voice lifecycle record without allocation or runtime locks.
// ------------------=
pub fn write(prefix: &[u8], event: &[u8], now: u64) {
    #[cfg(target_os="none")]
    {
        let message=core::str::from_utf8(event).unwrap_or("invalid_event");
        let level=if message.contains("failed") || message.contains("error") || message.contains("timeout") || message.contains("recovery") {"WARNING"} else {"INFO"};
        let component=match prefix {b"[WHISPER] "=>"recognition",b"[VOICE OUT] "=>"playback",_=>"conversation"};
        super::super::pipeline_log::emit(level,component,format_args!(r#"event="{message}" sample_ns={now}"#));
    }
    let mut record = [0u8; 128];
    let prefix_len = prefix.len().min(24);
    record[..prefix_len].copy_from_slice(&prefix[..prefix_len]);
    let event_len = event.len().min(record.len() - prefix_len - 25);
    record[prefix_len..prefix_len + event_len].copy_from_slice(&event[..event_len]);
    let mut at = prefix_len + event_len;
    record[at..at + 4].copy_from_slice(b" ns=");
    at += 4;
    let mut digits = [0u8; 20];
    let mut first = digits.len();
    let mut value = now;
    loop {
        first -= 1;
        digits[first] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let count = digits.len() - first;
    record[at..at + count].copy_from_slice(&digits[first..]);
    at += count;
    record[at] = b'\n';
    unsafe {
        crate::output::write(&record[..at + 1]);
    }
}

// ------------------------=
// FUNC: timing
// DESC: Writes one versioned numeric timing record, without transcripts, PCM, allocation or runtime locks.
// ------------------=
pub fn timing(values: [u64; 10]) {
    numeric(b"[VOICE TIMING] ", &values);
}

// ------------------------=
// FUNC: numeric
// DESC: Emits a bounded versioned numeric record in one serial write, never user content.
// ------------------=
pub fn numeric(prefix: &[u8], values: &[u64]) {
    if prefix.len() > 24 || values.len() > 24 { return; }
    #[cfg(target_os="none")]
    super::super::pipeline_log::emit(if prefix==b"[VOICE SPAN] " && values.get(7).is_some_and(|s|*s!=0) {"WARNING"} else {"FINE"}, "metrics", format_args!("{}", Fields(prefix,values)));
    const HEX: &[u8] = b"0123456789abcdef";
    let mut record = [0u8; 24 + 24 * 17];
    record[..prefix.len()].copy_from_slice(prefix);
    let mut at = prefix.len();
    for &value in values {
        let digits = ((64 - value.leading_zeros() as usize + 3) / 4).max(1);
        for shift in (0..digits).rev() {
            record[at] = HEX[((value >> (shift * 4)) & 15) as usize];
            at += 1;
        }
        record[at] = b' ';
        at += 1;
    }
    if at == prefix.len() { return; }
    record[at - 1] = b'\n';
    unsafe { crate::output::write(&record[..at]); }
}

// ------------------------=
// FUNC: span
// DESC: Records a correlated phase interval and work units; invalid clocks remain explicitly invalid.
// ------------------=
pub fn span(phase: u64, job: u64, part: u64, start: u64, end: u64, units: u64, status: u64) {
    numeric(b"[VOICE SPAN] ", &[1, phase, job, part, start, end, units, status,
        u64::from(start != 0 && end >= start)]);
}

#[cfg(target_os="none")]
struct Fields<'a>(&'a [u8], &'a [u64]);
#[cfg(target_os="none")]
impl core::fmt::Display for Fields<'_> {
    // ------------------------=
    // FUNC: fmt
    // DESC: Gives every phase and measurement a readable decimal field name while preserving versioned correlation IDs and missing-event masks.
    // ------------------=
    fn fmt(&self,f:&mut core::fmt::Formatter<'_>)->core::fmt::Result {
        let (event,keys):(&str,&[&str])=match self.0 {
            b"[VOICE TIMING] "=>("turn",&["version","turn","present","endpoint_ns","transcript_ns","submitted_ns","first_text_ns","speech_queued_ns","first_audio_ns","drained_ns"]),
            b"[VOICE PHASES] "=>("phases",&["version","turn","present","endpoint_ns","transcript_ns","submitted_ns","first_text_ns","speech_queued_ns","first_audio_ns","drained_ns","capture_started_ns","recognition_ready_ns","first_display_ns","generation_done_ns","mic_rearmed_ns","mic_rearmed_present","service_calls","service_ns","service_max_ns","render_calls","render_ns","render_max_ns"]),
            b"[VOICE LINKS] "=>("correlation",&["version","turn","recognition_job","chat_turn","speech_job","state"]),
            b"[VOICE LLM] "=>("inference",&["version","turn","chat_turn","load_ns","first_token_ns","first_visible_ns","total_ns","prefill_work_ns","decode_work_ns","max_slice_ns","max_pump_ns","reused_tokens","new_tokens","output_tokens"]),
            b"[VOICE SPAN] "=>("span",&["version","phase","job","part","start_ns","end_ns","units","status","clock_valid"]),
            _=>("measurement",&[]),
        };
        write!(f,"event={event}")?;
        for (i,value) in self.1.iter().enumerate(){write!(f," {}={value}",keys.get(i).copied().unwrap_or("value"))?;}
        if event=="span" && self.1.len()==9 {
            let name=match self.1[1] {1=>"recognition_queue",2=>"recognition_compute",3=>"synthesis_queue",4=>"synthesis_compute",5=>"audio_publish",6=>"playback",7=>"playback_queue",8=>"recognition_warmup",9=>"intent_routing",10=>"intent_dispatch",11=>"context_preparation",12=>"prompt_preparation",_=>"other"};
            write!(f," phase_name={name}")?;
            if self.1[8]!=0 {write!(f," elapsed_ns={}",self.1[5].saturating_sub(self.1[4]))?;}
        }
        Ok(())
    }
}
