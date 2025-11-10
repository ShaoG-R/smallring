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
