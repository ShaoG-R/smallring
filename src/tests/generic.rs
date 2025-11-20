//! Comprehensive tests for generic ring buffer
//!
//! 通用环形缓冲区的全面测试

use crate::generic::{RingBuf, RingBufError};
use std::sync::{Arc, Mutex};
use std::thread;

// ============================================================================
// SEGMENT 1: Advanced Push/Pop and Boundary Tests
// 第1段：高级推送/弹出和边界测试
// ============================================================================

#[test]
fn test_push_pop_alternating_pattern() {
    // Test alternating push/pop to verify index management
    // 测试交替推送/弹出以验证索引管理
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

    for i in 0..100 {
        buf.push(i);
        assert_eq!(buf.pop().unwrap(), i);
        assert!(buf.is_empty());
    }
}

#[test]
fn test_push_pop_stress_wrapping() {
    // Stress test with many wrap-arounds
    // 多次环绕的压力测试
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    // Fill buffer multiple times to force many wraps
    for cycle in 0..50 {
        for i in 0..8 {
            buf.push(cycle * 100 + i);
        }

        for i in 0..4 {
            assert_eq!(buf.pop().unwrap(), cycle * 100 + i);
        }
    }
}

#[test]
fn test_overwrite_mode_exact_capacity() {
    // Test overwrite behavior at exact capacity boundary
    // 测试在精确容量边界处的覆盖行为
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

    // Fill to exact capacity
    for i in 0..4 {
        assert_eq!(buf.push(i), None);
    }

    assert!(buf.is_full());

    // Each push should now overwrite and return the oldest value
    for i in 4..20 {
        let overwritten = buf.push(i);
        assert_eq!(overwritten, Some(i - 4));
        assert_eq!(buf.len(), 4);
        assert!(buf.is_full());
    }

    // Verify final content: should be [16, 17, 18, 19]
    assert_eq!(buf.pop().unwrap(), 16);
    assert_eq!(buf.pop().unwrap(), 17);
    assert_eq!(buf.pop().unwrap(), 18);
    assert_eq!(buf.pop().unwrap(), 19);
}

#[test]
fn test_non_overwrite_mode_exactly_full() {
    // Test non-overwrite mode at capacity boundaries
    // 测试容量边界处的非覆盖模式
    let mut buf: RingBuf<i32, 32, false> = RingBuf::new(8);

    // Fill to exact capacity
    for i in 0..8 {
        assert!(buf.push(i).is_ok());
    }

    assert!(buf.is_full());
    assert_eq!(buf.len(), 8);

    // Next push should fail
    assert_eq!(buf.push(99), Err(RingBufError::Full(99)));
    assert_eq!(buf.len(), 8);

    // Pop one and try again
    assert_eq!(buf.pop().unwrap(), 0);
    assert!(!buf.is_full());
    assert!(buf.push(99).is_ok());

    // Should now have [1,2,3,4,5,6,7,99]
    for i in 1..8 {
        assert_eq!(buf.pop().unwrap(), i);
    }
    assert_eq!(buf.pop().unwrap(), 99);
}

#[test]
fn test_pop_until_empty_with_refill() {
    // Test completely emptying and refilling multiple times
    // 测试多次完全清空和重新填充
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(16);

    for round in 0..10 {
        // Fill completely
        for i in 0..16 {
            buf.push(round * 100 + i);
        }

        // Empty completely
        for i in 0..16 {
            assert_eq!(buf.pop().unwrap(), round * 100 + i);
        }

        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }
}

#[test]
fn test_peek_consistency_during_operations() {
    // Verify peek returns correct value through various operations
    // 验证在各种操作中 peek 返回正确值
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

    assert_eq!(buf.peek(), None);

    buf.push(10);
    assert_eq!(buf.peek(), Some(&10));
    assert_eq!(buf.len(), 1);

    buf.push(20);
    buf.push(30);
    assert_eq!(buf.peek(), Some(&10)); // Still first element

    buf.pop().unwrap();
    assert_eq!(buf.peek(), Some(&20)); // Now second element

    buf.pop().unwrap();
    assert_eq!(buf.peek(), Some(&30));

    buf.pop().unwrap();
    assert_eq!(buf.peek(), None);
}

