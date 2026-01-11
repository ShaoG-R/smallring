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
use std::fmt;
use std::sync::atomic::Ordering;

/// Iterator over ring buffer elements
///
/// 环形缓冲区元素的迭代器
pub struct Iter<'a, T> {
    first: std::slice::Iter<'a, T>,
    second: std::slice::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.first.next().or_else(|| self.second.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (first_low, first_high) = self.first.size_hint();
        let (second_low, second_high) = self.second.size_hint();
        let low = first_low + second_low;
        let high = first_high.and_then(|fh| second_high.map(|sh| fh + sh));
        (low, high)
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.first.len() + self.second.len()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.second.next_back().or_else(|| self.first.next_back())
    }
}

/// Mutable iterator over ring buffer elements
///
/// 环形缓冲区元素的可变迭代器
pub struct IterMut<'a, T> {
    first: std::slice::IterMut<'a, T>,
    second: std::slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.first.next().or_else(|| self.second.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (first_low, first_high) = self.first.size_hint();
        let (second_low, second_high) = self.second.size_hint();
        let low = first_low + second_low;
        let high = first_high.and_then(|fh| second_high.map(|sh| fh + sh));
        (low, high)
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.first.len() + self.second.len()
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.second.next_back().or_else(|| self.first.next_back())
    }
}

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
    fn push_impl(ringbuf: &mut RingBuf<T, N, OVERWRITE>, value: T) -> Self::PushOutput;
}

/// Marker struct for compile-time dispatch.
pub struct PushMarker<const OVERWRITE: bool>;

impl<T, const N: usize> PushDispatch<T, N, true> for PushMarker<true> {
    /// Returns `Some(T)` if an element was overwritten, `None` otherwise.
    type PushOutput = Option<T>;

    #[inline]
    fn push_impl(ringbuf: &mut RingBuf<T, N, true>, value: T) -> Self::PushOutput {
        let write = ringbuf.core.write_idx().fetch_add(1, Ordering::Relaxed);
        let index = write & ringbuf.core.mask();

        let read = ringbuf.core.read_idx().load(Ordering::Acquire);

        if write.wrapping_sub(read) >= ringbuf.core.capacity() {
            // Buffer was full. We must replace the value and advance read_idx.

            // Atomically advance read index.
            ringbuf
                .core
                .read_idx()
                .compare_exchange(
                    read,
                    read.wrapping_add(1),
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .ok(); // We don't care if this fails, another writer might be doing it.

            // 替换 ptr 处的值，并返回旧值
            unsafe { Some(ringbuf.core.replace_at(index, value)) }
        } else {
            // Buffer not full. Just write.
            unsafe {
                ringbuf.core.write_at(index, value);
            }
            None
        }
    }
}

impl<T, const N: usize> PushDispatch<T, N, false> for PushMarker<false> {
    /// Returns `Ok(())` on success, or `Err(RingBufError::Full(T))` if full.
    type PushOutput = Result<(), RingBufError<T>>;

