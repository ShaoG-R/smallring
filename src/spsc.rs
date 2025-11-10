/// High-performance SPSC Ring Buffer with stack/heap optimization
/// 
/// 基于栈/堆优化的高性能 SPSC 环形缓冲区
/// 
/// This implementation uses FixedVec to store data on the stack for small capacities (≤32),
/// avoiding heap allocation overhead and improving `new()` performance.
/// 
/// 此实现使用 FixedVec 在栈上存储小容量数据（≤32），避免堆分配开销，提升 `new()` 性能。
use std::num::NonZero;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use super::core::RingBufCore;

/// Ring buffer error for push operations
/// 
/// push 操作的环形缓冲区错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushError<T> {
    /// Buffer is full
    /// 
    /// 缓冲区已满
    Full(T),
}

/// Ring buffer error for pop operations
/// 
/// pop 操作的环形缓冲区错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopError {
    /// Buffer is empty
    /// 
    /// 缓冲区为空
    Empty,
}

/// Shared data between producer and consumer
/// 
/// 生产者和消费者之间的共享数据
/// 
/// # Type Parameters
/// - `T`: Element type
/// - `N`: Stack capacity threshold (elements stored on stack when capacity ≤ N)
/// 
/// # 类型参数
/// - `T`: 元素类型
/// - `N`: 栈容量阈值（当容量 ≤ N 时元素存储在栈上）
pub struct SharedData<T, const N: usize> {
    /// Core ring buffer implementation
    /// 
    /// 核心环形缓冲区实现
    core: RingBufCore<T, N>,
}

/// Producer half of the ring buffer
/// 
/// 环形缓冲区的生产者端
/// 
/// # Type Parameters
/// - `T`: Element type
/// - `N`: Stack capacity threshold
/// 
/// # 类型参数
/// - `T`: 元素类型
/// - `N`: 栈容量阈值
pub struct Producer<T, const N: usize> {
    /// Shared data
    /// 
    /// 共享数据
    shared: Arc<SharedData<T, N>>,
    
    /// Cached read index for performance (avoid reading atomic repeatedly)
    /// 
    /// 缓存的读索引以提升性能（避免重复读取原子变量）
    cached_read: usize,
}

/// Consumer half of the ring buffer
/// 
/// 环形缓冲区的消费者端
/// 
/// # Type Parameters
/// - `T`: Element type
/// - `N`: Stack capacity threshold
/// 
/// # 类型参数
/// - `T`: 元素类型
/// - `N`: 栈容量阈值
pub struct Consumer<T, const N: usize> {
    /// Shared data
    /// 
    /// 共享数据
    shared: Arc<SharedData<T, N>>,
    
    /// Cached write index for performance (avoid reading atomic repeatedly)
    /// 
    /// 缓存的写索引以提升性能（避免重复读取原子变量）
    cached_write: usize,
}

/// Draining iterator for the ring buffer
/// 
/// 环形缓冲区的消费迭代器
/// 
/// This iterator removes and returns elements from the buffer until it's empty.
/// 
/// 此迭代器从缓冲区中移除并返回元素，直到缓冲区为空。
/// 
/// # Type Parameters
/// - `T`: Element type
/// - `N`: Stack capacity threshold
/// 
/// # 类型参数
/// - `T`: 元素类型
/// - `N`: 栈容量阈值
pub struct Drain<'a, T, const N: usize> {
    consumer: &'a mut Consumer<T, N>,
}

impl<'a, T, const N: usize> Iterator for Drain<'a, T, N> {
    type Item = T;
    
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.consumer.pop().ok()
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.consumer.slots();
        (len, Some(len))
    }
}

impl<T, const N: usize> SharedData<T, N> {
    /// Get the capacity of the buffer
    /// 
    /// 获取缓冲区容量
    #[inline]
    pub fn capacity(&self) -> usize {
        self.core.capacity()
    }
}

/// Create a new ring buffer with the specified capacity
/// 
/// 创建指定容量的新环形缓冲区
/// 
/// # Type Parameters
/// - `T`: Element type
/// - `N`: Stack capacity threshold (use stack storage when capacity ≤ N, heap otherwise)
/// 
/// # Parameters
/// - `capacity`: Desired capacity (will be rounded up to next power of 2)
/// 
/// # Returns
/// A tuple of (Producer, Consumer)
/// 
/// # 类型参数
/// - `T`: 元素类型
/// - `N`: 栈容量阈值（当容量 ≤ N 时使用栈存储，否则使用堆）
/// 
/// # 参数
/// - `capacity`: 期望容量（将向上取整到下一个 2 的幂次）
/// 
/// # 返回值
/// 返回 (Producer, Consumer) 元组
pub fn new<T, const N: usize>(capacity: NonZero<usize>) -> (Producer<T, N>, Consumer<T, N>) {
        let core = RingBufCore::new(capacity.get());
        
        let shared = Arc::new(SharedData {
            core,
        });
        
        let producer = Producer {
            shared: shared.clone(),
            cached_read: 0,
        };
        
        let consumer = Consumer {
            shared,
            cached_write: 0,
        };
        
        (producer, consumer)
}

impl<T, const N: usize> Producer<T, N> {
    /// Get the capacity of the buffer
    /// 
    /// 获取缓冲区容量
    #[inline]
    pub fn capacity(&self) -> usize {
        self.shared.core.capacity()
    }
    
    /// Get the number of elements currently in the buffer
    /// 
    /// 获取缓冲区中当前的元素数量
    #[inline]
    pub fn slots(&self) -> usize {
        let write = self.shared.core.write_idx().load(Ordering::Relaxed);
        let read = self.shared.core.read_idx().load(Ordering::Acquire);
        write.wrapping_sub(read)
    }
    
    /// Get the number of elements currently in the buffer (alias for `slots`)
    /// 
    /// 获取缓冲区中当前的元素数量（`slots` 的别名）
    #[inline]
    pub fn len(&self) -> usize {
        self.slots()
    }

