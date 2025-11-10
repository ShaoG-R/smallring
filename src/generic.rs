//! Generic lock-free ring buffer with stack/heap optimization
//! 
//! 通用的无锁环形缓冲区，带有栈/堆优化
//! 
//! This module provides a generic ring buffer implementation that supports:
//! - Arbitrary element types
//! - Stack allocation for small capacities (≤N)
//! - Heap allocation for large capacities (>N)
//! - Overwrite mode (auto-overwrite oldest data when full)
//! - Non-overwrite mode (reject writes when full)
//! 
//! 本模块提供通用的环形缓冲区实现，支持：
//! - 任意元素类型
//! - 小容量栈分配（≤N）
//! - 大容量堆分配（>N）
//! - 覆盖模式（满时自动覆盖最旧数据）
//! - 非覆盖模式（满时拒绝写入）

use super::core::RingBufCore;
use std::sync::atomic::Ordering;

/// Ring buffer operation error
/// 
/// 环形缓冲区操作错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingBufError<T> {
    /// Buffer is full (non-overwrite mode only)
    /// 
    /// 缓冲区已满（仅非覆盖模式）
    Full(T),
    
    /// Buffer is empty
    /// 
    /// 缓冲区为空
    Empty,
}


/// Generic ring buffer with configurable behavior
/// 
/// 可配置行为的通用环形缓冲区
/// 
/// # Type Parameters
/// - `T`: Element type
/// - `N`: Stack capacity threshold (use stack when capacity ≤ N, heap otherwise)
/// - `OVERWRITE`: Compile-time overwrite mode flag (true = overwrite, false = reject when full)
/// 
/// # 类型参数
/// - `T`: 元素类型
/// - `N`: 栈容量阈值（容量 ≤ N 时使用栈，否则使用堆）
/// - `OVERWRITE`: 编译期覆盖模式标志（true = 覆盖，false = 满时拒绝）
/// 
/// # Features
/// 
/// - **Power-of-2 capacity**: Automatically rounds up to next power of 2 for efficient masking
/// - **Lock-free**: Uses atomic operations for thread-safe concurrent access
/// - **Stack/Heap optimization**: Small buffers stored on stack, large buffers on heap
/// - **Compile-time mode selection**: Zero-cost abstraction for overwrite vs non-overwrite behavior
/// 
/// # 特性
/// 
/// - **2的幂次容量**: 自动向上取整到下一个2的幂次以实现高效的掩码操作
/// - **无锁**: 使用原子操作实现线程安全的并发访问
/// - **栈/堆优化**: 小缓冲区存储在栈上，大缓冲区存储在堆上
/// - **编译期模式选择**: 覆盖和非覆盖行为的零成本抽象
pub struct RingBuf<T, const N: usize, const OVERWRITE: bool = true> {
    /// Core ring buffer implementation
    /// 
    /// 核心环形缓冲区实现
    core: RingBufCore<T, N>,
}

// 辅助 Trait，用于根据 OVERWRITE 分发 push 的实现

/// Internal trait to dispatch push behavior based on OVERWRITE.
pub trait PushDispatch<T, const N: usize, const OVERWRITE: bool> {
    /// The return type of the push operation.
    type PushOutput;

    /// The actual push implementation.
    fn push_impl(ringbuf: &RingBuf<T, N, OVERWRITE>, value: T) -> Self::PushOutput;
}

/// Marker struct for compile-time dispatch.
pub struct PushMarker<const OVERWRITE: bool>;

impl<T, const N: usize> PushDispatch<T, N, true> for PushMarker<true> {
    /// Returns `Some(T)` if an element was overwritten, `None` otherwise.
    type PushOutput = Option<T>;

    #[inline]
    fn push_impl(ringbuf: &RingBuf<T, N, true>, value: T) -> Self::PushOutput {
        let write = ringbuf.core.write_idx().fetch_add(1, Ordering::Relaxed);
        let index = write & ringbuf.core.mask();
        
        let read = ringbuf.core.read_idx().load(Ordering::Acquire);
        
        if write.wrapping_sub(read) >= ringbuf.core.capacity() {
            // Buffer was full. We must replace the value and advance read_idx.
            
            // Atomically advance read index.
            ringbuf.core.read_idx().compare_exchange(
                read,
                read.wrapping_add(1),
                Ordering::Release,
                Ordering::Relaxed,
            ).ok(); // We don't care if this fails, another writer might be doing it.
            
            // 替换 ptr 处的值，并返回旧值
            unsafe { Some(ringbuf.core.replace_at(index, value)) }
        } else {
            // Buffer not full. Just write.
            unsafe { ringbuf.core.write_at(index, value); }
            None
        }
    }
}

