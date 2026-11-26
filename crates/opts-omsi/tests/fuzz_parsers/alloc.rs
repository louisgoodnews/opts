//! Counting global allocator for the fuzz tests: forwards to `System` and records the largest
//! single request since the last reset.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static PEAK: AtomicUsize = AtomicUsize::new(0);

pub struct PeakAlloc;

/// Clears the recorded peak.
pub fn reset_peak() {
    PEAK.store(0, Ordering::Relaxed);
}

/// The largest single request since the last `reset_peak`.
pub fn peak() -> usize {
    PEAK.load(Ordering::Relaxed)
}

// SAFETY: every method forwards its arguments unchanged to `System`, which is itself a sound
// `GlobalAlloc`, so the caller's contract (a valid layout, a pointer from this allocator) is
// exactly the one `System` requires. The only extra state is an atomic peak, which is safe to
// update from any thread.
unsafe impl GlobalAlloc for PeakAlloc {
    // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract (non-zero size, valid layout),
    // and `System.alloc` requires the same, so passing `layout` on is sound.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        PEAK.fetch_max(layout.size(), Ordering::Relaxed);
        // SAFETY: `layout` is the caller's, unchanged, so `System.alloc`'s contract holds.
        unsafe { System.alloc(layout) }
    }

    // SAFETY: the caller passes back a pointer and layout this allocator returned, which is
    // exactly what `System.dealloc` requires.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` and `layout` are the caller's, unchanged, so `System.dealloc` holds.
        unsafe { System.dealloc(ptr, layout) }
    }

    // SAFETY: the caller passes a pointer and layout this allocator returned and a non-zero
    // `new_size`, which is exactly what `System.realloc` requires.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        PEAK.fetch_max(new_size, Ordering::Relaxed);
        // SAFETY: the arguments are the caller's, unchanged, so `System.realloc`'s contract holds.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