    #[inline]
    fn push_impl(ringbuf: &mut RingBuf<T, N, false>, value: T) -> Self::PushOutput {
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

        Self { core }
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
    /// let mut buf_overwrite: RingBuf<i32, 32, true> = RingBuf::new(2);
    /// assert_eq!(buf_overwrite.push(1), None);
    /// assert_eq!(buf_overwrite.push(2), None);
    /// assert_eq!(buf_overwrite.push(3), Some(1)); // Overwrote 1
    ///
    /// // Non-overwrite mode (OVERWRITE = false)
    /// let mut buf_no_overwrite: RingBuf<i32, 32, false> = RingBuf::new(2);
    /// assert_eq!(buf_no_overwrite.push(1), Ok(()));
    /// assert_eq!(buf_no_overwrite.push(2), Ok(()));
    /// assert!(matches!(buf_no_overwrite.push(3), Err(RingBufError::Full(3))));
    /// ```
    #[inline]
    pub fn push(
        &mut self,
        value: T,
    ) -> <PushMarker<OVERWRITE> as PushDispatch<T, N, OVERWRITE>>::PushOutput
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
    /// let mut buf: RingBuf<i32, 32> = RingBuf::new(4); // Defaults to OVERWRITE = true
    /// buf.push(42);
    /// assert_eq!(buf.pop().unwrap(), 42);
    /// ```
    pub fn pop(&mut self) -> Result<T, RingBufError<T>> {
        let read = self.core.read_idx().load(Ordering::Relaxed);
        let write = self.core.write_idx().load(Ordering::Acquire);

        // Check if buffer is empty
        if read == write {
            return Err(RingBufError::Empty);
        }

        // Read value from buffer
        let index = read & self.core.mask();
        let value = unsafe { self.core.read_at(index) };

        // Update read index
        self.core
            .read_idx()
            .store(read.wrapping_add(1), Ordering::Release);

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
    /// # Examples
    ///
    /// ```
    /// use smallring::generic::RingBuf;
    ///
    /// let mut buf: RingBuf<i32, 32> = RingBuf::new(4);
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
        unsafe { Some(self.core.peek_at(index)) }
    }

    /// Clear all elements from the buffer
    ///
    /// 清空缓冲区中的所有元素
    ///
    /// This method pops and drops all elements currently in the buffer.
    ///
    /// 此方法弹出并 drop 缓冲区中当前的所有元素。
    pub fn clear(&mut self) {
        while self.pop().is_ok() {
            // Elements are dropped automatically
        }
    }
}

impl<T, const N: usize, const OVERWRITE: bool> RingBuf<T, N, OVERWRITE> {
    /// Get readable data as one or two contiguous slices
    ///
    /// 将可读数据作为一个或两个连续切片获取
    ///
    /// Due to the circular nature of the ring buffer, data may be split into two parts.
    /// The first slice contains data from the read position to either the end of the buffer
    /// or the write position. The second slice (if non-empty) contains data from the
    /// beginning of the buffer.
    ///
    /// 由于环形缓冲区的循环特性，数据可能被分成两部分。
    /// 第一个切片包含从读位置到缓冲区末尾或写位置的数据。
    /// 第二个切片（如果非空）包含从缓冲区开头的数据。
    ///
    /// # Returns
    ///
    /// A tuple of two slices `(first, second)` where:
    /// - `first` contains the initial contiguous data
    /// - `second` contains wrapped-around data (may be empty)
    ///
    /// # 返回值
    ///
    /// 返回两个切片的元组 `(first, second)`，其中：
    /// - `first` 包含初始连续数据
    /// - `second` 包含环绕的数据（可能为空）
    ///
    /// # Examples
    ///
    /// ```
    /// use smallring::generic::RingBuf;
    ///
    /// let mut buf: RingBuf<i32, 32> = RingBuf::new(8);
    /// buf.push(1);
    /// buf.push(2);
    /// buf.push(3);
    ///
    /// let (first, second) = buf.as_slices();
    /// assert_eq!(first, &[1, 2, 3]);
    /// assert_eq!(second, &[]);
    /// ```
    pub fn as_slices(&self) -> (&[T], &[T]) {
        let read = self.core.read_idx().load(Ordering::Acquire);
        let write = self.core.write_idx().load(Ordering::Acquire);

        let len = write.wrapping_sub(read).min(self.core.capacity());
        if len == 0 {
            return (&[], &[]);
        }

        let read_idx = read & self.core.mask();
        let write_idx = write & self.core.mask();

        unsafe {
            let buffer_ptr = self.core.buffer_ptr();

            if read_idx < write_idx || len == self.core.capacity() {
                // Data is contiguous or buffer is full
                if len == self.core.capacity() {
                    // Full buffer - return two slices
                    let first_len = self.core.capacity() - read_idx;
                    let first = std::slice::from_raw_parts(buffer_ptr.add(read_idx), first_len);
                    let second = if read_idx > 0 {
                        std::slice::from_raw_parts(buffer_ptr, read_idx)
                    } else {
                        &[]
                    };
                    (first, second)
                } else {
                    // Data is contiguous
                    let slice = std::slice::from_raw_parts(buffer_ptr.add(read_idx), len);
                    (slice, &[])
                }
            } else {
                // Data wraps around
                let first_len = self.core.capacity() - read_idx;
                let second_len = len - first_len;
                let first = std::slice::from_raw_parts(buffer_ptr.add(read_idx), first_len);
                let second = std::slice::from_raw_parts(buffer_ptr, second_len);
                (first, second)
            }
        }
    }

