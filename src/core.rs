//! Core ring buffer implementation - Shared logic for SPSC and generic buffers
//! 
//! 核心环形缓冲区实现 - SPSC 和通用缓冲区的共享逻辑
//! 
//! This module extracts common functionality to reduce code duplication:
//! - Buffer storage management with FixedVec
//! - Index masking and capacity calculations
//! - Batch operation helpers (slice copy with wrap-around handling)
//! 
//! 此模块提取通用功能以减少代码冗余：
//! - 使用 FixedVec 的缓冲区存储管理
//! - 索引掩码和容量计算
//! - 批量操作辅助（带环绕处理的切片拷贝）

use super::vec::FixedVec;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr;

/// Core ring buffer storage structure
/// 
/// 核心环形缓冲区存储结构
/// 
/// # Type Parameters
/// - `T`: Element type
/// - `N`: Stack capacity threshold (elements stored on stack when capacity ≤ N)
/// 
/// # 类型参数
/// - `T`: 元素类型
/// - `N`: 栈容量阈值（当容量 ≤ N 时元素存储在栈上）
pub struct RingBufCore<T, const N: usize> {
    /// Buffer storage using FixedVec for stack allocation optimization
    /// 
    /// 使用 FixedVec 的缓冲区存储，优化栈分配
    buffer: FixedVec<T, N>,
    
    /// Actual capacity (power of 2)
    /// 
    /// 实际容量（2 的幂次）
    capacity: usize,
    
    /// Mask for fast modulo operation (capacity - 1)
    /// 
    /// 快速取模运算的掩码（capacity - 1）
    mask: usize,
    
    /// Write index (monotonically increasing)
    /// 
    /// 写入索引（单调递增）
    write_idx: AtomicUsize,
    
    /// Read index (monotonically increasing)
    /// 
    /// 读取索引（单调递增）
    read_idx: AtomicUsize,
}

impl<T, const N: usize> RingBufCore<T, N> {
    /// Create a new core buffer with the specified capacity
    /// 
    /// 创建指定容量的新核心缓冲区
    /// 
    /// Capacity will be rounded up to the next power of 2 for efficient masking.
    /// 
    /// 容量将向上取整到下一个 2 的幂次以实现高效的掩码操作。
    /// 
    /// # Parameters
    /// - `capacity`: Desired capacity (will be rounded up to next power of 2)
    /// 
    /// # 参数
    /// - `capacity`: 期望容量（将向上取整到下一个 2 的幂次）
    pub fn new(capacity: usize) -> Self {
        let actual_capacity = round_to_power_of_two(capacity);
        let mask = actual_capacity - 1;
        
        let mut buffer = FixedVec::with_capacity(actual_capacity);
        unsafe {
            buffer.set_len(actual_capacity);
        }
        
        Self {
            buffer,
            capacity: actual_capacity,
            mask,
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
        }
    }
    
    /// Get the capacity of the buffer
    /// 
    /// 获取缓冲区容量
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    
    /// Get the mask for fast modulo operation
    /// 
    /// 获取快速取模运算的掩码
    #[inline]
    pub fn mask(&self) -> usize {
        self.mask
    }
    
    /// Get a reference to the write index
    /// 
    /// 获取写索引的引用
    #[inline]
    pub fn write_idx(&self) -> &AtomicUsize {
        &self.write_idx
    }
    
    /// Get a reference to the read index
    /// 
    /// 获取读索引的引用
    #[inline]
    pub fn read_idx(&self) -> &AtomicUsize {
        &self.read_idx
    }
    
    /// Calculate the number of elements currently in the buffer
    /// 
    /// 计算缓冲区中当前的元素数量
    /// 
    /// # Safety
    /// This uses Acquire ordering to ensure proper synchronization
    /// 
    /// # 安全性
    /// 使用 Acquire 顺序确保正确的同步
    #[inline]
    pub fn len(&self) -> usize {
        let write = self.write_idx.load(Ordering::Acquire);
        let read = self.read_idx.load(Ordering::Acquire);
        write.wrapping_sub(read).min(self.capacity)
    }
    
