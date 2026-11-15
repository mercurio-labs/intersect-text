//! Benchmark-host allocator: size-class pooling over aligned chunks.
//!
//! The runtime makes many small allocations; the Windows process heap spends
//! a large share of a load serving them. This host-side pool serves small
//! requests from 1 MiB chunks with per-class free lists and leaves large
//! requests to the system heap. `recycle`, called between loads (where the
//! Java host requests a full collection), returns completely free chunks to
//! sequential use so a later load gets the same locality as the first.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

const STEP: usize = 16;
const CLASSES: usize = 128; // 16-byte steps up to 2048
const CHUNK: usize = 1 << 20;

#[repr(C)]
struct ChunkHeader {
    /// Every chunk ever obtained, newest first.
    next: *mut ChunkHeader,
    /// Bytes handed out from this chunk, header included.
    used: usize,
    /// Scratch: bytes of this chunk found on free lists during `recycle`.
    free: usize,
    /// Next completely free chunk awaiting reuse.
    next_free: *mut ChunkHeader,
}
const HEADER: usize = std::mem::size_of::<ChunkHeader>().next_multiple_of(STEP);

struct State {
    free: [*mut u8; CLASSES],
    chunks: *mut ChunkHeader,
    free_chunks: *mut ChunkHeader,
    current: *mut ChunkHeader,
}
pub struct Pool {
    lock: AtomicBool,
    state: UnsafeCell<State>,
}
unsafe impl Sync for Pool {}

/// Large requests get their own address range, reserved well beyond the
/// request and committed on demand, so a growing buffer extends in place
/// instead of being copied to fresh pages.
const LARGE: usize = 1 << 18;
const LARGE_HEADER: usize = 16;
const PAGE: usize = 1 << 12;
#[cfg(windows)]
mod large {
    use super::{LARGE_HEADER, PAGE};
    unsafe extern "system" {
        fn VirtualAlloc(address: *mut u8, size: usize, kind: u32, protect: u32) -> *mut u8;
        fn VirtualFree(address: *mut u8, size: usize, kind: u32) -> i32;
    }
    const COMMIT: u32 = 0x1000;
    const RESERVE: u32 = 0x2000;
    const RELEASE: u32 = 0x8000;
    const READWRITE: u32 = 0x04;
    fn committed(size: usize) -> usize {
        (size + LARGE_HEADER).next_multiple_of(PAGE)
    }
    pub unsafe fn alloc(size: usize) -> *mut u8 {
        unsafe {
            let Some(reserved) = committed(size).checked_mul(8) else {
                return std::ptr::null_mut();
            };
            let reserved = reserved.next_multiple_of(1 << 16);
            let base = VirtualAlloc(std::ptr::null_mut(), reserved, RESERVE, READWRITE);
            if base.is_null() || VirtualAlloc(base, committed(size), COMMIT, READWRITE).is_null() {
                if !base.is_null() {
                    VirtualFree(base, 0, RELEASE);
                }
                return std::ptr::null_mut();
            }
            *(base as *mut usize) = reserved;
            base.add(LARGE_HEADER)
        }
    }
    /// Extend in place when the reservation allows; null otherwise.
    pub unsafe fn grow(ptr: *mut u8, old: usize, new: usize) -> *mut u8 {
        unsafe {
            let base = ptr.sub(LARGE_HEADER);
            if committed(new) > *(base as *mut usize) {
                return std::ptr::null_mut();
            }
            if committed(new) > committed(old)
                && VirtualAlloc(
                    base.add(committed(old)),
                    committed(new) - committed(old),
                    COMMIT,
                    READWRITE,
                )
                .is_null()
            {
                return std::ptr::null_mut();
            }
            ptr
        }
    }
    pub unsafe fn free(ptr: *mut u8) {
        unsafe {
            VirtualFree(ptr.sub(LARGE_HEADER), 0, RELEASE);
        }
    }
}
fn large(layout: &Layout, size: usize) -> bool {
    cfg!(windows) && size >= LARGE && layout.align() <= LARGE_HEADER
}
fn pooled(layout: &Layout, size: usize) -> bool {
    size != 0 && size <= CLASSES * STEP && layout.align() <= STEP
}
fn chunk_of(ptr: *mut u8) -> *mut ChunkHeader {
    ((ptr as usize) & !(CHUNK - 1)) as *mut ChunkHeader
}

