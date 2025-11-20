//! Comprehensive tests for atomic ring buffer
//!
//! 原子环形缓冲区的全面测试

use crate::atomic::AtomicRingBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::{AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize};
use std::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize};
use std::sync::{Arc, Barrier};
use std::thread;

// ============================================================================
// SEGMENT 1: Advanced Push/Pop and Boundary Tests
// 第1段：高级推送/弹出和边界测试
// ============================================================================

#[test]
fn test_push_pop_alternating_pattern() {
    // Test alternating push/pop to verify index management
    // 测试交替推送/弹出以验证索引管理
    let buf: AtomicRingBuf<AtomicU64, 32, true> = AtomicRingBuf::new(4);

    for i in 0..100 {
        buf.push(i, Ordering::Relaxed);
        assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), i);
        assert!(buf.is_empty());
    }
}

#[test]
fn test_push_pop_stress_wrapping() {
    // Stress test with many wrap-arounds
    // 多次环绕的压力测试
    let buf: AtomicRingBuf<AtomicU64, 64, true> = AtomicRingBuf::new(8);

    // Fill buffer multiple times to force many wraps
    for cycle in 0..50 {
        for i in 0..8 {
            buf.push(cycle * 100 + i, Ordering::Relaxed);
        }

        for i in 0..4 {
            assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), cycle * 100 + i);
        }
    }
}

#[test]
fn test_overwrite_mode_exact_capacity() {
    // Test overwrite behavior at exact capacity boundary
    // 测试在精确容量边界处的覆盖行为
    let buf: AtomicRingBuf<AtomicU64, 32, true> = AtomicRingBuf::new(4);

    // Fill to exact capacity
    for i in 0..4 {
        assert_eq!(buf.push(i, Ordering::Relaxed), None);
    }

    assert!(buf.is_full());

    // Each push should now overwrite and return the oldest value
    for i in 4..20 {
        let overwritten = buf.push(i, Ordering::Relaxed);
        assert_eq!(overwritten, Some(i - 4));
        assert_eq!(buf.len(), 4);
        assert!(buf.is_full());
    }

    // Verify final content: should be [16, 17, 18, 19]
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 16);
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 17);
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 18);
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 19);
}

#[test]
fn test_non_overwrite_mode_exactly_full() {
    // Test non-overwrite mode at capacity boundaries
    // 测试容量边界处的非覆盖模式
    let buf: AtomicRingBuf<AtomicU64, 32, false> = AtomicRingBuf::new(8);

    // Fill to exact capacity
    for i in 0..8 {
        assert!(buf.push(i, Ordering::Relaxed).is_ok());
    }

    assert!(buf.is_full());
    assert_eq!(buf.len(), 8);

    // Next push should fail
    assert_eq!(buf.push(99, Ordering::Relaxed), Err(99));
    assert_eq!(buf.len(), 8);

    // Pop one and try again
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 0);
    assert!(!buf.is_full());
    assert!(buf.push(99, Ordering::Relaxed).is_ok());

    // Should now have [1,2,3,4,5,6,7,99]
    for i in 1..8 {
        assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), i);
    }
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 99);
}

#[test]
fn test_pop_until_empty_with_refill() {
    // Test completely emptying and refilling multiple times
    // 测试多次完全清空和重新填充
    let buf: AtomicRingBuf<AtomicU64, 32, true> = AtomicRingBuf::new(16);

    for round in 0..10 {
        // Fill completely
        for i in 0..16 {
            buf.push(round * 100 + i, Ordering::Relaxed);
        }

        // Empty completely
        for i in 0..16 {
            assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), round * 100 + i);
        }

        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }
}