    /// Check if the buffer is empty
    /// 
    /// 检查缓冲区是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        let write = self.write_idx.load(Ordering::Acquire);
        let read = self.read_idx.load(Ordering::Acquire);
        write == read
    }
    
    /// Check if the buffer is full
    /// 
    /// 检查缓冲区是否已满
    #[inline]
    pub fn is_full(&self) -> bool {
        let write = self.write_idx.load(Ordering::Acquire);
        let read = self.read_idx.load(Ordering::Acquire);
        write.wrapping_sub(read) >= self.capacity
    }
    
    /// Write a single element at the specified index
    /// 
    /// 在指定索引处写入单个元素
    /// 
    /// # Safety
    /// Caller must ensure:
    /// - The index is within bounds (use mask() to calculate)
    /// - No concurrent writes to the same index
    /// - Proper memory ordering is maintained
    /// 
    /// # 安全性
    /// 调用者必须确保：
    /// - 索引在边界内（使用 mask() 计算）
    /// - 没有对同一索引的并发写入
    /// - 维护正确的内存顺序
    #[inline]
    pub unsafe fn write_at(&self, index: usize, value: T) {
        unsafe {
            let ptr = self.buffer.get_unchecked_ptr(index).cast::<T>() as *mut T;
            ptr.write(value);
        }
    }
    
    /// Read a single element at the specified index
    /// 
    /// 在指定索引处读取单个元素
    /// 
    /// # Safety
    /// Caller must ensure:
    /// - The index is within bounds (use mask() to calculate)
    /// - The element at this index has been initialized
    /// - No concurrent writes to the same index
    /// 
    /// # 安全性
    /// 调用者必须确保：
    /// - 索引在边界内（使用 mask() 计算）
    /// - 该索引处的元素已初始化
    /// - 没有对同一索引的并发写入
    #[inline]
    pub unsafe fn read_at(&self, index: usize) -> T {
        unsafe {
            let ptr = self.buffer.get_unchecked_ptr(index).cast::<T>();
            ptr.read()
        }
    }
    
    /// Replace element at the specified index and return the old value
    /// 
    /// 替换指定索引处的元素并返回旧值
    /// 
    /// # Safety
    /// Caller must ensure:
    /// - The index is within bounds (use mask() to calculate)
    /// - The element at this index has been initialized
    /// - No concurrent writes to the same index
    /// 
    /// # 安全性
    /// 调用者必须确保：
    /// - 索引在边界内（使用 mask() 计算）
    /// - 该索引处的元素已初始化
    /// - 没有对同一索引的并发写入
    #[inline]
    pub unsafe fn replace_at(&self, index: usize, value: T) -> T {
        unsafe {
            let ptr = self.buffer.get_unchecked_ptr(index).cast::<T>() as *mut T;
            ptr::replace(ptr, value)
        }
    }
    
    /// Peek at an element without removing it
    /// 
    /// 查看元素但不移除它
    /// 
    /// # Safety
    /// Caller must ensure:
    /// - The index is within bounds (use mask() to calculate)
    /// - The element at this index has been initialized
    /// 
    /// # 安全性
    /// 调用者必须确保：
    /// - 索引在边界内（使用 mask() 计算）
    /// - 该索引处的元素已初始化
    #[inline]
    pub unsafe fn peek_at(&self, index: usize) -> &T {
        unsafe {
            let ptr = self.buffer.get_unchecked_ptr(index).cast::<T>();
            &*ptr
        }
    }
    
    /// Get a pointer to the buffer for direct access
    /// 
    /// 获取缓冲区指针以供直接访问
    /// 
    /// # Safety
    /// 
    /// Caller must ensure:
    /// - Proper bounds checking when accessing elements
    /// - No data races with concurrent access
    /// - Elements are initialized before reading
    /// 
    /// # 安全性
    /// 
    /// 调用者必须确保：
    /// - 访问元素时进行适当的边界检查
    /// - 与并发访问无数据竞争
    /// - 在读取前元素已初始化
    #[inline]
    pub unsafe fn buffer_ptr(&self) -> *const T {
        self.buffer.as_ptr().cast::<T>()
    }

    /// Get a pointer to the element at the specified index
    /// 
    /// 获取指定索引处的元素指针
    /// 
    /// # Safety
    /// 
    /// Caller must ensure:
    /// - The index is within bounds (use mask() to calculate)
    /// 
    /// # 安全性
    /// 
    /// 调用者必须确保：
    /// - 索引在边界内（使用 mask() 计算）
    /// - 该索引处的元素已初始化
    #[inline]
    pub unsafe fn buffer_ptr_at(&self, index: usize) -> *const T {
        unsafe {
            self.buffer.get_unchecked_ptr(index).cast()
        }
    }
}

