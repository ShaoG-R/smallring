/// RingBuf performance benchmark
/// 
/// 对比自定义 SmallVec-based RingBuf 与 rtrb 的性能
/// 
/// 重点测试：
/// 1. new() 创建性能（SmallVec 栈分配优势）
/// 2. push/pop 吞吐性能
/// 3. 不同容量下的表现

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use smallring::spsc;
use std::num::NonZero;
use std::time::Duration;
use std::hint::black_box;

/// Benchmark: RingBuffer creation performance
/// 
/// 对比不同容量下的 new() 性能
/// SmallVec 在小容量（≤32）时应有明显优势
fn benchmark_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuf_creation");
    
    for capacity in [4, 8, 16, 32, 64, 128, 256] {
        // Custom SmallVec-based RingBuf
        group.bench_with_input(
            BenchmarkId::new("custom_smallvec", capacity),
            &capacity,
            |b, &cap| {
                b.iter(|| {
                    let (producer, consumer) = spsc::new::<u64, 32>(NonZero::new(cap).unwrap());
                    black_box((producer, consumer));
                });
            },
        );
        
        // rtrb for comparison
        group.bench_with_input(
            BenchmarkId::new("rtrb", capacity),
            &capacity,
            |b, &cap| {
                b.iter(|| {
                    let (producer, consumer) = rtrb::RingBuffer::<u64>::new(black_box(cap));
                    black_box((producer, consumer));
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark: Single-threaded push/pop throughput
/// 
/// 单线程 push/pop 吞吐量测试
fn benchmark_single_thread_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuf_single_thread");
    
    for capacity in [8, 32, 128] {
        let operations = 10000;
        group.throughput(Throughput::Elements(operations));
        
        // Custom SmallVec-based RingBuf
        group.bench_with_input(
            BenchmarkId::new("custom_smallvec", capacity),
            &capacity,
            |b, &cap| {
                b.iter(|| {
                    let (mut producer, mut consumer) = spsc::new::<u64, 32>(NonZero::new(cap).unwrap());
                    
                    for i in 0..operations {
                        // Fill buffer as much as possible
                        let _ = producer.push(black_box(i));
                        
                        // Pop if we have data
                        let _ = consumer.pop();
                    }
                });
            },
        );
        
        // rtrb for comparison
        group.bench_with_input(
            BenchmarkId::new("rtrb", capacity),
            &capacity,
            |b, &cap| {
                b.iter(|| {
                    let (mut producer, mut consumer) = rtrb::RingBuffer::<u64>::new(cap);
                    
                    for i in 0..operations {
                        let _ = producer.push(black_box(i));
                        let _ = consumer.pop();
                    }
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark: Batch push/pop operations
/// 
/// 批量 push/pop 操作性能
fn benchmark_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuf_batch_ops");
    
    let capacity = 64;
    let batch_size = 32;
    
    // Custom SmallVec-based RingBuf - batch push then batch pop
    group.bench_function("custom_smallvec_batch", |b| {
        b.iter(|| {
            let (mut producer, mut consumer) = spsc::new::<u64, 32>(NonZero::new(capacity).unwrap());
            
            // Push batch
            for i in 0..batch_size {
                producer.push(black_box(i)).unwrap();
            }
            
            // Pop batch
            for _ in 0..batch_size {
                black_box(consumer.pop().unwrap());
            }
        });
    });
    
    // rtrb - batch push then batch pop
    group.bench_function("rtrb_batch", |b| {
        b.iter(|| {
            let (mut producer, mut consumer) = rtrb::RingBuffer::<u64>::new(capacity);
            
            // Push batch
            for i in 0..batch_size {
                producer.push(black_box(i)).unwrap();
            }
            
            // Pop batch
            for _ in 0..batch_size {
                black_box(consumer.pop().unwrap());
            }
        });
    });
    
    group.finish();
}

/// Benchmark: Multi-threaded producer-consumer
/// 
/// 多线程生产者-消费者性能
fn benchmark_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuf_concurrent");
    group.measurement_time(Duration::from_secs(10));
    
    let capacity = 128;
    let messages = 10000;
    
    // Custom SmallVec-based RingBuf
    group.bench_function("custom_smallvec_concurrent", |b| {
        b.iter(|| {
            let (mut producer, mut consumer) = spsc::new::<u64, 32>(NonZero::new(capacity).unwrap());
            
            let producer_handle = std::thread::spawn(move || {
                for i in 0..messages {
                    loop {
                        if producer.push(black_box(i)).is_ok() {
                            break;
                        }
                        std::hint::spin_loop();
                    }
                }
            });
            
            let consumer_handle = std::thread::spawn(move || {
                let mut count = 0;
                while count < messages {
                    if consumer.pop().is_ok() {
                        count += 1;
                    } else {
                        std::hint::spin_loop();
                    }
                }
            });
            
            producer_handle.join().unwrap();
            consumer_handle.join().unwrap();
        });
    });
    
    // rtrb
    group.bench_function("rtrb_concurrent", |b| {
        b.iter(|| {
            let (mut producer, mut consumer) = rtrb::RingBuffer::<u64>::new(capacity);
            
            let producer_handle = std::thread::spawn(move || {
                for i in 0..messages {
                    loop {
                        if producer.push(black_box(i)).is_ok() {
                            break;
                        }
                        std::hint::spin_loop();
                    }
                }
            });
            
            let consumer_handle = std::thread::spawn(move || {
                let mut count = 0;
                while count < messages {
                    if consumer.pop().is_ok() {
                        count += 1;
                    } else {
                        std::hint::spin_loop();
                    }
                }
            });
            
            producer_handle.join().unwrap();
            consumer_handle.join().unwrap();
        });
    });
    
    group.finish();
}

/// Benchmark: Small capacity optimization test
/// 
/// 小容量优化测试 - 验证 SmallVec 栈分配的优势
fn benchmark_small_capacity_advantage(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuf_small_capacity");
    
    // Test specifically at capacity=32 (stack allocation boundary)
    let capacity = 32;
    let iterations = 1000;
    
    group.bench_function("custom_smallvec_create_destroy_32", |b| {
        b.iter(|| {
            for _ in 0..iterations {
                let (mut producer, mut consumer) = spsc::new::<u64, 32>(NonZero::new(capacity).unwrap());
                
                // Do some work
                producer.push(42).unwrap();
                black_box(consumer.pop().unwrap());
                
                // Drop happens automatically
            }
        });
    });
    
    group.bench_function("rtrb_create_destroy_32", |b| {
        b.iter(|| {
            for _ in 0..iterations {
                let (mut producer, mut consumer) = rtrb::RingBuffer::<u64>::new(black_box(capacity));
                
                // Do some work
                producer.push(42).unwrap();
                black_box(consumer.pop().unwrap());
                
                // Drop happens automatically
            }
        });
    });
    
    group.finish();
}

/// Benchmark: Push performance under different conditions
/// 
/// 不同条件下的 push 性能
fn benchmark_push_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuf_push");
    
    let capacity = 64;
    let push_count = 50; // Leave some space to avoid full buffer
    
    // Custom SmallVec-based RingBuf
    group.bench_function("custom_smallvec_push", |b| {
        b.iter(|| {
            let (mut producer, _consumer) = spsc::new::<u64, 32>(NonZero::new(capacity).unwrap());
            
            for i in 0..push_count {
                producer.push(black_box(i)).unwrap();
            }
        });
    });
    
    // rtrb
    group.bench_function("rtrb_push", |b| {
        b.iter(|| {
            let (mut producer, _consumer) = rtrb::RingBuffer::<u64>::new(capacity);
            
            for i in 0..push_count {
                producer.push(black_box(i)).unwrap();
            }
        });
    });
    
    group.finish();
}

/// Benchmark: Pop performance
/// 
/// Pop 性能测试
fn benchmark_pop_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuf_pop");
    
    let capacity = 64;
    let items = 50;
    
    // Custom SmallVec-based RingBuf
    group.bench_function("custom_smallvec_pop", |b| {
        b.iter_batched(
            || {
                let (mut producer, consumer) = spsc::new::<u64, 32>(NonZero::new(capacity).unwrap());
                for i in 0..items {
                    producer.push(i).unwrap();
                }
                consumer
            },
            |mut consumer| {
                for _ in 0..items {
                    black_box(consumer.pop().unwrap());
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
    
    // rtrb
    group.bench_function("rtrb_pop", |b| {
        b.iter_batched(
            || {
                let (mut producer, consumer) = rtrb::RingBuffer::<u64>::new(capacity);
                for i in 0..items {
                    producer.push(i).unwrap();
                }
                consumer
            },
            |mut consumer| {
                for _ in 0..items {
                    black_box(consumer.pop().unwrap());
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_creation,
    benchmark_single_thread_throughput,
    benchmark_batch_operations,
    benchmark_concurrent,
    benchmark_small_capacity_advantage,
    benchmark_push_only,
    benchmark_pop_only,
);

criterion_main!(benches);