#[test]
fn test_peek_consistency_during_operations() {
    // Verify peek returns correct value through various operations
    // 验证在各种操作中 peek 返回正确值
    let buf: AtomicRingBuf<AtomicU64, 32, true> = AtomicRingBuf::new(8);

    assert_eq!(buf.peek(Ordering::Relaxed), None);

    buf.push(10, Ordering::Relaxed);
    assert_eq!(buf.peek(Ordering::Relaxed), Some(10));
    assert_eq!(buf.len(), 1);

    buf.push(20, Ordering::Relaxed);
    buf.push(30, Ordering::Relaxed);
    assert_eq!(buf.peek(Ordering::Relaxed), Some(10)); // Still first element

    buf.pop(Ordering::Relaxed).unwrap();
    assert_eq!(buf.peek(Ordering::Relaxed), Some(20)); // Now second element

    buf.pop(Ordering::Relaxed).unwrap();
    assert_eq!(buf.peek(Ordering::Relaxed), Some(30));

    buf.pop(Ordering::Relaxed).unwrap();
    assert_eq!(buf.peek(Ordering::Relaxed), None);
}

#[test]
fn test_clear_with_various_states() {
    // Test clear on empty, partial, and full buffers
    // 测试在空、部分和满缓冲区上的清空操作
    let buf: AtomicRingBuf<AtomicU64, 32, true> = AtomicRingBuf::new(8);

    // Clear empty buffer
    buf.clear();
    assert!(buf.is_empty());

    // Clear partial buffer
    buf.push(1, Ordering::Relaxed);
    buf.push(2, Ordering::Relaxed);
    buf.push(3, Ordering::Relaxed);
    buf.clear();
    assert!(buf.is_empty());

    // Clear full buffer
    for i in 0..8 {
        buf.push(i, Ordering::Relaxed);
    }
    assert!(buf.is_full());
    buf.clear();
    assert!(buf.is_empty());

    // Verify can use after clear
    buf.push(100, Ordering::Relaxed);
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 100);
}

// ============================================================================
// SEGMENT 2: Read All, Iterator, and Memory Ordering Tests
// 第2段：读取全部、迭代器和内存顺序测试
// ============================================================================

