//! # smallring - High-Performance Lock-Free Ring Buffers
//! 
//! smallring - 高性能无锁环形缓冲区
//! 
//! `smallring` is a collection of high-performance lock-free ring buffer implementations
//! with automatic stack/heap optimization. This library provides two complementary modules
//! for different use cases:
//! 
//! `smallring` 是一个高性能无锁环形缓冲区实现集合，具有自动栈/堆优化能力。
//! 本库为不同的使用场景提供了两个互补的模块：
//! 
//! ## Modules
//! 
//! 模块
//! 
//! - **[`spsc`]** - Single Producer Single Consumer (SPSC) ring buffer with split producer/consumer handles
//! - **[`generic`]** - Generic ring buffer with compile-time configurable overwrite behavior
//! 
//! - **[`spsc`]** - 单生产者单消费者（SPSC）环形缓冲区，具有分离的生产者/消费者句柄
//! - **[`generic`]** - 通用环形缓冲区，具有编译期可配置的覆盖行为
//! 
//! ## Features
//! 
//! 特性
//! 
//! - **Lock-Free** - Thread-safe operations using atomic primitives
//! - **Stack/Heap Optimization** - Small buffers automatically use stack storage for better performance
//! - **High Performance** - Optimized with minimal atomic overhead and efficient masking
//! - **Type Safe** - Full Rust type system guarantees
//! - **Zero Copy** - Data is moved directly without extra copying
//! - **Power-of-2 Capacity** - Automatic rounding for efficient modulo operations
//! 
//! - **无锁设计** - 使用原子原语实现线程安全的操作
//! - **栈/堆优化** - 小缓冲区自动使用栈存储以获得更好的性能
//! - **高性能** - 通过最小化原子操作开销和高效的掩码操作进行优化
//! - **类型安全** - 完整的 Rust 类型系统保证
//! - **零拷贝** - 数据直接移动，无额外拷贝开销
//! - **2的幂次容量** - 自动向上取整以实现高效的取模操作
//! 
//! ## Quick Start
//! 
//! 快速开始
//! 
//! ### SPSC Module - Split Producer/Consumer
//! 
//! SPSC 模块 - 分离的生产者/消费者
//! 
//! ```rust
//! use smallring::spsc::new;
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
//! ### Generic Module - Shared Buffer with Configurable Behavior
//! 
//! Generic 模块 - 具有可配置行为的共享缓冲区
//! 
//! ```rust
//! use smallring::generic::RingBuf;
//! 
//! // Overwrite mode: automatically overwrites oldest data when full
//! // 覆盖模式：满时自动覆盖最旧的数据
//! let buf_overwrite: RingBuf<i32, 32, true> = RingBuf::new(4);
//! buf_overwrite.push(1); // Returns None
//! buf_overwrite.push(2);
//! buf_overwrite.push(3);
//! buf_overwrite.push(4);
//! buf_overwrite.push(5); // Returns Some(1), overwrote oldest
//! 
//! // Non-overwrite mode: rejects writes when full
//! // 非覆盖模式：满时拒绝写入
//! let buf_reject: RingBuf<i32, 32, false> = RingBuf::new(4);
//! buf_reject.push(1).unwrap(); // Returns Ok(())
//! buf_reject.push(2).unwrap();
//! buf_reject.push(3).unwrap();
//! buf_reject.push(4).unwrap();
//! assert!(buf_reject.push(5).is_err()); // Returns Err(Full(5))
//! ```
//! 
//! ## Choosing Between SPSC and Generic
//! 
//! 在 SPSC 和 Generic 之间选择
//! 
//! | Feature | SPSC Module | Generic Module |
//! |---------|-------------|----------------|
//! | Use Case | Cross-thread communication | Single-thread or shared access |
//! | Handles | Split Producer/Consumer | Shared RingBuf |
//! | Overwrite | Always rejects when full | Compile-time configurable |
//! | Cache Optimization | Cached read/write indices | Direct atomic access |
//! | Drop Behavior | Consumer auto-cleans on drop | Manual cleanup via `clear()` |
//! 
//! | 特性 | SPSC 模块 | Generic 模块 |
//! |------|-----------|--------------|
//! | 使用场景 | 跨线程通信 | 单线程或共享访问 |
//! | 句柄 | 分离的 Producer/Consumer | 共享的 RingBuf |
//! | 覆盖行为 | 总是拒绝满时写入 | 编译期可配置 |
//! | 缓存优化 | 缓存的读写索引 | 直接原子访问 |
//! | Drop 行为 | Consumer 自动清理 | 需手动调用 `clear()` |
//! 
//! **Choose SPSC when:**
//! - You need cross-thread communication with separated producer/consumer roles
//! - You want automatic cleanup on Consumer drop
//! - Performance is critical and you can leverage cached indices
//! 
//! **选择 SPSC 当：**
//! - 你需要跨线程通信，具有分离的生产者/消费者角色
//! - 你希望 Consumer drop 时自动清理
//! - 性能至关重要，你可以利用缓存的索引
//! 
//! **Choose Generic when:**
//! - You need shared access from a single thread or within `Arc`
//! - You want compile-time configurable overwrite behavior
//! - You need multiple concurrent readers/writers (with appropriate synchronization)
//! 
//! **选择 Generic 当：**
//! - 你需要从单线程或 `Arc` 中进行共享访问
//! - 你需要编译期可配置的覆盖行为
//! - 你需要多个并发读写者（需适当的同步机制）
//! 
//! ## Multi-threaded Usage (SPSC)
//! 
//! 多线程使用（SPSC）
//! 
//! ```rust
//! use smallring::spsc::new;
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
//! ## Shared Access with Generic Module
//! 
//! 使用 Generic 模块的共享访问
//! 
//! ```rust
//! use smallring::generic::RingBuf;
//! use std::sync::Arc;
//! use std::thread;
//! 
//! // Overwrite mode is thread-safe for concurrent writers
//! // 覆盖模式对于并发写入者是线程安全的
//! let buf = Arc::new(RingBuf::<u64, 128, true>::new(128));
//! let mut handles = vec![];
//! 
//! // Multiple writer threads
//! // 多个写入线程
//! for thread_id in 0..4 {
//!     let buf_clone = Arc::clone(&buf);
//!     let handle = thread::spawn(move || {
//!         for i in 0..100 {
//!             let value = (thread_id * 100 + i) as u64;
//!             buf_clone.push(value); // Automatically overwrites old data
//!         }
//!     });
//!     handles.push(handle);
//! }
//! 
//! for handle in handles {
//!     handle.join().unwrap();
//! }
//! ```
//! 
//! ## Stack/Heap Optimization
//! 
//! 栈/堆优化
//! 
//! Both modules use generic constant `N` to control the stack/heap optimization threshold.
//! When capacity ≤ N, data is stored on the stack; otherwise, it's allocated on the heap.
//! 
//! 两个模块都使用泛型常量 `N` 来控制栈/堆优化的阈值。
//! 当容量 ≤ N 时，数据存储在栈上；否则，在堆上分配。
//! 
//! ```rust
//! use smallring::spsc::new;
//! use smallring::generic::RingBuf;
//! use std::num::NonZero;
//! 
//! // SPSC: Capacity ≤ 32, uses stack storage (faster initialization)
//! // SPSC：容量 ≤ 32，使用栈存储（更快的初始化）
//! let (prod, cons) = new::<u64, 32>(NonZero::new(16).unwrap());
//! 
//! // SPSC: Capacity > 32, uses heap storage
//! // SPSC：容量 > 32，使用堆存储
//! let (prod, cons) = new::<u64, 32>(NonZero::new(64).unwrap());
//! 
//! // Generic: Larger stack threshold for larger stack storage
//! // Generic：更大的栈阈值可用于更大的栈存储
//! let buf: RingBuf<u64, 128, true> = RingBuf::new(100);
//! 
//! // Generic: Very large stack threshold (use with caution)
//! // Generic：非常大的栈阈值（谨慎使用）
//! let buf: RingBuf<u64, 256, false> = RingBuf::new(200);
//! ```
//! 
//! ## API Overview
//! 
//! API 概览
//! 
//! ### SPSC Module - Producer Methods
//! 
//! SPSC 模块 - 生产者方法
//! 
//! - `push(value) -> Result<(), PushError<T>>` - Push a single element
//! - `push_slice(&[T]) -> usize` - Push multiple elements (requires `T: Copy`)
//! - `capacity() -> usize` - Get buffer capacity
//! - `len() / slots() -> usize` - Get number of elements in buffer
//! - `free_slots() -> usize` - Get available space
//! - `is_full() -> bool` - Check if buffer is full
//! - `is_empty() -> bool` - Check if buffer is empty
//! 
//! - `push(value) -> Result<(), PushError<T>>` - 推送单个元素
//! - `push_slice(&[T]) -> usize` - 批量推送多个元素（需要 `T: Copy`）
//! - `capacity() -> usize` - 获取缓冲区容量
//! - `len() / slots() -> usize` - 获取缓冲区中的元素数量
//! - `free_slots() -> usize` - 获取可用空间
//! - `is_full() -> bool` - 检查缓冲区是否已满
//! - `is_empty() -> bool` - 检查缓冲区是否为空
//! 
//! ### SPSC Module - Consumer Methods
//! 
//! SPSC 模块 - 消费者方法
//! 
//! - `pop() -> Result<T, PopError>` - Pop a single element
//! - `pop_slice(&mut [T]) -> usize` - Pop multiple elements (requires `T: Copy`)
//! - `peek() -> Option<&T>` - View first element without removing
//! - `drain() -> Drain<'_, T, N>` - Create draining iterator
//! - `clear()` - Remove all elements
//! - `capacity() -> usize` - Get buffer capacity
//! - `len() / slots() -> usize` - Get number of elements in buffer
//! - `is_empty() -> bool` - Check if buffer is empty
//! 
//! - `pop() -> Result<T, PopError>` - 弹出单个元素
//! - `pop_slice(&mut [T]) -> usize` - 批量弹出多个元素（需要 `T: Copy`）
//! - `peek() -> Option<&T>` - 查看第一个元素但不移除
//! - `drain() -> Drain<'_, T, N>` - 创建消费迭代器
//! - `clear()` - 移除所有元素
//! - `capacity() -> usize` - 获取缓冲区容量
//! - `len() / slots() -> usize` - 获取缓冲区中的元素数量
//! - `is_empty() -> bool` - 检查缓冲区是否为空
//! 
//! ### Generic Module - RingBuf Methods
//! 
//! Generic 模块 - RingBuf 方法
//! 
//! - `new(capacity) -> Self` - Create a new ring buffer
//! - `push(value)` - Push element (return type depends on `OVERWRITE` flag)
//!   - `OVERWRITE=true`: Returns `Option<T>` (Some if element was overwritten)
//!   - `OVERWRITE=false`: Returns `Result<(), RingBufError<T>>`
//! - `pop() -> Result<T, RingBufError<T>>` - Pop a single element
//! - `push_slice(&[T]) -> usize` - Push multiple elements (requires `T: Copy`)
//! - `pop_slice(&mut [T]) -> usize` - Pop multiple elements (requires `T: Copy`)
//! - `peek() -> Option<&T>` - View first element without removing
//! - `clear()` - Remove all elements
//! - `capacity() -> usize` - Get buffer capacity
//! - `len() -> usize` - Get number of elements in buffer
//! - `is_empty() -> bool` - Check if buffer is empty
//! - `is_full() -> bool` - Check if buffer is full
//! 
//! - `new(capacity) -> Self` - 创建新的环形缓冲区
//! - `push(value)` - 推送元素（返回类型取决于 `OVERWRITE` 标志）
//!   - `OVERWRITE=true`：返回 `Option<T>`（如果覆盖了元素则为 Some）
//!   - `OVERWRITE=false`：返回 `Result<(), RingBufError<T>>`
//! - `pop() -> Result<T, RingBufError<T>>` - 弹出单个元素
//! - `push_slice(&[T]) -> usize` - 批量推送多个元素（需要 `T: Copy`）
//! - `pop_slice(&mut [T]) -> usize` - 批量弹出多个元素（需要 `T: Copy`）
//! - `peek() -> Option<&T>` - 查看第一个元素但不移除
//! - `clear()` - 移除所有元素
//! - `capacity() -> usize` - 获取缓冲区容量
//! - `len() -> usize` - 获取缓冲区中的元素数量
//! - `is_empty() -> bool` - 检查缓冲区是否为空
//! - `is_full() -> bool` - 检查缓冲区是否已满
//! 
//! ## Batch Operations Example
//! 
//! 批量操作示例
//! 
//! Both modules support efficient batch operations for `Copy` types:
//! 
//! 两个模块都支持对 `Copy` 类型进行高效的批量操作：
//! 
//! ```rust
//! use smallring::spsc::new;
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
//! Generic module also supports batch operations:
//! 
//! Generic 模块也支持批量操作：
//! 
//! ```rust
//! use smallring::generic::RingBuf;
//! 
//! let buf: RingBuf<u32, 64, true> = RingBuf::new(32);
//! 
//! // Push multiple elements at once
//! // 一次推送多个元素
//! let data = [1, 2, 3, 4, 5];
//! let pushed = buf.push_slice(&data);
//! assert_eq!(pushed, 5);
//! 
//! // Pop multiple elements at once
//! // 一次弹出多个元素
//! let mut output = [0u32; 3];
//! let popped = buf.pop_slice(&mut output);
//! assert_eq!(popped, 3);
//! assert_eq!(output, [1, 2, 3]);
//! ```
//! 
//! ## Peek and Query Example
//! 
//! 窥视和查询示例
//! 
//! ### SPSC Module
//! 
//! SPSC 模块
//! 
//! ```rust
//! use smallring::spsc::new;
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
//! ### Generic Module
//! 
//! Generic 模块
//! 
//! ```rust
//! use smallring::generic::RingBuf;
//! 
//! let buf: RingBuf<i32, 32, false> = RingBuf::new(8);
//! 
//! buf.push(42).unwrap();
//! buf.push(100).unwrap();
//! 
//! // Check capacity and usage
//! // 检查容量和使用情况
//! assert_eq!(buf.capacity(), 8);
//! assert_eq!(buf.len(), 2);
//! assert!(!buf.is_full());
//! assert!(!buf.is_empty());
//! 
//! // Peek at first element
//! // 查看第一个元素
//! assert_eq!(buf.peek(), Some(&42));
//! 
//! // Pop and verify
//! // 弹出并验证
//! assert_eq!(buf.pop().unwrap(), 42);
//! assert_eq!(buf.len(), 1);
//! ```
//! 
//! ## Performance Tips
//! 
//! 性能提示
//! 
//! 1. **Choose appropriate capacity** - Capacity is automatically rounded up to power of 2 for efficient masking.
//!    Choose power-of-2 sizes to avoid wasted space.
//! 2. **Use batch operations** - `push_slice` and `pop_slice` are significantly faster than individual operations
//!    when working with `Copy` types.
//! 3. **Choose appropriate N** - Stack storage significantly improves performance for small buffers and eliminates
//!    heap allocation overhead. Common values: 32, 64, 128.
//! 4. **Use peek when needed** - Avoid pop + re-push patterns. Use `peek()` to inspect without consuming.
//! 5. **SPSC vs Generic** - Use SPSC module for cross-thread communication with optimal caching. Use Generic module
//!    when you need shared access or configurable overwrite behavior.
//! 6. **Avoid false sharing** - In multi-threaded scenarios, ensure producer and consumer are on different cache lines.
//! 
//! 1. **选择合适的容量** - 容量会自动向上取整到 2 的幂次以实现高效的掩码操作。
//!    选择 2 的幂次大小可避免浪费空间。
//! 2. **使用批量操作** - 在处理 `Copy` 类型时，`push_slice` 和 `pop_slice` 比单个操作快得多。
//! 3. **选择合适的 N** - 对于小缓冲区，栈存储可显著提升性能并消除堆分配开销。
//!    常用值：32、64、128。
//! 4. **在需要时使用 peek** - 避免 pop + 重新 push 的模式。使用 `peek()` 进行非消费性检查。
//! 5. **SPSC vs Generic** - 对于跨线程通信，使用 SPSC 模块以获得最佳缓存效果。
//!    需要共享访问或可配置覆盖行为时使用 Generic 模块。
//! 6. **避免伪共享** - 在多线程场景中，确保生产者和消费者位于不同的缓存行。
//! 
//! ## Important Notes
//! 
//! 重要注意事项
//! 
//! ### Common to Both Modules
//! 
//! 两个模块的共同特性
//! 
//! - **Capacity rounding** - All capacities are automatically rounded up to the nearest power of 2 for efficient
//!   masking operations.
//! - **Element lifecycle** - Elements are properly dropped when popped or when the buffer is cleaned up.
//! - **Memory layout** - Uses `MaybeUninit<T>` internally for safe uninitialized memory handling.
//! - **Power-of-2 optimization** - Fast modulo operations using bitwise AND instead of division.
//! 
//! - **容量取整** - 所有容量都会自动向上取整到最接近的 2 的幂次以实现高效的掩码操作。
//! - **元素生命周期** - 元素在弹出或缓冲区清理时会被正确地 drop。
//! - **内存布局** - 内部使用 `MaybeUninit<T>` 以安全地处理未初始化的内存。
//! - **2的幂次优化** - 使用按位与运算代替除法实现快速取模操作。
//! 
//! ### SPSC Module Specifics
//! 
//! SPSC 模块特性
//! 
//! - **Thread safety** - Designed specifically for Single Producer Single Consumer scenarios across threads.
//! - **Automatic cleanup** - `Consumer` automatically cleans up remaining elements when dropped.
//! - **Cached indices** - Producer and Consumer cache read/write indices for better performance.
//! - **No overwrite** - Always rejects writes when full; returns `PushError::Full`.
//! 
//! - **线程安全** - 专为跨线程的单生产者单消费者场景设计。
//! - **自动清理** - `Consumer` 在被 drop 时自动清理剩余元素。
//! - **缓存索引** - Producer 和 Consumer 缓存读写索引以提升性能。
//! - **无覆盖** - 满时总是拒绝写入；返回 `PushError::Full`。
//! 
//! ### Generic Module Specifics
//! 
//! Generic 模块特性
//! 
//! - **Flexible concurrency** - Can be shared across threads using `Arc` or used in single-threaded scenarios.
//! - **Configurable overwrite** - Compile-time `OVERWRITE` flag controls behavior when full:
//!   - `true`: Automatically overwrites oldest data (circular buffer semantics)
//!   - `false`: Rejects new writes and returns error
//! - **Manual cleanup** - Does NOT automatically clean up on drop. Call `clear()` explicitly if needed.
//! - **Zero-cost abstraction** - Overwrite behavior selected at compile time with no runtime overhead.
//! 
//! - **灵活的并发** - 可以通过 `Arc` 在线程间共享，或用于单线程场景。
//! - **可配置覆盖** - 编译期 `OVERWRITE` 标志控制满时的行为：
//!   - `true`：自动覆盖最旧的数据（循环缓冲区语义）
//!   - `false`：拒绝新写入并返回错误
//! - **手动清理** - 不会在 drop 时自动清理。需要时请显式调用 `clear()`。
//! - **零成本抽象** - 覆盖行为在编译期选择，无运行时开销。
//! 
//! ## Safety Guarantees
//! 
//! 安全保证
//! 
//! - **Type safety** - Full Rust type system guarantees; no undefined behavior.
//! - **Memory safety** - All unsafe code is carefully encapsulated and verified.
//! - **Thread safety** - `Send` and `Sync` implementations only where appropriate.
//! - **No data races** - Lock-free algorithms guarantee freedom from data races.
//! 
//! - **类型安全** - 完整的 Rust 类型系统保证；无未定义行为。
//! - **内存安全** - 所有 unsafe 代码都经过仔细封装和验证。
//! - **线程安全** - 仅在适当的情况下实现 `Send` 和 `Sync`。
//! - **无数据竞争** - 无锁算法保证无数据竞争。

// Public modules
// 公开模块
pub mod spsc;
pub mod generic;

// Internal modules
// 内部模块
mod vec;
mod core;

// Re-exports for convenience
// 便捷的重新导出
pub use spsc::new;

/// Prelude module with commonly used types
/// 
/// 包含常用类型的 prelude 模块
/// 
/// Import this module to get convenient access to the most commonly used types:
/// 
/// 导入此模块以便捷地访问最常用的类型：
/// 
/// ```rust
/// use smallring::prelude::*;
/// use std::num::NonZero;
/// 
/// let (mut prod, mut cons) = new::<i32, 32>(NonZero::new(8).unwrap());
/// let buf: RingBuf<i32, 32, true> = RingBuf::new(8);
/// ```
pub mod prelude {
    pub use crate::spsc::{new, Producer, Consumer, PushError, PopError};
    pub use crate::generic::{RingBuf, RingBufError};
}