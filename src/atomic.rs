//! Atomic type ring buffer with specialized trait
//! 
//! 为原子类型专门设计的环形缓冲区
//! 
//! This module provides ring buffer implementation optimized for atomic types like AtomicU64.
//! Unlike generic ring buffers, atomic ring buffers don't move values but operate through
//! atomic load/store operations.
//! 
//! 本模块提供针对原子类型（如 AtomicU64）优化的环形缓冲区实现。
//! 与通用环形缓冲区不同，原子环形缓冲区不移动值，而是通过原子 load/store 操作进行读写。

use super::core::RingBufCore;
use std::sync::atomic::Ordering;

/// Trait for types that support atomic operations
/// 
/// 支持原子操作的类型 trait
/// 
/// This trait abstracts atomic operations to allow generic implementation
/// over different atomic types (AtomicU8, AtomicU64, AtomicUsize, etc.)
/// 
/// 此 trait 抽象了原子操作，允许在不同原子类型上实现泛型
/// (AtomicU8, AtomicU64, AtomicUsize 等)
pub trait AtomicElement: Send + Sync + Default {
    /// The underlying primitive type
    /// 
    /// 底层原始类型
    type Primitive: Copy;
    
    /// Create a new instance with the given value
    /// 
    /// 用给定值创建新实例
    fn new(val: Self::Primitive) -> Self;
    
    /// Load the value with specified ordering
    /// 
    /// 使用指定的内存顺序加载值
    fn load(&self, order: Ordering) -> Self::Primitive;
    
    /// Store a value with specified ordering
    /// 
    /// 使用指定的内存顺序存储值
    fn store(&self, val: Self::Primitive, order: Ordering);
    