#[test]
fn test_read_all_various_sizes() {
    // Test read_all with different buffer sizes
    // 测试不同缓冲区大小的 read_all
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    // Empty buffer
    let values = buf.read_all(Ordering::Acquire);
    assert_eq!(values.len(), 0);

    // Partial buffer
    for i in 0..5 {
        buf.push(i * 10, Ordering::Relaxed);
    }
    let values = buf.read_all(Ordering::Acquire);
    assert_eq!(values, vec![0, 10, 20, 30, 40]);
    assert_eq!(buf.len(), 5); // Elements still in buffer

    // Full buffer
    buf.clear();
    for i in 0..8 {
        buf.push(i, Ordering::Relaxed);
    }
    let values = buf.read_all(Ordering::Acquire);
    assert_eq!(values, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn test_read_all_after_wrapping() {
    // Test read_all with wrapped buffer
    // 测试环绕缓冲区的 read_all
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    // Create wrap-around scenario
    for i in 0..15 {
        buf.push(i, Ordering::Relaxed);
    }

    // Buffer should contain [7, 8, 9, 10, 11, 12, 13, 14]
    let values = buf.read_all(Ordering::Acquire);
    assert_eq!(values, vec![7, 8, 9, 10, 11, 12, 13, 14]);
}

#[test]
fn test_iter_basic_iteration() {
    // Basic iterator functionality
    // 基本迭代器功能
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    for i in 0..5 {
        buf.push(i * 100, Ordering::Relaxed);
    }

    let values: Vec<u64> = buf
        .iter()
        .map(|atom| atom.load(Ordering::Acquire))
        .collect();
    assert_eq!(values, vec![0, 100, 200, 300, 400]);
}

#[test]
fn test_iter_with_wrapped_buffer() {
    // Iterator should work correctly with wrapped data
    // 迭代器应该正确处理环绕的数据
    let buf: AtomicRingBuf<AtomicU64, 64, true> = AtomicRingBuf::new(8);

    // Create wrap-around
    for i in 0..15 {
        buf.push(i, Ordering::Relaxed);
    }

    // Buffer contains [7, 8, 9, 10, 11, 12, 13, 14]
    let values: Vec<u64> = buf
        .iter()
        .map(|atom| atom.load(Ordering::Acquire))
        .collect();
    assert_eq!(values, vec![7, 8, 9, 10, 11, 12, 13, 14]);
}

#[test]
fn test_iter_size_hints() {
    // Test ExactSizeIterator implementation
    // 测试 ExactSizeIterator 实现
    let buf: AtomicRingBuf<AtomicU64, 64, true> = AtomicRingBuf::new(16);

    for i in 0..8 {
        buf.push(i, Ordering::Relaxed);
    }

    let mut iter = buf.iter();
    assert_eq!(iter.len(), 8);

    iter.next();
    assert_eq!(iter.len(), 7);

    iter.next();
    iter.next();
    assert_eq!(iter.len(), 5);
}

#[test]
fn test_iter_chaining_and_filtering() {
    // Test iterator chaining and filtering operations
    // 测试迭代器链接和过滤操作
    let buf: AtomicRingBuf<AtomicU64, 64, true> = AtomicRingBuf::new(16);

    for i in 0..10 {
        buf.push(i, Ordering::Relaxed);
    }

    // Chain with filter and map
    let result: Vec<u64> = buf
        .iter()
        .map(|atom| atom.load(Ordering::Acquire))
        .filter(|&x| x % 2 == 0)
        .map(|x| x * 10)
        .collect();

    assert_eq!(result, vec![0, 20, 40, 60, 80]);
}

#[test]
fn test_memory_ordering_relaxed() {
    // Test with Relaxed ordering
    // 测试 Relaxed 内存顺序
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    buf.push(42, Ordering::Relaxed);
    buf.push(99, Ordering::Relaxed);

    assert_eq!(buf.pop(Ordering::Relaxed), Some(42));
    assert_eq!(buf.pop(Ordering::Relaxed), Some(99));
}

#[test]
fn test_memory_ordering_acquire_release() {
    // Test with Acquire/Release ordering
    // 测试 Acquire/Release 内存顺序
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    buf.push(100, Ordering::Release);
    buf.push(200, Ordering::Release);

    assert_eq!(buf.peek(Ordering::Acquire), Some(100));
    assert_eq!(buf.pop(Ordering::Acquire), Some(100));
    assert_eq!(buf.pop(Ordering::Acquire), Some(200));
}

#[test]
fn test_memory_ordering_seq_cst() {
    // Test with SeqCst ordering
    // 测试 SeqCst 内存顺序
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    buf.push(42, Ordering::SeqCst);
    let value = buf.pop(Ordering::SeqCst);
    assert_eq!(value, Some(42));
}

#[test]
fn test_get_unchecked_direct_access() {
    // Test direct element access
    // 测试直接元素访问
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    buf.push(10, Ordering::Relaxed);
    buf.push(20, Ordering::Relaxed);
    buf.push(30, Ordering::Relaxed);

    unsafe {
        let elem0 = buf.get_unchecked(0);
        assert_eq!(elem0.load(Ordering::Acquire), 10);

        let elem1 = buf.get_unchecked(1);
        assert_eq!(elem1.load(Ordering::Acquire), 20);

        let elem2 = buf.get_unchecked(2);
        assert_eq!(elem2.load(Ordering::Acquire), 30);
    }
}

// ============================================================================
// SEGMENT 3: Atomic Numeric Operations Tests
// 第3段：原子数值操作测试
// ============================================================================

#[test]
fn test_fetch_add_at_basic() {
    // Basic fetch_add_at functionality
    // 基本的 fetch_add_at 功能
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    buf.push(10, Ordering::Relaxed);
    buf.push(20, Ordering::Relaxed);
    buf.push(30, Ordering::Relaxed);

    let old = unsafe { buf.fetch_add_at(0, 5, Ordering::Relaxed) };
    assert_eq!(old, 10);
    assert_eq!(buf.peek(Ordering::Acquire).unwrap(), 15);

    let old = unsafe { buf.fetch_add_at(1, 100, Ordering::Relaxed) };
    assert_eq!(old, 20);

    let old = unsafe { buf.fetch_add_at(2, 7, Ordering::Relaxed) };
    assert_eq!(old, 30);

    // Verify all values
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 15);
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 120);
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 37);
}

