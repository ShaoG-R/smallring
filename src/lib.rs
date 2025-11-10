//! # High-Performance SPSC Ring Buffer
//! 
//! 高性能 SPSC 环形缓冲区
//! 
//! `smallring` is a high-performance lock-free Single Producer Single Consumer (SPSC)
//! ring buffer implementation with automatic stack/heap optimization.
//! 
//! `smallring` 是一个高性能的单生产者单消费者（SPSC）无锁环形缓冲区实现，
//! 具有自动栈/堆优化能力。
//! 
//! ## Features
//! 
//! 特性
//! 
//! - **Lock-Free** - Thread-safe communication using atomic operations
//! - **Stack/Heap Optimization** - Small buffers automatically use stack storage
//! - **High Performance** - Optimized for SPSC scenarios with minimal atomic overhead
//! - **Type Safe** - Full Rust type system guarantees
//! - **Zero Copy** - Data is moved directly without extra copying
//! 
//! - **无锁设计** - 使用原子操作实现线程安全的无锁通信
//! - **栈/堆优化** - 小容量数据自动存储在栈上，避免堆分配开销
//! - **高性能** - 针对 SPSC 场景优化，最小化原子操作开销
//! - **类型安全** - 完整的 Rust 类型系统保证
//! - **零拷贝** - 数据直接移动，无额外拷贝开销
//! 
//! ## Quick Start
//! 
//! 快速开始
//! 
//! ```rust
//! use smallring::new;
//! use std::num::NonZero;
//! 
//! // Create a ring buffer with capacity 8, stack threshold 32
//! // 创建一个容量为 8 的环形缓冲区，栈容量阈值为 32
//! let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(8).unwrap());
//! 
//! // Producer pushes data
//! // 生产者推送数据
//! producer.push(42).unwrap();
//! producer.push(100).unwrap();
//! 
//! // Consumer pops data
//! // 消费者获取数据
//! assert_eq!(consumer.pop().unwrap(), 42);
//! assert_eq!(consumer.pop().unwrap(), 100);
//! ```
//! 
//! ## Multi-threaded Usage
//! 
//! 多线程使用
//! 
//! ```rust
//! use smallring::new;
//! use std::thread;
//! use std::num::NonZero;
//! 
//! let (mut producer, mut consumer) = new::<String, 64>(NonZero::new(32).unwrap());
//! 
//! // Producer thread
//! // 生产者线程
//! let producer_handle = thread::spawn(move || {
//!     for i in 0..100 {
//!         let msg = format!("Message {}", i);
//!         while producer.push(msg.clone()).is_err() {
//!             thread::yield_now();
//!         }
//!     }
//! });
//! 
//! // Consumer thread
//! // 消费者线程
//! let consumer_handle = thread::spawn(move || {
//!     let mut received = Vec::new();
//!     for _ in 0..100 {
//!         loop {
//!             match consumer.pop() {
//!                 Ok(msg) => {
//!                     received.push(msg);
//!                     break;
//!                 }
//!                 Err(_) => thread::yield_now(),
//!             }
//!         }
//!     }
//!     received
//! });
//! 
//! producer_handle.join().unwrap();
//! let messages = consumer_handle.join().unwrap();
//! assert_eq!(messages.len(), 100);
//! ```
//! 
//! ## Stack/Heap Optimization
//! 
//! 栈/堆优化
//! 
//! `smallring` uses generic constant `N` to control the stack/heap optimization threshold:
//! 
//! `smallring` 使用泛型常量 `N` 来控制栈/堆优化的阈值：
//! 
//! ```rust
//! use smallring::new;
//! use std::num::NonZero;
//! 
//! // Capacity ≤ 32, uses stack storage (faster initialization)
//! // 容量 ≤ 32，使用栈存储（更快的初始化）
//! let (prod, cons) = new::<u64, 32>(NonZero::new(16).unwrap());
//! 
//! // Capacity > 32, uses heap storage
//! // 容量 > 32，使用堆存储
//! let (prod, cons) = new::<u64, 32>(NonZero::new(64).unwrap());
//! 
//! // Larger stack threshold for larger stack storage
//! // 更大的栈阈值可用于更大的栈存储
//! let (prod, cons) = new::<u64, 128>(NonZero::new(100).unwrap());
//! ```
//! 
//! ## API Overview
//! 
//! API 概览
//! 
//! ### Producer Methods
//! 
//! 生产者方法
//! 
//! - `push(value)` - Push a single element
//! - `push_slice(&[T])` - Push multiple elements (requires `T: Copy`)
//! - `capacity()` - Get buffer capacity
//! - `len()` / `slots()` - Get number of elements in buffer
//! - `free_slots()` - Get available space
//! - `is_full()` - Check if buffer is full
//! 
//! - `push(value)` - 推送单个元素
//! - `push_slice(&[T])` - 批量推送多个元素（需要 `T: Copy`）
//! - `capacity()` - 获取缓冲区容量
//! - `len()` / `slots()` - 获取缓冲区中的元素数量
//! - `free_slots()` - 获取可用空间
//! - `is_full()` - 检查缓冲区是否已满
//! 
//! ### Consumer Methods
//! 
//! 消费者方法
//! 
//! - `pop()` - Pop a single element
//! - `pop_slice(&mut [T])` - Pop multiple elements (requires `T: Copy`)
//! - `peek()` - View first element without removing
//! - `drain()` - Create draining iterator
//! - `clear()` - Remove all elements
//! - `capacity()` - Get buffer capacity
//! - `len()` / `slots()` - Get number of elements in buffer
//! - `is_empty()` - Check if buffer is empty
//! 
//! - `pop()` - 弹出单个元素
//! - `pop_slice(&mut [T])` - 批量弹出多个元素（需要 `T: Copy`）
//! - `peek()` - 查看第一个元素但不移除
//! - `drain()` - 创建消费迭代器
//! - `clear()` - 移除所有元素
//! - `capacity()` - 获取缓冲区容量
//! - `len()` / `slots()` - 获取缓冲区中的元素数量
//! - `is_empty()` - 检查缓冲区是否为空
//! 
//! ## Batch Operations Example
//! 
//! 批量操作示例
//! 
//! ```rust
//! use smallring::new;
//! use std::num::NonZero;
//! 
//! let (mut producer, mut consumer) = new::<u32, 64>(NonZero::new(32).unwrap());
//! 
//! // Batch push - much faster than pushing one by one
//! // 批量推送 - 比逐个推送快得多
//! let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
//! let pushed = producer.push_slice(&data);
//! assert_eq!(pushed, 10);
//! 
//! // Batch pop
//! // 批量弹出
//! let mut output = [0u32; 5];
//! let popped = consumer.pop_slice(&mut output);
//! assert_eq!(popped, 5);
//! assert_eq!(output, [1, 2, 3, 4, 5]);
//! 
//! // Drain remaining elements
//! // 清空剩余元素
//! let remaining: Vec<u32> = consumer.drain().collect();
//! assert_eq!(remaining, vec![6, 7, 8, 9, 10]);
//! ```
//! 
//! ## Peek and Query Example
//! 
//! 窥视和查询示例
//! 
//! ```rust
//! use smallring::new;
//! use std::num::NonZero;
//! 
//! let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(8).unwrap());
//! 
//! producer.push(42).unwrap();
//! producer.push(100).unwrap();
//! 
//! // Check capacity and usage
//! // 检查容量和使用情况
//! assert_eq!(producer.capacity(), 8);
//! assert_eq!(producer.len(), 2);
//! assert_eq!(producer.free_slots(), 6);
//! assert!(!producer.is_full());
//! 
//! // Peek at first element without consuming
//! // 查看第一个元素但不消费
//! assert_eq!(consumer.peek(), Some(&42));
//! assert_eq!(consumer.len(), 2); // Still 2 elements
//! 
//! // Now consume it
//! // 现在消费它
//! assert_eq!(consumer.pop(), Ok(42));
//! assert_eq!(consumer.len(), 1);
//! ```
//! 
//! ## Performance Tips
//! 
//! 性能提示
//! 
//! 1. **Choose appropriate capacity** - Capacity is rounded up to power of 2
//! 2. **Use batch operations** - `push_slice` and `pop_slice` are much faster than individual operations
//! 3. **Choose appropriate N** - Stack storage significantly improves performance for small buffers
//! 4. **Use peek when needed** - Avoid pop + re-push patterns
//! 
//! 1. **选择合适的容量** - 容量会向上取整到 2 的幂次，选择 2 的幂次可避免浪费
//! 2. **使用批量操作** - `push_slice` 和 `pop_slice` 比单个操作快得多
//! 3. **选择合适的 N** - 对于小缓冲区，使用栈存储可显著提升性能
//! 4. **在需要时使用 peek** - 避免 pop + 重新 push 的模式
//! 
//! ## Notes
//! 
//! 注意事项
//! 
//! - Capacity is automatically rounded up to the nearest power of 2
//! - Only supports Single Producer Single Consumer (SPSC) scenarios
//! - Remaining elements are automatically cleaned up when `Consumer` is dropped
//! 
//! - 容量会自动向上取整到最接近的 2 的幂次
//! - 仅支持单生产者单消费者（SPSC）场景
//! - `Consumer` 被 drop 时会自动清理缓冲区中的剩余元素

pub mod spsc;
pub mod generic;
mod vec;
mod core;