    /// Check if the buffer is empty
    /// 
    /// 检查缓冲区是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        let write = self.shared.core.write_idx().load(Ordering::Relaxed);
        let read = self.shared.core.read_idx().load(Ordering::Acquire);
        write == read
    }
    
    /// Get the number of free slots in the buffer
    /// 
    /// 获取缓冲区中的空闲空间数量
    #[inline]
    pub fn free_slots(&self) -> usize {
        self.shared.core.capacity() - self.slots()
    }
    
    /// Check if the buffer is full
    /// 
    /// 检查缓冲区是否已满
    #[inline]
    pub fn is_full(&self) -> bool {
        let write = self.shared.core.write_idx().load(Ordering::Relaxed);
        let read = self.shared.core.read_idx().load(Ordering::Acquire);
        write.wrapping_sub(read) >= self.shared.core.capacity()
    }
    
    /// Push a value into the buffer
    /// 
    /// 向缓冲区推送一个值
    /// 
    /// # Errors
    /// Returns `PushError::Full` if the buffer is full
    /// 
    /// # 错误
    /// 如果缓冲区满则返回 `PushError::Full`
    #[inline]
    pub fn push(&mut self, value: T) -> Result<(), PushError<T>> {
        let write = self.shared.core.write_idx().load(Ordering::Relaxed);
        let mut read = self.cached_read;
        
        // Check if buffer is full
        // 检查缓冲区是否已满
        if write.wrapping_sub(read) >= self.shared.core.capacity() {
            // Update cached read index from consumer
            // 从消费者更新缓存的读索引
            read = self.shared.core.read_idx().load(Ordering::Acquire);
            self.cached_read = read;
            
            if write.wrapping_sub(read) >= self.shared.core.capacity() {
                return Err(PushError::Full(value));
            }
        }
        
        // Write value to buffer
        // 将值写入缓冲区
        let index = write & self.shared.core.mask();
        unsafe {
            self.shared.core.write_at(index, value);
        }
        
        // Update write index with Release ordering to ensure visibility
        // 使用 Release 顺序更新写索引以确保可见性
        self.shared.core.write_idx().store(write.wrapping_add(1), Ordering::Release);
        
        Ok(())
    }
}

impl<T: Copy, const N: usize> Producer<T, N> {
    /// Push multiple values from a slice into the buffer
    /// 
    /// 将切片中的多个值批量推送到缓冲区
    /// 
    /// This method attempts to push as many elements as possible from the slice.
    /// It returns the number of elements successfully pushed.
    /// 
    /// 此方法尝试从切片中推送尽可能多的元素。
    /// 返回成功推送的元素数量。
    /// 
    /// # Parameters
    /// - `values`: Slice of values to push
    /// 
    /// # Returns
    /// Number of elements successfully pushed (0 to values.len())
    /// 
    /// # 参数
    /// - `values`: 要推送的值的切片
    /// 
    /// # 返回值
    /// 成功推送的元素数量（0 到 values.len()）
    /// 
    /// # Performance
    /// This method uses optimized memory copy operations for better performance
    /// than pushing elements one by one.
    /// 
    /// # 性能
    /// 此方法使用优化的内存拷贝操作，性能优于逐个推送元素。
    #[inline]
    pub fn push_slice(&mut self, values: &[T]) -> usize {
        if values.is_empty() {
            return 0;
        }
        
        let write = self.shared.core.write_idx().load(Ordering::Relaxed);
        let mut read = self.cached_read;
        
        // Calculate available space
        // 计算可用空间
        let mut available = self.shared.core.capacity().saturating_sub(write.wrapping_sub(read));
        
        if available == 0 {
            // Update cached read index
            // 更新缓存的读索引
            read = self.shared.core.read_idx().load(Ordering::Acquire);
            self.cached_read = read;
            available = self.shared.core.capacity().saturating_sub(write.wrapping_sub(read));
            
            if available == 0 {
                return 0;
            }
        }
        
        // Determine how many elements we can push
        // 确定我们可以推送多少元素
        let to_push = available.min(values.len());
        
        // Use core's batch copy functionality
        // 使用核心模块的批量拷贝功能
        unsafe {
            self.shared.core.copy_from_slice(write, values, to_push);
        }
        
        // Update write index with Release ordering
        // 使用 Release 顺序更新写索引
        self.shared.core.write_idx().store(write.wrapping_add(to_push), Ordering::Release);
        
        to_push
    }
}

impl<T, const N: usize> Consumer<T, N> {
    /// Pop a value from the buffer
    /// 
    /// 从缓冲区弹出一个值
    /// 
    /// # Errors
    /// Returns `PopError::Empty` if the buffer is empty
    /// 
    /// # 错误
    /// 如果缓冲区空则返回 `PopError::Empty`
    #[inline]
    pub fn pop(&mut self) -> Result<T, PopError> {
        let read = self.shared.core.read_idx().load(Ordering::Relaxed);
        let mut write = self.cached_write;
        
        // Check if buffer is empty
        // 检查缓冲区是否为空
        if read == write {
            // Update cached write index from producer
            // 从生产者更新缓存的写索引
            write = self.shared.core.write_idx().load(Ordering::Acquire);
            self.cached_write = write;
            
            if read == write {
                return Err(PopError::Empty);
            }
        }
        
        // Read value from buffer
        // 从缓冲区读取值
        let index = read & self.shared.core.mask();
        let value = unsafe {
            self.shared.core.read_at(index)
        };
        
        // Update read index with Release ordering to ensure visibility
        // 使用 Release 顺序更新读索引以确保可见性
        self.shared.core.read_idx().store(read.wrapping_add(1), Ordering::Release);
        
        Ok(value)
    }
    