#[test]
fn test_fetch_sub_at_basic() {
    // Basic fetch_sub_at functionality
    // 基本的 fetch_sub_at 功能
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    buf.push(100, Ordering::Relaxed);
    buf.push(200, Ordering::Relaxed);
    buf.push(50, Ordering::Relaxed);

    let old = unsafe { buf.fetch_sub_at(0, 10, Ordering::Relaxed) };
    assert_eq!(old, 100);
    assert_eq!(buf.peek(Ordering::Acquire).unwrap(), 90);

    let old = unsafe { buf.fetch_sub_at(1, 150, Ordering::Relaxed) };
    assert_eq!(old, 200);

    // Verify values
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 90);
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 50);
}

#[test]
fn test_fetch_add_sub_alternating() {
    // Alternate between fetch_add and fetch_sub
    // 交替使用 fetch_add 和 fetch_sub
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    buf.push(100, Ordering::Relaxed);

    for _ in 0..10 {
        unsafe { buf.fetch_add_at(0, 10, Ordering::Relaxed) };
        unsafe { buf.fetch_sub_at(0, 5, Ordering::Relaxed) };
    }

    // Should be 100 + 10*10 - 5*10 = 150
    assert_eq!(buf.peek(Ordering::Acquire).unwrap(), 150);
}

#[test]
fn test_fetch_operations_with_signed_types() {
    // Test with signed atomic types
    // 测试有符号原子类型
    let buf: AtomicRingBuf<AtomicI64, 32> = AtomicRingBuf::new(8);

    buf.push(50, Ordering::Relaxed);
    buf.push(-30, Ordering::Relaxed);

    let old = unsafe { buf.fetch_add_at(0, 20, Ordering::Relaxed) };
    assert_eq!(old, 50);
    assert_eq!(buf.peek(Ordering::Acquire).unwrap(), 70);

    let old = unsafe { buf.fetch_sub_at(1, -10, Ordering::Relaxed) };
    assert_eq!(old, -30);

    // Verify values
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 70);
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), -20);
}

#[test]
fn test_fetch_operations_multiple_elements() {
    // Perform fetch operations on multiple elements
    // 在多个元素上执行 fetch 操作
    let buf: AtomicRingBuf<AtomicU32, 64> = AtomicRingBuf::new(16);

    for i in 0..8 {
        buf.push(i * 10, Ordering::Relaxed);
    }

    // Add to all even positions
    for i in (0..8).step_by(2) {
        unsafe { buf.fetch_add_at(i, 1000, Ordering::Relaxed) };
    }

    // Subtract from all odd positions
    for i in (1..8).step_by(2) {
        unsafe { buf.fetch_sub_at(i, 5, Ordering::Relaxed) };
    }

    // Verify results
    let values = buf.read_all(Ordering::Acquire);
    assert_eq!(values[0], 1000); // 0 + 1000
    assert_eq!(values[1], 5); // 10 - 5
    assert_eq!(values[2], 1020); // 20 + 1000
    assert_eq!(values[3], 25); // 30 - 5
}

#[test]
fn test_atomic_u8_operations() {
    // Test with AtomicU8
    // 测试 AtomicU8
    let buf: AtomicRingBuf<AtomicU8, 32> = AtomicRingBuf::new(8);

    buf.push(100u8, Ordering::Relaxed);
    buf.push(200u8, Ordering::Relaxed);

    assert_eq!(buf.pop(Ordering::Relaxed), Some(100u8));
    assert_eq!(buf.pop(Ordering::Relaxed), Some(200u8));
}