    /// Create an iterator over the elements in the buffer
    ///
    /// 创建一个遍历缓冲区元素的迭代器
    ///
    /// The iterator yields references to elements in FIFO order (oldest to newest).
    ///
    /// 迭代器按 FIFO 顺序（从最旧到最新）返回元素的引用。
    ///
    /// # Examples
    ///
    /// ```
    /// use smallring::generic::RingBuf;
    ///
    /// let mut buf: RingBuf<i32, 32> = RingBuf::new(8);
    /// buf.push(1);
    /// buf.push(2);
    /// buf.push(3);
    ///
    /// let values: Vec<_> = buf.iter().copied().collect();
    /// assert_eq!(values, vec![1, 2, 3]);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        let (first, second) = self.as_slices();
        Iter {
            first: first.iter(),
            second: second.iter(),
        }
    }

    /// Get mutable readable data as one or two contiguous slices
    ///
    /// 将可读数据作为一个或两个可变连续切片获取
    ///
    /// Due to the circular nature of the ring buffer, data may be split into two parts.
    /// The first slice contains data from the read position to either the end of the buffer
    /// or the write position. The second slice (if non-empty) contains data from the
    /// beginning of the buffer.
    ///
    /// 由于环形缓冲区的循环特性，数据可能被分成两部分。
    /// 第一个切片包含从读位置到缓冲区末尾或写位置的数据。
    /// 第二个切片（如果非空）包含从缓冲区开头的数据。
    ///
    /// # Returns
    ///
    /// A tuple of two mutable slices `(first, second)` where:
    /// - `first` contains the initial contiguous data
    /// - `second` contains wrapped-around data (may be empty)
    ///
    /// # 返回值
    ///
    /// 返回两个可变切片的元组 `(first, second)`，其中：
    /// - `first` 包含初始连续数据
    /// - `second` 包含环绕的数据（可能为空）
    ///
    /// # Examples
    ///
    /// ```
    /// use smallring::generic::RingBuf;
    ///
    /// let mut buf: RingBuf<i32, 32> = RingBuf::new(8);
    /// buf.push(1);
    /// buf.push(2);
    /// buf.push(3);
    ///
    /// let (first, second) = buf.as_mut_slices();
    /// // Modify elements in place
    /// for x in first.iter_mut() {
    ///     *x *= 2;
    /// }
    /// for x in second.iter_mut() {
    ///     *x *= 2;
    /// }
    ///
    /// assert_eq!(buf.pop().unwrap(), 2);
    /// assert_eq!(buf.pop().unwrap(), 4);
    /// assert_eq!(buf.pop().unwrap(), 6);
    /// ```
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        let read = self.core.read_idx().load(Ordering::Acquire);
        let write = self.core.write_idx().load(Ordering::Acquire);

        let len = write.wrapping_sub(read).min(self.core.capacity());
        if len == 0 {
            return (&mut [], &mut []);
        }

        let read_idx = read & self.core.mask();
        let write_idx = write & self.core.mask();

        unsafe {
            let buffer_ptr = self.core.buffer_ptr() as *mut T;

            if read_idx < write_idx || len == self.core.capacity() {
                // Data is contiguous or buffer is full
                if len == self.core.capacity() {
                    // Full buffer - return two slices
                    let first_len = self.core.capacity() - read_idx;
                    let first = std::slice::from_raw_parts_mut(buffer_ptr.add(read_idx), first_len);
                    let second = if read_idx > 0 {
                        std::slice::from_raw_parts_mut(buffer_ptr, read_idx)
                    } else {
                        &mut []
                    };
                    (first, second)
                } else {
                    // Data is contiguous
                    let slice = std::slice::from_raw_parts_mut(buffer_ptr.add(read_idx), len);
                    (slice, &mut [])
                }
            } else {
                // Data wraps around
                let first_len = self.core.capacity() - read_idx;
                let second_len = len - first_len;
                let first = std::slice::from_raw_parts_mut(buffer_ptr.add(read_idx), first_len);
                let second = std::slice::from_raw_parts_mut(buffer_ptr, second_len);
                (first, second)
            }
        }
    }

    /// Create a mutable iterator over the elements in the buffer
    ///
    /// 创建一个遍历缓冲区元素的可变迭代器
    ///
    /// The iterator yields mutable references to elements in FIFO order (oldest to newest).
    ///
    /// 迭代器按 FIFO 顺序（从最旧到最新）返回元素的可变引用。
    ///
    /// # Examples
    ///
    /// ```
    /// use smallring::generic::RingBuf;
    ///
    /// let mut buf: RingBuf<i32, 32> = RingBuf::new(8);
    /// buf.push(1);
    /// buf.push(2);
    /// buf.push(3);
    ///
    /// // Double all values
    /// for x in buf.iter_mut() {
    ///     *x *= 2;
    /// }
    ///
    /// let values: Vec<_> = buf.iter().copied().collect();
    /// assert_eq!(values, vec![2, 4, 6]);
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        let (first, second) = self.as_mut_slices();
        IterMut {
            first: first.iter_mut(),
            second: second.iter_mut(),
        }
    }
}