#[test]
fn test_clear_with_various_states() {
    // Test clear on empty, partial, and full buffers
    // 测试在空、部分和满缓冲区上的清空操作
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

    // Clear empty buffer
    buf.clear();
    assert!(buf.is_empty());

    // Clear partial buffer
    buf.push(1);
    buf.push(2);
    buf.push(3);
    buf.clear();
    assert!(buf.is_empty());

    // Clear full buffer
    for i in 0..8 {
        buf.push(i);
    }
    assert!(buf.is_full());
    buf.clear();
    assert!(buf.is_empty());

    // Verify can use after clear
    buf.push(100);
    assert_eq!(buf.pop().unwrap(), 100);
}

// ============================================================================
// SEGMENT 2: Advanced Slice Operations and Wrapping
// 第2段：高级切片操作和环绕测试
// ============================================================================

#[test]
fn test_push_slice_larger_than_capacity() {
    // Push slice larger than buffer capacity
    // 推送大于缓冲区容量的切片
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    let large_data: Vec<i32> = (0..20).collect();
    let pushed = buf.push_slice(&large_data);

    // In overwrite mode, all elements are "accepted"
    // but only last 8 fit in buffer
    assert_eq!(pushed, 20);
    assert_eq!(buf.len(), 8);

    // Should contain last 8 elements: [12, 13, 14, 15, 16, 17, 18, 19]
    for i in 0..8 {
        assert_eq!(buf.pop().unwrap(), 12 + i);
    }
}

#[test]
fn test_push_slice_exact_capacity() {
    // Push slice exactly equal to capacity
    // 推送与容量完全相等的切片
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    let data: Vec<i32> = (0..8).collect();
    let pushed = buf.push_slice(&data);

    assert_eq!(pushed, 8);
    assert!(buf.is_full());
    assert_eq!(buf.len(), 8);

    for i in 0..8 {
        assert_eq!(buf.pop().unwrap(), i);
    }
}

#[test]
fn test_push_slice_partial_overwrite() {
    // Test partial overwrite with slice
    // 测试切片的部分覆盖
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    // Fill with initial data
    for i in 0..8 {
        buf.push(i * 10);
    }

    // Push slice that partially overwrites
    let data = [100, 101, 102, 103, 104];
    buf.push_slice(&data);

    // Should have: [50, 60, 70, 100, 101, 102, 103, 104]
    assert_eq!(buf.len(), 8);
    assert_eq!(buf.pop().unwrap(), 50);
    assert_eq!(buf.pop().unwrap(), 60);
    assert_eq!(buf.pop().unwrap(), 70);
    assert_eq!(buf.pop().unwrap(), 100);
    assert_eq!(buf.pop().unwrap(), 101);
    assert_eq!(buf.pop().unwrap(), 102);
    assert_eq!(buf.pop().unwrap(), 103);
    assert_eq!(buf.pop().unwrap(), 104);
}

#[test]
fn test_push_slice_non_overwrite_partial_fill() {
    // Non-overwrite mode: partially filled buffer
    // 非覆盖模式：部分填充的缓冲区
    let mut buf: RingBuf<i32, 64, false> = RingBuf::new(8);

    buf.push(1).unwrap();
    buf.push(2).unwrap();

    let data = [10, 20, 30, 40, 50, 60, 70, 80, 90];
    let pushed = buf.push_slice(&data);

    // Only 6 elements should fit (8 - 2 = 6)
    assert_eq!(pushed, 6);
    assert!(buf.is_full());
    assert_eq!(buf.len(), 8);

    // Verify content: [1, 2, 10, 20, 30, 40, 50, 60]
    assert_eq!(buf.pop().unwrap(), 1);
    assert_eq!(buf.pop().unwrap(), 2);
    for i in 0..6 {
        assert_eq!(buf.pop().unwrap(), (i + 1) * 10);
    }
}