#[test]
fn test_atomic_u16_operations() {
    // Test with AtomicU16
    // 测试 AtomicU16
    let buf: AtomicRingBuf<AtomicU16, 32> = AtomicRingBuf::new(8);

    buf.push(1000u16, Ordering::Relaxed);
    buf.push(2000u16, Ordering::Relaxed);

    let old = unsafe { buf.fetch_add_at(0, 500, Ordering::Relaxed) };
    assert_eq!(old, 1000);
    assert_eq!(buf.pop(Ordering::Relaxed), Some(1500u16));
}

#[test]
fn test_atomic_usize_operations() {
    // Test with AtomicUsize
    // 测试 AtomicUsize
    let buf: AtomicRingBuf<AtomicUsize, 32> = AtomicRingBuf::new(8);

    buf.push(1000usize, Ordering::Relaxed);
    buf.push(2000usize, Ordering::Relaxed);

    assert_eq!(buf.len(), 2);
    assert_eq!(buf.pop(Ordering::Relaxed), Some(1000usize));
}

// ============================================================================
// SEGMENT 4: Concurrent and Multi-threaded Tests
// 第4段：并发和多线程测试
// ============================================================================

#[test]
fn test_concurrent_push_overwrite_mode() {
    // Multiple threads pushing concurrently in overwrite mode
    // 多线程在覆盖模式下并发推送
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 256, true>::new(64));
    let mut handles = vec![];

    for thread_id in 0..8 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let value = (thread_id as u64) * 1000 + i;
                buf_clone.push(value, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 8 threads * 50 writes = 400 total writes
    // Buffer capacity is 64, so should have 64 elements
    assert_eq!(buf.len(), 64);
}

#[test]
fn test_concurrent_push_pop() {
    // Concurrent pushers and poppers
    // 并发推送者和弹出者
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 128, true>::new(64));
    let mut handles = vec![];

    // Pushers
    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                buf_clone.push((thread_id as u64) * 1000 + i, Ordering::Release);
                thread::sleep(std::time::Duration::from_micros(1));
            }
        });
        handles.push(handle);
    }

    // Poppers
    let popped_count = Arc::new(AtomicUsize::new(0));
    for _ in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let count_clone = Arc::clone(&popped_count);
        let handle = thread::spawn(move || {
            for _ in 0..50 {
                if buf_clone.pop(Ordering::Acquire).is_some() {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                }
                thread::sleep(std::time::Duration::from_micros(1));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Total pushed: 400, total popped attempts: 200
    let total_popped = popped_count.load(Ordering::Acquire);
    let remaining = buf.len();

    // Some elements should have been popped and some should remain
    assert!(total_popped > 0);
    assert!(remaining <= 64);
}

#[test]
fn test_concurrent_non_overwrite_mode() {
    // Concurrent access in non-overwrite mode
    // 非覆盖模式下的并发访问
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 128, false>::new(64));
    let mut handles = vec![];
    let success_count = Arc::new(AtomicUsize::new(0));

    for thread_id in 0..8 {
        let buf_clone = Arc::clone(&buf);
        let count_clone = Arc::clone(&success_count);
        let handle = thread::spawn(move || {
            let mut local_success = 0;
            for i in 0..50 {
                let value = (thread_id as u64) * 1000 + i;
                if buf_clone.push(value, Ordering::SeqCst).is_ok() {
                    local_success += 1;
                }
            }
            count_clone.fetch_add(local_success, Ordering::Relaxed);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let success = success_count.load(Ordering::Acquire);

    // In non-overwrite mode, exactly 'success' elements should have been pushed
    // and it should not exceed capacity
    assert_eq!(buf.len(), success.min(64));
    assert!(success <= 64);
}

#[test]
fn test_concurrent_read_all() {
    // Multiple threads reading concurrently while one thread writes
    // 多线程并发读取，同时有一个线程写入
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 128, true>::new(64));
    let mut handles = vec![];

    // Writer thread
    let buf_clone = Arc::clone(&buf);
    let writer = thread::spawn(move || {
        for i in 0..200 {
            buf_clone.push(i, Ordering::Release);
            thread::sleep(std::time::Duration::from_micros(10));
        }
    });

    // Reader threads
    for _ in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for _ in 0..50 {
                let _values = buf_clone.read_all(Ordering::Acquire);
                thread::sleep(std::time::Duration::from_micros(20));
            }
        });
        handles.push(handle);
    }

    writer.join().unwrap();
    for handle in handles {
        handle.join().unwrap();
    }

    // Buffer should be at capacity
    assert_eq!(buf.len(), 64);
}

