//! Static executable TLS only: x86-64 variant II and AArch64 variant I.
//! Dynamic modules/TLSDESC and general-dynamic relocations remain unsupported.
use super::ImageError;
pub const MAX_TLS: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Template {
    pub source: usize,
    pub address: usize,
    pub file_size: usize,
    pub memory_size: usize,
    pub alignment: usize,
}
pub struct Block<'a> {
    storage: &'a mut [u8],
    thread_pointer: usize,
    data: usize,
    length: usize,
}

// ------------------------=
// FUNC: align
// DESC: Rounds an address up without overflowing on malicious ELF metadata.
// ------------------=
fn align(value: usize, alignment: usize) -> Result<usize, ImageError> {
    value
        .checked_add(alignment - 1)
        .map(|v| v & !(alignment - 1))
        .ok_or(ImageError::Bounds)
}

impl Template {
    // ------------------------=
    // FUNC: validate
    // DESC: Rejects oversized, misaligned or truncated static TLS templates before allocating or copying.
    // ------------------=
    pub fn validate(&self, image: &[u8]) -> Result<(), ImageError> {
        if !self.alignment.is_power_of_two()
            || self.alignment > 4096
            || self.address % self.alignment != 0
            || self.source % self.alignment != 0
            || self.memory_size > MAX_TLS
            || self.file_size > self.memory_size
            || self
                .source
                .checked_add(self.file_size)
                .filter(|end| *end <= image.len())
                .is_none()
            || self.address.checked_add(self.memory_size).is_none()
        {
            return Err(ImageError::Bounds);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: initialize
    // DESC: Copies tdata and zeroes tbss into uniquely borrowed stable per-thread storage using the target ABI layout.
    // ------------------=
    pub fn initialize<'a>(
        &self,
        image: &[u8],
        machine: u16,
        storage: &'a mut [u8],
    ) -> Result<Block<'a>, ImageError> {
        self.validate(image)?;
        let base = storage.as_ptr() as usize;
        let alignment = self.alignment.max(16);
        let start = align(base, alignment)?
            .checked_sub(base)
            .ok_or(ImageError::Bounds)?;
        let (tp, data, end) = match machine {
            62 => {
                let tp = start + align(self.memory_size, alignment)?;
                (tp, tp - align(self.memory_size, self.alignment)?, tp + 16)
            }
            183 => {
                let data = start + align(16, self.alignment)?;
                (start, data, data + self.memory_size)
            }
            _ => return Err(ImageError::Architecture),
        };
        if end > storage.len() {
            return Err(ImageError::Bounds);
        }
        storage[..end].fill(0);
        storage[data..data + self.file_size]
            .copy_from_slice(&image[self.source..self.source + self.file_size]);
        let pointer = base.checked_add(tp).ok_or(ImageError::Bounds)?;
        if machine == 62 {
            storage[tp..tp + 8].copy_from_slice(&(pointer as u64).to_le_bytes());
        }
        Ok(Block {
            storage,
            thread_pointer: pointer,
            data,
            length: self.memory_size,
        })
    }
}
impl Block<'_> {
    // ------------------------=
    // FUNC: thread_pointer
    // DESC: Returns the target ABI thread pointer while the block remains uniquely borrowed and alive.
    // ------------------=
    pub fn thread_pointer(&self) -> usize {
        self.thread_pointer
    }
    // ------------------------=
    // FUNC: data
    // DESC: Exposes initialized TLS bytes for loader verification before execution.
    // ------------------=
    pub fn data(&self) -> &[u8] {
        &self.storage[self.data..self.data + self.length]
    }
}

#[cfg(all(
    target_os = "none",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
// ------------------------=
// FUNC: replace_thread_pointer
// DESC: Installs the CPU-local TLS pointer and returns the prior pointer for scheduler or trusted-invocation restoration.
// ------------------=
/// # Safety
/// Caller must run privileged, pin execution to this CPU, prevent conflicting
/// context switches, and keep the referenced TLS allocation alive until restored.
/// This is not a memory-isolation boundary. Never call from a hosted process.
pub unsafe fn replace_thread_pointer(pointer: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        let low: u32;
        let high: u32;
        core::arch::asm!("rdmsr", in("ecx") 0xc0000100u32, out("eax") low, out("edx") high, options(nostack, preserves_flags));
        core::arch::asm!("wrmsr", in("ecx") 0xc0000100u32, in("eax") pointer as u32, in("edx") (pointer >> 32) as u32, options(nostack, preserves_flags));
        ((high as usize) << 32) | low as usize
    }
    #[cfg(target_arch = "aarch64")]
    {
        let previous: usize;
        core::arch::asm!("mrs {old}, tpidr_el0", "msr tpidr_el0, {new}", "isb", old = out(reg) previous, new = in(reg) pointer, options(nostack, preserves_flags));
        previous
    }
}