    /// Check if the buffer is empty
    /// 
    /// 检查缓冲区是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        let read = self.shared.core.read_idx().load(Ordering::Relaxed);
        let write = self.shared.core.write_idx().load(Ordering::Acquire);
        read == write
    }
    
    /// Get the number of elements currently in the buffer
    /// 
    /// 获取缓冲区中当前的元素数量
    #[inline]
    pub fn slots(&self) -> usize {
        let read = self.shared.core.read_idx().load(Ordering::Relaxed);
        let write = self.shared.core.write_idx().load(Ordering::Acquire);
        write.wrapping_sub(read)
    }
    
    /// Get the number of elements currently in the buffer (alias for `slots`)
    /// 
    /// 获取缓冲区中当前的元素数量（`slots` 的别名）
    #[inline]
    pub fn len(&self) -> usize {
        self.slots()
    }
    
    /// Get the capacity of the buffer
    /// 
    /// 获取缓冲区容量
    #[inline]
    pub fn capacity(&self) -> usize {
        self.shared.core.capacity()
    }
    
    /// Peek at the first element without removing it
    /// 
    /// 查看第一个元素但不移除它
    /// 
    /// # Returns
    /// `Some(&T)` if there is an element, `None` if the buffer is empty
    /// 
    /// # 返回值
    /// 如果有元素则返回 `Some(&T)`，如果缓冲区为空则返回 `None`
    /// 
    /// # Safety
    /// The returned reference is valid only as long as no other operations
    /// are performed on the Consumer that might modify the buffer.
    /// 
    /// # 安全性
    /// 返回的引用仅在未对 Consumer 执行可能修改缓冲区的其他操作时有效。
    #[inline]
    pub fn peek(&self) -> Option<&T> {
        let read = self.shared.core.read_idx().load(Ordering::Relaxed);
        let write = self.shared.core.write_idx().load(Ordering::Acquire);
        
        if read == write {
            return None;
        }
        
        let index = read & self.shared.core.mask();
        unsafe {
            Some(self.shared.core.peek_at(index))
        }
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
            // 元素自动被 drop
        }
    }
    
    /// Create a draining iterator
    /// 
    /// 创建一个消费迭代器
    /// 
    /// Returns an iterator that removes and returns elements from the buffer.
    /// The iterator will continue until the buffer is empty.
    /// 
    /// 返回一个从缓冲区中移除并返回元素的迭代器。
    /// 迭代器将持续运行直到缓冲区为空。
    /// 
    /// # Examples
    /// 
    /// ```
    /// use smallring::new;
    /// use std::num::NonZero;
    /// 
    /// let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(8).unwrap());
    /// producer.push(1).unwrap();
    /// producer.push(2).unwrap();
    /// producer.push(3).unwrap();
    /// 
    /// let items: Vec<i32> = consumer.drain().collect();
    /// assert_eq!(items, vec![1, 2, 3]);
    /// assert!(consumer.is_empty());
    /// ```
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, T, N> {
        Drain { consumer: self }
    }
    
    /// Get a reference to the shared buffer data
    /// 
    /// 获取共享缓冲区数据的引用
    #[inline]
    pub fn buffer(&self) -> &SharedData<T, N> {
        &self.shared
    }
}

impl<T: Copy, const N: usize> Consumer<T, N> {
    /// Pop multiple values into a slice
    /// 
    /// 将多个值批量弹出到切片
    /// 
    /// This method attempts to pop as many elements as possible into the provided slice.
    /// It returns the number of elements successfully popped.
    /// 
    /// 此方法尝试将尽可能多的元素弹出到提供的切片中。
    /// 返回成功弹出的元素数量。
    /// 
    /// # Parameters
    /// - `dest`: Destination slice to pop values into
    /// 
    /// # Returns
    /// Number of elements successfully popped (0 to dest.len())
    /// 
    /// # 参数
    /// - `dest`: 用于接收值的目标切片
    /// 
    /// # 返回值
    /// 成功弹出的元素数量（0 到 dest.len()）
    /// 
    /// # Performance
    /// This method uses optimized memory copy operations for better performance
    /// than popping elements one by one.
    /// 
    /// # 性能
    /// 此方法使用优化的内存拷贝操作，性能优于逐个弹出元素。
    #[inline]
    pub fn pop_slice(&mut self, dest: &mut [T]) -> usize {
        if dest.is_empty() {
            return 0;
        }
        
        let read = self.shared.core.read_idx().load(Ordering::Relaxed);
        let mut write = self.cached_write;
        
        // Calculate available elements
        // 计算可用元素数量
        let mut available = write.wrapping_sub(read);
        
        if available == 0 {
            // Update cached write index
            // 更新缓存的写索引
            write = self.shared.core.write_idx().load(Ordering::Acquire);
            self.cached_write = write;
            available = write.wrapping_sub(read);
            
            if available == 0 {
                return 0;
            }
        }
        
        // Determine how many elements we can pop
        // 确定我们可以弹出多少元素
        let to_pop = available.min(dest.len());
        
        // Use core's batch copy functionality
        // 使用核心模块的批量拷贝功能
        unsafe {
            self.shared.core.copy_to_slice(read, dest, to_pop);
        }
        
        // Update read index with Release ordering
        // 使用 Release 顺序更新读索引
        self.shared.core.read_idx().store(read.wrapping_add(to_pop), Ordering::Release);
        
        to_pop
    }
}

impl<T, const N: usize> Drop for Consumer<T, N> {
    fn drop(&mut self) {
        // Clean up any remaining elements in the buffer
        // 清理缓冲区中的剩余元素
        while self.pop().is_ok() {
            // Elements are dropped automatically
            // 元素自动被 drop
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_push_pop() {
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());
        
        assert!(producer.push(1).is_ok());
        assert!(producer.push(2).is_ok());
        assert!(producer.push(3).is_ok());
        
        assert_eq!(consumer.pop().unwrap(), 1);
        assert_eq!(consumer.pop().unwrap(), 2);
        assert_eq!(consumer.pop().unwrap(), 3);
        assert!(consumer.pop().is_err());
    }
    
    #[test]
    fn test_capacity_rounding() {
        let (_, consumer) = new::<i32, 32>(NonZero::new(5).unwrap());
        // 5 should round up to 8 (next power of 2)
        assert_eq!(consumer.buffer().capacity(), 8);
        
        let (_, consumer) = new::<i32, 64>(NonZero::new(32).unwrap());
        assert_eq!(consumer.buffer().capacity(), 32);
        
        let (_, consumer) = new::<i32, 128>(NonZero::new(33).unwrap());
        // 33 should round up to 64
        assert_eq!(consumer.buffer().capacity(), 64);
    }
    
    #[test]
    fn test_buffer_full() {
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());
        // Actual capacity is 4, but we can only store 3 items (one slot reserved)
        
        assert!(producer.push(1).is_ok());
        assert!(producer.push(2).is_ok());
        assert!(producer.push(3).is_ok());
        assert!(producer.push(4).is_ok());
        
        // Buffer should be full now
        assert!(matches!(producer.push(5), Err(PushError::Full(5))));
        
        // Pop one item to make space
        assert_eq!(consumer.pop().unwrap(), 1);
        
        // Now we should be able to push again
        assert!(producer.push(5).is_ok());
    }
    
    #[test]
    fn test_buffer_empty() {
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());
        
        assert!(consumer.pop().is_err());
        assert!(consumer.is_empty());
        
        producer.push(42).unwrap();
        assert!(!consumer.is_empty());
        
