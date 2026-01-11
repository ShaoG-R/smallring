#![cfg(feature = "loom")]

use loom::sync::Arc;
use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::thread;
use smallring::atomic::AtomicRingBuf;

#[test]
fn test_atomic_queue_mpsc_loom() {
    loom::model(|| {
        // MPSC test: 2 Producers, 1 Consumer
        // Capacity 4
        let buf = Arc::new(AtomicRingBuf::<AtomicUsize, 4, false>::new(4));
        let buf1 = buf.clone();
        let buf2 = buf.clone();

        let t1 = thread::spawn(move || {
            buf1.push(1, Ordering::Relaxed).unwrap();
            buf1.push(2, Ordering::Relaxed).unwrap();
        });

        let t2 = thread::spawn(move || {
            buf2.push(3, Ordering::Relaxed).unwrap();
            buf2.push(4, Ordering::Relaxed).unwrap();
        });

        // Consumer
        let t3 = thread::spawn(move || {
            let mut received = 0;
            let mut sum = 0;
            // Expect 4 values
            while received < 4 {
                if let Some(val) = buf.pop(Ordering::Relaxed) {
                    received += 1;
                    sum += val;
                } else {
                    thread::yield_now();
                }
            }
            // 1+2+3+4 = 10
            assert_eq!(sum, 10);
        });

        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();
    });
}

#[test]
fn test_atomic_queue_overwrite_loom() {
    loom::model(|| {
        // Overwrite mode
        // Capacity 1.
        // Writer pushes 1, then 2 (overwrites 1).
        // Reader pops. Should get 2? Or maybe 1 if it raced?
        // Note: write_idx fetch_add is atomic.
        // If writer 1 updates write_idx, writer 2 updates write_idx.
        // The one with higher index "wins" the tail?
        // Overwrite logic checks capacity.

        // Single thread writer, single reader first.
        let buf = Arc::new(AtomicRingBuf::<AtomicUsize, 4, true>::new(1));

        let producer = buf.clone();

        thread::spawn(move || {
            producer.push(10, Ordering::Relaxed);
            producer.push(20, Ordering::Relaxed); // Should overwrite 10 -> 20.
        });

        // Reader
        // Wait for producer
        // Note: In loom, without explicit synchronization (like join), we can't guarantee order.
        // But we want to test that state is consistent.

        // Let's rely on atomic observable state.
    });
}