impl<T, const N: usize> PushDispatch<T, N, false> for PushMarker<false> {
    /// Returns `Ok(())` on success, or `Err(RingBufError::Full(T))` if full.
    type PushOutput = Result<(), RingBufError<T>>;

    #[inline]
    fn push_impl(ringbuf: &RingBuf<T, N, false>, value: T) -> Self::PushOutput {
        let write = ringbuf.core.write_idx().fetch_add(1, Ordering::Relaxed);
        
        let read = ringbuf.core.read_idx().load(Ordering::Acquire);
        if write.wrapping_sub(read) >= ringbuf.core.capacity() {
            // Full. Revert write index and return error.
            ringbuf.core.write_idx().fetch_sub(1, Ordering::Relaxed);
            Err(RingBufError::Full(value))
        } else {
            // Not full. Write value.
            let index = write & ringbuf.core.mask();
            unsafe {
                ringbuf.core.write_at(index, value);
            }
            Ok(())
        }
    }
}


impl<T, const N: usize, const OVERWRITE: bool> RingBuf<T, N, OVERWRITE> {
    /// Create a new ring buffer with the specified capacity
    /// 
    /// 创建指定容量的新环形缓冲区
    /// 
    /// Capacity will be rounded up to the next power of 2 for efficient masking.
    /// The overwrite mode is determined at compile time via the OVERWRITE const parameter.
    /// 
    /// 容量将向上取整到下一个 2 的幂次以实现高效的掩码操作。
    /// 覆盖模式通过 OVERWRITE 常量参数在编译期确定。
    /// 
    /// # Parameters
    /// 
    /// - `capacity`: Desired capacity (will be rounded up to next power of 2)
    /// 
    /// # 参数
    /// 
    /// - `capacity`: 期望容量（将向上取整到下一个 2 的幂次）
    /// 
    /// # Examples
    /// 
    /// ```
    /// use smallring::generic::RingBuf;
    ///
    /// // Create a ring buffer with overwrite mode
    /// let buf: RingBuf<i32, 32, true> = RingBuf::new(10);
    /// assert_eq!(buf.capacity(), 16); // Rounded up to power of 2
    ///
    /// // Create a ring buffer with non-overwrite mode
    /// let buf: RingBuf<i32, 32, false> = RingBuf::new(10);
    /// ```
    pub fn new(capacity: usize) -> Self {
        let core = RingBufCore::new(capacity);
        
        Self {
            core,
        }
    }
    
    /// Get the capacity of the ring buffer
    /// 
    /// 获取环形缓冲区的容量
    #[inline]
    pub fn capacity(&self) -> usize {
        self.core.capacity()
    }
    
    /// Get the number of elements currently in the buffer
    /// 
    /// 获取缓冲区中当前的元素数量
    #[inline]
    pub fn len(&self) -> usize {
        self.core.len()
    }
    
