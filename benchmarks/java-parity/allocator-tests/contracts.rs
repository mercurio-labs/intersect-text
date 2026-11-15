#[path = "../host_pool.rs"]
mod allocator;

use allocator::Pool;
use std::alloc::{GlobalAlloc, Layout};

#[test]
fn resized_blocks_preserve_contents_and_required_alignment() {
    let pool = Pool::new();
    for alignment in [1, 8, 16, 64, 4096] {
        let mut layout = Layout::from_size_align(7, alignment).unwrap();
        // SAFETY: Each allocation is checked before access, resized using its
        // current layout, and freed once using its final layout.
        unsafe {
            let mut pointer = pool.alloc_zeroed(layout);
            assert!(!pointer.is_null());
            assert_eq!(pointer as usize % alignment, 0);
            assert_eq!(std::slice::from_raw_parts(pointer, 7), &[0; 7]);
            std::ptr::write_bytes(pointer, 0x5a, 7);
            for size in [15, 64, 4096, 262_144, 524_288, 32, 3] {
                let next = pool.realloc(pointer, layout, size);
                assert!(!next.is_null());
                assert_eq!(next as usize % alignment, 0);
                assert!(
                    std::slice::from_raw_parts(next, layout.size().min(size))
                        .iter()
                        .all(|byte| *byte == 0x5a)
                );
                std::ptr::write_bytes(next, 0x5a, size);
                pointer = next;
                layout = Layout::from_size_align(size, alignment).unwrap();
            }
            pool.dealloc(pointer, layout);
        }
    }
    pool.recycle();
}

#[test]
fn recycling_chunks_preserves_live_blocks_and_allows_further_allocations() {
    let pool = Pool::new();
    let layout = Layout::from_size_align(2048, 16).unwrap();
    let mut retained = Vec::new();
    // SAFETY: Each checked pointer remains live until its single matching
    // deallocation. Recycling must preserve all still-live allocations.
    unsafe {
        for index in 0..1600 {
            let pointer = pool.alloc(layout);
            assert!(!pointer.is_null());
            std::ptr::write_bytes(pointer, index as u8, layout.size());
            retained.push((pointer, index as u8));
        }
        let mut live = Vec::new();
        for (index, (pointer, value)) in retained.into_iter().enumerate() {
            if index % 3 == 0 {
                live.push((pointer, value));
            } else {
                pool.dealloc(pointer, layout);
            }
        }
        pool.recycle();
        let mut replacements = Vec::new();
        for _ in 0..1600 {
            let pointer = pool.alloc_zeroed(layout);
            assert!(!pointer.is_null());
            assert!(
                std::slice::from_raw_parts(pointer, layout.size())
                    .iter()
                    .all(|byte| *byte == 0)
            );
            std::ptr::write_bytes(pointer, 0xee, layout.size());
            replacements.push(pointer);
        }
        for (pointer, value) in live {
            assert!(
                std::slice::from_raw_parts(pointer, layout.size())
                    .iter()
                    .all(|byte| *byte == value)
            );
            pool.dealloc(pointer, layout);
        }
        for pointer in replacements {
            pool.dealloc(pointer, layout);
        }
    }
    pool.recycle();
}

#[cfg(windows)]
#[test]
fn large_buffer_growth_stays_in_place_within_its_reservation() {
    let pool = Pool::new();
    let layout = Layout::from_size_align((1 << 18) + 4096, 16).unwrap();
    // SAFETY: Access is restricted to each checked allocation's committed
    // size; reallocation and deallocation use the corresponding layout.
    unsafe {
        let pointer = pool.alloc_zeroed(layout);
        assert!(!pointer.is_null());
        assert!(
            std::slice::from_raw_parts(pointer, layout.size())
                .iter()
                .all(|byte| *byte == 0)
        );
        std::ptr::write_bytes(pointer, 0xa5, layout.size());
        let size = layout.size() * 2;
        let grown = pool.realloc(pointer, layout, size);
        assert_eq!(grown, pointer);
        assert!(
            std::slice::from_raw_parts(grown, layout.size())
                .iter()
                .all(|byte| *byte == 0xa5)
        );
        std::ptr::write_bytes(grown.add(layout.size()), 0x3c, size - layout.size());
        pool.dealloc(grown, Layout::from_size_align(size, 16).unwrap());
    }
}