    /// Swap the value and return the old value
    /// 
    /// 交换值并返回旧值
    fn swap(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
    
    /// Compare and exchange
    /// 
    /// 比较并交换
    fn compare_exchange(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive>;
}

// Implement for common atomic types
use std::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize, AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicBool};

macro_rules! impl_atomic_element {
    ($atomic:ty, $primitive:ty) => {
        impl AtomicElement for $atomic {
            type Primitive = $primitive;
            
            #[inline]
            fn new(val: Self::Primitive) -> Self {
                Self::new(val)
            }
            
            #[inline]
            fn load(&self, order: Ordering) -> Self::Primitive {
                self.load(order)
            }
            
            #[inline]
            fn store(&self, val: Self::Primitive, order: Ordering) {
                self.store(val, order);
            }
            
            #[inline]
            fn swap(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive {
                self.swap(val, order)
            }
            
            #[inline]
            fn compare_exchange(
                &self,
                current: Self::Primitive,
                new: Self::Primitive,
                success: Ordering,
                failure: Ordering,
            ) -> Result<Self::Primitive, Self::Primitive> {
                self.compare_exchange(current, new, success, failure)
            }
        }
    };
}

impl_atomic_element!(AtomicU8, u8);
impl_atomic_element!(AtomicU16, u16);
impl_atomic_element!(AtomicU32, u32);
impl_atomic_element!(AtomicU64, u64);
impl_atomic_element!(AtomicUsize, usize);
impl_atomic_element!(AtomicI8, i8);
impl_atomic_element!(AtomicI16, i16);
impl_atomic_element!(AtomicI32, i32);
impl_atomic_element!(AtomicI64, i64);
impl_atomic_element!(AtomicIsize, isize);
impl_atomic_element!(AtomicBool, bool);

/// Internal trait to dispatch push behavior based on OVERWRITE
/// 
/// 根据 OVERWRITE 分发 push 行为的内部 trait
pub trait PushDispatch<T: AtomicElement, const N: usize, const OVERWRITE: bool> {
    /// The return type of the push operation
    /// 
    /// push 操作的返回类型
    type PushOutput;
    
    /// The actual push implementation
    /// 
    /// 实际的 push 实现
    fn push_impl(
        ringbuf: &AtomicRingBuf<T, N, OVERWRITE>,
        value: T::Primitive,
        order: Ordering,
    ) -> Self::PushOutput;
}

/// Marker struct for compile-time dispatch
/// 
/// 编译期分发的标记结构
pub struct PushMarker<const OVERWRITE: bool>;

impl<T: AtomicElement, const N: usize> PushDispatch<T, N, true> for PushMarker<true> {
    /// Returns `Some(T::Primitive)` if an element was overwritten, `None` otherwise
    /// 
    /// 如果覆盖了元素则返回 `Some(T::Primitive)`，否则返回 `None`
    type PushOutput = Option<T::Primitive>;
    
    #[inline]
    fn push_impl(
        ringbuf: &AtomicRingBuf<T, N, true>,
        value: T::Primitive,
        order: Ordering,
    ) -> Self::PushOutput {
        let write = ringbuf.core.write_idx().fetch_add(1, Ordering::Relaxed);
        let read = ringbuf.core.read_idx().load(Ordering::Acquire);
        
        if write.wrapping_sub(read) >= ringbuf.core.capacity() {
            // Buffer was full - overwrite and advance read index
            ringbuf.core.read_idx().compare_exchange(
                read,
                read.wrapping_add(1),
                Ordering::Release,
                Ordering::Relaxed,
            ).ok();
            
            let index = write & ringbuf.core.mask();
            let old_value = unsafe {
                let slot = ringbuf.core.peek_at(index);
                slot.swap(value, order)
            };
            Some(old_value)
        } else {
            // Buffer not full - just store
            let index = write & ringbuf.core.mask();
            unsafe {
                let slot = ringbuf.core.peek_at(index);
                slot.store(value, order);
            }
            None
        }
    }
}

impl<T: AtomicElement, const N: usize> PushDispatch<T, N, false> for PushMarker<false> {
    /// Returns `Ok(())` on success, or `Err(value)` if full
    /// 
    /// 成功时返回 `Ok(())`，如果满则返回 `Err(value)`
    type PushOutput = Result<(), T::Primitive>;
    
    #[inline]
    fn push_impl(
        ringbuf: &AtomicRingBuf<T, N, false>,
        value: T::Primitive,
        order: Ordering,
    ) -> Self::PushOutput {
        let write = ringbuf.core.write_idx().fetch_add(1, Ordering::Relaxed);
        let read = ringbuf.core.read_idx().load(Ordering::Acquire);
        
        if write.wrapping_sub(read) >= ringbuf.core.capacity() {
            // Full - revert write index and return error
            ringbuf.core.write_idx().fetch_sub(1, Ordering::Relaxed);
            Err(value)
        } else {
            // Not full - store value
            let index = write & ringbuf.core.mask();
            unsafe {
                let slot = ringbuf.core.peek_at(index);
                slot.store(value, order);
            }
            Ok(())
        }
    }
}

/// Ring buffer specialized for atomic types
/// 
/// 为原子类型特化的环形缓冲区
/// 
/// # Type Parameters
/// - `T`: Atomic type implementing AtomicElement
/// - `N`: Stack capacity threshold
/// - `OVERWRITE`: Overwrite mode (true = overwrite oldest, false = reject when full)
/// 
/// # 类型参数
/// - `T`: 实现 AtomicElement 的原子类型
/// - `N`: 栈容量阈值
/// - `OVERWRITE`: 覆盖模式（true = 覆盖最旧的，false = 满时拒绝）
pub struct AtomicRingBuf<T: AtomicElement, const N: usize, const OVERWRITE: bool = true> {
    core: RingBufCore<T, N>,
}

impl<T: AtomicElement, const N: usize, const OVERWRITE: bool> AtomicRingBuf<T, N, OVERWRITE> {
    /// Create a new atomic ring buffer
    /// 
    /// 创建新的原子环形缓冲区
    pub fn new(capacity: usize) -> Self {
        let core = RingBufCore::new(capacity);
        
        // Initialize all slots with default values
        // SAFETY: We are initializing each slot within the valid capacity range
        unsafe {
            for i in 0..core.capacity() {
                core.write_at(i, T::default());
            }
        }
        
        Self { core }
    }
    
    /// Get capacity
    /// 
    /// 获取容量
    #[inline]
    pub fn capacity(&self) -> usize {
        self.core.capacity()
    }
    
    /// Get current length
    /// 
    /// 获取当前长度
    #[inline]
    pub fn len(&self) -> usize {
        self.core.len()
    }
    
    /// Check if empty
    /// 
    /// 检查是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.core.is_empty()
    }
    
    /// Check if full
    /// 
    /// 检查是否已满
    #[inline]
    pub fn is_full(&self) -> bool {
        self.core.is_full()
    }
    
    /// Push a value into the buffer
    /// 
    /// 向缓冲区推送一个值
    /// 
    /// # Behavior
    /// 
    /// - **Overwrite mode (OVERWRITE=true)**: Always succeeds. Returns `Some(T::Primitive)` if an element was overwritten, `None` otherwise.
    /// - **Non-overwrite mode (OVERWRITE=false)**: Returns `Err(value)` if buffer is full, `Ok(())` otherwise.
    /// 
    /// # 行为
    /// 
    /// - **覆盖模式 (OVERWRITE=true)**: 总是成功。如果覆盖了元素则返回 `Some(T::Primitive)`，否则返回 `None`。
    /// - **非覆盖模式 (OVERWRITE=false)**: 如果缓冲区满则返回 `Err(value)`，否则返回 `Ok(())`。
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use smallring::atomic::{AtomicRingBuf, AtomicElement};
    /// use std::sync::atomic::{AtomicU64, Ordering};
    /// 
    /// // Overwrite mode
    /// let buf_ow: AtomicRingBuf<AtomicU64, 32, true> = AtomicRingBuf::new(2);
    /// assert_eq!(buf_ow.push(1, Ordering::Relaxed), None);
    /// assert_eq!(buf_ow.push(2, Ordering::Relaxed), None);
    /// assert_eq!(buf_ow.push(3, Ordering::Relaxed), Some(1)); // Overwrote 1
    /// 
    /// // Non-overwrite mode
    /// let buf_no: AtomicRingBuf<AtomicU64, 32, false> = AtomicRingBuf::new(2);
    /// assert_eq!(buf_no.push(1, Ordering::Relaxed), Ok(()));
    /// assert_eq!(buf_no.push(2, Ordering::Relaxed), Ok(()));
    /// assert_eq!(buf_no.push(3, Ordering::Relaxed), Err(3)); // Full
    /// ```
    #[inline]
    pub fn push(&self, value: T::Primitive, order: Ordering) -> <PushMarker<OVERWRITE> as PushDispatch<T, N, OVERWRITE>>::PushOutput
    where
        PushMarker<OVERWRITE>: PushDispatch<T, N, OVERWRITE>,
    {
        PushMarker::<OVERWRITE>::push_impl(self, value, order)
    }
    
    /// Pop a value from the buffer
    /// 
    /// 从缓冲区弹出一个值
    #[inline]
    pub fn pop(&self, order: Ordering) -> Option<T::Primitive> {
        let read = self.core.read_idx().load(Ordering::Relaxed);
        let write = self.core.write_idx().load(Ordering::Acquire);
        
        if read == write {
            return None;
        }
        
        let index = read & self.core.mask();
        let value = unsafe {
            let slot = self.core.peek_at(index);
            slot.load(order)
        };
        
        self.core.read_idx().store(read.wrapping_add(1), Ordering::Release);
        Some(value)
    }
    
    /// Peek at the next value without removing it
    /// 
    /// 查看下一个值但不移除
    #[inline]
    pub fn peek(&self, order: Ordering) -> Option<T::Primitive> {
        let read = self.core.read_idx().load(Ordering::Acquire);
        let write = self.core.write_idx().load(Ordering::Acquire);
        
        if read == write {
            return None;
        }
        
        let index = read & self.core.mask();
        unsafe {
            let slot = self.core.peek_at(index);
            Some(slot.load(order))
        }
    }
    
    /// Access a slot directly by offset from read position
    /// 
    /// 通过从读位置的偏移直接访问槽位
    /// 
    /// # Safety
    /// Caller must ensure offset < len()
    #[inline]
    pub unsafe fn get_unchecked(&self, offset: usize) -> &T {
        let read = self.core.read_idx().load(Ordering::Acquire);
        let index = read.wrapping_add(offset) & self.core.mask();
        // SAFETY: Caller guarantees offset < len(), so index is valid
        unsafe { self.core.peek_at(index) }
    }
    
    /// Clear all elements from the buffer
    /// 
    /// 清空缓冲区中的所有元素
    /// 
    /// Resets the buffer to empty state by synchronizing read and write indices.
    /// 
    /// 通过同步读写索引将缓冲区重置为空状态。
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use smallring::atomic::AtomicRingBuf;
    /// use std::sync::atomic::{AtomicU64, Ordering};
    /// 
    /// let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
    /// buf.push(1, Ordering::Relaxed);
    /// buf.push(2, Ordering::Relaxed);
    /// assert_eq!(buf.len(), 2);
    /// 
    /// buf.clear();
    /// assert_eq!(buf.len(), 0);
    /// assert!(buf.is_empty());
    /// ```
    #[inline]
    pub fn clear(&self) {
        let write = self.core.write_idx().load(Ordering::Acquire);
        self.core.read_idx().store(write, Ordering::Release);
    }
    
    /// Read all valid elements from the buffer
    /// 
    /// 读取缓冲区中所有有效元素
    /// 
    /// Returns a vector of all elements currently in the buffer, in FIFO order.
    /// This does not remove elements from the buffer.
    /// 
    /// 返回缓冲区中当前所有元素的向量，按 FIFO 顺序排列。
    /// 此操作不会从缓冲区中移除元素。
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use smallring::atomic::AtomicRingBuf;
    /// use std::sync::atomic::{AtomicU64, Ordering};
    /// 
    /// let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
    /// buf.push(1, Ordering::Relaxed);
    /// buf.push(2, Ordering::Relaxed);
    /// buf.push(3, Ordering::Relaxed);
    /// 
    /// let values = buf.read_all(Ordering::Acquire);
    /// assert_eq!(values, vec![1, 2, 3]);
    /// assert_eq!(buf.len(), 3); // Elements still in buffer
    /// ```
    #[inline]
    pub fn read_all(&self, order: Ordering) -> Vec<T::Primitive> {
        let read = self.core.read_idx().load(Ordering::Acquire);
        let write = self.core.write_idx().load(Ordering::Acquire);
        
        let len = write.wrapping_sub(read).min(self.core.capacity());
        let mut values = Vec::with_capacity(len);
        
        for i in 0..len {
            let index = read.wrapping_add(i) & self.core.mask();
            let value = unsafe {
                let slot = self.core.peek_at(index);
                slot.load(order)
            };
            values.push(value);
        }
        
        values
    }
    
    /// Get an iterator over elements in the buffer
    /// 
    /// 获取缓冲区元素的迭代器
    /// 
    /// Returns an iterator that yields references to atomic elements in FIFO order.
    /// The iterator provides read-only access to the underlying atomic values.
    /// 
    /// 返回一个按 FIFO 顺序产生原子元素引用的迭代器。
    /// 迭代器提供对底层原子值的只读访问。
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use smallring::atomic::AtomicRingBuf;
    /// use std::sync::atomic::{AtomicU64, Ordering};
    /// 
    /// let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
    /// buf.push(1, Ordering::Relaxed);
    /// buf.push(2, Ordering::Relaxed);
    /// buf.push(3, Ordering::Relaxed);
    /// 
    /// let values: Vec<u64> = buf.iter()
    ///     .map(|atom| atom.load(Ordering::Acquire))
    ///     .collect();
    /// assert_eq!(values, vec![1, 2, 3]);
    /// ```
    #[inline]
    pub fn iter(&self) -> AtomicIter<'_, T, N, OVERWRITE> {
        let read = self.core.read_idx().load(Ordering::Acquire);
        let write = self.core.write_idx().load(Ordering::Acquire);
        let len = write.wrapping_sub(read).min(self.core.capacity());
        
        AtomicIter {
            ringbuf: self,
            start: read,
            remaining: len,
        }
    }
}

