use super::capability::{CapabilityId, CapabilityManager, CapabilityType};
use super::execution::SecurityIdentity;

pub const MAX_SUBSCRIPTIONS: usize = 16;
pub const EVENT_QUEUE_CAPACITY: usize = 8;
pub const MAX_EVENT_PAYLOAD: usize = 96;
pub const RECORD_CAPACITY: usize = 32;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventClass {
    Signal,
    StateChange,
    Record,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoutingDomain {
    System,
    Storage,
    Device,
    Session,
    Application,
    FutureAi,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OverflowPolicy {
    DropOldest,
    DropNewest,
    LatestOnly,
    Coalesce,
    DisconnectSlowSubscriber,
    Durable,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Event {
    pub id: u64,
    pub type_id: u32,
    pub payload_version: u16,
    pub class: EventClass,
    pub domain: RoutingDomain,
    pub source: SecurityIdentity,
    pub sequence: u64,
    pub timestamp: u64,
    pub priority: u8,
    pub scope: u64,
    pub correlation_id: u64,
    pub causation_id: u64,
    pub payload_len: u8,
    pub payload: [u8; MAX_EVENT_PAYLOAD],
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EventFilter {
    pub type_id: u32,
    pub scope: Option<u64>,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventError {
    Full,
    UnknownSubscription,
    AccessDenied,
    RateLimited,
    PayloadTooLarge,
    QueueFull,
}
#[derive(Clone, Copy)]
struct Subscription {
    lease_id: u64,
    holder: SecurityIdentity,
    capability: CapabilityId,
    filter: EventFilter,
    expires_at: u64,
    overflow: OverflowPolicy,
    queue_limit: usize,
    queue: [Option<Event>; EVENT_QUEUE_CAPACITY],
    head: usize,
    len: usize,
    last_sequence: u64,
    gap: bool,
    connected: bool,
}
impl Subscription {
    // ------------------------=
    // FUNC: push
    // DESC: Implements the push operation.
    // ------------------=
    fn push(&mut self, event: Event) -> Result<(), EventError> {
        let limit = self.queue_limit.min(EVENT_QUEUE_CAPACITY).max(1);
        if self.len >= limit {
            match self.overflow {
                OverflowPolicy::DropOldest => {
                    self.queue[self.head] = None;
                    self.head = (self.head + 1) % EVENT_QUEUE_CAPACITY;
                    self.len -= 1
                }
                OverflowPolicy::DropNewest => return Ok(()),
                OverflowPolicy::LatestOnly => {
                    self.queue = [None; EVENT_QUEUE_CAPACITY];
                    self.head = 0;
                    self.len = 0
                }
                OverflowPolicy::Coalesce => {
                    if let Some(last) =
                        self.queue[(self.head + self.len - 1) % EVENT_QUEUE_CAPACITY].as_mut()
                    {
                        last.sequence = event.sequence;
                        last.timestamp = event.timestamp;
                        last.payload = event.payload;
                        last.payload_len = event.payload_len;
                        return Ok(());
                    }
                }
                OverflowPolicy::DisconnectSlowSubscriber => {
                    self.connected = false;
                    return Err(EventError::QueueFull);
                }
                OverflowPolicy::Durable => return Err(EventError::QueueFull),
            }
        }
        let at = (self.head + self.len) % EVENT_QUEUE_CAPACITY;
        self.queue[at] = Some(event);
        self.len += 1;
        Ok(())
    }
    // ------------------------=
    // FUNC: pop
    // DESC: Implements the pop operation.
    // ------------------=
    fn pop(&mut self) -> Option<Event> {
        if self.len == 0 {
            return None;
        }
        let e = self.queue[self.head].take();
        self.head = (self.head + 1) % EVENT_QUEUE_CAPACITY;
        self.len -= 1;
        if let Some(event) = e {
            if self.last_sequence != 0 && event.sequence != self.last_sequence + 1 {
                self.gap = true
            }
            self.last_sequence = event.sequence;
        }
        e
    }
}
pub struct EventFabric {
    subscriptions: [Option<Subscription>; MAX_SUBSCRIPTIONS],
    records: [Option<Event>; RECORD_CAPACITY],
    record_len: usize,
    next_event: u64,
    next_lease: u64,
    topic_sequences: [(u32, u64); 16],
    topic_count: usize,
    window_second: u64,
    window_count: u32,
    events_per_second: u32,
    burst_limit: u32,
}
impl EventFabric {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            subscriptions: [None; MAX_SUBSCRIPTIONS],
            records: [None; RECORD_CAPACITY],
            record_len: 0,
            next_event: 1,
            next_lease: 1,
            topic_sequences: [(0, 0); 16],
            topic_count: 0,
            window_second: 0,
            window_count: 0,
            events_per_second: 128,
            burst_limit: 32,
        }
    }
    // ------------------------=
    // FUNC: set_rate_limit
    // DESC: Writes or updates set rate limit data.
    // ------------------=
    pub fn set_rate_limit(&mut self, events_per_second: u32, burst_limit: u32) {
        self.events_per_second = events_per_second;
        self.burst_limit = burst_limit
    }
    // ------------------------=
    // FUNC: subscribe
    // DESC: Implements the subscribe operation.
    // ------------------=
    pub fn subscribe(
        &mut self,
        holder: SecurityIdentity,
        capability: CapabilityId,
        filter: EventFilter,
        expires_at: u64,
        overflow: OverflowPolicy,
        queue_limit: usize,
        capabilities: &CapabilityManager,
        now: u64,
    ) -> Result<u64, EventError> {
        capabilities
            .validate(
                capability,
                holder,
                CapabilityType::EventSubscribe,
                filter.type_id as u64,
                1,
                filter.scope.unwrap_or(0),
                now,
            )
            .map_err(|_| EventError::AccessDenied)?;
        let slot = self
            .subscriptions
            .iter()
            .position(Option::is_none)
            .ok_or(EventError::Full)?;
        let id = self.next_lease;
        self.next_lease += 1;
        self.subscriptions[slot] = Some(Subscription {
            lease_id: id,
            holder,
            capability,
            filter,
            expires_at,
            overflow,
            queue_limit,
            queue: [None; EVENT_QUEUE_CAPACITY],
            head: 0,
            len: 0,
            last_sequence: 0,
            gap: false,
            connected: true,
        });
        Ok(id)
    }
    // ------------------------=
    // FUNC: next_sequence
    // DESC: Calculates and returns next sequence.
    // ------------------=
    fn next_sequence(&mut self, type_id: u32) -> u64 {
        if let Some(entry) = self.topic_sequences[..self.topic_count]
            .iter_mut()
            .find(|e| e.0 == type_id)
        {
            entry.1 += 1;
            return entry.1;
        }
        if self.topic_count < self.topic_sequences.len() {
            self.topic_sequences[self.topic_count] = (type_id, 1);
            self.topic_count += 1;
        }
        1
    }
    // ------------------------=
    // FUNC: publish
    // DESC: Implements the publish operation.
    // ------------------=
    pub fn publish(
        &mut self,
        class: EventClass,
        domain: RoutingDomain,
        type_id: u32,
        source: SecurityIdentity,
        scope: u64,
        correlation_id: u64,
        causation_id: u64,
        payload: &[u8],
        priority: u8,
        now: u64,
        capabilities: &CapabilityManager,
        publish_capability: CapabilityId,
    ) -> Result<u64, EventError> {
        if payload.len() > MAX_EVENT_PAYLOAD {
            return Err(EventError::PayloadTooLarge);
        }
        capabilities
            .validate(
                publish_capability,
                source,
                CapabilityType::EventPublish,
                type_id as u64,
                1,
                scope,
                now,
            )
            .map_err(|_| EventError::AccessDenied)?;
        let second = now / 1000;
        if second != self.window_second {
            self.window_second = second;
            self.window_count = 0
        }
        let allowance = self.events_per_second.saturating_add(self.burst_limit);
        if self.window_count >= allowance && priority < 200 {
            return Err(EventError::RateLimited);
        }
        self.window_count += 1;
        let sequence = self.next_sequence(type_id);
        let id = self.next_event;
        self.next_event += 1;
        let mut bytes = [0u8; MAX_EVENT_PAYLOAD];
        bytes[..payload.len()].copy_from_slice(payload);
        let event = Event {
            id,
            type_id,
            payload_version: 1,
            class,
            domain,
            source,
            sequence,
            timestamp: now,
            priority,
            scope,
            correlation_id,
            causation_id,
            payload_len: payload.len() as u8,
            payload: bytes,
        };
        if class == EventClass::Record {
            if self.record_len < RECORD_CAPACITY {
                self.records[self.record_len] = Some(event);
                self.record_len += 1
            } else {
                return Err(EventError::QueueFull);
            }
        }
        for subscription in self.subscriptions.iter_mut().flatten() {
            if !subscription.connected
                || now >= subscription.expires_at
                || subscription.filter.type_id != type_id
                || subscription
                    .filter
                    .scope
                    .map(|s| s != scope)
                    .unwrap_or(false)
            {
                continue;
            }
            if capabilities
                .validate(
                    subscription.capability,
                    subscription.holder,
                    CapabilityType::EventSubscribe,
                    type_id as u64,
                    1,
                    subscription.filter.scope.unwrap_or(0),
                    now,
                )
                .is_err()
            {
                continue;
            }
            let _ = subscription.push(event);
        }
        Ok(id)
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Implements the receive operation.
    // ------------------=
    pub fn receive(&mut self, lease: u64, now: u64) -> Result<Event, EventError> {
        let sub = self
            .subscriptions
            .iter_mut()
            .flatten()
            .find(|s| s.lease_id == lease)
            .ok_or(EventError::UnknownSubscription)?;
        if !sub.connected || now >= sub.expires_at {
            return Err(EventError::UnknownSubscription);
        }
        sub.pop().ok_or(EventError::QueueFull)
    }
    // ------------------------=
    // FUNC: sequence_gap
    // DESC: Implements the sequence gap operation.
    // ------------------=
    pub fn sequence_gap(&self, lease: u64) -> Result<bool, EventError> {
        self.subscriptions
            .iter()
            .flatten()
            .find(|s| s.lease_id == lease)
            .map(|s| s.gap)
            .ok_or(EventError::UnknownSubscription)
    }
    // ------------------------=
    // FUNC: queue_depth
    // DESC: Implements the queue depth operation.
    // ------------------=
    pub fn queue_depth(&self, lease: u64) -> Option<usize> {
        self.subscriptions
            .iter()
            .flatten()
            .find(|s| s.lease_id == lease)
            .map(|s| s.len)
    }
    // ------------------------=
    // FUNC: expire
    // DESC: Implements the expire operation.
    // ------------------=
    pub fn expire(&mut self, now: u64) {
        for entry in &mut self.subscriptions {
            if entry
                .map(|s| now >= s.expires_at || !s.connected)
                .unwrap_or(false)
            {
                *entry = None
            }
        }
    }
    // ------------------------=
    // FUNC: subscription_count
    // DESC: Implements the subscription count operation.
    // ------------------=
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.iter().flatten().count()
    }
    // ------------------------=
    // FUNC: record_count
    // DESC: Implements the record count operation.
    // ------------------=
    pub fn record_count(&self) -> usize {
        self.record_len
    }
}