impl Pool {
    pub const fn new() -> Self {
        Pool {
            lock: AtomicBool::new(false),
            state: UnsafeCell::new(State {
                free: [std::ptr::null_mut(); CLASSES],
                chunks: std::ptr::null_mut(),
                free_chunks: std::ptr::null_mut(),
                current: std::ptr::null_mut(),
            }),
        }
    }
    fn lock(&self) -> &mut State {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        unsafe { &mut *self.state.get() }
    }
    fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
    /// Return every chunk whose blocks are all free to sequential use.
    pub fn recycle(&self) {
        let s = self.lock();
        unsafe {
            // Count free bytes per chunk.
            for class in 0..CLASSES {
                let mut block = s.free[class];
                while !block.is_null() {
                    (*chunk_of(block)).free += (class + 1) * STEP;
                    block = *(block as *mut *mut u8);
                }
            }
            // Keep only blocks of chunks that still hold live allocations.
            for class in 0..CLASSES {
                let mut block = s.free[class];
                let mut kept: *mut u8 = std::ptr::null_mut();
                while !block.is_null() {
                    let next = *(block as *mut *mut u8);
                    let chunk = chunk_of(block);
                    if chunk == s.current || (*chunk).free + HEADER != (*chunk).used {
                        *(block as *mut *mut u8) = kept;
                        kept = block;
                    }
                    block = next;
                }
                s.free[class] = kept;
            }
            let mut chunk = s.chunks;
            s.free_chunks = std::ptr::null_mut();
            let mut last: *mut ChunkHeader = std::ptr::null_mut();
            // Oldest chunks first, so reuse follows the original order.
            let mut ordered: *mut ChunkHeader = std::ptr::null_mut();
            while !chunk.is_null() {
                let free = (*chunk).free;
                (*chunk).free = 0;
                if chunk != s.current && (*chunk).used > HEADER && free + HEADER == (*chunk).used {
                    (*chunk).used = HEADER;
                    (*chunk).next_free = ordered;
                    ordered = chunk;
                } else if chunk != s.current && (*chunk).used == HEADER {
                    // Already empty and waiting.
                    (*chunk).next_free = ordered;
                    ordered = chunk;
                }
                last = chunk;
                chunk = (*chunk).next;
            }
            let _ = last;
            s.free_chunks = ordered;
        }
        self.unlock();
    }
    unsafe fn bump(s: &mut State, need: usize) -> *mut u8 {
        unsafe {
            if s.current.is_null() || CHUNK - (*s.current).used < need {
                let chunk = if !s.free_chunks.is_null() {
                    let chunk = s.free_chunks;
                    s.free_chunks = (*chunk).next_free;
                    chunk
                } else {
                    let chunk = System.alloc(Layout::from_size_align_unchecked(CHUNK, CHUNK))
                        as *mut ChunkHeader;
                    if chunk.is_null() {
                        return std::ptr::null_mut();
                    }
                    (*chunk).next = s.chunks;
                    (*chunk).free = 0;
                    s.chunks = chunk;
                    chunk
                };
                (*chunk).used = HEADER;
                (*chunk).next_free = std::ptr::null_mut();
                s.current = chunk;
            }
            let chunk = s.current;
            let block = (chunk as *mut u8).add((*chunk).used);
            (*chunk).used += need;
            block
        }
    }
}
unsafe impl GlobalAlloc for Pool {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        #[cfg(windows)]
        if large(&layout, size) {
            return unsafe { large::alloc(size) };
        }
        if !pooled(&layout, size) {
            return unsafe { System.alloc(layout) };
        }
        let class = (size - 1) / STEP;
        let s = self.lock();
        let head = s.free[class];
        let out = if !head.is_null() {
            let next = unsafe { *(head as *mut *mut u8) };
            s.free[class] = next;
            #[cfg(target_arch = "x86_64")]
            unsafe {
                std::arch::x86_64::_mm_prefetch(next as *const i8, std::arch::x86_64::_MM_HINT_T0);
            }
            head
        } else {
            unsafe { Self::bump(s, (class + 1) * STEP) }
        };
        self.unlock();
        out
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let out = unsafe { self.alloc(layout) };
        // Freshly committed pages are already zero.
        if !out.is_null() && !large(&layout, layout.size()) {
            unsafe { std::ptr::write_bytes(out, 0, layout.size()) };
        }
        out
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old = layout.size();
        #[cfg(windows)]
        if large(&layout, old) && large(&layout, new_size) {
            let grown = unsafe { large::grow(ptr, old, new_size) };
            if !grown.is_null() {
                return grown;
            }
        }
        let moves = large(&layout, old) || large(&layout, new_size);
        match (pooled(&layout, old), pooled(&layout, new_size)) {
            // Same size class: the block already fits.
            (true, true) if (old - 1) / STEP == (new_size - 1) / STEP => ptr,
            (false, false) if old != 0 && new_size != 0 && !moves => unsafe {
                System.realloc(ptr, layout, new_size)
            },
            _ => unsafe {
                let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
                let new = self.alloc(new_layout);
                if !new.is_null() {
                    std::ptr::copy_nonoverlapping(ptr, new, old.min(new_size));
                    self.dealloc(ptr, layout);
                }
                new
            },
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        #[cfg(windows)]
        if large(&layout, size) {
            return unsafe { large::free(ptr) };
        }
        if !pooled(&layout, size) {
            return unsafe { System.dealloc(ptr, layout) };
        }
        let class = (size - 1) / STEP;
        let s = self.lock();
        unsafe { *(ptr as *mut *mut u8) = s.free[class] };
        s.free[class] = ptr;
        self.unlock();
    }
}