/// Iterator over atomic ring buffer elements
/// 
/// 原子环形缓冲区元素的迭代器
pub struct AtomicIter<'a, T: AtomicElement, const N: usize, const OVERWRITE: bool> {
    ringbuf: &'a AtomicRingBuf<T, N, OVERWRITE>,
    start: usize,
    remaining: usize,
}

impl<'a, T: AtomicElement, const N: usize, const OVERWRITE: bool> Iterator for AtomicIter<'a, T, N, OVERWRITE> {
    type Item = &'a T;
    
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        
        let index = self.start & self.ringbuf.core.mask();
        let element = unsafe {
            self.ringbuf.core.peek_at(index)
        };
        
        self.start = self.start.wrapping_add(1);
        self.remaining -= 1;
        Some(element)
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, T: AtomicElement, const N: usize, const OVERWRITE: bool> ExactSizeIterator for AtomicIter<'a, T, N, OVERWRITE> {
    #[inline]
    fn len(&self) -> usize {
        self.remaining
    }
}

/// Trait for atomic numeric operations on ring buffer elements
/// 
/// 环形缓冲区元素的原子数值操作 trait
/// 
/// This trait provides atomic arithmetic operations for numeric atomic types.
/// 
/// 此 trait 为数值原子类型提供原子算术操作。
pub trait AtomicNumeric: AtomicElement {
    /// Atomically add to the value at the given offset from read position
    /// 
    /// 原子地将值加到从读位置开始的给定偏移处
    fn fetch_add(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
    
    /// Atomically subtract from the value at the given offset from read position
    /// 
    /// 原子地从给定偏移处的值减去
    fn fetch_sub(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
}

macro_rules! impl_atomic_numeric {
    ($atomic:ty, $primitive:ty) => {
        impl AtomicNumeric for $atomic {
            #[inline]
            fn fetch_add(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive {
                self.fetch_add(val, order)
            }
            
            #[inline]
            fn fetch_sub(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive {
                self.fetch_sub(val, order)
            }
        }
    };
}

impl_atomic_numeric!(AtomicU8, u8);
impl_atomic_numeric!(AtomicU16, u16);
impl_atomic_numeric!(AtomicU32, u32);
impl_atomic_numeric!(AtomicU64, u64);
impl_atomic_numeric!(AtomicUsize, usize);
impl_atomic_numeric!(AtomicI8, i8);
impl_atomic_numeric!(AtomicI16, i16);
impl_atomic_numeric!(AtomicI32, i32);
impl_atomic_numeric!(AtomicI64, i64);
impl_atomic_numeric!(AtomicIsize, isize);

impl<T: AtomicElement + AtomicNumeric, const N: usize, const OVERWRITE: bool> AtomicRingBuf<T, N, OVERWRITE> {
    /// Atomically add to an element at the given offset from read position
    /// 
    /// 原子地将值加到从读位置开始的给定偏移处的元素
    /// 
    /// # Safety
    /// Caller must ensure offset < len()
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use smallring::atomic::AtomicRingBuf;
    /// use std::sync::atomic::{AtomicU64, Ordering};
    /// 
    /// let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
    /// buf.push(10, Ordering::Relaxed);
    /// buf.push(20, Ordering::Relaxed);
    /// 
    /// // Add 5 to the first element
    /// let old = unsafe { buf.fetch_add_at(0, 5, Ordering::Relaxed) };
    /// assert_eq!(old, 10);
    /// assert_eq!(buf.peek(Ordering::Acquire).unwrap(), 15);
    /// ```
    #[inline]
    pub unsafe fn fetch_add_at(&self, offset: usize, val: T::Primitive, order: Ordering) -> T::Primitive {
        let element = unsafe { self.get_unchecked(offset) };
        element.fetch_add(val, order)
    }
    
    /// Atomically subtract from an element at the given offset from read position
    /// 
    /// 原子地从读位置开始的给定偏移处的元素减去值
    /// 
    /// # Safety
    /// Caller must ensure offset < len()
    #[inline]
    pub unsafe fn fetch_sub_at(&self, offset: usize, val: T::Primitive, order: Ordering) -> T::Primitive {
        let element = unsafe { self.get_unchecked(offset) };
        element.fetch_sub(val, order)
    }
}

unsafe impl<T: AtomicElement, const N: usize, const OVERWRITE: bool> Send for AtomicRingBuf<T, N, OVERWRITE> {}
unsafe impl<T: AtomicElement, const N: usize, const OVERWRITE: bool> Sync for AtomicRingBuf<T, N, OVERWRITE> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, AtomicU32, AtomicBool};
    
    #[test]
    fn test_basic_push_pop() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(4);
        
        buf.push(1, Ordering::Relaxed);
        buf.push(2, Ordering::Relaxed);
        buf.push(3, Ordering::Relaxed);
        
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.pop(Ordering::Relaxed), Some(1));
        assert_eq!(buf.pop(Ordering::Relaxed), Some(2));
        assert_eq!(buf.pop(Ordering::Relaxed), Some(3));
        assert_eq!(buf.pop(Ordering::Relaxed), None);
    }
    