        consumer.pop().unwrap();
        assert!(consumer.is_empty());
    }
    
    #[test]
    fn test_slots() {
        let (mut producer, consumer) = new::<i32, 32>(NonZero::new(8).unwrap());
        
        assert_eq!(consumer.slots(), 0);
        
        producer.push(1).unwrap();
        producer.push(2).unwrap();
        producer.push(3).unwrap();
        
        assert_eq!(consumer.slots(), 3);
    }
    
    #[test]
    fn test_wrap_around() {
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());
        
        // Fill and empty the buffer multiple times to test wrap-around
        for round in 0..10 {
            for i in 0..4 {
                producer.push(round * 10 + i).unwrap();
            }
            
            for i in 0..4 {
                assert_eq!(consumer.pop().unwrap(), round * 10 + i);
            }
        }
    }
    
    #[test]
    fn test_drop_cleanup() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        
        #[derive(Debug)]
        struct DropCounter {
            counter: Arc<AtomicUsize>,
        }
        
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.counter.fetch_add(1, Ordering::SeqCst);
            }
        }
        
        let counter = Arc::new(AtomicUsize::new(0));
        
        {
            let (mut producer, consumer) = new::<DropCounter, 32>(NonZero::new(8).unwrap());
            
            for _ in 0..5 {
                producer.push(DropCounter { counter: counter.clone() }).unwrap();
            }
            
            // Drop consumer, which should drop all remaining items
            drop(consumer);
        }
        
        // All 5 items should have been dropped
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }
    
    #[test]
    fn test_concurrent_access() {
        use std::thread;
        
        let (mut producer, mut consumer) = new::<u64, 128>(NonZero::new(128).unwrap());
        
        let producer_handle = thread::spawn(move || {
            for i in 0..1000 {
                loop {
                    if producer.push(i).is_ok() {
                        break;
                    }
                    thread::yield_now();
                }
            }
        });
        
        let consumer_handle = thread::spawn(move || {
            let mut received = Vec::new();
            for _ in 0..1000 {
                loop {
                    match consumer.pop() {
                        Ok(val) => {
                            received.push(val);
                            break;
                        }
                        Err(_) => thread::yield_now(),
                    }
                }
            }
            received
        });
        
        producer_handle.join().unwrap();
        let received = consumer_handle.join().unwrap();
        
        // Verify all numbers were received in order
        assert_eq!(received.len(), 1000);
        for (i, &val) in received.iter().enumerate() {
            assert_eq!(val, i as u64);
        }
    }
    
    #[test]
    fn test_small_capacity_stack_allocation() {
        // Test that small capacities (≤32) use stack allocation
        // This test mainly ensures the code compiles and works with FixedVec
        let (mut producer, mut consumer) = new::<u8, 32>(NonZero::new(16).unwrap());
        
        for i in 0..10 {
            producer.push(i).unwrap();
        }
        
        for i in 0..10 {
            assert_eq!(consumer.pop().unwrap(), i);
        }
    }
    
    #[test]
    fn test_large_capacity_heap_allocation() {
        // Test that large capacities (>32) work correctly with heap allocation
        let (mut producer, mut consumer) = new::<u8, 32>(NonZero::new(64).unwrap());
        
        for i in 0..50 {
            producer.push(i).unwrap();
        }
        
        for i in 0..50 {
            assert_eq!(consumer.pop().unwrap(), i);
        }
    }
    
    // ==================== 边界条件测试 / Boundary Condition Tests ====================
    
    #[test]
    fn test_capacity_one() {
        // Test minimum capacity
        // 测试最小容量
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(1).unwrap());
        
        // After rounding, capacity should be 1 (2^0)
        // 向上取整后，容量应为 1 (2^0)
        assert_eq!(consumer.buffer().capacity(), 1);
        
        assert!(producer.push(42).is_ok());
        assert!(matches!(producer.push(99), Err(PushError::Full(99))));
        
        assert_eq!(consumer.pop().unwrap(), 42);
        assert!(consumer.pop().is_err());
    }
    
    #[test]
    fn test_power_of_two_capacities() {
        // Test various power-of-2 capacities
        // 测试各种 2 的幂次容量
        for power in 0..10 {
            let capacity = 1 << power; // 2^power
            let (_, consumer) = new::<u8, 128>(NonZero::new(capacity).unwrap());
            assert_eq!(consumer.buffer().capacity(), capacity);
        }
    }
    
    #[test]
    fn test_non_power_of_two_rounding() {
        // Test that non-power-of-2 capacities are rounded up correctly
        // 测试非 2 的幂次容量正确向上取整
        let test_cases = vec![
            (3, 4),
            (5, 8),
            (7, 8),
            (9, 16),
            (15, 16),
            (17, 32),
            (31, 32),
            (33, 64),
            (100, 128),
            (1000, 1024),
        ];
        
        for (input, expected) in test_cases {
            let (_, consumer) = new::<u8, 128>(NonZero::new(input).unwrap());
            assert_eq!(consumer.buffer().capacity(), expected,
                "Capacity {} should round up to {}", input, expected);
        }
    }
    
    #[test]
    fn test_single_element_operations() {
        // Test push and pop with single element repeatedly
        // 测试单个元素的重复推送和弹出
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());
        
        for i in 0..100 {
            producer.push(i).unwrap();
            assert_eq!(consumer.slots(), 1);
            assert_eq!(consumer.pop().unwrap(), i);
            assert_eq!(consumer.slots(), 0);
            assert!(consumer.is_empty());
        }
    }
    
    #[test]
    fn test_alternating_push_pop() {
        // Test alternating push and pop operations
        // 测试交替推送和弹出操作
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(8).unwrap());
        
        for i in 0..50 {
            producer.push(i * 2).unwrap();
            producer.push(i * 2 + 1).unwrap();
            assert_eq!(consumer.pop().unwrap(), i * 2);
            assert_eq!(consumer.pop().unwrap(), i * 2 + 1);
        }
    }
    
    #[test]
    fn test_index_wrapping() {
        // Test that indices wrap correctly after overflow
        // 测试索引在溢出后正确环绕
        let (mut producer, mut consumer) = new::<usize, 32>(NonZero::new(4).unwrap());
        
        // Fill and empty many times to cause index wrapping
        // 多次填充和清空以导致索引环绕
        for iteration in 0..1000 {
            for i in 0..4 {
                producer.push(iteration * 4 + i).unwrap();
            }
            for i in 0..4 {
                assert_eq!(consumer.pop().unwrap(), iteration * 4 + i);
            }
        }
    }
    
    // ==================== 不同 N 值测试 / Different N Value Tests ====================
    
    #[test]
    fn test_various_stack_thresholds() {
        // Test with N = 16
        let (mut p1, mut c1) = new::<u32, 16>(NonZero::new(8).unwrap());
        p1.push(1).unwrap();
        assert_eq!(c1.pop().unwrap(), 1);
        
        // Test with N = 64
        let (mut p2, mut c2) = new::<u32, 64>(NonZero::new(32).unwrap());
        p2.push(2).unwrap();
        assert_eq!(c2.pop().unwrap(), 2);
        
        // Test with N = 128
        let (mut p3, mut c3) = new::<u32, 128>(NonZero::new(64).unwrap());
        p3.push(3).unwrap();
        assert_eq!(c3.pop().unwrap(), 3);
        
        // Test with N = 256
        let (mut p4, mut c4) = new::<u32, 256>(NonZero::new(128).unwrap());
        p4.push(4).unwrap();
        assert_eq!(c4.pop().unwrap(), 4);
    }
    
    #[test]
    fn test_small_n_with_large_capacity() {
        // Test N=8 with capacity > N (should use heap)
        // 测试 N=8 但容量 > N（应使用堆）
        let (mut producer, mut consumer) = new::<u64, 8>(NonZero::new(32).unwrap());
        
        for i in 0..20 {
            producer.push(i).unwrap();
        }
        
        for i in 0..20 {
            assert_eq!(consumer.pop().unwrap(), i);
        }
    }
    
    #[test]
    fn test_large_n_with_small_capacity() {
        // Test N=256 with capacity < N (should use stack)
        // 测试 N=256 但容量 < N（应使用栈）
        let (mut producer, mut consumer) = new::<u64, 256>(NonZero::new(16).unwrap());
        
        for i in 0..10 {
            producer.push(i).unwrap();
        }
        
        for i in 0..10 {
            assert_eq!(consumer.pop().unwrap(), i);
        }
    }
    
    // ==================== 类型测试 / Type Tests ====================
    
    #[test]
    fn test_zero_sized_types() {
        // Test with zero-sized type
        // 测试零大小类型
        let (mut producer, mut consumer) = new::<(), 32>(NonZero::new(4).unwrap());
        
        producer.push(()).unwrap();
        producer.push(()).unwrap();
        
        assert_eq!(consumer.pop().unwrap(), ());
        assert_eq!(consumer.pop().unwrap(), ());
    }
    
    #[test]
    fn test_large_types() {
        // Test with large struct
        // 测试大型结构体
        #[derive(Debug, PartialEq, Clone)]
        struct LargeStruct {
            data: [u64; 32],
        }
        
        let (mut producer, mut consumer) = new::<LargeStruct, 32>(NonZero::new(4).unwrap());
        
        let item1 = LargeStruct { data: [1; 32] };
        let item2 = LargeStruct { data: [2; 32] };
        
        producer.push(item1.clone()).unwrap();
        producer.push(item2.clone()).unwrap();
        
        assert_eq!(consumer.pop().unwrap(), item1);
        assert_eq!(consumer.pop().unwrap(), item2);
    }
    
    #[test]
    fn test_string_type() {
        // Test with String (heap-allocated type)
        // 测试 String（堆分配类型）
        let (mut producer, mut consumer) = new::<String, 32>(NonZero::new(8).unwrap());
        
        let messages = vec!["Hello", "World", "Rust", "Ring", "Buffer"];
        
        for msg in &messages {
            producer.push(msg.to_string()).unwrap();
        }
        
        for msg in &messages {
            assert_eq!(consumer.pop().unwrap(), msg.to_string());
        }
    }
    
    #[test]
    fn test_option_type() {
        // Test with Option<T>
        // 测试 Option<T>
        let (mut producer, mut consumer) = new::<Option<i32>, 32>(NonZero::new(4).unwrap());
        
        producer.push(Some(42)).unwrap();
        producer.push(None).unwrap();
        producer.push(Some(100)).unwrap();
        
        assert_eq!(consumer.pop().unwrap(), Some(42));
        assert_eq!(consumer.pop().unwrap(), None);
        assert_eq!(consumer.pop().unwrap(), Some(100));
    }
    
    // ==================== 并发测试 / Concurrency Tests ====================
    
    #[test]
    fn test_concurrent_small_buffer() {
        // Test concurrent access with small buffer (high contention)
        // 测试小缓冲区的并发访问（高竞争）
        use std::thread;
        
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(4).unwrap());
        
        let count = 100;
        
        let producer_handle = thread::spawn(move || {
            for i in 0..count {
                loop {
                    if producer.push(i).is_ok() {
                        break;
                    }
                    thread::yield_now();
                }
            }
        });
        
        let consumer_handle = thread::spawn(move || {
            let mut sum = 0;
            for _ in 0..count {
                loop {
                    match consumer.pop() {
                        Ok(val) => {
                            sum += val;
                            break;
                        }
                        Err(_) => thread::yield_now(),
                    }
                }
            }
            sum
        });
        
        producer_handle.join().unwrap();
        let sum = consumer_handle.join().unwrap();
        
        // Sum of 0..100 = 100*99/2 = 4950
        assert_eq!(sum, (count * (count - 1)) / 2);
    }
    
    #[test]
    fn test_concurrent_large_buffer() {
        // Test concurrent access with large buffer (low contention)
        // 测试大缓冲区的并发访问（低竞争）
        use std::thread;
        
        let (mut producer, mut consumer) = new::<u64, 512>(NonZero::new(512).unwrap());
        
        let count = 10000;
        
        let producer_handle = thread::spawn(move || {
            for i in 0..count {
                loop {
                    if producer.push(i).is_ok() {
                        break;
                    }
                    thread::yield_now();
                }
            }
        });
        
        let consumer_handle = thread::spawn(move || {
            let mut received = 0;
            for _ in 0..count {
                loop {
                    match consumer.pop() {
                        Ok(_) => {
                            received += 1;
                            break;
                        }
                        Err(_) => thread::yield_now(),
                    }
                }
            }
            received
        });
        
        producer_handle.join().unwrap();
        let received = consumer_handle.join().unwrap();
        
        assert_eq!(received, count);
    }
    
    #[test]
    fn test_concurrent_with_different_speeds() {
        // Test when producer and consumer have different speeds
        // 测试生产者和消费者速度不同的情况
        use std::thread;
        use std::time::Duration;
        
        let (mut producer, mut consumer) = new::<u32, 64>(NonZero::new(32).unwrap());
        
        let producer_handle = thread::spawn(move || {
            for i in 0..50 {
                loop {
                    if producer.push(i).is_ok() {
                        break;
                    }
                    thread::yield_now();
                }
                // Slow producer
                // 慢速生产者
                if i % 10 == 0 {
                    thread::sleep(Duration::from_micros(1));
                }
            }
        });
        
        let consumer_handle = thread::spawn(move || {
            let mut received = Vec::new();
            for _ in 0..50 {
                loop {
                    match consumer.pop() {
                        Ok(val) => {
                            received.push(val);
                            break;
                        }
                        Err(_) => thread::yield_now(),
                    }
                }
            }
            received
        });
        
        producer_handle.join().unwrap();
        let received = consumer_handle.join().unwrap();
        
        assert_eq!(received.len(), 50);
        for (i, &val) in received.iter().enumerate() {
            assert_eq!(val, i as u32);
        }
    }
    
    // ==================== 错误处理测试 / Error Handling Tests ====================
    
    #[test]
    fn test_push_error_value_returned() {
        // Test that PushError returns the value
        // 测试 PushError 返回值
        let (mut producer, _consumer) = new::<String, 32>(NonZero::new(2).unwrap());
        
        producer.push("first".to_string()).unwrap();
        producer.push("second".to_string()).unwrap();
        
        let value = "third".to_string();
        match producer.push(value.clone()) {
            Err(PushError::Full(returned_value)) => {
                assert_eq!(returned_value, value);
            }
            Ok(_) => panic!("Expected PushError::Full"),
        }
    }
    
    #[test]
    fn test_pop_error() {
        // Test PopError::Empty
        // 测试 PopError::Empty
        let (_producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());
        
        match consumer.pop() {
            Err(PopError::Empty) => {} // Expected
            Ok(_) => panic!("Expected PopError::Empty"),
        }
    }
    
    // ==================== Consumer 方法测试 / Consumer Method Tests ====================
    
    #[test]
    fn test_is_empty_after_operations() {
        // Test is_empty() with various operations
        // 测试各种操作后的 is_empty()
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());
        
        assert!(consumer.is_empty());
        
        producer.push(1).unwrap();
        assert!(!consumer.is_empty());
        
        producer.push(2).unwrap();
        assert!(!consumer.is_empty());
        
        consumer.pop().unwrap();
        assert!(!consumer.is_empty());
        
        consumer.pop().unwrap();
        assert!(consumer.is_empty());
    }
    
    #[test]
    fn test_slots_accuracy() {
        // Test that slots() returns accurate count
        // 测试 slots() 返回准确计数
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(16).unwrap());
        
        assert_eq!(consumer.slots(), 0);
        
        for i in 1..=10 {
            producer.push(i).unwrap();
            assert_eq!(consumer.slots(), i as usize);
        }
        
        for i in (0..10).rev() {
            consumer.pop().unwrap();
            assert_eq!(consumer.slots(), i);
        }
        
        assert_eq!(consumer.slots(), 0);
    }
    
    #[test]
    fn test_slots_with_wrap_around() {
        // Test slots() after indices wrap around
        // 测试索引环绕后的 slots()
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(4).unwrap());
        
        // Cause many wrap-arounds
        // 导致多次环绕
        for _ in 0..100 {
            for i in 0..3 {
                producer.push(i).unwrap();
            }
            assert_eq!(consumer.slots(), 3);
            
            for _ in 0..3 {
                consumer.pop().unwrap();
            }
            assert_eq!(consumer.slots(), 0);
        }
    }
    
    // ==================== Drop 和内存测试 / Drop and Memory Tests ====================
    
    #[test]
    fn test_partial_drop_cleanup() {
        // Test that consumer drops only remaining items
        // 测试消费者仅 drop 剩余项
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        
        #[derive(Debug)]
        struct DropCounter {
            counter: Arc<AtomicUsize>,
        }
        
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.counter.fetch_add(1, Ordering::SeqCst);
            }
        }
        
        let counter = Arc::new(AtomicUsize::new(0));
        
        {
            let (mut producer, mut consumer) = new::<DropCounter, 32>(NonZero::new(16).unwrap());
            
            // Push 10 items (capacity is 16, so no overflow)
            for _ in 0..10 {
                producer.push(DropCounter { counter: counter.clone() }).unwrap();
            }
            
            // Pop 6 items (they should be dropped)
            for _ in 0..6 {
                consumer.pop().unwrap();
            }
            
            // At this point, 6 items have been dropped
            assert_eq!(counter.load(Ordering::SeqCst), 6);
            
            // Drop consumer, which should drop remaining 4 items
            drop(consumer);
        }
        
        // All 10 items should have been dropped
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
    
    #[test]
    fn test_empty_buffer_drop() {
        // Test dropping empty buffer
        // 测试 drop 空缓冲区
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        
        #[derive(Debug)]
        struct DropCounter {
            counter: Arc<AtomicUsize>,
        }
        
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.counter.fetch_add(1, Ordering::SeqCst);
            }
        }
        
        let counter = Arc::new(AtomicUsize::new(0));
        
        {
            let (mut producer, mut consumer) = new::<DropCounter, 32>(NonZero::new(8).unwrap());
            
            producer.push(DropCounter { counter: counter.clone() }).unwrap();
            consumer.pop().unwrap();
            
            // Buffer is now empty
            assert!(consumer.is_empty());
            
            // Drop should not drop anything
            drop(consumer);
        }
        
        // Only 1 item should have been dropped (the one we popped)
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
    
    // ==================== 压力测试 / Stress Tests ====================
    
    #[test]
    fn test_high_throughput() {
        // Stress test with high throughput
        // 高吞吐量压力测试
        use std::thread;
        
        let (mut producer, mut consumer) = new::<u64, 256>(NonZero::new(256).unwrap());
        let count = 100000;
        
        let producer_handle = thread::spawn(move || {
            for i in 0..count {
                loop {
                    if producer.push(i).is_ok() {
                        break;
                    }
                    thread::yield_now();
                }
            }
        });
        
        let consumer_handle = thread::spawn(move || {
            let mut last = None;
            for _ in 0..count {
                loop {
                    match consumer.pop() {
                        Ok(val) => {
                            if let Some(prev) = last {
                                assert_eq!(val, prev + 1, "Values must be sequential");
                            }
                            last = Some(val);
                            break;
                        }
                        Err(_) => thread::yield_now(),
                    }
                }
            }
            last
        });
        
        producer_handle.join().unwrap();
        let last = consumer_handle.join().unwrap();
        
        assert_eq!(last, Some(count - 1));
    }
    
    // ==================== 新增 API 测试 / New API Tests ====================
    
    #[test]
    fn test_producer_capacity_queries() {
        // Test Producer capacity query methods
        // 测试 Producer 容量查询方法
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(8).unwrap());
        
        assert_eq!(producer.capacity(), 8);
        assert_eq!(producer.len(), 0);
        assert_eq!(producer.slots(), 0);
        assert_eq!(producer.free_slots(), 8);
        assert!(!producer.is_full());
        
        producer.push(1).unwrap();
        producer.push(2).unwrap();
        producer.push(3).unwrap();
        
        assert_eq!(producer.len(), 3);
        assert_eq!(producer.slots(), 3);
        assert_eq!(producer.free_slots(), 5);
        assert!(!producer.is_full());
        
        // Fill the buffer
        producer.push(4).unwrap();
        producer.push(5).unwrap();
        producer.push(6).unwrap();
        producer.push(7).unwrap();
        producer.push(8).unwrap();
        
        assert_eq!(producer.len(), 8);
        assert_eq!(producer.slots(), 8);
        assert_eq!(producer.free_slots(), 0);
        assert!(producer.is_full());
        
        // Pop one and check again
        consumer.pop().unwrap();
        
        assert_eq!(producer.len(), 7);
        assert_eq!(producer.free_slots(), 1);
        assert!(!producer.is_full());
    }
    
    #[test]
    fn test_consumer_len_and_capacity() {
        // Test Consumer len() and capacity() methods
        // 测试 Consumer 的 len() 和 capacity() 方法
        let (mut producer, consumer) = new::<i32, 32>(NonZero::new(16).unwrap());
        
        assert_eq!(consumer.len(), 0);
        assert_eq!(consumer.capacity(), 16);
        
        for i in 0..10 {
            producer.push(i).unwrap();
        }
        
        assert_eq!(consumer.len(), 10);
        assert_eq!(consumer.capacity(), 16);
    }
    
    #[test]
    fn test_peek() {
        // Test peek operation
        // 测试 peek 操作
        let (mut producer, consumer) = new::<i32, 32>(NonZero::new(8).unwrap());
        
        // Peek empty buffer
        assert!(consumer.peek().is_none());
        
        producer.push(42).unwrap();
        producer.push(100).unwrap();
        producer.push(200).unwrap();
        
        // Peek should return first element without removing it
        assert_eq!(consumer.peek(), Some(&42));
        assert_eq!(consumer.peek(), Some(&42)); // Peek again, should be same
        assert_eq!(consumer.len(), 3); // Length unchanged
    }
    
    #[test]
    fn test_peek_after_pop() {
        // Test peek after pop operations
        // 测试 pop 后的 peek 操作
        let (mut producer, mut consumer) = new::<String, 32>(NonZero::new(8).unwrap());
        
        producer.push("first".to_string()).unwrap();
        producer.push("second".to_string()).unwrap();
        producer.push("third".to_string()).unwrap();
        
        assert_eq!(consumer.peek(), Some(&"first".to_string()));
        consumer.pop().unwrap();
        
        assert_eq!(consumer.peek(), Some(&"second".to_string()));
        consumer.pop().unwrap();
        
        assert_eq!(consumer.peek(), Some(&"third".to_string()));
        consumer.pop().unwrap();
        
        assert!(consumer.peek().is_none());
    }
    
    #[test]
    fn test_push_slice_basic() {
        // Test basic push_slice operation
        // 测试基本的 push_slice 操作
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(16).unwrap());
        
        let data = [1, 2, 3, 4, 5];
        let pushed = producer.push_slice(&data);
        
        assert_eq!(pushed, 5);
        assert_eq!(consumer.len(), 5);
        
        for i in 0..5 {
            assert_eq!(consumer.pop().unwrap(), data[i]);
        }
    }
    
    #[test]
    fn test_push_slice_partial() {
        // Test push_slice when buffer is partially full
        // 测试缓冲区部分满时的 push_slice
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(8).unwrap());
        
        // Fill with 5 elements, leaving room for 3
        let initial = [1, 2, 3, 4, 5];
        producer.push_slice(&initial);
        
        // Try to push 10 more, should only push 3
        let more = [6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let pushed = producer.push_slice(&more);
        
        assert_eq!(pushed, 3);
        assert_eq!(consumer.len(), 8);
        assert!(producer.is_full());
        
        // Verify values
        for i in 1..=8 {
            assert_eq!(consumer.pop().unwrap(), i);
        }
    }
    
    #[test]
    fn test_push_slice_wrap_around() {
        // Test push_slice with wrap-around
        // 测试 push_slice 的环绕情况
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(8).unwrap());
        
        // Fill buffer
        producer.push_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        
        // Pop some elements
        for _ in 0..5 {
            consumer.pop().unwrap();
        }
        
        // Push more elements (will cause wrap-around)
        let data = [10, 11, 12, 13, 14];
        let pushed = producer.push_slice(&data);
        
        assert_eq!(pushed, 5);
        
        // Verify all values
        assert_eq!(consumer.pop().unwrap(), 6);
        assert_eq!(consumer.pop().unwrap(), 7);
        assert_eq!(consumer.pop().unwrap(), 8);
        assert_eq!(consumer.pop().unwrap(), 10);
        assert_eq!(consumer.pop().unwrap(), 11);
        assert_eq!(consumer.pop().unwrap(), 12);
        assert_eq!(consumer.pop().unwrap(), 13);
        assert_eq!(consumer.pop().unwrap(), 14);
    }
    
    #[test]
    fn test_push_slice_empty() {
        // Test push_slice with empty slice
        // 测试空切片的 push_slice
        let (mut producer, _consumer) = new::<u32, 32>(NonZero::new(8).unwrap());
        
        let pushed = producer.push_slice(&[]);
        assert_eq!(pushed, 0);
    }
    
    #[test]
    fn test_pop_slice_basic() {
        // Test basic pop_slice operation
        // 测试基本的 pop_slice 操作
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(16).unwrap());
        
        // Push some data
        for i in 0..10 {
            producer.push(i).unwrap();
        }
        
        let mut dest = [0u32; 5];
        let popped = consumer.pop_slice(&mut dest);
        
        assert_eq!(popped, 5);
        assert_eq!(dest, [0, 1, 2, 3, 4]);
        assert_eq!(consumer.len(), 5);
    }
    
    #[test]
    fn test_pop_slice_partial() {
        // Test pop_slice when buffer has fewer elements than dest
        // 测试当缓冲区元素少于目标切片时的 pop_slice
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(16).unwrap());
        
        producer.push(1).unwrap();
        producer.push(2).unwrap();
        producer.push(3).unwrap();
        
        let mut dest = [0u32; 10];
        let popped = consumer.pop_slice(&mut dest);
        
        assert_eq!(popped, 3);
        assert_eq!(&dest[0..3], &[1, 2, 3]);
        assert!(consumer.is_empty());
    }
    
    #[test]
    fn test_pop_slice_wrap_around() {
        // Test pop_slice with wrap-around
        // 测试 pop_slice 的环绕情况
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(8).unwrap());
        
        // Fill buffer
        producer.push_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        
        // Pop 5 elements
        let mut temp = [0u32; 5];
        let popped = consumer.pop_slice(&mut temp);
        assert_eq!(popped, 5);
        assert_eq!(temp, [1, 2, 3, 4, 5]);
        
        // Push 5 more elements (will cause wrap-around in the ring buffer)
        let pushed = producer.push_slice(&[9, 10, 11, 12, 13]);
        assert_eq!(pushed, 5);
        
        // Pop remaining 3 elements from first batch
        let mut dest1 = [0u32; 3];
        let popped1 = consumer.pop_slice(&mut dest1);
        assert_eq!(popped1, 3);
        assert_eq!(dest1, [6, 7, 8]);
        
        // Pop 5 elements from second batch
        let mut dest2 = [0u32; 5];
        let popped2 = consumer.pop_slice(&mut dest2);
        assert_eq!(popped2, 5);
        assert_eq!(dest2, [9, 10, 11, 12, 13]);
    }
    
    #[test]
    fn test_pop_slice_empty() {
        // Test pop_slice on empty buffer
        // 测试空缓冲区的 pop_slice
        let (_producer, mut consumer) = new::<u32, 32>(NonZero::new(8).unwrap());
        
        let mut dest = [0u32; 5];
        let popped = consumer.pop_slice(&mut dest);
        
        assert_eq!(popped, 0);
    }
    
    #[test]
    fn test_clear() {
        // Test clear operation
        // 测试 clear 操作
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(16).unwrap());
        
        for i in 0..10 {
            producer.push(i).unwrap();
        }
        
        assert_eq!(consumer.len(), 10);
        
        consumer.clear();
        
        assert_eq!(consumer.len(), 0);
        assert!(consumer.is_empty());
    }
    
    #[test]
    fn test_clear_with_drop() {
        // Test that clear properly drops all elements
        // 测试 clear 正确 drop 所有元素
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        
        #[derive(Debug)]
        struct DropCounter {
            counter: Arc<AtomicUsize>,
        }
        
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.counter.fetch_add(1, Ordering::SeqCst);
            }
        }
        
        let counter = Arc::new(AtomicUsize::new(0));
        
        {
            let (mut producer, mut consumer) = new::<DropCounter, 32>(NonZero::new(16).unwrap());
            
            for _ in 0..8 {
                producer.push(DropCounter { counter: counter.clone() }).unwrap();
            }
            
            assert_eq!(counter.load(Ordering::SeqCst), 0);
            
            consumer.clear();
            
            assert_eq!(counter.load(Ordering::SeqCst), 8);
        }
    }
    
    #[test]
    fn test_drain_iterator() {
        // Test drain iterator
        // 测试 drain 迭代器
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(16).unwrap());
        
        for i in 0..10 {
            producer.push(i).unwrap();
        }
        
        let collected: Vec<i32> = consumer.drain().collect();
        
        assert_eq!(collected, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert!(consumer.is_empty());
    }
    
    #[test]
    fn test_drain_empty() {
        // Test drain on empty buffer
        // 测试空缓冲区的 drain
        let (_producer, mut consumer) = new::<i32, 32>(NonZero::new(8).unwrap());
        
        let collected: Vec<i32> = consumer.drain().collect();
        
        assert!(collected.is_empty());
    }
    
    #[test]
    fn test_drain_size_hint() {
        // Test drain iterator size_hint
        // 测试 drain 迭代器的 size_hint
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(16).unwrap());
        
        for i in 0..5 {
            producer.push(i).unwrap();
        }
        
        let mut drain = consumer.drain();
        
        assert_eq!(drain.size_hint(), (5, Some(5)));
        
        drain.next();
        assert_eq!(drain.size_hint(), (4, Some(4)));
        
        drain.next();
        assert_eq!(drain.size_hint(), (3, Some(3)));
    }
    
    #[test]
    fn test_drain_partial() {
        // Test partially consuming drain iterator
        // 测试部分消费 drain 迭代器
        let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(16).unwrap());
        
        for i in 0..10 {
            producer.push(i).unwrap();
        }
        
        let mut drain = consumer.drain();
        
        assert_eq!(drain.next(), Some(0));
        assert_eq!(drain.next(), Some(1));
        assert_eq!(drain.next(), Some(2));
        
        drop(drain); // Drop the iterator
        
        // Buffer should have 7 elements left
        assert_eq!(consumer.len(), 7);
    }
    
    #[test]
    fn test_combined_operations() {
        // Test combining various new APIs
        // 测试组合使用各种新 API
        let (mut producer, mut consumer) = new::<u32, 32>(NonZero::new(16).unwrap());
        
        // Batch push
        let data = [1, 2, 3, 4, 5];
        producer.push_slice(&data);
        
        assert_eq!(producer.len(), 5);
        assert_eq!(consumer.len(), 5);
        assert_eq!(consumer.capacity(), 16);
        
        // Peek
        assert_eq!(consumer.peek(), Some(&1));
        
        // Batch pop
        let mut dest = [0u32; 3];
        consumer.pop_slice(&mut dest);
        assert_eq!(dest, [1, 2, 3]);
        
        assert_eq!(consumer.len(), 2);
        assert_eq!(producer.free_slots(), 14);
        
        // Clear remaining
        consumer.clear();
        assert!(consumer.is_empty());
        assert!(!producer.is_full());
    }
}