    /// Check if the buffer is empty
    /// 
    /// 检查缓冲区是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.core.is_empty()
    }
    
    /// Check if the buffer is full
    /// 
    /// 检查缓冲区是否已满
    #[inline]
    pub fn is_full(&self) -> bool {
        self.core.is_full()
    }
    
    // [MODIFIED] -------------------------------------------------------------
    // `push` 函数现在使用 Trait 分发，具有依赖于 OVERWRITE 的返回类型
    
    /// Push an element into the buffer
    /// 
    /// 向缓冲区推送一个元素
    /// 
    /// # Behavior
    /// 
    /// - **Overwrite mode (OVERWRITE=true)**: Always succeeds. Returns `Some(T)` if an element was overwritten, `None` otherwise.
    /// - **Non-overwrite mode (OVERWRITE=false)**: Returns `Err(RingBufError::Full(value))` if buffer is full, `Ok(())` otherwise.
    /// 
    /// # 行为
    /// 
    /// - **覆盖模式 (OVERWRITE=true)**: 总是成功。如果覆盖了元素则返回 `Some(T)`，否则返回 `None`。
    /// - **非覆盖模式 (OVERWRITE=false)**: 如果缓冲区满则返回 `Err(RingBufError::Full(value))`，否则返回 `Ok(())`。
    /// 
    /// # Examples
    /// 
    /// ```
    /// use smallring::generic::{RingBuf, RingBufError};
    ///
    /// // Overwrite mode (OVERWRITE = true)
    /// let buf_overwrite: RingBuf<i32, 32, true> = RingBuf::new(2);
    /// assert_eq!(buf_overwrite.push(1), None);
    /// assert_eq!(buf_overwrite.push(2), None);
    /// assert_eq!(buf_overwrite.push(3), Some(1)); // Overwrote 1
    ///
    /// // Non-overwrite mode (OVERWRITE = false)
    /// let buf_no_overwrite: RingBuf<i32, 32, false> = RingBuf::new(2);
    /// assert_eq!(buf_no_overwrite.push(1), Ok(()));
    /// assert_eq!(buf_no_overwrite.push(2), Ok(()));
    /// assert!(matches!(buf_no_overwrite.push(3), Err(RingBufError::Full(3))));
    /// ```
    #[inline]
    pub fn push(&self, value: T) -> <PushMarker<OVERWRITE> as PushDispatch<T, N, OVERWRITE>>::PushOutput
    where
        PushMarker<OVERWRITE>: PushDispatch<T, N, OVERWRITE>,
    {
        PushMarker::<OVERWRITE>::push_impl(self, value)
    }
    // ----------------------------------------------------------- [END MODIFIED]

    
    /// Pop an element from the buffer
    /// 
    /// 从缓冲区弹出一个元素
    /// 
    /// # Errors
    /// 
    /// Returns `Err(RingBufError::Empty)` if the buffer is empty.
    /// 
    /// # 错误
    /// 
    /// 如果缓冲区为空则返回 `Err(RingBufError::Empty)`。
    /// 
    /// # Examples
    /// 
    /// ```
    /// use smallring::generic::RingBuf;
    ///
    /// let buf: RingBuf<i32, 32> = RingBuf::new(4); // Defaults to OVERWRITE = true
    /// buf.push(42);
    /// assert_eq!(buf.pop().unwrap(), 42);
    /// ```
    pub fn pop(&self) -> Result<T, RingBufError<T>> {
        let read = self.core.read_idx().load(Ordering::Relaxed);
        let write = self.core.write_idx().load(Ordering::Acquire);
        
        // Check if buffer is empty
        if read == write {
            return Err(RingBufError::Empty);
        }
        
        // Read value from buffer
        let index = read & self.core.mask();
        let value = unsafe {
            self.core.read_at(index)
        };
        
        // Update read index
        self.core.read_idx().store(read.wrapping_add(1), Ordering::Release);
        
        Ok(value)
    }
    
    /// Peek at the first element without removing it
    /// 
    /// 查看第一个元素但不移除它
    /// 
    /// # Returns
    /// 
    /// `Some(&T)` if there is an element, `None` if the buffer is empty
    /// 
    /// # 返回值
    /// 
    /// 如果有元素则返回 `Some(&T)`，如果缓冲区为空则返回 `None`
    /// 
    /// # Safety
    /// 
    /// The returned reference is valid only as long as no other operations
    /// are performed that might modify the buffer.
    /// 
    /// # 安全性
    /// 
    /// 返回的引用仅在未执行可能修改缓冲区的其他操作时有效。
    /// 
    /// # Examples
    /// 
    /// ```
    /// use smallring::generic::RingBuf;
    ///
    /// let buf: RingBuf<i32, 32> = RingBuf::new(4);
    /// buf.push(42);
    /// assert_eq!(buf.peek(), Some(&42));
    /// assert_eq!(buf.len(), 1); // Element still in buffer
    /// ```
    #[inline]
    pub fn peek(&self) -> Option<&T> {
        let read = self.core.read_idx().load(Ordering::Relaxed);
        let write = self.core.write_idx().load(Ordering::Acquire);
        
        if read == write {
            return None;
        }
        
        let index = read & self.core.mask();
        unsafe {
            Some(self.core.peek_at(index))
        }
    }
    
    /// Clear all elements from the buffer
    /// 
    /// 清空缓冲区中的所有元素
    /// 
    /// This method pops and drops all elements currently in the buffer.
    /// 
    /// 此方法弹出并 drop 缓冲区中当前的所有元素。
    pub fn clear(&self) {
        while self.pop().is_ok() {
            // Elements are dropped automatically
        }
    }
}