impl<T: Clone, const N: usize, const OVERWRITE: bool> Clone for RingBuf<T, N, OVERWRITE> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
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
    pub fn push_slice(&mut self, values: &[T]) -> usize {
        if values.is_empty() {
            return 0;
        }

        let write = self
            .core
            .write_idx()
            .fetch_add(values.len(), Ordering::Relaxed);
        let read = self.core.read_idx().load(Ordering::Acquire);

        let (to_push, value_offset, overwrite_count) = if OVERWRITE {
            // Overwrite mode: push all elements (or last capacity elements if values.len() > capacity)
            // If pushing more than capacity, advance read index
            if values.len() >= self.core.capacity() {
                // All old data will be overwritten
                // Only keep last capacity elements from values
                // 所有旧数据将被覆盖，只保留 values 中最后 capacity 个元素
                let to_push = self.core.capacity();
                let value_offset = values.len() - to_push;
                let old_read = read;
                self.core.read_idx().store(
                    write.wrapping_add(values.len()).wrapping_sub(to_push),
                    Ordering::Release,
                );
                // All elements in buffer will be overwritten
                let overwrite_count = (write.wrapping_sub(old_read)).min(self.core.capacity());
                (to_push, value_offset, overwrite_count)
            } else if write.wrapping_sub(read) + values.len() > self.core.capacity() {
                // Partial overwrite: advance read index by overflow amount
                let overflow = write.wrapping_sub(read) + values.len() - self.core.capacity();
                self.core.read_idx().fetch_add(overflow, Ordering::Release);
                (values.len(), 0, overflow)
            } else {
                (values.len(), 0, 0)
            }
        } else {
            // Non-overwrite mode: push what fits
            let available = self
                .core
                .capacity()
                .saturating_sub(write.wrapping_sub(read));
            let to_push = available.min(values.len());

            if to_push < values.len() {
                // Revert excess write index increment
                self.core
                    .write_idx()
                    .fetch_sub(values.len() - to_push, Ordering::Relaxed);
            }

            (to_push, 0, 0)
        };

        if to_push == 0 {
            return 0;
        }

        // Drop elements that will be overwritten (in overwrite mode)
        // Drop 将被覆盖的元素（覆盖模式）
        if OVERWRITE && overwrite_count > 0 {
            unsafe {
                for i in 0..overwrite_count {
                    let idx = (write.wrapping_add(value_offset).wrapping_add(i)) & self.core.mask();
                    let ptr = self.core.buffer_ptr_at(idx) as *mut T;
                    std::ptr::drop_in_place(ptr);
                }
            }
        }

        // Use core's batch copy functionality
        // 使用核心模块的批量拷贝功能
        // Copy from values[value_offset..] to buffer
        unsafe {
            self.core.copy_from_slice(
                write.wrapping_add(value_offset),
                &values[value_offset..],
                to_push,
            );
        }

        // Return number of elements pushed
        // Overwrite mode: return total values.len() (all "accepted")
        // Non-overwrite mode: return actual pushed count
        if OVERWRITE { values.len() } else { to_push }
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
    pub fn pop_slice(&mut self, dest: &mut [T]) -> usize {
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

impl<T: fmt::Debug, const N: usize, const OVERWRITE: bool> fmt::Debug for RingBuf<T, N, OVERWRITE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RingBuf")
            .field("capacity", &self.core.capacity())
            .field("len", &self.core.len())
            .field("is_empty", &self.core.is_empty())
            .field("is_full", &self.core.is_full())
            .field("overwrite_mode", &OVERWRITE)
            .finish()
    }
}

// Ensure RingBuf is Send and Sync if T is Send
unsafe impl<T: Send, const N: usize, const OVERWRITE: bool> Send for RingBuf<T, N, OVERWRITE> {}
unsafe impl<T: Send, const N: usize, const OVERWRITE: bool> Sync for RingBuf<T, N, OVERWRITE> {}