/// Batch copy operations for Copy types
/// 
/// Copy 类型的批量拷贝操作
impl<T: Copy, const N: usize> RingBufCore<T, N> {
    /// Copy multiple elements from a slice to the buffer
    /// 
    /// 将切片中的多个元素拷贝到缓冲区
    /// 
    /// Handles wrap-around automatically by splitting into two copies if necessary.
    /// 
    /// 自动处理环绕，必要时分成两次拷贝。
    /// 
    /// # Parameters
    /// - `start_write`: Starting write index (before masking)
    /// - `values`: Source slice
    /// - `count`: Number of elements to copy
    /// 
    /// # 参数
    /// - `start_write`: 起始写索引（掩码前）
    /// - `values`: 源切片
    /// - `count`: 要拷贝的元素数量
    /// 
    /// # Safety
    /// Caller must ensure:
    /// - count <= values.len()
    /// - count <= capacity
    /// - No concurrent access to the affected indices
    /// 
    /// # 安全性
    /// 调用者必须确保：
    /// - count <= values.len()
    /// - count <= capacity
    /// - 对受影响索引没有并发访问
    pub unsafe fn copy_from_slice(&self, start_write: usize, values: &[T], count: usize) {
        if count == 0 {
            return;
        }
        
        unsafe {
            let start_index = start_write & self.mask;
            
            // Check if we need to wrap around
            if count <= self.capacity - start_index {
                // No wrap-around: single continuous copy
                // 无环绕：单次连续拷贝
                let dst = self.buffer.get_unchecked_ptr(start_index).cast::<T>() as *mut T;
                ptr::copy_nonoverlapping(values.as_ptr(), dst, count);
            } else {
                // Wrap-around: two copies
                // 环绕：两次拷贝
                let first_part = self.capacity - start_index;
                let second_part = count - first_part;
                
                // Copy first part (from start_index to end of buffer)
                // 拷贝第一部分（从 start_index 到缓冲区末尾）
                let dst1 = self.buffer.get_unchecked_ptr(start_index).cast::<T>() as *mut T;
                ptr::copy_nonoverlapping(values.as_ptr(), dst1, first_part);
                
                // Copy second part (from beginning of buffer)
                // 拷贝第二部分（从缓冲区开头）
                let dst2 = self.buffer.get_unchecked_ptr(0).cast::<T>() as *mut T;
                ptr::copy_nonoverlapping(values.as_ptr().add(first_part), dst2, second_part);
            }
        }
    }
    
