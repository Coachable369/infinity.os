//! Bounded native AI audio journal. Producers never perform disk I/O or wait for a lock.
use core::{cell::UnsafeCell, fmt::Write, sync::atomic::{AtomicBool, AtomicU64, Ordering}};
pub const DIRECTORY: &[u8] = b"/Logs/ai.audiopipeline";
const RECORD_BYTES: usize = 1024;
const RECORDS: usize = 128;
const PAGE_BYTES: usize = 16 * 1024;
const PAGES: u8 = 16;
#[derive(Clone, Copy)]
struct Record { bytes: [u8; RECORD_BYTES], len: usize }
impl Record {
    const EMPTY: Self = Self { bytes: [0; RECORD_BYTES], len: 0 };
}
impl core::fmt::Write for Record {
    // ------------------------=
    // FUNC: write_str
    // DESC: Rejects oversized records atomically instead of publishing truncated log lines.
    // ------------------=
    fn write_str(&mut self, value: &str) -> core::fmt::Result {
        if value.len() > self.bytes.len() - self.len { return Err(core::fmt::Error); }
        self.bytes[self.len..self.len+value.len()].copy_from_slice(value.as_bytes());
        self.len += value.len(); Ok(())
    }
}
struct Ring { records: [Record; RECORDS], head: usize, len: usize }
pub struct Journal { busy: AtomicBool, lost: AtomicU64, ring: UnsafeCell<Ring> }
// All accesses to ring are guarded by a nonblocking acquire/release exchange.
unsafe impl Sync for Journal {}
impl Journal {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves bounded storage independent of turn length and worker count.
    // ------------------=
    pub const fn new() -> Self {
        Self { busy: AtomicBool::new(false), lost: AtomicU64::new(0), ring: UnsafeCell::new(Ring { records: [Record::EMPTY; RECORDS], head: 0, len: 0 }) }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Queues a complete record without waiting; contention and overflow are explicitly counted.
    // ------------------=
    fn push(&self, record: Record) {
        if self.busy.compare_exchange(false,true,Ordering::Acquire,Ordering::Relaxed).is_err() {
            self.lost.fetch_add(1,Ordering::Relaxed); return;
        }
        let ring=unsafe { &mut *self.ring.get() };
        if ring.len==RECORDS { self.lost.fetch_add(1,Ordering::Relaxed); }
        else { ring.records[(ring.head+ring.len)%RECORDS]=record;ring.len+=1; }
        self.busy.store(false,Ordering::Release);
    }
    // ------------------------=
    // FUNC: drain
    // DESC: Copies whole records to the persistence batch and releases producers before any storage work.
    // ------------------=
    fn drain(&self, out: &mut [u8]) -> usize {
        if self.busy.compare_exchange(false,true,Ordering::Acquire,Ordering::Relaxed).is_err() { return 0; }
        let ring=unsafe { &mut *self.ring.get() }; let mut n=0;
        while ring.len>0 {
            let record=&ring.records[ring.head];
            if record.len>out.len()-n {break;}
            out[n..n+record.len].copy_from_slice(&record.bytes[..record.len]);n+=record.len;
            ring.records[ring.head]=Record::EMPTY;ring.head=(ring.head+1)%RECORDS;ring.len-=1;
        }
        self.busy.store(false,Ordering::Release);n
    }
}
pub static JOURNAL: Journal = Journal::new();
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
static WRITES: AtomicU64 = AtomicU64::new(0);
static FAILURES: AtomicU64 = AtomicU64::new(0);
static LAST_WRITE_NS: AtomicU64 = AtomicU64::new(0);

// ------------------------=
// FUNC: status
// DESC: Reports successful durable batches, failures, dropped records and last persistence cost independently of voice success.
// ------------------=
pub fn status() -> [u64;4] {
    [WRITES.load(Ordering::Relaxed),FAILURES.load(Ordering::Relaxed),JOURNAL.lost.load(Ordering::Relaxed),LAST_WRITE_NS.load(Ordering::Relaxed)]
}
// ------------------------=
// FUNC: emit
// DESC: Formats a Java-style single-line record using JUL levels and an actual UTC timestamp when available; always includes monotonic time.
// ------------------=
#[cfg(target_os="none")]
pub fn emit(level: &str, component: &str, message: core::fmt::Arguments<'_>) {
    let now=super::qwen::workers::clock_ns();
    let mut record=Record::EMPTY;
    let utc=super::clock::SYSTEM.utc_nanoseconds(now);
    let timestamp=utc.and_then(|ns|super::clock::local_time(ns/1_000_000_000,0).map(|t|(t,ns)));
    let result=if let Some((t,ns))=timestamp {
        write!(record,"{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",t.year,t.month,t.day,t.hour,t.minute,t.second,(ns/1_000_000)%1000)
    } else {write!(record,"UTC_UNAVAILABLE")};
    let sequence=SEQUENCE.fetch_add(1,Ordering::Relaxed)+1;
    if result.and_then(|_|write!(record," {level} infinity.ai.audio.{component} seq={sequence} mono_ns={now} {message}\n")).is_ok() {
        JOURNAL.push(record);
    } else {JOURNAL.lost.fetch_add(1,Ordering::Relaxed);}
}
struct Batch { bytes: [u8;PAGE_BYTES], len: usize, slot: u8, last: u64, saved: usize }
impl Batch {
    // ------------------------=
    // FUNC: new
    // DESC: Keeps a failed write intact for retry and caps rotating log storage to sixteen pages.
    // ------------------=
    const fn new()->Self {Self{bytes:[0;PAGE_BYTES],len:0,slot:0,last:0,saved:0}}
    // ------------------------=
    // FUNC: flush
    // DESC: Persists no more than one batch per five seconds; a failed sink cannot consume queued evidence or advance rotation.
    // ------------------=
    fn flush(&mut self,journal:&Journal,now:u64,mut sink:impl FnMut(u8,&[u8])->bool)->Option<bool> {
        if now.saturating_sub(self.last)<5_000_000_000{return None;}
        self.last=now;
        self.len+=journal.drain(&mut self.bytes[self.len..]);
        if self.len==self.saved {return None;}
        if !sink(self.slot,&self.bytes[..self.len]) {return Some(false);}
        self.saved=self.len;
        if PAGE_BYTES-self.len<RECORD_BYTES {
            self.bytes.fill(0);self.len=0;self.saved=0;self.slot=(self.slot+1)%PAGES;
        }
        Some(true)
    }
}
#[cfg(target_os="none")]
static mut BATCH: Batch=Batch::new();
// ------------------------=
// FUNC: poll
// DESC: Flushes on the console owner thread outside runtime locks, measuring disk cost without recursion into the voice pipeline.
// ------------------=
#[cfg(target_os="none")]
pub fn poll() {
    let start=super::qwen::workers::clock_ns();
    static STARTED: AtomicBool=AtomicBool::new(false);
    if !STARTED.swap(true,Ordering::Relaxed) {emit("INFO","journal",format_args!("event=logger_started schema=1 retention_bytes={} flush_interval_ms=5000",PAGE_BYTES*PAGES as usize));}
    static REPORTED_DROPS:AtomicU64=AtomicU64::new(0);
    let dropped=JOURNAL.lost.load(Ordering::Relaxed);
    if dropped!=REPORTED_DROPS.swap(dropped,Ordering::Relaxed) {emit("WARNING","journal",format_args!("event=records_dropped total={dropped}"));}
    let result=unsafe {(&mut *(&raw mut BATCH)).flush(&JOURNAL,start,|slot,bytes|crate::storage::audio_pipeline_log_commit(slot,bytes).is_ok())};
    if let Some(ok)=result {
        let elapsed=super::qwen::workers::clock_ns().saturating_sub(start);
        LAST_WRITE_NS.store(elapsed,Ordering::Relaxed);
        if ok {WRITES.fetch_add(1,Ordering::Relaxed);} else {FAILURES.fetch_add(1,Ordering::Relaxed);}
        // Serial-only sink diagnostics avoid generating a permanent self-logging loop.
        if !ok {unsafe {crate::output::write(b"[AI AUDIO LOG] persistence failed; batch retained\n");}}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: bounded_queue_preserves_complete_records
    // DESC: Verifies overflow accounting, nonblocking contention and whole-record draining using actual queued bytes.
    // ------------------=
    #[test]
    fn bounded_queue_preserves_complete_records() {
        let journal=Journal::new();let mut record=Record::EMPTY;write!(record,"INFO test\n").unwrap();
        for _ in 0..RECORDS+2{journal.push(record);}
        assert_eq!(journal.lost.load(Ordering::Relaxed),2);
        let mut small=[0;9];assert_eq!(journal.drain(&mut small),0);
        let mut out=[0;PAGE_BYTES];let n=journal.drain(&mut out);
        assert_eq!(n,RECORDS*record.len);assert!(out[..n].chunks(record.len).all(|v|v==&record.bytes[..record.len]));
        journal.busy.store(true,Ordering::Relaxed);journal.push(record);
        assert_eq!(journal.lost.load(Ordering::Relaxed),3);journal.busy.store(false,Ordering::Release);
        assert_eq!(journal.drain(&mut out),0);
    }
    // ------------------------=
    // FUNC: failed_persistence_retries_without_losing_or_reordering_records
    // DESC: Exercises retry, rate limit, durable payload preservation and wraparound through the actual batch state machine.
    // ------------------=
    #[test]
    fn failed_persistence_retries_without_losing_or_reordering_records() {
        let journal=Journal::new();let mut batch=Batch::new();let mut record=Record::EMPTY;write!(record,"INFO first\n").unwrap();journal.push(record);
        assert_eq!(batch.flush(&journal,5_000_000_000,|_,_|false),Some(false));
        assert_eq!(batch.slot,0);assert_eq!(batch.len,record.len);
        assert_eq!(batch.flush(&journal,6_000_000_000,|_,_|panic!("early write")),None);
        assert_eq!(batch.flush(&journal,10_000_000_000,|slot,bytes|{assert_eq!(slot,0);assert_eq!(bytes,&record.bytes[..record.len]);true}),Some(true));
        assert_eq!(batch.flush(&journal,15_000_000_000,|_,_|panic!("unchanged page")),None);
        let mut large=Record::EMPTY;large.bytes.fill(b'x');large.len=RECORD_BYTES;
        for step in 0..PAGES+2 {
            for _ in 0..16 {journal.push(large);}
            let old_slot=batch.slot;
            assert_eq!(batch.flush(&journal,(step as u64+4)*5_000_000_000,|slot,bytes|{assert_eq!(slot,old_slot);assert!(bytes.len()>PAGE_BYTES-RECORD_BYTES);true}),Some(true));
            assert_eq!(batch.slot,(old_slot+1)%PAGES);assert_eq!(batch.len,0);
            assert!(batch.bytes.iter().all(|v|*v==0));
        }
    }
}
