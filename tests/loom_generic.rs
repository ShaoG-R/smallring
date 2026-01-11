#![cfg(feature = "loom")]

use loom::thread;
use smallring::generic::RingBuf;

#[test]
fn test_generic_ringbuf_send_loom() {
    // RingBuf is single-threaded (requires &mut self) but is Send.
    // Test passing it between threads safely.
    loom::model(|| {
        let mut buf = RingBuf::<usize, 4, false>::new(2);

        buf.push(10).unwrap();

        // Spawn a thread that takes ownership, pushes another, then returns it?
        // Or just consumes it.

        // Move to thread
        let t = thread::spawn(move || {
            // Verify we can access it
            assert_eq!(buf.pop().unwrap(), 10);
            buf.push(20).unwrap();
            buf
        });

        let mut buf_returned = t.join().unwrap();
        assert_eq!(buf_returned.pop().unwrap(), 20);
    });
}
