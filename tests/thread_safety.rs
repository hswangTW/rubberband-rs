use rubberband::{LiveShifterBuilder, RubberBandError};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

/// Test concurrent processing
///
/// The underlying C++ implementation does not allow concurrent calls to `shift()`. This test
/// is designed to verify that the Rust wrapper correctly handles this.
#[test]
fn test_concurrent_processing() {
    let builder = LiveShifterBuilder::new(44100, 1).unwrap();
    let shifter = Arc::new(builder.build());
    let error_count = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];

    // Try to call `process` concurrently from multiple threads.
    for _ in 0..4 {
        let shifter = shifter.clone();
        let error_count = error_count.clone();
        let handle = thread::spawn(move || {
            let block_size = 512;
            let input = vec![vec![0.5f32; block_size]];
            let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();

            for _ in 0..100 {
                let result = shifter.process(&input_slices);
                assert!(matches!(result, Ok(_) | Err(RubberBandError::ProcessInProgress)));
                if let Err(RubberBandError::ProcessInProgress) = result {
                    error_count.fetch_add(1, Ordering::Relaxed);
                }

                thread::sleep(Duration::from_micros(100));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(error_count.load(Ordering::Relaxed) > 0, "Expected at least one ProcessInProgress error");
}
