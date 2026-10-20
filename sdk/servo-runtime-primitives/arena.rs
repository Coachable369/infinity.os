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
    failed_request: usize,
    heads: [usize; ORDERS],
    free_orders: Option<NonNull<u8>>,
    _owner: PhantomData<&'a mut [u8]>,
}

impl<'a> Arena<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Partitions the granted buffer into aligned buddy roots without discarding half of its capacity.
    // ------------------=
    pub fn new(bytes: &'a mut [u8]) -> Option<Self> {
        if bytes.len() < 1 << MIN_ORDER { return None; }
        let start = bytes.as_mut_ptr() as usize;
        let minimum = 1usize << MIN_ORDER;
        let end = start.checked_add(bytes.len())? & !(minimum - 1);
        let mut aligned = start.checked_add(minimum - 1)? & !(minimum - 1);
        // Large browser heaps need constant-time buddy membership checks. Keep
        // one byte per minimum block inside the grant, never in host memory.
        // Small arenas retain the metadata-free path and exact tiny capacities.
        let free_orders = if end.checked_sub(aligned)? > 65536 {
            let count = (end - aligned) / minimum;
            let tags = NonNull::new(bytes.as_mut_ptr().wrapping_add(aligned - start))?;
            unsafe { tags.as_ptr().write_bytes(0, count); }
            aligned = aligned.checked_add(count)?.checked_add(minimum - 1)? & !(minimum - 1);
            Some(tags)
        } else { None };
        let capacity = end.checked_sub(aligned)?;
        if capacity < minimum { return None; }
        let base = NonNull::new(bytes.as_mut_ptr().wrapping_add(aligned - start))?;
        let mut arena = Self { base, capacity, allocated: 0, peak: 0, failed_request: 0,
            heads: [EMPTY; ORDERS], free_orders, _owner: PhantomData };
        let mut offset = 0;
        while offset < capacity {
            let address = aligned + offset;
            let remaining = capacity - offset;
            let order = (address.trailing_zeros() as usize)
                .min(ORDERS - 1 - remaining.leading_zeros() as usize);
            arena.push(order, offset);
            offset += 1usize << order;
        }
        Some(arena)
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
        let Some(order) = Self::order(layout) else { self.failed_request = layout.size(); return None; };
        let top = ORDERS - 1 - self.capacity.leading_zeros() as usize;
        if order > top { self.failed_request = layout.size(); return None; }
        let Some(found) = (order..=top).find(|index| self.heads[*index] != EMPTY) else {
            self.failed_request = layout.size(); return None;
        };
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
        let top = ORDERS - 1 - self.capacity.leading_zeros() as usize;
        let mut offset = (pointer.as_ptr() as usize).checked_sub(self.base.as_ptr() as usize)
            .expect("foreign allocation");
        assert!(order <= top && offset < self.capacity && pointer.as_ptr() as usize % (1 << order) == 0);
        self.allocated -= 1 << order;
        while order < top {
            let address = (self.base.as_ptr() as usize + offset) ^ (1 << order);
            let Some(buddy) = address.checked_sub(self.base.as_ptr() as usize) else { break; };
            if buddy >= self.capacity || (1usize << order) > self.capacity - buddy { break; }
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
    // FUNC: failed_request
    // DESC: Reports the latest exhausted request without allocating diagnostic storage.
    // ------------------=
    pub fn failed_request(&self) -> usize { self.failed_request }
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
    // FUNC: previous
    // DESC: Reads the backward link from an indexed free block only.
    // ------------------=
    fn previous(&self, offset: usize) -> usize {
        unsafe { self.base.as_ptr().add(offset).cast::<usize>().add(1).read() }
    }
    // ------------------------=
    // FUNC: link_previous
    // DESC: Maintains constant-time unlink metadata within the minimum sixteen-byte free block.
    // ------------------=
    fn link_previous(&mut self, offset: usize, previous: usize) {
        unsafe { self.base.as_ptr().add(offset).cast::<usize>().add(1).write(previous); }
    }
    // ------------------------=
    // FUNC: mark_free
    // DESC: Records size-class membership without trusting bytes belonging to live allocations.
    // ------------------=
    fn mark_free(&mut self, offset: usize, order: Option<usize>) {
        if let Some(tags) = self.free_orders {
            unsafe { tags.as_ptr().add(offset >> MIN_ORDER).write(order.map_or(0, |value| (value + 1) as u8)); }
        }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Adds one owned free block to its size class without allocating metadata.
    // ------------------=
    fn push(&mut self, order: usize, offset: usize) {
        let head = self.heads[order];
        self.link(offset, head);
        self.link_previous(offset, EMPTY);
        if head != EMPTY { self.link_previous(head, offset); }
        self.heads[order] = offset;
        self.mark_free(offset, Some(order));
    }
    // ------------------------=
    // FUNC: pop
    // DESC: Removes the head block from a size class.
    // ------------------=
    fn pop(&mut self, order: usize) -> Option<usize> {
        let offset = self.heads[order];
        if offset == EMPTY { return None; }
        self.unlink(order, offset, EMPTY);
        Some(offset)
    }
    // ------------------------=
    // FUNC: unlink
    // DESC: Removes a known member without scanning unrelated free blocks.
    // ------------------=
    fn unlink(&mut self, order: usize, offset: usize, previous: usize) {
        let next = self.next(offset);
        if previous == EMPTY { self.heads[order] = next; }
        else { self.link(previous, next); }
        if next != EMPTY { self.link_previous(next, previous); }
        self.mark_free(offset, None);
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes an exact free buddy while preserving the rest of the size-class chain.
    // ------------------=
    fn remove(&mut self, order: usize, wanted: usize) -> bool {
        if let Some(tags) = self.free_orders {
            if unsafe { tags.as_ptr().add(wanted >> MIN_ORDER).read() } != (order + 1) as u8 {
                return false;
            }
            self.unlink(order, wanted, self.previous(wanted));
            return true;
        }
        let mut current = self.heads[order];
        let mut previous = EMPTY;
        while current != EMPTY {
            let next = self.next(current);
            if current == wanted {
                self.unlink(order, current, previous);
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
    extern crate std;
    use super::*;
    #[repr(align(4096))]
    struct Memory([u8; 4096]);
    // ------------------------=
    // FUNC: indexed_heap_reclaims_fragmented_live_payloads
    // DESC: Exercises indexed unlink under large fragmentation, overwritten payloads, and complete reuse.
    // ------------------=
    #[test]
    fn indexed_heap_reclaims_fragmented_live_payloads() {
        let mut memory = std::vec![0xa5u8; 2 * 1024 * 1024 + 32];
        let mut arena = Arena::new(&mut memory[16..2 * 1024 * 1024 + 16]).unwrap();
        assert!(arena.free_orders.is_some());
        let layout = Layout::from_size_align(16, 16).unwrap();
        let mut blocks = std::vec::Vec::new();
        while let Some(pointer) = arena.allocate(layout) {
            unsafe { pointer.as_ptr().write_bytes(0xff, 16); }
            blocks.push(pointer);
        }
        let count = blocks.len();
        assert!(count > 100_000);
        assert_eq!(arena.allocated(), count * 16);
        // Leave a long free chain with all buddies live; then merge it in the
        // opposite order. Live bytes deliberately resemble invalid pointers.
        for index in (0..count).step_by(2) {
            unsafe { arena.release(blocks[index], layout); }
        }
        for index in (1..count).step_by(2).rev() {
            unsafe {
                assert!(core::slice::from_raw_parts(blocks[index].as_ptr(), 16).iter().all(|b| *b == 0xff));
                arena.release(blocks[index], layout);
            }
        }
        assert_eq!(arena.allocated(), 0);
        let mut recovered = 0;
        while arena.allocate(layout).is_some() { recovered += 1; }
        assert_eq!(recovered, count);
        drop(arena);
        assert!(memory[..16].iter().all(|byte| *byte == 0xa5));
        assert!(memory[2 * 1024 * 1024 + 16..].iter().all(|byte| *byte == 0xa5));
    }
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
        assert_eq!(arena.failed_request(), 23);
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
            assert_eq!(arena.capacity(), 4064);
            assert!(arena.allocate(Layout::from_size_align(4096, 1).unwrap()).is_none());
            assert_eq!(arena.failed_request(), 4096);
            assert_eq!(arena.allocated(), 0);
            let layout = Layout::from_size_align(0, 16).unwrap();
            let pointer = arena.allocate(layout).unwrap();
            unsafe { arena.release(pointer, layout); }
        }
        assert_eq!(&memory.0[..3], &[0xa5; 3]);
        assert_eq!(&memory.0[4093..], &[0xa5; 3]);
    }
    // ------------------------=
    // FUNC: multiple_roots_reclaim_without_crossing_grant
    // DESC: Exhausts a non-power-of-two grant and restores every aligned root after scrambled frees.
    // ------------------=
    #[test]
    fn multiple_roots_reclaim_without_crossing_grant() {
        let mut memory = Memory([0xa5; 4096]);
        {
            let mut arena = Arena::new(&mut memory.0[16..4080]).unwrap();
            let small = Layout::from_size_align(16, 16).unwrap();
            let mut pointers = [NonNull::dangling(); 254];
            for (index, slot) in pointers.iter_mut().enumerate() {
                *slot = arena.allocate(small).unwrap();
                unsafe { slot.as_ptr().write_bytes(index as u8, 16); }
            }
            assert_eq!(arena.allocated(), 4064);
            assert!(arena.allocate(small).is_none());
            for (index, pointer) in pointers.iter().enumerate() {
                assert!(unsafe { core::slice::from_raw_parts(pointer.as_ptr(), 16) }
                    .iter().all(|byte| *byte == index as u8));
            }
            for step in 0..254 { unsafe { arena.release(pointers[(step * 17) % 254], small); } }
            assert_eq!(arena.allocated(), 0);
            for order in (4..=10).rev() {
                let layout = Layout::from_size_align(1 << order, 1 << order).unwrap();
                for _ in 0..2 {
                    let pointer = arena.allocate(layout).unwrap();
                    assert_eq!(pointer.as_ptr() as usize % (1 << order), 0);
                }
            }
            assert_eq!(arena.allocated(), 4064);
        }
        assert_eq!(&memory.0[..16], &[0xa5; 16]);
        assert_eq!(&memory.0[4080..], &[0xa5; 16]);
    }
}
