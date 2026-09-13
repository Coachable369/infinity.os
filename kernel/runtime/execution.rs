//! Architecture-neutral execution-context identities and MMU ownership.
//! Native contexts bind to a CR3-compatible root only after the architecture
//! mapper has completed every required mapping and protection transition.

pub const MAX_CONTEXTS: usize = 48;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SecurityIdentity(pub [u8; 16]);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ContextHandle(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AddressSpaceToken(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContextState {
    Defined,
    Runnable,
    Running,
    Waiting,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PriorityClass {
    Background,
    Normal,
    System,
    Critical,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MemoryRegion {
    pub base: u64,
    pub length: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResourceBudget {
    pub memory_limit: u64,
    pub cpu_weight: u16,
    pub message_queue_limit: u16,
    pub io_priority: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ResourceUsage {
    pub memory_bytes: u64,
    pub cpu_ticks: u64,
    pub queued_messages: u16,
    pub io_units: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExecutionContext {
    pub handle: ContextHandle,
    pub security_identity: SecurityIdentity,
    pub service_identity: u32,
    pub image_identity: u32,
    pub address_space: AddressSpaceToken,
    pub memory: MemoryRegion,
    pub endpoint: u16,
    pub priority: PriorityClass,
    pub state: ContextState,
    pub budget: ResourceBudget,
    pub usage: ResourceUsage,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExecutionError {
    Full,
    InvalidRegion,
    RegionOverlap,
    UnknownContext,
    BudgetExceeded,
    InvalidAddressSpace,
}

pub struct ExecutionManager {
    contexts: [Option<ExecutionContext>; MAX_CONTEXTS],
    next_handle: u16,
    next_identity: u64,
}

impl ExecutionManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            contexts: [None; MAX_CONTEXTS],
            next_handle: 1,
            next_identity: 1,
        }
    }

    // ------------------------=
    // FUNC: create
    // DESC: Implements the create operation.
    // ------------------=
    pub fn create(
        &mut self,
        service_identity: u32,
        image_identity: u32,
        memory: MemoryRegion,
        endpoint: u16,
        priority: PriorityClass,
        budget: ResourceBudget,
    ) -> Result<ContextHandle, ExecutionError> {
        if memory.length == 0 || memory.base.checked_add(memory.length).is_none() {
            return Err(ExecutionError::InvalidRegion);
        }
        let end = memory.base + memory.length;
        if self.contexts.iter().flatten().any(|context| {
            let other_end = context.memory.base + context.memory.length;
            memory.base < other_end && context.memory.base < end
        }) {
            return Err(ExecutionError::RegionOverlap);
        }
        let slot = self
            .contexts
            .iter()
            .position(Option::is_none)
            .ok_or(ExecutionError::Full)?;
        let handle = ContextHandle(self.next_handle);
        self.next_handle = self.next_handle.wrapping_add(1).max(1);
        let nonce = self.next_identity;
        self.next_identity = self.next_identity.wrapping_add(1).max(1);
        let mut identity = [0u8; 16];
        identity[..4].copy_from_slice(&service_identity.to_le_bytes());
        identity[4..8].copy_from_slice(&image_identity.to_le_bytes());
        identity[8..].copy_from_slice(&nonce.to_le_bytes());
        self.contexts[slot] = Some(ExecutionContext {
            handle,
            security_identity: SecurityIdentity(identity),
            service_identity,
            image_identity,
            address_space: AddressSpaceToken(0),
            memory,
            endpoint,
            priority,
            state: ContextState::Defined,
            budget,
            usage: ResourceUsage::default(),
        });
        Ok(handle)
    }

    // ------------------------=
    // FUNC: get
    // DESC: Implements the get operation.
    // ------------------=
    pub fn get(&self, handle: ContextHandle) -> Option<&ExecutionContext> {
        self.contexts
            .iter()
            .flatten()
            .find(|context| context.handle == handle)
    }
    // ------------------------=
    // FUNC: get_mut
    // DESC: Reads get mut data.
    // ------------------=
    pub fn get_mut(&mut self, handle: ContextHandle) -> Option<&mut ExecutionContext> {
        self.contexts
            .iter_mut()
            .flatten()
            .find(|context| context.handle == handle)
    }
    // ------------------------=
    // FUNC: count
    // DESC: Implements the count operation.
    // ------------------=
    pub fn count(&self) -> usize {
        self.contexts.iter().flatten().count()
    }
    // ------------------------=
    // FUNC: nth
    // DESC: Calculates and returns nth.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<&ExecutionContext> {
        self.contexts.iter().flatten().nth(index)
    }
    // ------------------------=
    // FUNC: set_state
    // DESC: Writes or updates set state data.
    // ------------------=
    pub fn set_state(
        &mut self,
        handle: ContextHandle,
        state: ContextState,
    ) -> Result<(), ExecutionError> {
        self.get_mut(handle)
            .ok_or(ExecutionError::UnknownContext)
            .map(|context| context.state = state)
    }
    // ------------------------=
    // FUNC: account_memory
    // DESC: Implements the account memory operation.
    // ------------------=
    pub fn account_memory(
        &mut self,
        handle: ContextHandle,
        bytes: u64,
    ) -> Result<(), ExecutionError> {
        let context = self.get_mut(handle).ok_or(ExecutionError::UnknownContext)?;
        if bytes > context.budget.memory_limit {
            return Err(ExecutionError::BudgetExceeded);
        }
        context.usage.memory_bytes = bytes;
        Ok(())
    }
    // ------------------------=
    // FUNC: account_queue
    // DESC: Implements the account queue operation.
    // ------------------=
    pub fn account_queue(
        &mut self,
        handle: ContextHandle,
        messages: u16,
    ) -> Result<(), ExecutionError> {
        let context = self.get_mut(handle).ok_or(ExecutionError::UnknownContext)?;
        if messages > context.budget.message_queue_limit {
            return Err(ExecutionError::BudgetExceeded);
        }
        context.usage.queued_messages = messages;
        Ok(())
    }
    // ------------------------=
    // FUNC: account_cpu_tick
    // DESC: Implements the account cpu tick operation.
    // ------------------=
    pub fn account_cpu_tick(&mut self, handle: ContextHandle) -> Result<(), ExecutionError> {
        let context = self.get_mut(handle).ok_or(ExecutionError::UnknownContext)?;
        context.usage.cpu_ticks = context.usage.cpu_ticks.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: bind_address_space
    // DESC: Binds a context to a nonzero page-aligned architecture root after mapping validation succeeds.
    // ------------------=
    pub fn bind_address_space(&mut self, handle: ContextHandle, root: u64) -> Result<(), ExecutionError> {
        if root == 0 || root & 0xfff != 0 { return Err(ExecutionError::InvalidAddressSpace); }
        self.get_mut(handle).ok_or(ExecutionError::UnknownContext)?
            .address_space = AddressSpaceToken(root);
        Ok(())
    }
    // ------------------------=
    // FUNC: fail
    // DESC: Implements the fail operation.
    // ------------------=
    pub fn fail(&mut self, handle: ContextHandle) -> Result<(), ExecutionError> {
        self.set_state(handle, ContextState::Failed)
    }

    // ------------------------=
    // FUNC: update_budget
    // DESC: Atomically changes enforceable resource and priority limits when current usage fits.
    // ------------------=
    pub fn update_budget(
        &mut self,
        handle: ContextHandle,
        budget: ResourceBudget,
        priority: PriorityClass,
    ) -> Result<(), ExecutionError> {
        let context = self.get_mut(handle).ok_or(ExecutionError::UnknownContext)?;
        if context.usage.memory_bytes > budget.memory_limit
            || context.usage.queued_messages > budget.message_queue_limit
        {
            return Err(ExecutionError::BudgetExceeded);
        }
        context.budget = budget;
        context.priority = priority;
        Ok(())
    }

    // ------------------------=
    // FUNC: relaunch
    // DESC: Rotates runtime identity, clears accounting, and makes a retained context runnable again.
    // ------------------=
    pub fn relaunch(&mut self, handle: ContextHandle) -> Result<(), ExecutionError> {
        let nonce = self.next_identity;
        self.next_identity = self.next_identity.wrapping_add(1).max(1);
        let context = self.get_mut(handle).ok_or(ExecutionError::UnknownContext)?;
        let mut identity = [0u8; 16];
        identity[..4].copy_from_slice(&context.service_identity.to_le_bytes());
        identity[4..8].copy_from_slice(&context.image_identity.to_le_bytes());
        identity[8..].copy_from_slice(&nonce.to_le_bytes());
        context.security_identity = SecurityIdentity(identity);
        context.usage = ResourceUsage::default();
        context.state = ContextState::Runnable;
        Ok(())
    }
    // ------------------------=
    // FUNC: destroy
    // DESC: Implements the destroy operation.
    // ------------------=
    pub fn destroy(&mut self, handle: ContextHandle) -> Result<(), ExecutionError> {
        let slot = self
            .contexts
            .iter()
            .position(|entry| entry.map(|c| c.handle) == Some(handle))
            .ok_or(ExecutionError::UnknownContext)?;
        self.contexts[slot] = None;
        Ok(())
    }
}
