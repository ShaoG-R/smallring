#![cfg(feature = "loom")]

use loom::thread;
use smallring::spsc;
use std::num::NonZero;

#[test]
fn test_spsc_loom() {
    loom::model(|| {
        let capacity = NonZero::new(2).unwrap();
        // Use N=4 (larger than capacity 2) to test Stack/FixedVec logic
        let (mut p, mut c) = spsc::new::<usize, 4>(capacity);

        let t1 = thread::spawn(move || {
            // Push 1
            p.push(10).unwrap();
            // Push 2
            p.push(20).unwrap();
        });

        let t2 = thread::spawn(move || {
            let mut v1 = None;
            let mut v2 = None;

            // Retry until we get values
            loop {
                if v1.is_none() {
                    if let Ok(v) = c.pop() {
                        v1 = Some(v);
                    }
                }
                if v1.is_some() && v2.is_none() {
                    if let Ok(v) = c.pop() {
                        v2 = Some(v);
                        break;
                    }
                }
                // Yield to let producer run
                thread::yield_now();
            }

            assert_eq!(v1, Some(10));
            assert_eq!(v2, Some(20));
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}

#[test]
fn test_spsc_wrap_loom() {
    loom::model(|| {
        let capacity = NonZero::new(2).unwrap();
        let (mut p, mut c) = spsc::new::<usize, 4>(capacity);

        let t1 = thread::spawn(move || {
            // First pass
            p.push(1).unwrap();
            p.push(2).unwrap();

            // Wait for consumer to empty partly or fully (simulated by yield/loom)
            // But we can just try pushing. If full, we yield.
            let mut p3 = false;
            let mut p4 = false;

            // Try to push 3 and 4
            loop {
                if !p3 {
                    if p.push(3).is_ok() {
                        p3 = true;
                    }
                }
                if p3 && !p4 {
                    if p.push(4).is_ok() {
                        p4 = true;
                    }
                }
                if p3 && p4 {
                    break;
                }
                thread::yield_now();
            }
        });

        let t2 = thread::spawn(move || {
            let mut counts = 0;
            let mut sum = 0;
            loop {
                match c.pop() {
                    Ok(v) => {
                        sum += v;
                        counts += 1;
                    }
                    Err(_) => {
                        thread::yield_now();
                    }
                }
                if counts == 4 {
                    break;
                }
            }
            assert_eq!(sum, 1 + 2 + 3 + 4); // 10
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}

#[test]
fn test_spsc_slice_loom() {
    loom::model(|| {
        let capacity = NonZero::new(4).unwrap();
        let (mut p, mut c) = spsc::new::<usize, 8>(capacity);

        let t1 = thread::spawn(move || {
            let data = [1, 2, 3];
            let n = p.push_slice(&data);
            assert_eq!(n, 3);

            thread::yield_now();

            let data2 = [4, 5];
            let mut pushed = 0;
            while pushed < data2.len() {
                let n = p.push_slice(&data2[pushed..]);
                pushed += n;
                if pushed < data2.len() {
                    thread::yield_now();
                }
            }
        });

        let t2 = thread::spawn(move || {
            let mut buf = [0; 8];
            let mut total_read = 0;
            loop {
                let n = c.pop_slice(&mut buf[total_read..]);
                total_read += n;
                if total_read >= 5 {
                    break;
                }
                if n == 0 {
                    thread::yield_now();
                }
            }
            assert_eq!(&buf[0..5], &[1, 2, 3, 4, 5]);
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
