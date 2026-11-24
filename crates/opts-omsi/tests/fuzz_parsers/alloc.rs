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

unsafe impl GlobalAlloc for PeakAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        PEAK.fetch_max(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        PEAK.fetch_max(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
