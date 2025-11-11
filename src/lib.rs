//! SmallRing - High-performance lock-free ring buffers
//! 
//! SmallRing - 高性能无锁环形缓冲区
//! 
//! This library provides a collection of high-performance, lock-free ring buffer implementations
//! optimized for different use cases. All implementations feature:
//! 
//! 本库提供了一系列针对不同用例优化的高性能无锁环形缓冲区实现。所有实现都具有以下特性：
//! 
//! ## Features / 特性
//! 
//! - **Lock-free**: All operations are lock-free and suitable for concurrent use
//! - **Stack/Heap optimization**: Small buffers use stack allocation to avoid heap overhead
//! - **Power-of-2 capacity**: Automatic capacity rounding for efficient masking operations
//! - **Zero-cost abstractions**: Compile-time optimizations for different modes
//! - **Memory safety**: Safe Rust API with unsafe internals carefully managed
//! 
//! - **无锁**: 所有操作都是无锁的，适合并发使用
//! - **栈/堆优化**: 小缓冲区使用栈分配以避免堆开销
//! - **2的幂次容量**: 自动容量取整以实现高效的掩码操作
//! - **零成本抽象**: 针对不同模式的编译期优化
//! - **内存安全**: 安全的 Rust API，内部 unsafe 代码得到谨慎管理
//! 
//! ## Modules / 模块
//! 
//! ### [`generic`] - Generic Ring Buffer
//! 
//! A versatile ring buffer that supports any element type with configurable overwrite behavior.
//! Ideal for general-purpose use cases where you need to store arbitrary data types.
//! 
//! 通用环形缓冲区，支持任意元素类型和可配置的覆盖行为。
//! 适用于需要存储任意数据类型的通用场景。
//! 
//! ### [`atomic`] - Atomic Type Ring Buffer
//! 
//! Specialized for atomic types (AtomicU64, AtomicI32, etc.). Operates through atomic
//! load/store operations without moving values, perfect for shared counters and metrics.
//! 
//! 专门针对原子类型（AtomicU64、AtomicI32 等）优化。通过原子 load/store 操作运行而不移动值，
//! 非常适合共享计数器和指标。
//! 
//! ### [`spsc`] - Single Producer Single Consumer (SPSC)
//! 
//! High-performance SPSC ring buffer with separate producer and consumer handles.
//! Optimized for scenarios with a single producer thread and a single consumer thread.
//! 
//! 高性能 SPSC 环形缓冲区，具有独立的生产者和消费者句柄。
//! 针对单生产者单消费者线程场景优化。
//! 
//! ## Usage Examples / 使用示例
//! 
//! ```rust
//! use smallring::generic::RingBuf;
//! 
//! // Generic ring buffer with overwrite mode
//! // 带覆盖模式的通用环形缓冲区
//! let mut buf: RingBuf<i32, 32, true> = RingBuf::new(4);
//! buf.push(1);
//! buf.push(2);
//! assert_eq!(buf.pop().unwrap(), 1);
//! 
//! // Atomic ring buffer for shared counters
//! // 用于共享计数器的原子环形缓冲区
//! use smallring::atomic::AtomicRingBuf;
//! use std::sync::atomic::{AtomicU64, Ordering};
//! 
//! let atomic_buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
//! atomic_buf.push(42, Ordering::Relaxed);
//! assert_eq!(atomic_buf.pop(Ordering::Acquire), Some(42));
//! 
//! // SPSC ring buffer for thread-safe communication
//! // 用于线程安全通信的 SPSC 环形缓冲区
//! use smallring::spsc::new;
//! use std::num::NonZero;
//! use std::thread;
//! 
//! let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(10).unwrap());
//! 
//! thread::spawn(move || {
//!     producer.push(1).unwrap();
//!     producer.push(2).unwrap();
//! });
//! 
//! thread::spawn(move || {
//!     assert_eq!(consumer.pop().unwrap(), 1);
//!     assert_eq!(consumer.pop().unwrap(), 2);
//! });
//! ```
//! 
//! ## Performance / 性能
//! 
//! All implementations are designed for high performance with:
//! - O(1) push and pop operations
//! - Minimal memory overhead
//! - Cache-friendly access patterns
//! - No dynamic allocation after initialization (for appropriate capacities)
//! 
//! 所有实现都为高性能而设计，具有：
//! - O(1) 的推送和弹出操作
//! - 最小的内存开销
//! - 缓存友好的访问模式
//! - 初始化后无动态分配（对于适当的容量）
//! 
//! ## Safety / 安全性
//! 
//! This library uses unsafe code internally for performance, but provides a fully safe API.
//! All unsafe blocks are carefully documented and reviewed for correctness.
//! 
//! 本库内部使用 unsafe 代码以提升性能，但提供完全安全的 API。
//! 所有 unsafe 块都经过仔细文档化和正确性审查。

// Public modules
// 公开模块
pub mod spsc;
pub mod generic;
pub mod atomic;

// Internal modules
// 内部模块
mod vec;
mod core;

#[cfg(test)]
mod tests;