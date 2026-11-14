//! Optional host-only requested-heap accounting. Run separately from timing trials.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
struct Tracking;
#[global_allocator]
static ALLOCATOR: Tracking = Tracking;
fn allocated(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Relaxed) + bytes;
    PEAK.fetch_max(live, Relaxed);
    CALLS.fetch_add(1, Relaxed);
    BYTES.fetch_add(bytes, Relaxed);
}
// SAFETY: forwards all layouts and pointers unchanged to System; counters do not
// allocate or dereference user pointers. Failed allocation leaves counters intact.
unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            allocated(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            allocated(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe {
            System.dealloc(p, layout);
        }
        LIVE.fetch_sub(layout.size(), Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, layout, size) };
        if !next.is_null() {
            LIVE.fetch_sub(layout.size(), Relaxed);
            allocated(size);
        }
        next
    }
}
pub fn begin() -> (usize, usize, usize) {
    let live = LIVE.load(Relaxed);
    PEAK.store(live, Relaxed);
    (live, CALLS.load(Relaxed), BYTES.load(Relaxed))
}
pub fn finish(start: (usize, usize, usize)) -> serde_json::Value {
    let live = LIVE.load(Relaxed);
    let peak = PEAK.load(Relaxed);
    let calls = CALLS.load(Relaxed) - start.1;
    let bytes = BYTES.load(Relaxed) - start.2;
    serde_json::json!({ "baseline_live_bytes": start.0, "end_live_bytes": live,
        "peak_live_bytes": peak, "allocation_calls": calls, "allocated_bytes": bytes })
}