impl<T: Copy, const N: usize, const OVERWRITE: bool> RingBuf<T, N, OVERWRITE> {
    /// Push multiple elements from a slice into the buffer
    /// 
    /// 将切片中的多个元素批量推送到缓冲区
    /// 
    /// # Behavior
    /// 
    /// - **Overwrite mode (OVERWRITE=true)**: Pushes all elements, overwriting old data if necessary
    /// - **Non-overwrite mode (OVERWRITE=false)**: Pushes as many elements as possible, returns number pushed
    /// 
    /// # 行为
    /// 
    /// - **覆盖模式 (OVERWRITE=true)**: 推送所有元素，必要时覆盖旧数据
    /// - **非覆盖模式 (OVERWRITE=false)**: 推送尽可能多的元素，返回推送的数量
    /// 
    /// # Returns
    /// 
    /// Number of elements successfully pushed (0 to values.len())
    /// 
    /// # 返回值
    /// 
    /// 成功推送的元素数量（0 到 values.len()）
    pub fn push_slice(&self, values: &[T]) -> usize {
        if values.is_empty() {
            return 0;
        }
        
        let write = self.core.write_idx().fetch_add(values.len(), Ordering::Relaxed);
        let read = self.core.read_idx().load(Ordering::Acquire);
        
        let to_push = if OVERWRITE {
            // Overwrite mode: push all elements
            // If pushing more than capacity, advance read index
            if values.len() >= self.core.capacity() {
                // All old data will be overwritten, set read to write - capacity
                self.core.read_idx().store(write.wrapping_add(values.len()).wrapping_sub(self.core.capacity()), Ordering::Release);
            } else if write.wrapping_sub(read) + values.len() > self.core.capacity() {
                // Partial overwrite: advance read index by overflow amount
                let overflow = write.wrapping_sub(read) + values.len() - self.core.capacity();
                self.core.read_idx().fetch_add(overflow, Ordering::Release);
            }
            values.len()
        } else {
            // Non-overwrite mode: push what fits
            let available = self.core.capacity().saturating_sub(write.wrapping_sub(read));
            let to_push = available.min(values.len());
            
            if to_push < values.len() {
                // Revert excess write index increment
                self.core.write_idx().fetch_sub(values.len() - to_push, Ordering::Relaxed);
            }
            
            to_push
        };
        
        if to_push == 0 {
            return 0;
        }
        
        // Use core's batch copy functionality
        // 使用核心模块的批量拷贝功能
        unsafe {
            self.core.copy_from_slice(write, values, to_push);
        }
        
        to_push
    }
    
    /// Pop multiple elements into a slice
    /// 
    /// 将多个元素批量弹出到切片
    /// 
    /// # Returns
    /// 
    /// Number of elements successfully popped (0 to dest.len())
    /// 
    /// # 返回值
    /// 
    /// 成功弹出的元素数量（0 到 dest.len()）
    pub fn pop_slice(&self, dest: &mut [T]) -> usize {
        if dest.is_empty() {
            return 0;
        }
        
        let read = self.core.read_idx().load(Ordering::Relaxed);
        let write = self.core.write_idx().load(Ordering::Acquire);
        
        // Calculate available elements
        let available = write.wrapping_sub(read).min(self.core.capacity());
        let to_pop = available.min(dest.len());
        
        if to_pop == 0 {
            return 0;
        }
        
        // Use core's batch copy functionality
        // 使用核心模块的批量拷贝功能
        unsafe {
            self.core.copy_to_slice(read, dest, to_pop);
        }
        
        // Update read index
        self.core.read_idx().fetch_add(to_pop, Ordering::Release);
        
        to_pop
    }
}

// Ensure RingBuf is Send and Sync if T is Send
unsafe impl<T: Send, const N: usize, const OVERWRITE: bool> Send for RingBuf<T, N, OVERWRITE> {}
unsafe impl<T: Send, const N: usize, const OVERWRITE: bool> Sync for RingBuf<T, N, OVERWRITE> {}