#[test]
fn test_concurrent_fetch_operations() {
    // Concurrent atomic fetch operations
    // 并发原子 fetch 操作
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 64, true>::new(8));

    // Initialize with values
    for _ in 0..8 {
        buf.push(0, Ordering::Relaxed);
    }

    let mut handles = vec![];

    // Multiple threads performing fetch_add
    for _ in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                for offset in 0..8 {
                    unsafe {
                        buf_clone.fetch_add_at(offset, 1, Ordering::SeqCst);
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Each element should have been incremented 400 times (4 threads * 100 iterations)
    let values = buf.read_all(Ordering::Acquire);
    for value in values {
        assert_eq!(value, 400);
    }
}

#[test]
fn test_concurrent_iterator_access() {
    // Multiple threads accessing iterator concurrently
    // 多线程并发访问迭代器
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 128, true>::new(32));

    // Fill buffer
    for i in 0..32 {
        buf.push(i * 10, Ordering::Relaxed);
    }

    let mut handles = vec![];

    for _ in 0..8 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let sum: u64 = buf_clone
                    .iter()
                    .map(|atom| atom.load(Ordering::Acquire))
                    .sum();
                // Sum should be consistent
                assert!(sum > 0);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_barrier_synchronized_access() {
    // Use barrier to synchronize multiple threads
    // 使用屏障同步多个线程
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 128, true>::new(64));
    let barrier = Arc::new(Barrier::new(4));
    let mut handles = vec![];

    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let barrier_clone = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            // Wait for all threads to be ready
            barrier_clone.wait();

            // All threads push simultaneously
            for i in 0..50 {
                buf_clone.push((thread_id as u64) * 1000 + i, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Total: 4 threads * 50 = 200 pushes, capacity 64
    assert_eq!(buf.len(), 64);
}

#[test]
fn test_stress_alternating_push_pop_multithread() {
    // Stress test with alternating push/pop from multiple threads
    // 多线程交替推送/弹出的压力测试
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 128, true>::new(32));
    let mut handles = vec![];

    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..200 {
                buf_clone.push((thread_id as u64) * 10000 + i, Ordering::Release);

                if i % 3 == 0 {
                    buf_clone.pop(Ordering::Acquire);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Buffer should be valid and within capacity
    assert!(buf.len() <= 32);
}

// ============================================================================
// SEGMENT 5: Edge Cases, Capacity, and Special Scenarios
// 第5段：边界情况、容量和特殊场景测试
// ============================================================================

#[test]
fn test_capacity_power_of_two_rounding() {
    // Test that capacity is always rounded to power of 2
    // 测试容量总是舍入到 2 的幂次
    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(1);
    assert_eq!(buf.capacity(), 1);

    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(2);
    assert_eq!(buf.capacity(), 2);

    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(3);
    assert_eq!(buf.capacity(), 4);

    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(7);
    assert_eq!(buf.capacity(), 8);

    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(15);
    assert_eq!(buf.capacity(), 16);

    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(100);
    assert_eq!(buf.capacity(), 128);
}

#[test]
fn test_minimum_capacity() {
    // Test with capacity of 1
    // 测试容量为 1
    let buf: AtomicRingBuf<AtomicU64, 32, true> = AtomicRingBuf::new(1);
    assert_eq!(buf.capacity(), 1);

    buf.push(42, Ordering::Relaxed);
    assert!(buf.is_full());
    assert_eq!(buf.len(), 1);

    // Next push should overwrite
    assert_eq!(buf.push(99, Ordering::Relaxed), Some(42));
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 99);
}

#[test]
fn test_very_large_capacity() {
    // Test with very large capacity
    // 测试非常大的容量
    let buf: AtomicRingBuf<AtomicU64, 128> = AtomicRingBuf::new(8192);
    assert_eq!(buf.capacity(), 8192);

    let buf: AtomicRingBuf<AtomicU8, 128> = AtomicRingBuf::new(65536);
    assert_eq!(buf.capacity(), 65536);
}

#[test]
fn test_wrapping_index_overflow() {
    // Test that wrapping arithmetic works correctly
    // 测试环绕算术正常工作
    let buf: AtomicRingBuf<AtomicU64, 64, true> = AtomicRingBuf::new(8);

    // Push and pop many times to force index wrapping
    for i in 0..10000u64 {
        buf.push(i, Ordering::Relaxed);
        if i % 2 == 0 {
            buf.pop(Ordering::Relaxed);
        }
    }

    // Buffer should still be valid
    assert!(buf.len() <= 8);
}

#[test]
fn test_peek_does_not_modify_buffer() {
    // Verify peek doesn't affect buffer state
    // 验证 peek 不影响缓冲区状态
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    for i in 0..5 {
        buf.push(i, Ordering::Relaxed);
    }

    let initial_len = buf.len();

    // Peek multiple times
    for _ in 0..10 {
        assert_eq!(buf.peek(Ordering::Acquire), Some(0));
        assert_eq!(buf.len(), initial_len);
    }

    // Actual pop should still work
    assert_eq!(buf.pop(Ordering::Relaxed).unwrap(), 0);
    assert_eq!(buf.len(), initial_len - 1);
}

#[test]
fn test_read_all_does_not_modify_buffer() {
    // Verify read_all doesn't affect buffer state
    // 验证 read_all 不影响缓冲区状态
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    for i in 0..5 {
        buf.push(i * 10, Ordering::Relaxed);
    }

    let initial_len = buf.len();

    // Call read_all multiple times
    for _ in 0..10 {
        let values = buf.read_all(Ordering::Acquire);
        assert_eq!(values.len(), initial_len);
        assert_eq!(buf.len(), initial_len);
    }
}

#[test]
fn test_multiple_clear_operations() {
    // Test clearing buffer multiple times
    // 测试多次清空缓冲区
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    for round in 0..5 {
        // Fill buffer
        for i in 0..8 {
            buf.push(round * 100 + i, Ordering::Relaxed);
        }

        assert_eq!(buf.len(), 8);

        // Clear
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }
}

#[test]
fn test_atomic_bool_comprehensive() {
    // Comprehensive test with AtomicBool
    // AtomicBool 的全面测试
    let buf: AtomicRingBuf<AtomicBool, 32, true> = AtomicRingBuf::new(8);

    // Alternate true and false
    for i in 0..16 {
        buf.push(i % 2 == 0, Ordering::Relaxed);
    }

    // Last 8 should be from i=8 to i=15: true, false, true, false, true, false, true, false
    let values = buf.read_all(Ordering::Acquire);
    assert_eq!(values.len(), 8);
    for (idx, &val) in values.iter().enumerate() {
        assert_eq!(val, idx % 2 == 0);
    }
}

#[test]
fn test_different_atomic_types_mixed() {
    // Test different atomic types independently
    // 独立测试不同的原子类型
    let buf_i8: AtomicRingBuf<AtomicI8, 32> = AtomicRingBuf::new(4);
    buf_i8.push(-10i8, Ordering::Relaxed);
    buf_i8.push(20i8, Ordering::Relaxed);
    assert_eq!(buf_i8.pop(Ordering::Relaxed), Some(-10i8));

    let buf_i16: AtomicRingBuf<AtomicI16, 32> = AtomicRingBuf::new(4);
    buf_i16.push(-1000i16, Ordering::Relaxed);
    buf_i16.push(2000i16, Ordering::Relaxed);
    assert_eq!(buf_i16.pop(Ordering::Relaxed), Some(-1000i16));

    let buf_i32: AtomicRingBuf<AtomicI32, 32> = AtomicRingBuf::new(4);
    buf_i32.push(-100000i32, Ordering::Relaxed);
    buf_i32.push(200000i32, Ordering::Relaxed);
    assert_eq!(buf_i32.pop(Ordering::Relaxed), Some(-100000i32));

    let buf_isize: AtomicRingBuf<AtomicIsize, 32> = AtomicRingBuf::new(4);
    buf_isize.push(-500isize, Ordering::Relaxed);
    buf_isize.push(600isize, Ordering::Relaxed);
    assert_eq!(buf_isize.pop(Ordering::Relaxed), Some(-500isize));
}

#[test]
fn test_stack_allocation_threshold() {
    // Test stack vs heap allocation based on N parameter
    // 测试基于 N 参数的栈与堆分配

    // Small capacity <= N should use stack
    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(32);
    assert_eq!(buf.capacity(), 32);

    // Larger capacity > N should use heap
    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(128);
    assert_eq!(buf.capacity(), 128);

    // Test with different N values
    let buf: AtomicRingBuf<AtomicU64, 256> = AtomicRingBuf::new(200);
    assert_eq!(buf.capacity(), 256);
}

#[test]
fn test_empty_buffer_operations() {
    // Test operations on empty buffer
    // 测试空缓冲区上的操作
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
    assert_eq!(buf.pop(Ordering::Relaxed), None);
    assert_eq!(buf.peek(Ordering::Relaxed), None);

    let values = buf.read_all(Ordering::Acquire);
    assert_eq!(values.len(), 0);

    let iter = buf.iter();
    assert_eq!(iter.len(), 0);
}

#[test]
fn test_full_buffer_operations() {
    // Test operations when buffer is full
    // 测试缓冲区满时的操作
    let buf: AtomicRingBuf<AtomicU64, 32, false> = AtomicRingBuf::new(4);

    // Fill completely
    for i in 0..4 {
        assert!(buf.push(i, Ordering::Relaxed).is_ok());
    }

    assert!(buf.is_full());
    assert_eq!(buf.len(), 4);

    // Try to push when full
    assert_eq!(buf.push(99, Ordering::Relaxed), Err(99));
    assert_eq!(buf.len(), 4);

    // Peek and read_all should still work
    assert_eq!(buf.peek(Ordering::Acquire), Some(0));
    let values = buf.read_all(Ordering::Acquire);
    assert_eq!(values.len(), 4);
}

#[test]
fn test_iterator_empty_buffer() {
    // Test iterator on empty buffer
    // 测试空缓冲区的迭代器
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

    let mut iter = buf.iter();
    assert!(iter.next().is_none());
    assert_eq!(iter.len(), 0);
}

#[test]
fn test_sequential_ordering_consistency() {
    // Test that FIFO order is maintained
    // 测试 FIFO 顺序得到保持
    let buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(16);

    // Push sequence
    for i in 0..100 {
        buf.push(i, Ordering::Release);
    }

    // Pop and verify FIFO order
    let start = 100 - buf.capacity() as u64;
    for expected in start..100 {
        assert_eq!(buf.pop(Ordering::Acquire).unwrap(), expected);
    }

    assert!(buf.is_empty());
}
