//! Native C event ABI backed by the existing capability-governed IEF queues.
use super::{InfinityRuntime, execution::SecurityIdentity,
    event::{EventError, EventFilter, OverflowPolicy, EVENT_QUEUE_CAPACITY},
    native_memory::{PageTable, copy_to_user, check_user_range}};

pub const EVENT_BYTES: usize = 128;

impl InfinityRuntime {
    // ------------------------=
    // FUNC: native_event_syscall
    // DESC: Provides bounded subscriptions and checked event copies; invalid user memory never consumes an event.
    // ------------------=
    pub fn native_event_syscall<T: PageTable>(&mut self, caller: SecurityIdentity,
        memory: &mut T, operation: u64, args: [u64; 6], now: u64) -> i64 {
        match operation {
            0x1110 => {
                let (Ok(type_id), Ok(limit)) = (u32::try_from(args[1]), usize::try_from(args[4])) else { return -3; };
                if limit == 0 || limit > EVENT_QUEUE_CAPACITY || args[3] <= now || args[5] != 0 { return -3; }
                match self.events.subscribe(caller, args[0], EventFilter { type_id, scope: Some(args[2]) },
                    args[3], OverflowPolicy::DropOldest, limit, &self.capabilities, now) {
                    Ok(lease) => lease as i64, Err(error) => abi_error(error),
                }
            }
            0x1111 => {
                if args[2] < EVENT_BYTES as u64 { return -3; }
                let (event, gap) = match self.events.peek_authorized(caller, args[0], &self.capabilities, now) {
                    Ok(Some(event)) => event, Ok(None) => return 0, Err(error) => return abi_error(error),
                };
                if check_user_range(memory, args[1], EVENT_BYTES, true).is_err() { return -5; }
                let mut bytes = [0u8; EVENT_BYTES];
                bytes[..4].copy_from_slice(&1u32.to_le_bytes());
                bytes[4..8].copy_from_slice(&event.type_id.to_le_bytes());
                bytes[8..16].copy_from_slice(&event.sequence.to_le_bytes());
                bytes[16..24].copy_from_slice(&event.timestamp.to_le_bytes());
                bytes[24..28].copy_from_slice(&(event.payload_len as u32).to_le_bytes());
                bytes[28..32].copy_from_slice(&(gap as u32).to_le_bytes());
                bytes[32..32 + event.payload_len as usize].copy_from_slice(&event.payload[..event.payload_len as usize]);
                if copy_to_user(memory, args[1], &bytes).is_err() { return -5; }
                match self.events.receive_authorized(caller, args[0], &self.capabilities, now) {
                    Ok(Some(_)) => 1, Ok(None) => 0, Err(error) => abi_error(error),
                }
            }
            0x1112 => match self.events.unsubscribe_owned(caller, args[0]) {
                Ok(()) => 0, Err(error) => abi_error(error),
            },
            _ => -6,
        }
    }
}

// ------------------------=
// FUNC: abi_error
// DESC: Maps service failures to native errors without revealing another process's queued data.
// ------------------=
fn abi_error(error: EventError) -> i64 {
    match error { EventError::AccessDenied => -1, EventError::UnknownSubscription => -2,
        EventError::Full | EventError::QueueFull => -4, _ => -3 }
}