    #[test]
    fn test_basic_overwrite_mode() {
        let buf: AtomicRingBuf<AtomicU64, 32, true> = AtomicRingBuf::new(2);
        
        assert_eq!(buf.push(1, Ordering::Relaxed), None);
        assert_eq!(buf.push(2, Ordering::Relaxed), None);
        assert_eq!(buf.push(3, Ordering::Relaxed), Some(1));
        
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.pop(Ordering::Relaxed), Some(2));
        assert_eq!(buf.pop(Ordering::Relaxed), Some(3));
    }
    
    #[test]
    fn test_basic_non_overwrite_mode() {
        let buf: AtomicRingBuf<AtomicU64, 32, false> = AtomicRingBuf::new(2);
        
        assert_eq!(buf.push(1, Ordering::Relaxed), Ok(()));
        assert_eq!(buf.push(2, Ordering::Relaxed), Ok(()));
        assert_eq!(buf.push(3, Ordering::Relaxed), Err(3));
        
        assert_eq!(buf.len(), 2);
        assert!(buf.is_full());
    }
    
    #[test]
    fn test_basic_peek() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(4);
        
        assert_eq!(buf.peek(Ordering::Relaxed), None);
        
        buf.push(42, Ordering::Relaxed);
        assert_eq!(buf.peek(Ordering::Relaxed), Some(42));
        assert_eq!(buf.len(), 1);
        
        buf.push(99, Ordering::Relaxed);
        assert_eq!(buf.peek(Ordering::Relaxed), Some(42));
        
        buf.pop(Ordering::Relaxed);
        assert_eq!(buf.peek(Ordering::Relaxed), Some(99));
    }
    
    #[test]
    fn test_basic_clear() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
        
        buf.push(1, Ordering::Relaxed);
        buf.push(2, Ordering::Relaxed);
        buf.push(3, Ordering::Relaxed);
        assert_eq!(buf.len(), 3);
        
        buf.clear();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        
        buf.push(10, Ordering::Relaxed);
        assert_eq!(buf.pop(Ordering::Relaxed), Some(10));
    }
    
    #[test]
    fn test_basic_capacity() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
        assert_eq!(buf.capacity(), 8);
        
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(5);
        assert_eq!(buf.capacity(), 8);
    }
    
    #[test]
    fn test_basic_is_empty_full() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(2);
        
        assert!(buf.is_empty());
        assert!(!buf.is_full());
        
        buf.push(1, Ordering::Relaxed);
        assert!(!buf.is_empty());
        assert!(!buf.is_full());
        
        buf.push(2, Ordering::Relaxed);
        assert!(!buf.is_empty());
        assert!(buf.is_full());
        
        buf.pop(Ordering::Relaxed);
        assert!(!buf.is_empty());
        assert!(!buf.is_full());
        
        buf.pop(Ordering::Relaxed);
        assert!(buf.is_empty());
        assert!(!buf.is_full());
    }
    
    #[test]
    fn test_basic_read_all() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
        
        buf.push(1, Ordering::Relaxed);
        buf.push(2, Ordering::Relaxed);
        buf.push(3, Ordering::Relaxed);
        
        let values = buf.read_all(Ordering::Acquire);
        assert_eq!(values, vec![1, 2, 3]);
        assert_eq!(buf.len(), 3);
    }
    
    #[test]
    fn test_basic_iter() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
        
        buf.push(10, Ordering::Relaxed);
        buf.push(20, Ordering::Relaxed);
        buf.push(30, Ordering::Relaxed);
        
        let values: Vec<u64> = buf.iter()
            .map(|atom| atom.load(Ordering::Acquire))
            .collect();
        assert_eq!(values, vec![10, 20, 30]);
    }
    
    #[test]
    fn test_basic_atomic_u32() {
        let buf: AtomicRingBuf<AtomicU32, 32> = AtomicRingBuf::new(4);
        
        buf.push(100u32, Ordering::Relaxed);
        buf.push(200u32, Ordering::Relaxed);
        
        assert_eq!(buf.pop(Ordering::Relaxed), Some(100u32));
        assert_eq!(buf.pop(Ordering::Relaxed), Some(200u32));
    }
    
    #[test]
    fn test_basic_atomic_bool() {
        let buf: AtomicRingBuf<AtomicBool, 32> = AtomicRingBuf::new(4);
        
        buf.push(true, Ordering::Relaxed);
        buf.push(false, Ordering::Relaxed);
        buf.push(true, Ordering::Relaxed);
        
        assert_eq!(buf.pop(Ordering::Relaxed), Some(true));
        assert_eq!(buf.pop(Ordering::Relaxed), Some(false));
        assert_eq!(buf.pop(Ordering::Relaxed), Some(true));
    }
    
    #[test]
    fn test_basic_fetch_add_at() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
        
        buf.push(10, Ordering::Relaxed);
        buf.push(20, Ordering::Relaxed);
        buf.push(30, Ordering::Relaxed);
        
        let old = unsafe { buf.fetch_add_at(0, 5, Ordering::Relaxed) };
        assert_eq!(old, 10);
        assert_eq!(buf.peek(Ordering::Acquire).unwrap(), 15);
        
        let old = unsafe { buf.fetch_add_at(1, 100, Ordering::Relaxed) };
        assert_eq!(old, 20);
    }
    
    #[test]
    fn test_basic_fetch_sub_at() {
        let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
        
        buf.push(100, Ordering::Relaxed);
        buf.push(200, Ordering::Relaxed);
        
        let old = unsafe { buf.fetch_sub_at(0, 10, Ordering::Relaxed) };
        assert_eq!(old, 100);
        assert_eq!(buf.peek(Ordering::Acquire).unwrap(), 90);
    }
}