// Note: RingBuf does NOT implement Drop because FixedVec stores MaybeUninit<T>.
// Elements are dropped when popped via pop() or clear().
// Any remaining elements when RingBuf is dropped will leak.
// For proper cleanup, call clear() before dropping.

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_capacity_rounding() {
        let buf: RingBuf<i32, 32, true> = RingBuf::new(5);
        assert_eq!(buf.capacity(), 8);
        
        let buf: RingBuf<i32, 32, true> = RingBuf::new(8);
        assert_eq!(buf.capacity(), 8);
        
        let buf: RingBuf<i32, 32, true> = RingBuf::new(9);
        assert_eq!(buf.capacity(), 16);
    }
    
    #[test]
    fn test_basic_push_pop() {
        let buf: RingBuf<i32, 32, true> = RingBuf::new(4);
        
        // [MODIFIED] Test updated for new push() API
        assert_eq!(buf.push(1), None);
        assert_eq!(buf.push(2), None);
        assert_eq!(buf.push(3), None);
        
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.pop().unwrap(), 1);
        assert_eq!(buf.pop().unwrap(), 2);
        assert_eq!(buf.pop().unwrap(), 3);
        assert!(buf.is_empty());
    }
    
    #[test]
    fn test_overwrite_mode() {
        let buf: RingBuf<i32, 32, true> = RingBuf::new(4);
        
        // [MODIFIED] Test updated for new push() API
        // Fill buffer
        assert_eq!(buf.push(1), None);
        assert_eq!(buf.push(2), None);
        assert_eq!(buf.push(3), None);
        assert_eq!(buf.push(4), None);
        
        // Overwrite oldest elements
        assert_eq!(buf.push(5), Some(1)); // Overwrote 1
        assert_eq!(buf.push(6), Some(2)); // Overwrote 2
        
        // Should get elements 3, 4, 5, 6 (1 and 2 were overwritten)
        assert_eq!(buf.pop().unwrap(), 3);
        assert_eq!(buf.pop().unwrap(), 4);
        assert_eq!(buf.pop().unwrap(), 5);
        assert_eq!(buf.pop().unwrap(), 6);
    }
    
    #[test]
    fn test_non_overwrite_mode() {
        let buf: RingBuf<i32, 32, false> = RingBuf::new(4);
        
        // [MODIFIED] Test updated for new push() API (unwrap() still works)
        // Fill buffer
        buf.push(1).unwrap();
        buf.push(2).unwrap();
        buf.push(3).unwrap();
        buf.push(4).unwrap();
        
        // Should fail to push when full
        assert!(matches!(buf.push(5), Err(RingBufError::Full(5))));
        
        // Pop one element
        assert_eq!(buf.pop().unwrap(), 1);
        
        // Now should be able to push again
        buf.push(5).unwrap();
    }
    
    #[test]
    fn test_empty_buffer() {
        let buf: RingBuf<i32, 32, true> = RingBuf::new(4);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(matches!(buf.pop(), Err(RingBufError::Empty)));
    }
    
    #[test]
    fn test_push_slice() {
        let buf: RingBuf<i32, 32, true> = RingBuf::new(8);
        
        let data = [1, 2, 3, 4, 5];
        let pushed = buf.push_slice(&data);
        assert_eq!(pushed, 5);
        assert_eq!(buf.len(), 5);
        
        assert_eq!(buf.pop().unwrap(), 1);
        assert_eq!(buf.pop().unwrap(), 2);
        assert_eq!(buf.pop().unwrap(), 3);
    }
    
    #[test]
    fn test_pop_slice() {
        let buf: RingBuf<i32, 32, true> = RingBuf::new(8);
        
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);
        
        let mut dest = [0i32; 3];
        let popped = buf.pop_slice(&mut dest);
        assert_eq!(popped, 3);
        assert_eq!(dest, [1, 2, 3]);
        
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.pop().unwrap(), 4);
    }
    
    #[test]
    fn test_clear() {
        let buf: RingBuf<i32, 32, true> = RingBuf::new(8);
        
        buf.push(1);
        buf.push(2);
        buf.push(3);
        
        assert_eq!(buf.len(), 3);
        buf.clear();
        assert!(buf.is_empty());
    }
    
    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;
        
        let buf = Arc::new(RingBuf::<u64, 128, true>::new(128));
        let mut handles = vec![];
        
        // Multiple writers
        for thread_id in 0..4 {
            let buf_clone = Arc::clone(&buf);
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let value = (thread_id * 100 + i) as u64;
                    // [MODIFIED] Test updated for new push() API
                    // We don't care about the return value in this test
                    buf_clone.push(value);
                }
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Should have written 400 elements total
        assert_eq!(buf.len(), 128); // Only last 128 fit in buffer
    }
}