// Note: RingBuf does NOT implement Drop because FixedVec stores MaybeUninit<T>.
// Elements are dropped when popped via pop() or clear().
// Any remaining elements when RingBuf is dropped will leak.
// For proper cleanup, call clear() before dropping.

#[cfg(all(test, not(feature = "loom")))]
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
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

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
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

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
        let mut buf: RingBuf<i32, 32, false> = RingBuf::new(4);

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
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(matches!(buf.pop(), Err(RingBufError::Empty)));
    }

    #[test]
    fn test_push_slice() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

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
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

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
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

        buf.push(1);
        buf.push(2);
        buf.push(3);

        assert_eq!(buf.len(), 3);
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let buf = Arc::new(Mutex::new(RingBuf::<u64, 128, true>::new(128)));
        let mut handles = vec![];

        // Multiple writers
        for thread_id in 0..4 {
            let buf_clone = Arc::clone(&buf);
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let value = (thread_id * 100 + i) as u64;
                    // [MODIFIED] Test updated for new push() API with Mutex
                    // We don't care about the return value in this test
                    buf_clone.lock().unwrap().push(value);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have written 400 elements total
        assert_eq!(buf.lock().unwrap().len(), 128); // Only last 128 fit in buffer
    }

    #[test]
    fn test_peek() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

        // Peek on empty buffer
        assert_eq!(buf.peek(), None);

        // Add elements
        buf.push(1);
        buf.push(2);
        buf.push(3);

        // Peek should return first element without removing it
        assert_eq!(buf.peek(), Some(&1));
        assert_eq!(buf.len(), 3);

        // Pop and peek again
        assert_eq!(buf.pop().unwrap(), 1);
        assert_eq!(buf.peek(), Some(&2));
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_is_full() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

        assert!(!buf.is_full());

        buf.push(1);
        buf.push(2);
        assert!(!buf.is_full());

        buf.push(3);
        buf.push(4);
        assert!(buf.is_full());

        buf.pop().unwrap();
        assert!(!buf.is_full());
    }

    #[test]
    fn test_as_slices_contiguous() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

        // Empty buffer
        let (first, second) = buf.as_slices();
        assert_eq!(first, &[]);
        assert_eq!(second, &[]);

        // Add elements (contiguous)
        buf.push(1);
        buf.push(2);
        buf.push(3);

        let (first, second) = buf.as_slices();
        assert_eq!(first, &[1, 2, 3]);
        assert_eq!(second, &[]);
    }

    #[test]
    fn test_as_slices_wrapped() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

        // Fill and wrap around
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);

        // Pop some elements to create gap
        buf.pop().unwrap();
        buf.pop().unwrap();

        // Add more to wrap around
        buf.push(5);
        buf.push(6);

        let (first, second) = buf.as_slices();
        assert_eq!(first.len() + second.len(), 4);

        // Verify we can read all elements
        let mut all_elements = Vec::new();
        all_elements.extend_from_slice(first);
        all_elements.extend_from_slice(second);
        assert_eq!(all_elements, vec![3, 4, 5, 6]);
    }

    #[test]
    fn test_iter() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

        // Empty iterator
        assert_eq!(buf.iter().count(), 0);

        // Add elements
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);

        // Collect iterator
        let values: Vec<_> = buf.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3, 4]);

        // Test ExactSizeIterator
        let iter = buf.iter();
        assert_eq!(iter.len(), 4);
    }

    #[test]
    fn test_iter_double_ended() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);

        // Test reverse iteration
        let values: Vec<_> = buf.iter().rev().copied().collect();
        assert_eq!(values, vec![4, 3, 2, 1]);

        // Test mixed forward/backward
        let mut iter = buf.iter();
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next_back(), Some(&4));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next_back(), Some(&3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_push_slice_overwrite() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

        // Fill buffer
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);

        // Push slice that causes overwrite
        let data = [5, 6, 7];
        let pushed = buf.push_slice(&data);
        assert_eq!(pushed, 3);

        // Should have [4, 5, 6, 7] now (1, 2, 3 overwritten)
        assert_eq!(buf.pop().unwrap(), 4);
        assert_eq!(buf.pop().unwrap(), 5);
        assert_eq!(buf.pop().unwrap(), 6);
        assert_eq!(buf.pop().unwrap(), 7);
    }

    #[test]
    fn test_push_slice_non_overwrite() {
        let mut buf: RingBuf<i32, 32, false> = RingBuf::new(4);

        // Fill buffer
        buf.push(1).unwrap();
        buf.push(2).unwrap();

        // Try to push more than available space
        let data = [3, 4, 5, 6];
        let pushed = buf.push_slice(&data);
        assert_eq!(pushed, 2); // Only 2 elements fit

        // Should have [1, 2, 3, 4]
        assert_eq!(buf.len(), 4);
        assert!(buf.is_full());
    }

    #[test]
    fn test_empty_slice_operations() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

        // Push empty slice
        let pushed = buf.push_slice(&[]);
        assert_eq!(pushed, 0);

        // Pop into empty slice
        buf.push(1);
        buf.push(2);

        let mut dest = [];
        let popped = buf.pop_slice(&mut dest);
        assert_eq!(popped, 0);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_as_mut_slices_contiguous() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

        // Empty buffer
        let (first, second) = buf.as_mut_slices();
        assert_eq!(first.len(), 0);
        assert_eq!(second.len(), 0);

        // Add elements (contiguous)
        buf.push(1);
        buf.push(2);
        buf.push(3);

        let (first, second) = buf.as_mut_slices();
        assert_eq!(first, &[1, 2, 3]);
        assert_eq!(second, &[]);

        // Modify elements
        for x in first.iter_mut() {
            *x *= 10;
        }

        assert_eq!(buf.pop().unwrap(), 10);
        assert_eq!(buf.pop().unwrap(), 20);
        assert_eq!(buf.pop().unwrap(), 30);
    }

    #[test]
    fn test_as_mut_slices_wrapped() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

        // Fill and wrap around
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);

        // Pop some elements to create gap
        buf.pop().unwrap();
        buf.pop().unwrap();

        // Add more to wrap around
        buf.push(5);
        buf.push(6);

        let (first, second) = buf.as_mut_slices();
        assert_eq!(first.len() + second.len(), 4);

        // Modify all elements
        for x in first.iter_mut() {
            *x += 100;
        }
        for x in second.iter_mut() {
            *x += 100;
        }

        // Verify modifications
        assert_eq!(buf.pop().unwrap(), 103);
        assert_eq!(buf.pop().unwrap(), 104);
        assert_eq!(buf.pop().unwrap(), 105);
        assert_eq!(buf.pop().unwrap(), 106);
    }

    #[test]
    fn test_iter_mut() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

        // Add elements
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);

        // Double all values using iter_mut
        for x in buf.iter_mut() {
            *x *= 2;
        }

        // Collect and verify
        let values: Vec<_> = buf.iter().copied().collect();
        assert_eq!(values, vec![2, 4, 6, 8]);

        // Test ExactSizeIterator
        let iter = buf.iter_mut();
        assert_eq!(iter.len(), 4);
    }

    #[test]
    fn test_iter_mut_double_ended() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(8);

        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);

        // Test reverse iteration with modification
        for x in buf.iter_mut().rev() {
            *x += 10;
        }

        let values: Vec<_> = buf.iter().copied().collect();
        assert_eq!(values, vec![11, 12, 13, 14]);

        // Test mixed forward/backward
        let mut iter = buf.iter_mut();
        if let Some(x) = iter.next() {
            *x = 100;
        }
        if let Some(x) = iter.next_back() {
            *x = 200;
        }

        let values: Vec<_> = buf.iter().copied().collect();
        assert_eq!(values, vec![100, 12, 13, 200]);
    }

    #[test]
    fn test_iter_mut_empty() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

        let mut iter = buf.iter_mut();
        assert_eq!(iter.next(), None);
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn test_iter_mut_wrapped() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);

        // Fill and wrap
        for i in 0..8 {
            buf.push(i);
        }

        // Should contain [4, 5, 6, 7]
        // Multiply all by 10
        for x in buf.iter_mut() {
            *x *= 10;
        }

        let values: Vec<i32> = buf.iter().copied().collect();
        assert_eq!(values, vec![40, 50, 60, 70]);
    }

    #[test]
    fn test_clone() {
        let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);
        buf.push(1);
        buf.push(2);
        buf.push(3);

        let buf_clone = buf.clone();

        assert_eq!(buf.len(), buf_clone.len());
        assert_eq!(buf.capacity(), buf_clone.capacity());

        let values: Vec<_> = buf_clone.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3]);

        // Modify original, clone should stay same
        buf.push(4);
        assert_eq!(buf.len(), 4);
        assert_eq!(buf_clone.len(), 3);
    }
}
