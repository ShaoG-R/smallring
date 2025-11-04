# smallring

[![Crates.io](https://img.shields.io/crates/v/smallring.svg)](https://crates.io/crates/smallring)
[![Documentation](https://docs.rs/smallring/badge.svg)](https://docs.rs/smallring)
[![License](https://img.shields.io/crates/l/smallring.svg)](https://github.com/ShaoG-R/smallring#license)

[English](README.md) | [简体中文](README_CN.md)

A high-performance lock-free Single Producer Single Consumer (SPSC) ring buffer implementation with automatic stack/heap optimization.

## Features

- **Lock-Free** - Thread-safe communication using atomic operations without mutexes
- **Stack/Heap Optimization** - Small buffers automatically use stack storage for better performance
- **High Performance** - Optimized for SPSC scenarios with minimal atomic overhead
- **Type Safe** - Full Rust type system guarantees with compile-time checks
- **Zero Copy** - Data is moved directly without extra copying
- **Automatic Cleanup** - Remaining elements are automatically dropped when Consumer is dropped

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
smallring = "0.1.0"
```

## Quick Start

```rust
use smallring::new;

// Create a ring buffer with capacity 8, stack threshold 32
let (mut producer, mut consumer) = new::<i32, 32>(8);

// Producer pushes data
producer.push(42).unwrap();
producer.push(100).unwrap();

// Consumer pops data
assert_eq!(consumer.pop().unwrap(), 42);
assert_eq!(consumer.pop().unwrap(), 100);
```

## Usage Examples

### Basic Single-Threaded Usage

```rust
use smallring::new;

fn main() {
    let (mut producer, mut consumer) = new::<String, 64>(16);
    
    // Push some data
    producer.push("Hello".to_string()).unwrap();
    producer.push("World".to_string()).unwrap();
    
    // Pop data in order
    println!("{}", consumer.pop().unwrap()); // "Hello"
    println!("{}", consumer.pop().unwrap()); // "World"
    
    // Check if empty
    assert!(consumer.is_empty());
}
```

### Multi-threaded Communication

```rust
use smallring::new;
use std::thread;

fn main() {
    let (mut producer, mut consumer) = new::<String, 64>(32);
    
    // Producer thread
    let producer_handle = thread::spawn(move || {
        for i in 0..100 {
            let msg = format!("Message {}", i);
            while producer.push(msg.clone()).is_err() {
                thread::yield_now();
            }
        }
    });
    
    // Consumer thread
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

### Error Handling

```rust
use smallring::{new, PushError, PopError};

let (mut producer, mut consumer) = new::<i32, 32>(4);

// Fill the buffer
for i in 0..4 {
    producer.push(i).unwrap();
}

// Buffer is full - push returns error with value
match producer.push(99) {
    Err(PushError::Full(value)) => {
        println!("Buffer full, couldn't push {}", value);
    }
    Ok(_) => {}
}

// Empty the buffer
while consumer.pop().is_ok() {}

// Buffer is empty - pop returns error
match consumer.pop() {
    Err(PopError::Empty) => {
        println!("Buffer is empty");
    }
    Ok(_) => {}
}
```

## Stack/Heap Optimization

`smallring` uses generic constant `N` to control the stack/heap optimization threshold:

```rust
use smallring::new;

// Capacity ≤ 32, uses stack storage (faster initialization, no heap allocation)
let (prod, cons) = new::<u64, 32>(16);

// Capacity > 32, uses heap storage (suitable for larger buffers)
let (prod, cons) = new::<u64, 32>(64);

// Larger stack threshold for more stack storage
let (prod, cons) = new::<u64, 128>(100);

// Very large buffers
let (prod, cons) = new::<u64, 256>(200);
```

**Guidelines:**
- For small buffers (≤32 elements): use `N=32` for optimal performance
- For medium buffers (≤128 elements): use `N=128` to avoid heap allocation
- For large buffers (>128 elements): heap allocation is used automatically
- Stack storage significantly improves `new()` performance and reduces memory allocator pressure

## API Overview

### Creating a Ring Buffer

```rust
pub fn new<T, const N: usize>(capacity: usize) -> (Producer<T, N>, Consumer<T, N>)
```

Creates a new ring buffer with the specified capacity. Capacity is automatically rounded up to the next power of 2.

### Producer Methods

- `push(&mut self, value: T) -> Result<(), PushError<T>>` - Push a value into the buffer
- Returns `Err(PushError::Full(value))` if buffer is full

### Consumer Methods

- `pop(&mut self) -> Result<T, PopError>` - Pop a value from the buffer
- `is_empty(&self) -> bool` - Check if the buffer is empty
- `slots(&self) -> usize` - Get the number of elements currently in the buffer

## Performance Considerations

### Capacity Selection

Capacity is automatically rounded up to the nearest power of 2 for efficient masking operations:

```rust
// Requested capacity → Actual capacity
// 5 → 8
// 10 → 16
// 30 → 32
// 100 → 128
```

**Recommendation:** Choose power-of-2 capacities to avoid wasted space.

### Batching Operations

For maximum throughput, batch push/pop operations when possible:

```rust
// Less efficient - many small pushes
for i in 0..1000 {
    while producer.push(i).is_err() {
        thread::yield_now();
    }
}

// More efficient - batch when buffer has space
let mut batch = vec![];
for i in 0..1000 {
    if let Err(PushError::Full(val)) = producer.push(i) {
        // Buffer full, wait for space
        thread::yield_now();
        // Retry this value
        while producer.push(val).is_err() {
            thread::yield_now();
        }
    }
}
```

### Choosing N (Stack Threshold)

- **Small N (32):** Minimal stack usage, suitable for most cases
- **Medium N (128):** Good balance for medium-sized buffers
- **Large N (256+):** Maximum performance for large buffers that fit in stack

Stack allocation is significantly faster than heap allocation for buffer initialization.

## Thread Safety

- **SPSC Only:** `smallring` is designed specifically for Single Producer Single Consumer scenarios
- `Producer` and `Consumer` are **not** `Sync`, ensuring single-threaded access
- `Producer` and `Consumer` are `Send`, allowing them to be moved between threads
- Atomic operations ensure memory ordering guarantees between producer and consumer threads

## Important Notes

1. **Capacity Rounding:** Requested capacity is rounded up to the next power of 2
2. **SPSC Only:** Not suitable for MPSC, SPMC, or MPMC scenarios
3. **Automatic Cleanup:** When `Consumer` is dropped, all remaining elements in the buffer are automatically dropped
4. **No Blocking:** `push` and `pop` never block; they return errors when buffer is full/empty

## Benchmarks

Performance characteristics (approximate, system-dependent):

- **Stack allocation** (`capacity ≤ N`): ~1-2 ns per `new()` call
- **Heap allocation** (`capacity > N`): ~50-100 ns per `new()` call
- **Push/Pop operations**: ~5-15 ns per operation in SPSC scenario
- **Throughput**: Up to 200M+ operations/second on modern hardware

## Minimum Supported Rust Version (MSRV)

Rust 1.87 or later is required due to const generics features.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Guidelines

- Follow Rust coding conventions
- Add tests for new features
- Update documentation as needed
- Ensure `cargo test` passes
- Run `cargo fmt` before committing

## Acknowledgments

Inspired by various ring buffer implementations in the Rust ecosystem, with a focus on simplicity, performance, and automatic stack/heap optimization.

## Related Projects

- [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam): General-purpose concurrent channels
- [ringbuf](https://github.com/agerasev/ringbuf): Another SPSC ring buffer implementation
- [rtrb](https://github.com/mgeier/rtrb): Realtime-safe SPSC ring buffer

## Support

- Documentation: [docs.rs/smallring](https://docs.rs/smallring)
- Repository: [github.com/ShaoG-R/smallring](https://github.com/ShaoG-R/smallring)
- Issues: [github.com/ShaoG-R/smallring/issues](https://github.com/ShaoG-R/smallring/issues)

