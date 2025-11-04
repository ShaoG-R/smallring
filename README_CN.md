# smallring

[![Crates.io](https://img.shields.io/crates/v/smallring.svg)](https://crates.io/crates/smallring)
[![Documentation](https://docs.rs/smallring/badge.svg)](https://docs.rs/smallring)
[![License](https://img.shields.io/crates/l/smallring.svg)](https://github.com/ShaoG-R/smallring#license)

[English](README.md) | [简体中文](README_CN.md)

高性能的单生产者单消费者（SPSC）无锁环形缓冲区实现，具有自动栈/堆优化能力。

## 特性

- **无锁设计** - 使用原子操作实现线程安全的无锁通信，无需互斥锁
- **栈/堆优化** - 小容量数据自动存储在栈上，提升性能
- **高性能** - 针对 SPSC 场景优化，最小化原子操作开销
- **类型安全** - 完整的 Rust 类型系统保证，编译期检查
- **零拷贝** - 数据直接移动，无额外拷贝开销
- **自动清理** - Consumer 被 drop 时自动清理缓冲区中的剩余元素

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
smallring = "0.1.0"
```

## 快速开始

```rust
use smallring::new;

// 创建一个容量为 8 的环形缓冲区，栈容量阈值为 32
let (mut producer, mut consumer) = new::<i32, 32>(8);

// 生产者推送数据
producer.push(42).unwrap();
producer.push(100).unwrap();

// 消费者获取数据
assert_eq!(consumer.pop().unwrap(), 42);
assert_eq!(consumer.pop().unwrap(), 100);
```

## 使用示例

### 基础单线程使用

```rust
use smallring::new;

fn main() {
    let (mut producer, mut consumer) = new::<String, 64>(16);
    
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

### 多线程通信

```rust
use smallring::new;
use std::thread;

fn main() {
    let (mut producer, mut consumer) = new::<String, 64>(32);
    
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

### 错误处理

```rust
use smallring::{new, PushError, PopError};

let (mut producer, mut consumer) = new::<i32, 32>(4);

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

## 栈/堆优化

`smallring` 使用泛型常量 `N` 来控制栈/堆优化的阈值：

```rust
use smallring::new;

// 容量 ≤ 32，使用栈存储（更快的初始化，无堆分配）
let (prod, cons) = new::<u64, 32>(16);

// 容量 > 32，使用堆存储（适用于更大的缓冲区）
let (prod, cons) = new::<u64, 32>(64);

// 更大的栈阈值可用于更大的栈存储
let (prod, cons) = new::<u64, 128>(100);

// 非常大的缓冲区
let (prod, cons) = new::<u64, 256>(200);
```

**使用指南：**
- 小缓冲区（≤32 个元素）：使用 `N=32` 以获得最佳性能
- 中等缓冲区（≤128 个元素）：使用 `N=128` 以避免堆分配
- 大缓冲区（>128 个元素）：自动使用堆分配
- 栈存储可显著提升 `new()` 的性能并减少内存分配器压力

## API 概览

### 创建环形缓冲区

```rust
pub fn new<T, const N: usize>(capacity: usize) -> (Producer<T, N>, Consumer<T, N>)
```

创建指定容量的新环形缓冲区。容量会自动向上取整到下一个 2 的幂次。

### Producer 方法

- `push(&mut self, value: T) -> Result<(), PushError<T>>` - 向缓冲区推送一个值
- 如果缓冲区满则返回 `Err(PushError::Full(value))`

### Consumer 方法

- `pop(&mut self) -> Result<T, PopError>` - 从缓冲区弹出一个值
- `is_empty(&self) -> bool` - 检查缓冲区是否为空
- `slots(&self) -> usize` - 获取缓冲区中当前的元素数量

## 性能考虑

### 容量选择

容量会自动向上取整到最接近的 2 的幂次以实现高效的掩码操作：

```rust
// 请求容量 → 实际容量
// 5 → 8
// 10 → 16
// 30 → 32
// 100 → 128
```

**建议：** 选择 2 的幂次作为容量以避免空间浪费。

### 批量操作

为获得最大吞吐量，尽可能批量进行 push/pop 操作：

```rust
// 效率较低 - 多次小规模推送
for i in 0..1000 {
    while producer.push(i).is_err() {
        thread::yield_now();
    }
}

// 效率更高 - 当缓冲区有空间时批量处理
let mut batch = vec![];
for i in 0..1000 {
    if let Err(PushError::Full(val)) = producer.push(i) {
        // 缓冲区满，等待空间
        thread::yield_now();
        // 重试这个值
        while producer.push(val).is_err() {
            thread::yield_now();
        }
    }
}
```

### 选择 N（栈阈值）

- **小 N（32）：** 最小的栈使用，适用于大多数情况
- **中等 N（128）：** 中等大小缓冲区的良好平衡
- **大 N（256+）：** 适合栈的大缓冲区可获得最大性能

栈分配比堆分配在缓冲区初始化时快得多。

## 线程安全

- **仅限 SPSC：** `smallring` 专为单生产者单消费者场景设计
- `Producer` 和 `Consumer` **不是** `Sync`，确保单线程访问
- `Producer` 和 `Consumer` 是 `Send`，允许在线程间移动
- 原子操作确保生产者和消费者线程之间的内存顺序保证

## 重要说明

1. **容量向上取整：** 请求的容量会向上取整到下一个 2 的幂次
2. **仅限 SPSC：** 不适用于 MPSC、SPMC 或 MPMC 场景
3. **自动清理：** `Consumer` 被 drop 时，缓冲区中的所有剩余元素都会自动 drop
4. **非阻塞：** `push` 和 `pop` 永不阻塞；缓冲区满/空时返回错误

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

