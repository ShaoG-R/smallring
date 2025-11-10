# smallring

[![Crates.io](https://img.shields.io/crates/v/smallring.svg)](https://crates.io/crates/smallring)
[![Documentation](https://docs.rs/smallring/badge.svg)](https://docs.rs/smallring)
[![License](https://img.shields.io/crates/l/smallring.svg)](https://github.com/ShaoG-R/smallring#license)

[English](README.md) | [简体中文](README_CN.md)

A collection of high-performance lock-free ring buffer implementations with automatic stack/heap optimization. Provides both SPSC (Single Producer Single Consumer) and generic ring buffers for different use cases.

## Features

- **Lock-Free** - Thread-safe operations using atomic primitives without mutexes
- **Two Complementary Modules** - SPSC for cross-thread communication, Generic for shared access with configurable behavior
- **Stack/Heap Optimization** - Small buffers automatically use stack storage for better performance
- **High Performance** - Optimized with minimal atomic overhead and efficient masking
- **Type Safe** - Full Rust type system guarantees with compile-time checks
- **Zero Copy** - Data is moved directly without extra copying
- **Configurable Overwrite** - Generic module supports compile-time overwrite mode selection
- **Power-of-2 Capacity** - Automatic rounding for efficient modulo operations

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
smallring = "0.1"
```

## Quick Start

### SPSC Module - Cross-thread Communication

```rust
use smallring::spsc::new;
use std::num::NonZero;

// Create a ring buffer with capacity 8, stack threshold 32
let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(8).unwrap());

// Producer pushes data
producer.push(42).unwrap();
producer.push(100).unwrap();

// Consumer pops data
assert_eq!(consumer.pop().unwrap(), 42);
assert_eq!(consumer.pop().unwrap(), 100);
```

### Generic Module - Shared Access with Configurable Behavior

```rust
use smallring::generic::RingBuf;

// Overwrite mode: automatically overwrites oldest data when full
let buf: RingBuf<i32, 32, true> = RingBuf::new(4);
buf.push(1); // Returns None
buf.push(2);
buf.push(3);
buf.push(4);
buf.push(5); // Returns Some(1), overwrote oldest

// Non-overwrite mode: rejects writes when full
let buf: RingBuf<i32, 32, false> = RingBuf::new(4);
buf.push(1).unwrap(); // Returns Ok(())
buf.push(2).unwrap();
buf.push(3).unwrap();
buf.push(4).unwrap();
assert!(buf.push(5).is_err()); // Returns Err(Full(5))
```

## Usage Examples

### Basic Single-Threaded Usage (SPSC)

```rust
use smallring::spsc::new;
use std::num::NonZero;

fn main() {
    let (mut producer, mut consumer) = new::<String, 64>(NonZero::new(16).unwrap());
    
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

### Multi-threaded Communication (SPSC)

```rust
use smallring::spsc::new;
use std::thread;
use std::num::NonZero;

fn main() {
    let (mut producer, mut consumer) = new::<String, 64>(NonZero::new(32).unwrap());
    
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

### Shared Access with Generic Module

```rust
use smallring::generic::RingBuf;
use std::sync::Arc;
use std::thread;

fn main() {
    // Overwrite mode is thread-safe for concurrent writers
    let buf = Arc::new(RingBuf::<u64, 128, true>::new(128));
    let mut handles = vec![];
    
    // Multiple writer threads
    for thread_id in 0..4 {
        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let value = (thread_id * 100 + i) as u64;
                buf_clone.push(value); // Automatically overwrites old data
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

### Error Handling (SPSC)

```rust
use smallring::spsc::{new, PushError, PopError};
use std::num::NonZero;

let (mut producer, mut consumer) = new::<i32, 32>(NonZero::new(4).unwrap());

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

### Batch Operations (SPSC)

```rust
use smallring::spsc::new;
use std::num::NonZero;

let (mut producer, mut consumer) = new::<u32, 64>(NonZero::new(32).unwrap());

// Push multiple elements at once (requires T: Copy)
let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
let pushed = producer.push_slice(&data);
assert_eq!(pushed, 10);

// Pop multiple elements at once
let mut output = [0u32; 5];
let popped = consumer.pop_slice(&mut output);
assert_eq!(popped, 5);
assert_eq!(output, [1, 2, 3, 4, 5]);

// Drain remaining elements
let remaining: Vec<u32> = consumer.drain().collect();
assert_eq!(remaining, vec![6, 7, 8, 9, 10]);
```

### Error Handling (Generic Module)

```rust
use smallring::generic::{RingBuf, RingBufError};

// Non-overwrite mode
let buf: RingBuf<i32, 32, false> = RingBuf::new(4);

// Fill the buffer
for i in 0..4 {
    buf.push(i).unwrap();
}

// Buffer is full - push returns error with value
match buf.push(99) {
    Err(RingBufError::Full(value)) => {
        println!("Buffer full, couldn't push {}", value);
    }
    Ok(_) => {}
}

// Empty the buffer
while buf.pop().is_ok() {}

// Buffer is empty - pop returns error
match buf.pop() {
    Err(RingBufError::Empty) => {
        println!("Buffer is empty");
    }
    Ok(_) => {}
}
```

## Choosing Between SPSC and Generic

| Feature | SPSC Module | Generic Module |
|---------|-------------|----------------|
| Use Case | Cross-thread communication | Single-thread or shared access |
| Handles | Split Producer/Consumer | Shared RingBuf |
| Overwrite | Always rejects when full | Compile-time configurable |
| Cache Optimization | Cached read/write indices | Direct atomic access |
| Drop Behavior | Consumer auto-cleans on drop | Manual cleanup via `clear()` |

**Choose SPSC when:**
- You need cross-thread communication with separated producer/consumer roles
- You want automatic cleanup on Consumer drop
- Performance is critical and you can leverage cached indices

**Choose Generic when:**
- You need shared access from a single thread or within `Arc`
- You want compile-time configurable overwrite behavior
- You need multiple concurrent readers/writers (with appropriate synchronization)

## Stack/Heap Optimization

Both modules use generic constant `N` to control the stack/heap optimization threshold. When capacity ≤ N, data is stored on the stack; otherwise, it's allocated on the heap.

```rust
use smallring::spsc::new;
use smallring::generic::RingBuf;
use std::num::NonZero;

// SPSC: Capacity ≤ 32, uses stack storage (faster initialization, no heap allocation)
let (prod, cons) = new::<u64, 32>(NonZero::new(16).unwrap());

// SPSC: Capacity > 32, uses heap storage (suitable for larger buffers)
let (prod, cons) = new::<u64, 32>(NonZero::new(64).unwrap());

// Generic: Larger stack threshold for larger stack storage
let buf: RingBuf<u64, 128, true> = RingBuf::new(100);

// Generic: Very large stack threshold (use with caution)
let buf: RingBuf<u64, 256, false> = RingBuf::new(200);
```

**Guidelines:**
- For small buffers (≤32 elements): use `N=32` for optimal performance
- For medium buffers (≤128 elements): use `N=128` to avoid heap allocation
- For large buffers (>128 elements): heap allocation is used automatically
- Stack storage significantly improves `new()` performance and reduces memory allocator pressure

## API Overview

### SPSC Module

**Creating a Ring Buffer:**
```rust
pub fn new<T, const N: usize>(capacity: NonZero<usize>) -> (Producer<T, N>, Consumer<T, N>)
```

**Producer Methods:**
- `push(&mut self, value: T) -> Result<(), PushError<T>>` - Push a single element
- `push_slice(&mut self, values: &[T]) -> usize` - Push multiple elements (requires `T: Copy`)
- `capacity() -> usize` - Get buffer capacity
- `len() / slots() -> usize` - Get number of elements in buffer
- `free_slots() -> usize` - Get available space
- `is_full() -> bool` - Check if buffer is full
- `is_empty() -> bool` - Check if buffer is empty

**Consumer Methods:**
- `pop(&mut self) -> Result<T, PopError>` - Pop a single element
- `pop_slice(&mut self, dest: &mut [T]) -> usize` - Pop multiple elements (requires `T: Copy`)
- `peek(&self) -> Option<&T>` - View first element without removing
- `drain(&mut self) -> Drain<'_, T, N>` - Create draining iterator
- `clear(&mut self)` - Remove all elements
- `capacity() -> usize` - Get buffer capacity
- `len() / slots() -> usize` - Get number of elements in buffer
- `is_empty() -> bool` - Check if buffer is empty

### Generic Module

**Creating a Ring Buffer:**
```rust
pub fn new(capacity: usize) -> RingBuf<T, N, OVERWRITE>
```

**RingBuf Methods:**
- `push(&self, value: T)` - Push element (return type depends on `OVERWRITE` flag)
  - `OVERWRITE=true`: Returns `Option<T>` (Some if element was overwritten)
  - `OVERWRITE=false`: Returns `Result<(), RingBufError<T>>`
- `pop(&self) -> Result<T, RingBufError<T>>` - Pop a single element
- `push_slice(&self, values: &[T]) -> usize` - Push multiple elements (requires `T: Copy`)
- `pop_slice(&self, dest: &mut [T]) -> usize` - Pop multiple elements (requires `T: Copy`)
- `peek(&self) -> Option<&T>` - View first element without removing
- `clear(&self)` - Remove all elements
- `capacity() -> usize` - Get buffer capacity
- `len() -> usize` - Get number of elements in buffer
- `is_empty() -> bool` - Check if buffer is empty
- `is_full() -> bool` - Check if buffer is full

## Performance Tips

1. **Choose appropriate capacity** - Capacity is automatically rounded up to power of 2 for efficient masking. Choose power-of-2 sizes to avoid wasted space.
2. **Use batch operations** - `push_slice` and `pop_slice` are significantly faster than individual operations when working with `Copy` types.
3. **Choose appropriate N** - Stack storage significantly improves performance for small buffers and eliminates heap allocation overhead. Common values: 32, 64, 128.
4. **Use peek when needed** - Avoid pop + re-push patterns. Use `peek()` to inspect without consuming.
5. **SPSC vs Generic** - Use SPSC module for cross-thread communication with optimal caching. Use Generic module when you need shared access or configurable overwrite behavior.
6. **Avoid false sharing** - In multi-threaded scenarios, ensure producer and consumer are on different cache lines.

### Capacity Selection

Capacity is automatically rounded up to the nearest power of 2:

```rust
// Requested capacity → Actual capacity
// 5 → 8
// 10 → 16
// 30 → 32
// 100 → 128
```

**Recommendation:** Choose power-of-2 capacities to avoid wasted space.

## Thread Safety

### SPSC Module

- Designed specifically for Single Producer Single Consumer scenarios across threads
- `Producer` and `Consumer` are **not** `Sync`, ensuring single-threaded access
- `Producer` and `Consumer` are `Send`, allowing them to be moved between threads
- Atomic operations ensure memory ordering guarantees between producer and consumer threads

### Generic Module

- `RingBuf` is `Send` and `Sync` when `T` is `Send`
- Can be shared across threads using `Arc`
- Thread-safe for concurrent operations (multiple writers or readers)
- Appropriate for both single-threaded and multi-threaded scenarios

## Important Notes

### Common to Both Modules

- **Capacity rounding** - All capacities are automatically rounded up to the nearest power of 2 for efficient masking operations
- **Element lifecycle** - Elements are properly dropped when popped or when the buffer is cleaned up
- **Memory layout** - Uses `MaybeUninit<T>` internally for safe uninitialized memory handling
- **Power-of-2 optimization** - Fast modulo operations using bitwise AND instead of division

### SPSC Module Specifics

- **Thread safety** - Designed specifically for Single Producer Single Consumer scenarios across threads
- **Automatic cleanup** - `Consumer` automatically cleans up remaining elements when dropped
- **Cached indices** - Producer and Consumer cache read/write indices for better performance
- **No overwrite** - Always rejects writes when full; returns `PushError::Full`

### Generic Module Specifics

- **Flexible concurrency** - Can be shared across threads using `Arc` or used in single-threaded scenarios
- **Configurable overwrite** - Compile-time `OVERWRITE` flag controls behavior when full:
  - `true`: Automatically overwrites oldest data (circular buffer semantics)
  - `false`: Rejects new writes and returns error
- **Manual cleanup** - Does NOT automatically clean up on drop. Call `clear()` explicitly if needed
- **Zero-cost abstraction** - Overwrite behavior selected at compile time with no runtime overhead

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

