use crate::alloc::{GlobalAlloc, Layout, System};
unsafe extern "C" {
    fn infinity_std_allocate(size: usize, align: usize) -> *mut u8;
    fn infinity_std_deallocate(ptr: *mut u8, size: usize, align: usize);
}
// SAFETY: Native ABI must provide disjoint aligned allocations and concurrent safety.
#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    // ------------------------=
    // FUNC: alloc
    // DESC: Requests native private memory; null propagates allocation failure.
    // ------------------=
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { infinity_std_allocate(layout.size(), layout.align()) }
    }
    // ------------------------=
    // FUNC: dealloc
    // DESC: Returns memory to the same owning native allocator.
    // ------------------=
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { infinity_std_deallocate(ptr, layout.size(), layout.align()) }
    }
}
