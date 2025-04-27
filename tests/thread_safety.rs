use rubberband::{LiveShifterBuilder, LiveShifterFormant, RubberBandError};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;
use rand::Rng;
use rand::distr::Uniform;

/// Helper function to test concurrent calls between `process()` and another method
///
/// This function creates two threads:
/// 1. One that repeatedly calls `process()`
/// 2. Another that repeatedly calls the provided method
///
/// This function can only check if errors or panics occur, but it cannot detect data races or
/// other undefined behavior.
///
/// # Arguments
/// * `method_call` - Closure that calls the method to test
fn test_method_concurrent_with_process<F>(method_call: F)
where F: Fn(&Arc<rubberband::LiveShifter>, usize, usize) + Send + 'static,
{
    let builder = LiveShifterBuilder::new(44100, 1).unwrap();
    let shifter = Arc::new(builder.build());
    let mut handles = vec![];

    // Create a thread that keeps calling `process`
    let shifter_ref = shifter.clone();
    let process_handle = thread::spawn(move || {
        let dist = Uniform::new(-1.0, 1.0).unwrap();
        let mut rng = rand::rng();

        let block_size = 512;
        let signal: Vec<f32> = (0..block_size).map(|_| rng.sample(dist)).collect();
        let input_slices: Vec<&[f32]> = vec![&signal];

        for _ in 0..100 {
            assert!(shifter_ref.process(&input_slices).is_ok());
            thread::sleep(Duration::from_millis(10));
        }
    });
    handles.push(process_handle);

    // Create a thread that keeps calling the test method
    let shifter_ref = shifter.clone();
    let method_handle = thread::spawn(move || {
        for i in 0..1000 {
            method_call(&shifter_ref, i, 1000);
            thread::sleep(Duration::from_millis(1));
        }
    });
    handles.push(method_handle);

    for handle in handles {
        handle.join().unwrap();
    }
}

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
                assert!(matches!(result, Ok(_) | Err(RubberBandError::OperationInProgress)));
                if let Err(RubberBandError::OperationInProgress) = result {
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

/// Test concurrent calls to `set_pitch_scale` and `process`
#[test]
fn test_set_pitch_scale() {
    let lower_bound = 0.5;
    let upper_bound = 2.0;
    test_method_concurrent_with_process(move |shifter, i, num_iter| {
        let scale = lower_bound + (upper_bound - lower_bound) * i as f64 / num_iter as f64;
        shifter.set_pitch_scale(scale);
    });
}

/// Test concurrent calls to `set_formant_scale` and `process`
#[test]
fn test_set_formant_scale() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let lower_bound = 0.5;
    let upper_bound = 2.0;
    let is_first_call: AtomicBool = AtomicBool::new(true);

    test_method_concurrent_with_process(move |shifter, i, num_iter| {
        let scale = lower_bound + (upper_bound - lower_bound) * i as f64 / num_iter as f64;
        if is_first_call.compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
            shifter.set_formant_option(LiveShifterFormant::Preserved);
            shifter.set_formant_scale(scale);
        } else {
            shifter.set_formant_scale(scale);
        }
    });
}

/// Test concurrent calls to `set_formant_option` and `process`
#[test]
fn test_set_formant_option() {
    test_method_concurrent_with_process(move |shifter, i, _| {
        let option = if i % 2 == 0 {
            LiveShifterFormant::Shifted
        } else {
            LiveShifterFormant::Preserved
        };
        shifter.set_formant_option(option);
    });
}

/// Test concurrent calls to `reset` and `process`
#[test]
fn test_reset() {
    let builder = LiveShifterBuilder::new(44100, 1).unwrap();
    let shifter = Arc::new(builder.build());
    let mut handles = vec![];

    // Create a thread that keeps calling `process`
    let shifter_ref = shifter.clone();
    let process_handle = thread::spawn(move || {
        let dist = Uniform::new(-1.0, 1.0).unwrap();
        let mut rng = rand::rng();

        let block_size = 512;
        let signal: Vec<f32> = (0..block_size).map(|_| rng.sample(dist)).collect();
        let input_slices: Vec<&[f32]> = vec![&signal];

        for _ in 0..100 {
            let result = shifter_ref.process(&input_slices);
            assert!(matches!(result, Ok(_) | Err(RubberBandError::OperationInProgress)));
            thread::sleep(Duration::from_millis(10));
        }
    });
    handles.push(process_handle);

    // Create a thread that keeps calling `reset`
    let shifter_ref = shifter.clone();
    let method_handle = thread::spawn(move || {
        for _ in 0..1000 {
            shifter_ref.reset();
            thread::sleep(Duration::from_millis(1));
        }
    });
    handles.push(method_handle);

    for handle in handles {
        handle.join().unwrap();
    }
}

/// Test concurrent calls to `start_delay` and `set_pitch_scale`
#[test]
fn test_start_delay() {
    let builder = LiveShifterBuilder::new(48000, 1).unwrap();
    let shifter = Arc::new(builder.build());
    let mut handles = vec![];

    // Compute the possible start delays for different pitch scales
    let pitch_scales: Vec<f64> = vec![0.5, 1.0, 2.0];
    let expected_delays: Vec<u32> = vec![3648, 2112, 2880];

    // Check if the set pitch scale values are reflected in the delays. (Check not all the delays
    // are the same as the initial value 2112)
    let delays: Vec<u32> = pitch_scales.iter().map(|&scale| {
        shifter.set_pitch_scale(scale);
        shifter.start_delay()
    }).collect();
    assert!(delays == expected_delays, "Delays are not as expected");

    // Create a thread that keeps calling `start_delay`
    let delays = Arc::new(RwLock::new(Vec::new()));
    let delays_for_thread = delays.clone();
    let shifter_ref = shifter.clone();
    let start_delay_handle = thread::spawn(move || {
        for _ in 0..1000 {
            let delay = shifter_ref.start_delay();
            delays_for_thread.write().unwrap().push(delay);
        }
    });
    handles.push(start_delay_handle);

    // Create a thread that keeps calling `set_pitch_scale`
    let shifter_ref = shifter.clone();
    let set_pitch_scale_handle = thread::spawn(move || {
        for i in 0..1000 {
            let scale = pitch_scales[i % pitch_scales.len()];
            shifter_ref.set_pitch_scale(scale);
        }
    });
    handles.push(set_pitch_scale_handle);

    for handle in handles {
        handle.join().unwrap();
    }

    // Check if all the delay values are among the expected delays
    let delays = delays.read().unwrap();
    for delay in delays.iter() {
        assert!(expected_delays.contains(&delay), "Delay {} not in expected delays {:?}", delay, expected_delays);
    }
}