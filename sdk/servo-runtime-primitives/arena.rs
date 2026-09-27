//! Reclaiming buddy allocator over an exclusively granted native memory region.
//! No host allocation, global memory discovery or framebuffer access.
use core::{alloc::Layout, marker::PhantomData, ptr::NonNull};

const ORDERS: usize = usize::BITS as usize;
const EMPTY: usize = usize::MAX;
const MIN_ORDER: usize = 4;

pub struct Arena<'a> {
    base: NonNull<u8>,
    capacity: usize,
    allocated: usize,
    peak: usize,
    heads: [usize; ORDERS],
    _owner: PhantomData<&'a mut [u8]>,
}

impl<'a> Arena<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Takes exclusive ownership of a buffer and selects an aligned power-of-two arena within it.
    // ------------------=
    pub fn new(bytes: &'a mut [u8]) -> Option<Self> {
        if bytes.len() < 1 << MIN_ORDER { return None; }
        let start = bytes.as_mut_ptr() as usize;
        let end = start.checked_add(bytes.len())?;
        let mut size = 1usize << (ORDERS - 1 - bytes.len().leading_zeros() as usize);
        loop {
            let aligned = start.checked_add(size - 1)? & !(size - 1);
            if aligned.checked_add(size)? <= end {
                let base = NonNull::new(bytes.as_mut_ptr().wrapping_add(aligned - start))?;
                let mut arena = Self { base, capacity: size, allocated: 0, peak: 0,
                    heads: [EMPTY; ORDERS], _owner: PhantomData };
                let order = size.trailing_zeros() as usize;
                arena.push(order, 0);
                return Some(arena);
            }
            size >>= 1;
            if size < 1 << MIN_ORDER { return None; }
        }
    }

    // ------------------------=
    // FUNC: order
    // DESC: Computes a checked size class satisfying both payload size and requested alignment.
    // ------------------=
    fn order(layout: Layout) -> Option<usize> {
        let size = layout.size().max(layout.align()).max(1 << MIN_ORDER).checked_next_power_of_two()?;
        Some(size.trailing_zeros() as usize)
    }

    // ------------------------=
    // FUNC: allocate
    // DESC: Splits a free block and returns disjoint aligned storage or explicit exhaustion.
    // ------------------=
    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let order = Self::order(layout)?;
        let top = self.capacity.trailing_zeros() as usize;
        if order > top { return None; }
        let found = (order..=top).find(|index| self.heads[*index] != EMPTY)?;
        let offset = self.pop(found)?;
        for index in (order..found).rev() { self.push(index, offset + (1 << index)); }
        self.allocated += 1 << order;
        self.peak = self.peak.max(self.allocated);
        // SAFETY: offset is an owned free block, now removed and split under exclusive access.
        Some(unsafe { NonNull::new_unchecked(self.base.as_ptr().add(offset)) })
    }

    // ------------------------=
    // FUNC: release
    // DESC: Reclaims an exact live allocation and merges free buddies back to larger blocks.
    // ------------------=
    /// # Safety
    /// Pointer must be a live allocation from this arena with its original layout;
    /// no outstanding use or reference may remain. Double frees are not allowed.
    pub unsafe fn release(&mut self, pointer: NonNull<u8>, layout: Layout) {
        let mut order = Self::order(layout).expect("invalid allocation layout");
        let top = self.capacity.trailing_zeros() as usize;
        let mut offset = (pointer.as_ptr() as usize).checked_sub(self.base.as_ptr() as usize)
            .expect("foreign allocation");
        assert!(order <= top && offset < self.capacity && offset % (1 << order) == 0);
        self.allocated -= 1 << order;
        while order < top {
            let buddy = offset ^ (1 << order);
            if !self.remove(order, buddy) { break; }
            offset = offset.min(buddy);
            order += 1;
        }
        self.push(order, offset);
    }

    // ------------------------=
    // FUNC: capacity
    // DESC: Reports the usable granted arena bytes after alignment trimming.
    // ------------------=
    pub fn capacity(&self) -> usize { self.capacity }
    // ------------------------=
    // FUNC: allocated
    // DESC: Reports reserved bytes including buddy rounding for resource accounting.
    // ------------------=
    pub fn allocated(&self) -> usize { self.allocated }
    // ------------------------=
    // FUNC: peak_allocated
    // DESC: Reports the exact high-water reservation, including transient blocks and buddy rounding.
    // ------------------=
    pub fn peak_allocated(&self) -> usize { self.peak }
    // ------------------------=
    // FUNC: next
    // DESC: Reads intrusive metadata only from a block currently on a free list.
    // ------------------=
    fn next(&self, offset: usize) -> usize {
        unsafe { self.base.as_ptr().add(offset).cast::<usize>().read() }
    }
    // ------------------------=
    // FUNC: link
    // DESC: Writes free-list metadata into allocator-owned, pointer-aligned free storage.
    // ------------------=
    fn link(&mut self, offset: usize, next: usize) {
        unsafe { self.base.as_ptr().add(offset).cast::<usize>().write(next); }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Adds one owned free block to its size class without allocating metadata.
    // ------------------=
    fn push(&mut self, order: usize, offset: usize) {
        self.link(offset, self.heads[order]);
        self.heads[order] = offset;
    }
    // ------------------------=
    // FUNC: pop
    // DESC: Removes the head block from a size class.
    // ------------------=
    fn pop(&mut self, order: usize) -> Option<usize> {
        let offset = self.heads[order];
        if offset == EMPTY { return None; }
        self.heads[order] = self.next(offset);
        Some(offset)
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes an exact free buddy while preserving the rest of the size-class chain.
    // ------------------=
    fn remove(&mut self, order: usize, wanted: usize) -> bool {
        let mut current = self.heads[order];
        let mut previous = EMPTY;
        while current != EMPTY {
            let next = self.next(current);
            if current == wanted {
                if previous == EMPTY { self.heads[order] = next; }
                else { self.link(previous, next); }
                return true;
            }
            previous = current;
            current = next;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[repr(align(4096))]
    struct Memory([u8; 4096]);
    // ------------------------=
    // FUNC: exhaustion_reclamation_and_alignment
    // DESC: Writes all allocations, verifies separation, then frees in scrambled order and recovers the entire arena.
    // ------------------=
    #[test]
    fn exhaustion_reclamation_and_alignment() {
        let mut memory = Memory([0; 4096]);
        let mut arena = Arena::new(&mut memory.0).unwrap();
        assert_eq!(arena.peak_allocated(), 0);
        let layout = Layout::from_size_align(23, 64).unwrap();
        let mut pointers = [NonNull::dangling(); 64];
        for (index, slot) in pointers.iter_mut().enumerate() {
            *slot = arena.allocate(layout).unwrap();
            assert_eq!(slot.as_ptr() as usize % 64, 0);
            unsafe { slot.as_ptr().write_bytes(index as u8, 23); }
        }
        assert_eq!(arena.allocated(), 4096);
        assert_eq!(arena.peak_allocated(), 4096);
        assert!(arena.allocate(layout).is_none());
        for (index, pointer) in pointers.iter().enumerate() {
            let bytes = unsafe { core::slice::from_raw_parts(pointer.as_ptr(), 23) };
            assert!(bytes.iter().all(|byte| *byte == index as u8));
        }
        for step in 0..64 { unsafe { arena.release(pointers[(step * 17) % 64], layout); } }
        assert_eq!(arena.allocated(), 0);
        assert_eq!(arena.peak_allocated(), 4096);
        let whole = Layout::from_size_align(4096, 4096).unwrap();
        assert!(arena.allocate(whole).is_some());
    }
    // ------------------------=
    // FUNC: misalignment_and_oversize_remain_bounded
    // DESC: Ensures trimming and failed requests never touch guard bytes or consume memory.
    // ------------------=
    #[test]
    fn misalignment_and_oversize_remain_bounded() {
        let mut memory = Memory([0xa5; 4096]);
        {
            let mut arena = Arena::new(&mut memory.0[3..4093]).unwrap();
            assert_eq!(arena.capacity(), 1024);
            assert!(arena.allocate(Layout::from_size_align(4096, 1).unwrap()).is_none());
            assert_eq!(arena.allocated(), 0);
            let layout = Layout::from_size_align(0, 16).unwrap();
            let pointer = arena.allocate(layout).unwrap();
            unsafe { arena.release(pointer, layout); }
        }
        assert_eq!(&memory.0[..3], &[0xa5; 3]);
        assert_eq!(&memory.0[4093..], &[0xa5; 3]);
    }
}
