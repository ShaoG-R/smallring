# smallring

[![Crates.io](https://img.shields.io/crates/v/smallring.svg)](https://crates.io/crates/smallring)
[![Documentation](https://docs.rs/smallring/badge.svg)](https://docs.rs/smallring)
[![License](https://img.shields.io/crates/l/smallring.svg)](https://github.com/ShaoG-R/smallring#license)

[English](README.md) | [简体中文](README_CN.md)

高性能无锁环形缓冲区实现集合，具有自动栈/堆优化能力。提供三个专门化模块：**Generic** 用于通用缓冲区、**Atomic** 用于原子类型、**SPSC** 用于跨线程通信。

## 特性

- **无锁设计** - 使用原子原语实现线程安全的操作，无需互斥锁
- **三个专门化模块** - Generic 用于共享访问，Atomic 用于原子类型，SPSC 用于跨线程通信
- **栈/堆优化** - 小缓冲区自动使用栈存储以获得更好的性能
- **高性能** - 通过最小化原子操作开销和高效的掩码操作进行优化
- **类型安全** - 完整的 Rust 类型系统保证，编译期检查
- **零拷贝** - 数据直接移动，无额外拷贝开销
- **可配置覆盖** - Generic 模块支持编译期覆盖模式选择
- **2的幂次容量** - 自动向上取整以实现高效的取模操作

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
smallring = "0.1"
```

## 快速开始

### Generic 模块 - 通用环形缓冲区

```rust
use smallring::generic::RingBuf;

// 覆盖模式：满时自动覆盖最旧的数据
let buf: RingBuf<i32, 32, true> = RingBuf::new(4);
buf.push(1); // 返回 None
buf.push(2);
buf.push(3);
buf.push(4);
buf.push(5); // 返回 Some(1)，覆盖了最旧的元素

// 非覆盖模式：满时拒绝写入
let buf: RingBuf<i32, 32, false> = RingBuf::new(4);
buf.push(1).unwrap(); // 返回 Ok(())
buf.push(2).unwrap();
buf.push(3).unwrap();
buf.push(4).unwrap();
assert!(buf.push(5).is_err()); // 返回 Err(Full(5))
```

### Atomic 模块 - 原子类型专用

```rust
use smallring::atomic::AtomicRingBuf;
use std::sync::atomic::{AtomicU64, Ordering};

// 创建原子值的环形缓冲区
let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);

// 推送和弹出原子值
buf.push(42, Ordering::Relaxed);
buf.push(100, Ordering::Relaxed);

assert_eq!(buf.pop(Ordering::Acquire), Some(42));
assert_eq!(buf.pop(Ordering::Acquire), Some(100));
```

### SPSC 模块 - 跨线程通信

```rust
use smallring::spsc::new;
use std::num::NonZero;

// 创建一个容量为 8 的环形缓冲区，栈容量阈值为 32
let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(8).unwrap());

// 生产者推送数据
producer.push(42).unwrap();
producer.push(100).unwrap();

// 消费者获取数据
assert_eq!(consumer.pop().unwrap(), 42);
assert_eq!(consumer.pop().unwrap(), 100);
```

## 使用示例

### Generic 模块示例

#### 基础单线程使用

```rust
use smallring::generic::RingBuf;

fn main() {
    let mut buf: RingBuf<String, 64, false> = RingBuf::new(16);
    
    // 推送一些数据
    buf.push("你好".to_string()).unwrap();
    buf.push("世界".to_string()).unwrap();
    
    // 按顺序弹出数据
    println!("{}", buf.pop().unwrap()); // "你好"
    println!("{}", buf.pop().unwrap()); // "世界"
    
    // 检查是否为空
    assert!(buf.is_empty());
}
```

#### 多线程共享访问

```rust
use smallring::generic::RingBuf;
use std::sync::Arc;
use std::thread;

fn main() {
    // 覆盖模式对于并发写入者是线程安全的
    let buf = Arc::new(RingBuf::<u64, 128, true>::new(128));
    let mut handles = vec![];
    
    // 多个写入线程
    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let value = (thread_id * 100 + i) as u64;
                buf_clone.push(value); // 自动覆盖旧数据
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

#### 错误处理

```rust
use smallring::generic::{RingBuf, RingBufError};

// 非覆盖模式
let buf: RingBuf<i32, 32, false> = RingBuf::new(4);

// 填满缓冲区
for i in 0..4 {
    buf.push(i).unwrap();
}

// 缓冲区已满 - push 返回错误及值
match buf.push(99) {
    Err(RingBufError::Full(value)) => {
        println!("缓冲区已满，无法推送 {}", value);
    }
    Ok(_) => {}
}

// 清空缓冲区
while buf.pop().is_ok() {}

// 缓冲区为空 - pop 返回错误
match buf.pop() {
    Err(RingBufError::Empty) => {
        println!("缓冲区为空");
    }
    Ok(_) => {}
}
```

### Atomic 模块示例

#### 基础原子操作

```rust
use smallring::atomic::AtomicRingBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn main() {
    let buf: AtomicRingBuf<AtomicU64, 32> = AtomicRingBuf::new(8);
    
    // 推送原子值
    buf.push(42, Ordering::Relaxed);
    buf.push(100, Ordering::Relaxed);
    
    // 弹出原子值
    assert_eq!(buf.pop(Ordering::Acquire), Some(42));
    assert_eq!(buf.pop(Ordering::Acquire), Some(100));
    
    // 检查是否为空
    assert!(buf.is_empty());
}
```

#### 共享原子计数器

```rust
use smallring::atomic::AtomicRingBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    let buf = Arc::new(AtomicRingBuf::<AtomicU64, 64>::new(32));
    let mut handles = vec![];
    
    // 多个线程推送原子值
    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let value = (thread_id * 50 + i) as u64;
                buf_clone.push(value, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

### SPSC 模块示例

#### 基础单线程使用

```rust
use smallring::spsc::new;
use std::num::NonZero;

fn main() {
    let (mut producer, mut consumer) = new::<String, 64>(NonZero::new(16).unwrap());
    
    // 推送一些数据
    producer.push("你好".to_string()).unwrap();
    producer.push("世界".to_string()).unwrap();
    
    // 按顺序弹出数据
    println!("{}", consumer.pop().unwrap()); // "你好"
    println!("{}", consumer.pop().unwrap()); // "世界"
    
    // 检查是否为空
    assert!(consumer.is_empty());
}
```

#### 多线程通信

```rust
use smallring::spsc::new;
use std::thread;
use std::num::NonZero;

fn main() {
    let (mut producer, mut consumer) = new::<String, 64>(NonZero::new(32).unwrap());
    
    // 生产者线程
    let producer_handle = thread::spawn(move || {
        for i in 0..100 {
            let msg = format!("消息 {}", i);
            while producer.push(msg.clone()).is_err() {
                thread::yield_now();
            }
        }
    });
    
    // 消费者线程
    let consumer_handle = thread::spawn(move || {
        let mut received = Vec::new();
        for _ in 0..100 {
            loop {
                match consumer.pop() {
                    Ok(msg) => {
                        received.push(msg);
                        break;
                    }
                    Err(_) => thread::yield_now(),
                }
            }
        }
        received
    });
    
    producer_handle.join().unwrap();
    let messages = consumer_handle.join().unwrap();
    assert_eq!(messages.len(), 100);
}
```

#### 错误处理

```rust
use smallring::spsc::{new, PushError, PopError};
use std::num::NonZero;

let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());

// 填满缓冲区
for i in 0..4 {
    producer.push(i).unwrap();
}

// 缓冲区已满 - push 返回错误及值
match producer.push(99) {
    Err(PushError::Full(value)) => {
        println!("缓冲区已满，无法推送 {}", value);
    }
    Ok(_) => {}
}

// 清空缓冲区
while consumer.pop().is_ok() {}

// 缓冲区为空 - pop 返回错误
match consumer.pop() {
    Err(PopError::Empty) => {
        println!("缓冲区为空");
    }
    Ok(_) => {}
}
```

#### 批量操作

```rust
use smallring::spsc::new;
use std::num::NonZero;

let (mut producer, mut consumer) = new::<u32, 64>(NonZero::new(32).unwrap());

// 一次推送多个元素（需要 T: Copy）
let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
let pushed = producer.push_slice(&data);
assert_eq!(pushed, 10);

// 一次弹出多个元素
let mut output = [0u32; 5];
let popped = consumer.pop_slice(&mut output);
assert_eq!(popped, 5);
assert_eq!(output, [1, 2, 3, 4, 5]);

// 清空剩余元素
let remaining: Vec<u32> = consumer.drain().collect();
assert_eq!(remaining, vec![6, 7, 8, 9, 10]);
```

## 模块对比

| 特性 | Generic | Atomic | SPSC |
|------|---------|--------|------|
| **使用场景** | 通用、共享访问 | 仅原子类型 | 跨线程通信 |
| **元素类型** | 任意类型 `T` | AtomicU8、AtomicU64 等 | 任意类型 `T` |
| **句柄** | 单个共享 `RingBuf` | 单个共享 `AtomicRingBuf` | 分离的 `Producer`/`Consumer` |
| **覆盖行为** | 编译期可配置 | 总是覆盖 | 总是拒绝满时写入 |
| **并发性** | 多个读写者 | 多个读写者 | 单生产者、单消费者 |
| **缓存优化** | 直接原子访问 | 直接原子访问 | 缓存的读写索引 |
| **Drop 行为** | 需手动调用 `clear()` | 需手动调用 `clear()` | Consumer 自动清理 |

**选择 Generic 当：**
- 你需要通用环形缓冲区支持任意元素类型
- 你需要编译期可配置的覆盖行为
- 你需要从单线程或 `Arc` 中进行共享访问

**选择 Atomic 当：**
- 你仅使用原子类型（AtomicU64、AtomicI32 等）
- 你需要存储原子值而不移动它们
- 你在构建共享计数器或指标

**选择 SPSC 当：**
- 你需要跨线程通信，具有分离的生产者/消费者角色
- 你希望 Consumer drop 时自动清理
- 性能至关重要，你可以利用缓存的索引

## 栈/堆优化

所有三个模块都使用泛型常量 `N` 来控制栈/堆优化的阈值。当容量 ≤ N 时，数据存储在栈上；否则，在堆上分配。

```rust
use smallring::spsc::new;
use smallring::generic::RingBuf;
use smallring::atomic::AtomicRingBuf;
use std::sync::atomic::AtomicU64;
use std::num::NonZero;

// SPSC：容量 ≤ 32，使用栈存储（更快的初始化，无堆分配）
let (prod, cons) = new::<u64, 32>(NonZero::new(16).unwrap());

// SPSC：容量 > 32，使用堆存储（适用于更大的缓冲区）
let (prod, cons) = new::<u64, 32>(NonZero::new(64).unwrap());

// Generic：更大的栈阈值可用于更大的栈存储
let buf: RingBuf<u64, 128, true> = RingBuf::new(100);

// Atomic：原子类型的栈阈值
let atomic_buf: AtomicRingBuf<AtomicU64, 64> = AtomicRingBuf::new(32);
```

**使用指南：**
- 小缓冲区（≤32 个元素）：使用 `N=32` 以获得最佳性能
- 中等缓冲区（≤128 个元素）：使用 `N=128` 以避免堆分配
- 大缓冲区（>128 个元素）：自动使用堆分配
- 栈存储可显著提升 `new()` 的性能并减少内存分配器压力

## API 概览

### Generic 模块

**创建环形缓冲区：**
```rust
pub fn new<T, const N: usize, const OVERWRITE: bool>(capacity: usize) -> RingBuf<T, N, OVERWRITE>
```

**RingBuf 方法：**
- `push(&mut self, value: T)` - 推送元素（返回类型取决于 `OVERWRITE` 标志）
  - `OVERWRITE=true`：返回 `Option<T>`（如果覆盖了元素则为 Some）
  - `OVERWRITE=false`：返回 `Result<(), RingBufError<T>>`
- `pop(&mut self) -> Result<T, RingBufError<T>>` - 弹出单个元素
- `push_slice(&mut self, values: &[T]) -> usize` - 批量推送多个元素（需要 `T: Copy`）
- `pop_slice(&mut self, dest: &mut [T]) -> usize` - 批量弹出多个元素（需要 `T: Copy`）
- `peek(&self) -> Option<&T>` - 查看第一个元素但不移除
- `clear(&mut self)` - 移除所有元素
- `as_slices(&self) -> (&[T], &[T])` - 获取可读数据的连续切片
- `as_mut_slices(&mut self) -> (&mut [T], &mut [T])` - 获取可读数据的可变连续切片
- `iter(&self) -> Iter<'_, T>` - 创建元素迭代器
- `iter_mut(&mut self) -> IterMut<'_, T>` - 创建可变元素迭代器
- `capacity() -> usize` - 获取缓冲区容量
- `len() -> usize` - 获取缓冲区中的元素数量
- `is_empty() -> bool` - 检查缓冲区是否为空
- `is_full() -> bool` - 检查缓冲区是否已满

### Atomic 模块

**创建环形缓冲区：**
```rust
pub fn new<E: AtomicElement, const N: usize>(capacity: usize) -> AtomicRingBuf<E, N>
```

**AtomicRingBuf 方法：**
- `push(&self, value: E::Primitive, order: Ordering)` - 推送原子值
- `pop(&self, order: Ordering) -> Option<E::Primitive>` - 弹出原子值
- `peek(&self, order: Ordering) -> Option<E::Primitive>` - 查看第一个元素但不移除
- `clear(&mut self)` - 移除所有元素
- `capacity() -> usize` - 获取缓冲区容量
- `len(&self, order: Ordering) -> usize` - 获取缓冲区中的元素数量
- `is_empty(&self, order: Ordering) -> bool` - 检查缓冲区是否为空
- `is_full(&self, order: Ordering) -> bool` - 检查缓冲区是否已满

**支持的原子类型：**
- `AtomicU8`、`AtomicU16`、`AtomicU32`、`AtomicU64`、`AtomicUsize`
- `AtomicI8`、`AtomicI16`、`AtomicI32`、`AtomicI64`、`AtomicIsize`
- `AtomicBool`

### SPSC 模块

**创建环形缓冲区：**
```rust
pub fn new<T, const N: usize>(capacity: NonZero<usize>) -> (Producer<T, N>, Consumer<T, N>)
```

**Producer 方法：**
- `push(&mut self, value: T) -> Result<(), PushError<T>>` - 推送单个元素
- `push_slice(&mut self, values: &[T]) -> usize` - 批量推送多个元素（需要 `T: Copy`）
- `capacity() -> usize` - 获取缓冲区容量
- `len() / slots() -> usize` - 获取缓冲区中的元素数量
- `free_slots() -> usize` - 获取可用空间
- `is_full() -> bool` - 检查缓冲区是否已满
- `is_empty() -> bool` - 检查缓冲区是否为空

**Consumer 方法：**
- `pop(&mut self) -> Result<T, PopError>` - 弹出单个元素
- `pop_slice(&mut self, dest: &mut [T]) -> usize` - 批量弹出多个元素（需要 `T: Copy`）
- `peek(&self) -> Option<&T>` - 查看第一个元素但不移除
- `drain(&mut self) -> Drain<'_, T, N>` - 创建消费迭代器
- `clear(&mut self)` - 移除所有元素
- `capacity() -> usize` - 获取缓冲区容量
- `len() / slots() -> usize` - 获取缓冲区中的元素数量
- `is_empty() -> bool` - 检查缓冲区是否为空

## 性能提示

1. **选择合适的容量** - 容量会自动向上取整到 2 的幂次以实现高效的掩码操作。选择 2 的幂次大小可避免浪费空间。
2. **使用批量操作** - 在处理 `Copy` 类型时，`push_slice` 和 `pop_slice` 比单个操作快得多。
3. **选择合适的 N** - 对于小缓冲区，栈存储可显著提升性能并消除堆分配开销。常用值：32、64、128。
4. **在需要时使用 peek** - 避免 pop + 重新 push 的模式。使用 `peek()` 进行非消费性检查。
5. **SPSC vs Generic** - 对于跨线程通信，使用 SPSC 模块以获得最佳缓存效果。需要共享访问或可配置覆盖行为时使用 Generic 模块。
6. **避免伪共享** - 在多线程场景中，确保生产者和消费者位于不同的缓存行。

### 容量选择

容量会自动向上取整到最接近的 2 的幂次：

```rust
// 请求容量 → 实际容量
// 5 → 8
// 10 → 16
// 30 → 32
// 100 → 128
```

**建议：** 选择 2 的幂次作为容量以避免空间浪费。

## 线程安全

### Generic 模块

- 当 `T` 是 `Send` 时，`RingBuf` 是 `Send` 和 `Sync`
- 可以通过 `Arc` 在线程间共享
- 并发操作（多个写入者或读取者）是线程安全的
- 适用于单线程和多线程场景

### Atomic 模块

- `AtomicRingBuf` 对所有支持的原子类型都是 `Send` 和 `Sync`
- 专为多线程间的共享访问设计
- 所有操作使用原子 load/store 和指定的内存顺序
- 完美用于构建线程安全的指标和计数器

### SPSC 模块

- 专为跨线程的单生产者单消费者场景设计
- `Producer` 和 `Consumer` **不是** `Sync`，确保单线程访问
- `Producer` 和 `Consumer` 是 `Send`，允许在线程间移动
- 原子操作确保生产者和消费者线程之间的内存顺序保证

## 重要说明

### 所有模块的共同特性

- **容量取整** - 所有容量都会自动向上取整到最接近的 2 的幂次以实现高效的掩码操作
- **元素生命周期** - 元素在弹出或缓冲区清理时会被正确地 drop
- **内存布局** - 内部使用 `MaybeUninit<T>` 以安全地处理未初始化的内存
- **2的幂次优化** - 使用按位与运算代替除法实现快速取模操作

### Generic 模块特性

- **灵活的并发** - 可以通过 `Arc` 在线程间共享，或用于单线程场景
- **可配置覆盖** - 编译期 `OVERWRITE` 标志控制满时的行为：
  - `true`：自动覆盖最旧的数据（循环缓冲区语义）
  - `false`：拒绝新写入并返回错误
- **手动清理** - 不会在 drop 时自动清理。需要时请显式调用 `clear()`
- **零成本抽象** - 覆盖行为在编译期选择，无运行时开销

### Atomic 模块特性

- **原子操作** - 所有操作使用原子原语而不移动值
- **内存顺序** - 每个操作接受 `Ordering` 参数以实现细粒度控制
- **类型安全** - `AtomicElement` trait 确保仅支持有效的原子类型
- **手动清理** - 不会在 drop 时自动清理。需要时请显式调用 `clear()`

### SPSC 模块特性

- **线程安全** - 专为跨线程的单生产者单消费者场景设计
- **自动清理** - `Consumer` 在被 drop 时自动清理剩余元素
- **缓存索引** - Producer 和 Consumer 缓存读写索引以提升性能
- **无覆盖** - 满时总是拒绝写入；返回 `PushError::Full`

## 性能基准

性能特征（近似值，取决于系统）：

- **栈分配**（`capacity ≤ N`）：每次 `new()` 调用约 1-2 纳秒
- **堆分配**（`capacity > N`）：每次 `new()` 调用约 50-100 纳秒
- **Push/Pop 操作**：在 SPSC 场景下每次操作约 5-15 纳秒
- **吞吐量**：在现代硬件上可达每秒 2 亿+ 次操作

## 最低支持的 Rust 版本（MSRV）

由于使用了 const generics 特性，需要 Rust 1.87 或更高版本。

## 许可证

可选以下任一许可证：

- Apache 许可证 2.0 版本（[LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0）
- MIT 许可证（[LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT）

由您选择。

## 贡献

欢迎贡献！请随时提交 Pull Request。

### 贡献指南

- 遵循 Rust 编码规范
- 为新功能添加测试
- 根据需要更新文档
- 确保 `cargo test` 通过
- 提交前运行 `cargo fmt`

## 致谢

受 Rust 生态系统中各种环形缓冲区实现的启发，专注于简单性、性能和自动栈/堆优化。

## 相关项目

- [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam)：通用并发通道
- [ringbuf](https://github.com/agerasev/ringbuf)：另一个 SPSC 环形缓冲区实现
- [rtrb](https://github.com/mgeier/rtrb)：实时安全的 SPSC 环形缓冲区

## 支持

- 文档：[docs.rs/smallring](https://docs.rs/smallring)
- 仓库：[github.com/ShaoG-R/smallring](https://github.com/ShaoG-R/smallring)
- 问题反馈：[github.com/ShaoG-R/smallring/issues](https://github.com/ShaoG-R/smallring/issues)