    /// Copy multiple elements from the buffer to a slice
    /// 
    /// 将缓冲区中的多个元素拷贝到切片
    /// 
    /// Handles wrap-around automatically by splitting into two copies if necessary.
    /// 
    /// 自动处理环绕，必要时分成两次拷贝。
    /// 
    /// # Parameters
    /// - `start_read`: Starting read index (before masking)
    /// - `dest`: Destination slice
    /// - `count`: Number of elements to copy
    /// 
    /// # 参数
    /// - `start_read`: 起始读索引（掩码前）
    /// - `dest`: 目标切片
    /// - `count`: 要拷贝的元素数量
    /// 
    /// # Safety
    /// Caller must ensure:
    /// - count <= dest.len()
    /// - count <= number of available elements
    /// - No concurrent access to the affected indices
    /// 
    /// # 安全性
    /// 调用者必须确保：
    /// - count <= dest.len()
    /// - count <= 可用元素数量
    /// - 对受影响索引没有并发访问
    pub unsafe fn copy_to_slice(&self, start_read: usize, dest: &mut [T], count: usize) {
        if count == 0 {
            return;
        }
        
        unsafe {
            let start_index = start_read & self.mask;
            
            // Check if we need to wrap around
            if count <= self.capacity - start_index {
                // No wrap-around: single continuous copy
                // 无环绕：单次连续拷贝
                let src = self.buffer.get_unchecked_ptr(start_index).cast::<T>();
                ptr::copy_nonoverlapping(src, dest.as_mut_ptr(), count);
            } else {
                // Wrap-around: two copies
                // 环绕：两次拷贝
                let first_part = self.capacity - start_index;
                let second_part = count - first_part;
                
                // Copy first part (from start_index to end of buffer)
                // 拷贝第一部分（从 start_index 到缓冲区末尾）
                let src1 = self.buffer.get_unchecked_ptr(start_index).cast::<T>();
                ptr::copy_nonoverlapping(src1, dest.as_mut_ptr(), first_part);
                
                // Copy second part (from beginning of buffer)
                // 拷贝第二部分（从缓冲区开头）
                let src2 = self.buffer.get_unchecked_ptr(0).cast::<T>();
                ptr::copy_nonoverlapping(src2, dest.as_mut_ptr().add(first_part), second_part);
            }
        }
    }
}

// Ensure RingBufCore is Send and Sync if T is Send
unsafe impl<T: Send, const N: usize> Send for RingBufCore<T, N> {}
unsafe impl<T: Send, const N: usize> Sync for RingBufCore<T, N> {}

/// Round up a capacity to the next power of 2
/// 
/// 将容量向上取整到下一个 2 的幂次
/// 
/// # Parameters
/// - `capacity`: Desired capacity
/// 
/// # Returns
/// The smallest power of 2 that is >= capacity (minimum 1)
/// 
/// # 参数
/// - `capacity`: 期望容量
/// 
/// # 返回值
/// >= capacity 的最小 2 的幂次（最小为 1）
#[inline]
pub fn round_to_power_of_two(capacity: usize) -> usize {
    if capacity == 0 {
        1
    } else {
        capacity.next_power_of_two()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_round_to_power_of_two() {
        assert_eq!(round_to_power_of_two(0), 1);
        assert_eq!(round_to_power_of_two(1), 1);
        assert_eq!(round_to_power_of_two(2), 2);
        assert_eq!(round_to_power_of_two(3), 4);
        assert_eq!(round_to_power_of_two(5), 8);
        assert_eq!(round_to_power_of_two(8), 8);
        assert_eq!(round_to_power_of_two(9), 16);
    }
    
    #[test]
    fn test_core_basic() {
        let core: RingBufCore<i32, 32> = RingBufCore::new(4);
        assert_eq!(core.capacity(), 4);
        assert_eq!(core.mask(), 3);
        assert!(core.is_empty());
        assert!(!core.is_full());
    }
    
    #[test]
    fn test_core_write_read() {
        let core: RingBufCore<i32, 32> = RingBufCore::new(4);
        
        unsafe {
            core.write_at(0, 42);
            let value = core.read_at(0);
            assert_eq!(value, 42);
        }
    }
    
    #[test]
    fn test_core_batch_copy_no_wrap() {
        let core: RingBufCore<i32, 32> = RingBufCore::new(8);
        let values = [1, 2, 3, 4];
        
        unsafe {
            core.copy_from_slice(0, &values, 4);
            
            let mut dest = [0i32; 4];
            core.copy_to_slice(0, &mut dest, 4);
            assert_eq!(dest, [1, 2, 3, 4]);
        }
    }
    
    #[test]
    fn test_core_batch_copy_with_wrap() {
        let core: RingBufCore<i32, 32> = RingBufCore::new(4);
        let values = [1, 2, 3];
        
        unsafe {
            // Write starting at index 3 (will wrap around)
            core.copy_from_slice(3, &values, 3);
            
            let mut dest = [0i32; 3];
            core.copy_to_slice(3, &mut dest, 3);
            assert_eq!(dest, [1, 2, 3]);
        }
    }
}