#[test]
fn test_pop_slice_exact_available() {
    // Pop exactly the number of available elements
    // 弹出恰好等于可用元素数量的切片
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    for i in 0..10 {
        buf.push(i);
    }

    let mut dest = [0i32; 10];
    let popped = buf.pop_slice(&mut dest);

    assert_eq!(popped, 10);
    assert_eq!(dest, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert!(buf.is_empty());
}

#[test]
fn test_pop_slice_more_than_available() {
    // Try to pop more than available
    // 尝试弹出超过可用数量的元素
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    for i in 0..5 {
        buf.push(i);
    }

    let mut dest = [0i32; 10];
    let popped = buf.pop_slice(&mut dest);

    // Should only pop 5
    assert_eq!(popped, 5);
    assert_eq!(&dest[..5], &[0, 1, 2, 3, 4]);
    // Rest of dest unchanged
    assert_eq!(&dest[5..], &[0, 0, 0, 0, 0]);
    assert!(buf.is_empty());
}

#[test]
fn test_pop_slice_wrapped_data() {
    // Pop slice when data is wrapped around
    // 当数据环绕时弹出切片
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    // Fill and create wrap-around
    for i in 0..12 {
        buf.push(i);
    }

    // Buffer should contain [4, 5, 6, 7, 8, 9, 10, 11]
    let mut dest = [0i32; 8];
    let popped = buf.pop_slice(&mut dest);

    assert_eq!(popped, 8);
    assert_eq!(dest, [4, 5, 6, 7, 8, 9, 10, 11]);
}

#[test]
fn test_as_slices_after_wrapping() {
    // Test as_slices with wrapped data at various positions
    // 测试在各种位置环绕数据的 as_slices
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    // Create specific wrap scenario
    for i in 0..10 {
        buf.push(i);
    }

    // Remove some from front
    buf.pop().unwrap(); // Remove 2
    buf.pop().unwrap(); // Remove 3

    // Buffer should have [4, 5, 6, 7, 8, 9] at wrapped positions
    let (first, second) = buf.as_slices();
    let mut all_data = Vec::new();
    all_data.extend_from_slice(first);
    all_data.extend_from_slice(second);

    assert_eq!(all_data.len(), 6);
    assert_eq!(all_data, vec![4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_as_slices_full_buffer() {
    // Test as_slices when buffer is completely full
    // 测试缓冲区完全满时的 as_slices
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    for i in 0..8 {
        buf.push(i);
    }

    let (first, second) = buf.as_slices();
    assert_eq!(first.len() + second.len(), 8);

    let mut all_data = Vec::new();
    all_data.extend_from_slice(first);
    all_data.extend_from_slice(second);
    assert_eq!(all_data, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn test_as_mut_slices_and_modify() {
    // Modify elements through as_mut_slices
    // 通过 as_mut_slices 修改元素
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    for i in 0..8 {
        buf.push(i);
    }

    // Modify all elements
    {
        let (first, second) = buf.as_mut_slices();
        for x in first.iter_mut() {
            *x *= 100;
        }
        for x in second.iter_mut() {
            *x *= 100;
        }
    }

    // Verify modifications
    for i in 0..8 {
        assert_eq!(buf.pop().unwrap(), i * 100);
    }
}

// ============================================================================
// SEGMENT 3: Advanced Iterator Tests
// 第3段：高级迭代器测试
// ============================================================================

#[test]
fn test_iter_chaining_and_filtering() {
    // Test iterator chaining and filtering operations
    // 测试迭代器链接和过滤操作
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    for i in 0..10 {
        buf.push(i);
    }

    // Chain with filter and map
    let result: Vec<i32> = buf
        .iter()
        .filter(|&&x| x % 2 == 0)
        .map(|&x| x * 10)
        .collect();

    assert_eq!(result, vec![0, 20, 40, 60, 80]);
}

#[test]
fn test_iter_with_wrapped_buffer() {
    // Iterator should work correctly with wrapped data
    // 迭代器应该正确处理环绕的数据
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    // Create wrap-around
    for i in 0..15 {
        buf.push(i);
    }

    // Buffer contains [7, 8, 9, 10, 11, 12, 13, 14]
    let values: Vec<i32> = buf.iter().copied().collect();
    assert_eq!(values, vec![7, 8, 9, 10, 11, 12, 13, 14]);
}

#[test]
fn test_iter_mut_with_complex_modifications() {
    // Complex modifications through iter_mut
    // 通过 iter_mut 进行复杂修改
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    for i in 0..10 {
        buf.push(i);
    }

    // Apply different modifications based on position
    for (idx, x) in buf.iter_mut().enumerate() {
        if idx % 2 == 0 {
            *x *= 2;
        } else {
            *x += 100;
        }
    }

    let values: Vec<i32> = buf.iter().copied().collect();
    assert_eq!(values, vec![0, 101, 4, 103, 8, 105, 12, 107, 16, 109]);
}

#[test]
fn test_iter_size_hints() {
    // Test ExactSizeIterator implementation
    // 测试 ExactSizeIterator 实现
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    for i in 0..8 {
        buf.push(i);
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
fn test_iter_double_ended_complex() {
    // Complex double-ended iteration patterns
    // 复杂的双向迭代模式
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    for i in 0..10 {
        buf.push(i * 10);
    }

    let mut iter = buf.iter();
    let mut collected = Vec::new();

    // Alternate between front and back
    collected.push(*iter.next().unwrap()); // 0
    collected.push(*iter.next_back().unwrap()); // 90
    collected.push(*iter.next().unwrap()); // 10
    collected.push(*iter.next_back().unwrap()); // 80
    collected.push(*iter.next().unwrap()); // 20

    assert_eq!(collected, vec![0, 90, 10, 80, 20]);
    assert_eq!(iter.len(), 5); // Remaining items
}

#[test]
fn test_iter_mut_double_ended_modifications() {
    // Double-ended iter_mut with modifications from both ends
    // 双向 iter_mut 从两端进行修改
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    for i in 1..=6 {
        buf.push(i);
    }

    let mut iter = buf.iter_mut();

    // Modify first element
    if let Some(x) = iter.next() {
        *x = 100;
    }

    // Modify last element
    if let Some(x) = iter.next_back() {
        *x = 200;
    }

    drop(iter);

    let values: Vec<i32> = buf.iter().copied().collect();
    assert_eq!(values, vec![100, 2, 3, 4, 5, 200]);
}

#[test]
fn test_iter_exhaust_from_both_ends() {
    // Exhaust iterator from both ends simultaneously
    // 同时从两端耗尽迭代器
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(8);

    for i in 0..6 {
        buf.push(i);
    }

    let mut iter = buf.iter();

    assert_eq!(iter.next(), Some(&0));
    assert_eq!(iter.next_back(), Some(&5));
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next_back(), Some(&4));
    assert_eq!(iter.next(), Some(&2));
    assert_eq!(iter.next_back(), Some(&3));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next_back(), None);
}

#[test]
fn test_iter_collect_to_various_collections() {
    // Test collecting iterator to various collection types
    // 测试将迭代器收集到各种集合类型
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    for i in 0..8 {
        buf.push(i);
    }

    // Collect to Vec
    let vec: Vec<i32> = buf.iter().copied().collect();
    assert_eq!(vec.len(), 8);

    // Collect to VecDeque
    let deque: std::collections::VecDeque<i32> = buf.iter().copied().collect();
    assert_eq!(deque.len(), 8);

    // Collect filtered results
    let filtered: Vec<i32> = buf.iter().copied().filter(|&x| x > 3).collect();
    assert_eq!(filtered, vec![4, 5, 6, 7]);
}

// ============================================================================
// SEGMENT 4: Concurrent and Multi-threaded Tests
// 第4段：并发和多线程测试
// ============================================================================

#[test]
fn test_concurrent_writers_overwrite_mode() {
    // Multiple threads writing concurrently in overwrite mode
    // 多线程在覆盖模式下并发写入
    let buf = Arc::new(Mutex::new(RingBuf::<u64, 256, true>::new(64)));
    let mut handles = vec![];

    for thread_id in 0..8 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let value = (thread_id as u64) * 1000 + i;
                buf_clone.lock().unwrap().push(value);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 8 threads * 50 writes = 400 total writes
    // Buffer capacity is 64, so should have 64 elements
    let buf = buf.lock().unwrap();
    assert_eq!(buf.len(), 64);
}

#[test]
fn test_concurrent_readers_and_writers() {
    // Concurrent readers and writers
    // 并发读者和写者
    let buf = Arc::new(Mutex::new(RingBuf::<i32, 128, true>::new(64)));
    let mut handles = vec![];

    // Writers
    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                buf_clone.lock().unwrap().push(thread_id * 1000 + i);
                thread::sleep(std::time::Duration::from_micros(1));
            }
        });
        handles.push(handle);
    }

    // Readers
    for _ in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for _ in 0..50 {
                if buf_clone.lock().unwrap().pop().is_ok() {}
                thread::sleep(std::time::Duration::from_micros(1));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Buffer should have some elements remaining
    // Exact count depends on timing
    let buf = buf.lock().unwrap();
    assert!(buf.len() <= 64);
}

#[test]
fn test_concurrent_push_slice_operations() {
    // Multiple threads using push_slice concurrently
    // 多线程并发使用 push_slice
    let buf = Arc::new(Mutex::new(RingBuf::<i32, 256, true>::new(128)));
    let mut handles = vec![];

    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            let data: Vec<i32> = ((thread_id * 100)..(thread_id * 100 + 20)).collect();
            for _ in 0..10 {
                buf_clone.lock().unwrap().push_slice(&data);
                thread::sleep(std::time::Duration::from_micros(10));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let buf = buf.lock().unwrap();
    assert_eq!(buf.len(), 128); // Should be at capacity
}

#[test]
fn test_concurrent_non_overwrite_mode() {
    // Concurrent access in non-overwrite mode
    // 非覆盖模式下的并发访问
    let buf = Arc::new(Mutex::new(RingBuf::<i32, 128, false>::new(64)));
    let mut handles = vec![];
    let success_count = Arc::new(Mutex::new(0));

    for thread_id in 0..8 {
        let buf_clone = Arc::clone(&buf);
        let count_clone = Arc::clone(&success_count);
        let handle = thread::spawn(move || {
            let mut local_success = 0;
            for i in 0..50 {
                if buf_clone.lock().unwrap().push(thread_id * 1000 + i).is_ok() {
                    local_success += 1;
                }
            }
            *count_clone.lock().unwrap() += local_success;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let buf = buf.lock().unwrap();
    let success = *success_count.lock().unwrap();

    // In non-overwrite mode, exactly 'success' elements should have been pushed
    assert_eq!(buf.len(), success.min(64));
    assert!(success <= 64); // Can't push more than capacity in non-overwrite mode
}

#[test]
fn test_stress_alternating_push_pop_multithread() {
    // Stress test with alternating push/pop from multiple threads
    // 多线程交替推送/弹出的压力测试
    let buf = Arc::new(Mutex::new(RingBuf::<u64, 128, true>::new(32)));
    let mut handles = vec![];

    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..200 {
                let mut guard = buf_clone.lock().unwrap();
                guard.push((thread_id as u64) * 10000 + i);

                if i % 3 == 0 {
                    guard.pop().ok();
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Buffer should be valid and within capacity
    let buf = buf.lock().unwrap();
    assert!(buf.len() <= 32);
}

// ============================================================================
// SEGMENT 5: Type-Specific and Complex Type Tests
// 第5段：类型特定和复杂类型测试
// ============================================================================

#[test]
fn test_string_type_operations() {
    // Test with String type
    // 测试 String 类型
    let mut buf: RingBuf<String, 64, true> = RingBuf::new(8);

    buf.push("Hello".to_string());
    buf.push("World".to_string());
    buf.push("Rust".to_string());
    buf.push("Ring".to_string());
    buf.push("Buffer".to_string());

    assert_eq!(buf.len(), 5);
    assert_eq!(buf.pop().unwrap(), "Hello");
    assert_eq!(buf.pop().unwrap(), "World");

    // Test overwrite with strings
    for i in 0..10 {
        buf.push(format!("Item_{}", i));
    }

    assert_eq!(buf.len(), 8);
}

#[test]
fn test_vec_type_operations() {
    // Test with Vec type
    // 测试 Vec 类型
    let mut buf: RingBuf<Vec<i32>, 64, true> = RingBuf::new(4);

    buf.push(vec![1, 2, 3]);
    buf.push(vec![4, 5, 6]);
    buf.push(vec![7, 8, 9]);

    let first = buf.pop().unwrap();
    assert_eq!(first, vec![1, 2, 3]);

    // Test with empty vecs
    buf.push(vec![]);
    let second = buf.pop().unwrap();
    assert_eq!(second, vec![4, 5, 6]); // FIFO order

    let third = buf.pop().unwrap();
    assert_eq!(third, vec![7, 8, 9]);

    let empty = buf.pop().unwrap();
    assert_eq!(empty, vec![]);
}

#[derive(Debug, Clone, PartialEq)]
struct TestStruct {
    id: u32,
    name: String,
    data: Vec<i32>,
}

#[test]
fn test_custom_struct_type() {
    // Test with custom struct
    // 测试自定义结构体
    let mut buf: RingBuf<TestStruct, 64, true> = RingBuf::new(8);

    buf.push(TestStruct {
        id: 1,
        name: "Alice".to_string(),
        data: vec![10, 20, 30],
    });

    buf.push(TestStruct {
        id: 2,
        name: "Bob".to_string(),
        data: vec![40, 50],
    });

    assert_eq!(buf.len(), 2);

    let first = buf.pop().unwrap();
    assert_eq!(first.id, 1);
    assert_eq!(first.name, "Alice");
    assert_eq!(first.data, vec![10, 20, 30]);
}

#[test]
fn test_option_type() {
    // Test with Option type
    // 测试 Option 类型
    let mut buf: RingBuf<Option<i32>, 64, true> = RingBuf::new(8);

    buf.push(Some(42));
    buf.push(None);
    buf.push(Some(100));
    buf.push(None);

    assert_eq!(buf.pop().unwrap(), Some(42));
    assert_eq!(buf.pop().unwrap(), None);
    assert_eq!(buf.pop().unwrap(), Some(100));
    assert_eq!(buf.pop().unwrap(), None);
}

#[test]
fn test_result_type() {
    // Test with Result type
    // 测试 Result 类型
    let mut buf: RingBuf<Result<i32, String>, 64, true> = RingBuf::new(8);

    buf.push(Ok(42));
    buf.push(Err("error".to_string()));
    buf.push(Ok(100));

    assert_eq!(buf.pop().unwrap(), Ok(42));
    assert_eq!(buf.pop().unwrap(), Err("error".to_string()));
    assert_eq!(buf.pop().unwrap(), Ok(100));
}

#[test]
fn test_tuple_type() {
    // Test with tuple type
    // 测试元组类型
    let mut buf: RingBuf<(i32, String, bool), 64, true> = RingBuf::new(8);

    buf.push((1, "one".to_string(), true));
    buf.push((2, "two".to_string(), false));
    buf.push((3, "three".to_string(), true));

    let (num, text, flag) = buf.pop().unwrap();
    assert_eq!(num, 1);
    assert_eq!(text, "one");
    assert_eq!(flag, true);
}

#[test]
fn test_nested_complex_type() {
    // Test with nested complex type
    // 测试嵌套复杂类型
    let mut buf: RingBuf<Vec<Option<String>>, 64, true> = RingBuf::new(4);

    buf.push(vec![Some("a".to_string()), None, Some("b".to_string())]);
    buf.push(vec![None, None]);
    buf.push(vec![Some("c".to_string())]);

    let first = buf.pop().unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(first[0], Some("a".to_string()));
    assert_eq!(first[1], None);
}

#[test]
fn test_large_struct_overwrite() {
    // Test overwriting large structs
    // 测试覆盖大型结构体
    #[derive(Clone, Debug)]
    struct LargeStruct {
        data: [u64; 100],
    }

    let mut buf: RingBuf<LargeStruct, 32, true> = RingBuf::new(4);

    for i in 0..10 {
        let mut s = LargeStruct { data: [0; 100] };
        s.data[0] = i;
        buf.push(s);
    }

    assert_eq!(buf.len(), 4);

    // Should have last 4 structs
    for expected in 6..10 {
        let s = buf.pop().unwrap();
        assert_eq!(s.data[0], expected);
    }
}

// ============================================================================
// SEGMENT 6: Edge Cases, Capacity, and Stack/Heap Allocation Tests
// 第6段：边界情况、容量和栈/堆分配测试
// ============================================================================

#[test]
fn test_capacity_power_of_two_rounding() {
    // Test that capacity is always rounded to power of 2
    // 测试容量总是回到 2 的幂次
    let buf: RingBuf<i32, 64, true> = RingBuf::new(1);
    assert_eq!(buf.capacity(), 1);

    let buf: RingBuf<i32, 64, true> = RingBuf::new(2);
    assert_eq!(buf.capacity(), 2);

    let buf: RingBuf<i32, 64, true> = RingBuf::new(3);
    assert_eq!(buf.capacity(), 4);

    let buf: RingBuf<i32, 64, true> = RingBuf::new(7);
    assert_eq!(buf.capacity(), 8);

    let buf: RingBuf<i32, 64, true> = RingBuf::new(15);
    assert_eq!(buf.capacity(), 16);

    let buf: RingBuf<i32, 64, true> = RingBuf::new(100);
    assert_eq!(buf.capacity(), 128);

    let buf: RingBuf<i32, 64, true> = RingBuf::new(1000);
    assert_eq!(buf.capacity(), 1024);
}

#[test]
fn test_stack_allocation_threshold() {
    // Test stack vs heap allocation based on N parameter
    // 测试基于 N 参数的栈与堆分配

    // Small capacity <= N should use stack
    let buf: RingBuf<i32, 64, true> = RingBuf::new(32);
    assert_eq!(buf.capacity(), 32);

    // Larger capacity > N should use heap
    let buf: RingBuf<i32, 64, true> = RingBuf::new(128);
    assert_eq!(buf.capacity(), 128);

    // Test with different N values
    let buf: RingBuf<i32, 256, true> = RingBuf::new(200);
    assert_eq!(buf.capacity(), 256);
}

#[test]
fn test_minimum_capacity() {
    // Test with capacity of 1
    // 测试容量为 1
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(1);
    assert_eq!(buf.capacity(), 1);

    buf.push(42);
    assert!(buf.is_full());
    assert_eq!(buf.len(), 1);

    // Next push should overwrite
    assert_eq!(buf.push(99), Some(42));
    assert_eq!(buf.pop().unwrap(), 99);
}

#[test]
fn test_very_large_capacity() {
    // Test with very large capacity
    // 测试非常大的容量
    let buf: RingBuf<i32, 128, true> = RingBuf::new(8192);
    assert_eq!(buf.capacity(), 8192);

    let buf: RingBuf<u8, 128, true> = RingBuf::new(65536);
    assert_eq!(buf.capacity(), 65536);
}

#[test]
fn test_wrapping_index_overflow() {
    // Test that wrapping arithmetic works correctly
    // 测试环绕算术正常工作
    let mut buf: RingBuf<u64, 64, true> = RingBuf::new(8);

    // Push and pop many times to force index wrapping
    for i in 0..10000u64 {
        buf.push(i);
        if i % 2 == 0 {
            buf.pop().ok();
        }
    }

    // Buffer should still be valid
    assert!(buf.len() <= 8);
}

#[test]
fn test_peek_does_not_modify_buffer() {
    // Verify peek doesn't affect buffer state
    // 验证 peek 不影响缓冲区状态
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

    for i in 0..5 {
        buf.push(i);
    }

    let initial_len = buf.len();

    // Peek multiple times
    for _ in 0..10 {
        assert_eq!(buf.peek(), Some(&0));
        assert_eq!(buf.len(), initial_len);
    }

    // Actual pop should still work
    assert_eq!(buf.pop().unwrap(), 0);
    assert_eq!(buf.len(), initial_len - 1);
}

#[test]
fn test_as_slices_does_not_modify_buffer() {
    // Verify as_slices doesn't affect buffer state
    // 验证 as_slices 不影响缓冲区状态
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

    for i in 0..5 {
        buf.push(i);
    }

    let initial_len = buf.len();

    // Call as_slices multiple times
    for _ in 0..10 {
        let (first, second) = buf.as_slices();
        let total = first.len() + second.len();
        assert_eq!(total, initial_len);
        assert_eq!(buf.len(), initial_len);
    }
}

#[test]
fn test_multiple_clear_operations() {
    // Test clearing buffer multiple times
    // 测试多次清空缓冲区
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

    for round in 0..5 {
        // Fill buffer
        for i in 0..8 {
            buf.push(round * 100 + i);
        }

        assert_eq!(buf.len(), 8);

        // Clear
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }
}

#[test]
fn test_push_slice_wrapping_multiple_times() {
    // Test push_slice with data that wraps multiple times
    // 测试多次环绕的 push_slice
    let mut buf: RingBuf<i32, 128, true> = RingBuf::new(16);

    // First fill
    let data1: Vec<i32> = (0..16).collect();
    buf.push_slice(&data1);
    assert_eq!(buf.len(), 16);

    // Push larger slice that causes multiple overwrites
    let data2: Vec<i32> = (100..140).collect(); // 40 elements
    let pushed = buf.push_slice(&data2);
    assert_eq!(pushed, 40);
    assert_eq!(buf.len(), 16); // Only last 16 fit

    // Should contain [124, 125, ..., 139]
    for i in 0..16 {
        assert_eq!(buf.pop().unwrap(), 124 + i);
    }
}

#[test]
fn test_non_overwrite_mode_error_recovery() {
    // Test error handling and recovery in non-overwrite mode
    // 测试非覆盖模式下的错误处理和恢复
    let mut buf: RingBuf<i32, 32, false> = RingBuf::new(4);

    // Fill buffer
    for i in 0..4 {
        assert!(buf.push(i).is_ok());
    }

    // Try to push more, should fail
    for i in 10..20 {
        let result = buf.push(i);
        assert!(result.is_err());
        if let Err(RingBufError::Full(val)) = result {
            assert_eq!(val, i); // Original value returned
        }
    }

    // Buffer should still have original 4 elements
    assert_eq!(buf.len(), 4);
    for i in 0..4 {
        assert_eq!(buf.pop().unwrap(), i);
    }
}

#[test]
fn test_empty_pop_error() {
    // Test pop error on empty buffer
    // 测试空缓冲区的弹出错误
    let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

    // Pop from empty buffer
    let result = buf.pop();
    assert!(result.is_err());
    assert!(matches!(result, Err(RingBufError::Empty)));

    // Add and remove one element
    buf.push(42);
    assert_eq!(buf.pop().unwrap(), 42);

    // Pop again from empty
    let result = buf.pop();
    assert!(matches!(result, Err(RingBufError::Empty)));
}

#[test]
fn test_iterator_on_empty_buffer() {
    // Test all iterator methods on empty buffer
    // 测试空缓冲区上的所有迭代器方法
    let buf: RingBuf<i32, 32, true> = RingBuf::new(8);

    assert_eq!(buf.iter().count(), 0);
    assert_eq!(buf.iter().len(), 0);
    assert_eq!(buf.iter().next(), None);
    assert_eq!(buf.iter().next_back(), None);

    let (first, second) = buf.as_slices();
    assert_eq!(first.len(), 0);
    assert_eq!(second.len(), 0);
}

#[test]
fn test_mixed_operations_sequence() {
    // Complex sequence of mixed operations
    // 复杂的混合操作序列
    let mut buf: RingBuf<i32, 64, true> = RingBuf::new(16);

    // Phase 1: Normal push/pop
    for i in 0..10 {
        buf.push(i);
    }
    for _ in 0..5 {
        buf.pop().unwrap();
    }

    // Phase 2: push_slice
    let slice = [100, 101, 102, 103, 104];
    buf.push_slice(&slice);

    // Phase 3: Iterator modification
    for x in buf.iter_mut() {
        *x *= 2;
    }

    // Phase 4: pop_slice
    let mut dest = [0i32; 5];
    buf.pop_slice(&mut dest);

    // Phase 5: Verify final state
    assert!(buf.len() > 0);
    buf.clear();
    assert!(buf.is_empty());
}
