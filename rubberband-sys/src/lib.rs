#![allow(non_upper_case_globals)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    mod live {
        use super::*;
        use std::f32::consts::PI;

        #[test]
        fn test_create_destroy_live() {
            unsafe {
                // Create a RubberBandLiveState with default options
                let sample_rate = 44100;
                let channels = 1;
                let options = 0; // Default options

                let state: RubberBandLiveState = rubberband_live_new(sample_rate, channels, options);
                assert!(!state.is_null(), "Failed to create RubberBandLiveState");

                // Clean up
                rubberband_live_delete(state);
            }
        }

        #[test]
        fn test_get_set_pitch_scale() {
            unsafe {
                // Create a RubberBandLiveState
                let sample_rate = 44100;
                let channels = 1;
                let options = 0;

                let state: RubberBandLiveState = rubberband_live_new(sample_rate, channels, options);
                assert!(!state.is_null());

                // Test setting and getting pitch scale
                let pitch_scale = 2.0; // Shift up by an octave
                rubberband_live_set_pitch_scale(state, pitch_scale);

                let retrieved_scale = rubberband_live_get_pitch_scale(state);
                assert!(
                    (retrieved_scale - pitch_scale).abs() < 0.001,
                    "Pitch scale mismatch: set {}, got {}", pitch_scale, retrieved_scale
                );

                // Clean up
                rubberband_live_delete(state);
            }
        }

        #[test]
        fn test_get_set_formant_scale() {
            unsafe {
                // Create a RubberBandLiveState
                let sample_rate = 44100;
                let channels = 1;
                let options = 0;

                let state: RubberBandLiveState = rubberband_live_new(sample_rate, channels, options);
                assert!(!state.is_null());

                // Test setting and getting formant scale
                let formant_scale = 2.0;
                rubberband_live_set_formant_scale(state, formant_scale);

                let retrieved_scale = rubberband_live_get_formant_scale(state);
                assert!(
                    (retrieved_scale - formant_scale).abs() < 0.001, 
                    "Formant scale mismatch: set {}, got {}", formant_scale, retrieved_scale
                );

                // Clean up
                rubberband_live_delete(state);
            }
        }

        #[test]
        fn test_get_channel_count() {
            unsafe {
                // Create RubberBandLiveState with different channel counts
                let sample_rate = 44100;
                let options = 0;

                // Test with 1 channel
                let mono_state: RubberBandLiveState = rubberband_live_new(sample_rate, 1, options);
                assert!(!mono_state.is_null());
                assert_eq!(rubberband_live_get_channel_count(mono_state), 1);
                rubberband_live_delete(mono_state);

                // Test with 2 channels
                let stereo_state = rubberband_live_new(sample_rate, 2, options);
                assert!(!stereo_state.is_null());
                assert_eq!(rubberband_live_get_channel_count(stereo_state), 2);
                rubberband_live_delete(stereo_state);
            }
        }

        #[test]
        fn test_basic_processing() {
            unsafe {
                // Create a RubberBandLiveState
                let sample_rate = 44100;
                let channels = 1;
                let options = 0;

                let state: RubberBandLiveState = rubberband_live_new(sample_rate, channels, options);
                assert!(!state.is_null());

                // Get the block size and start delay
                let block_size: usize = rubberband_live_get_block_size(state) as usize;
                let start_delay: usize = rubberband_live_get_start_delay(state) as usize;
                let num_samples: usize = (start_delay / block_size) * block_size + block_size;

                // Create input and output buffers
                let mut input_buffers = vec![vec![0.0f32; block_size]; 1];
                let mut output_buffers = vec![vec![0.0f32; block_size]; 1];
                let mut output_samples = vec![0.0f32; num_samples];

                // Process the audio block by block
                for n in (0..num_samples).step_by(block_size) {
                    // Fill input with a simple sine wave
                    for i in 0..block_size {
                        input_buffers[0][i] = ((n + i) as f32 * 0.1).sin(); // 0 dBFS
                    }

                    // Process the audio
                    let input_view = vec![input_buffers[0].as_ptr()];
                    let mut output_view = vec![output_buffers[0].as_mut_ptr()];
                    rubberband_live_shift(state, input_view.as_ptr(), output_view.as_mut_ptr());

                    // Save the output samples
                    for i in 0..block_size {
                        output_samples[n + i] = output_buffers[0][i];
                    }
                }

                // The first start_delay output samples should be zeros
                for i in 0..start_delay {
                    assert!(
                        output_samples[i].abs() < 1e-3,
                        concat!(
                            "Output sample should be smaller than -60 dBFS before the start delay ({}),",
                            "got {} at sample {}"
                        ),
                        start_delay,
                        i,
                        output_samples[i]
                    );
                }

                // The output samples should not be all zeros (basic sanity check)
                let mut sum = 0.0;
                for i in 0..num_samples {
                    sum += output_samples[i].abs();
                }
                assert!(sum > 0.0, "Output should contain non-zero samples");

                // Clean up
                rubberband_live_delete(state);
            }
        }

        #[test]
        fn test_octave_up() {
            unsafe {
                let sample_rate: usize = 48000;
                let channels = 1;
                let options = 0;

                let frequency = 1000.0;
                let period = sample_rate as f32 / frequency;

                let state: RubberBandLiveState = rubberband_live_new(sample_rate as u32, channels, options);
                assert!(!state.is_null());

                // Set the pitch scale to 2.0 (octave up)
                rubberband_live_set_pitch_scale(state, 2.0);

                // Create input and output buffers
                let block_size: usize = rubberband_live_get_block_size(state) as usize;
                let num_samples: usize = (sample_rate / block_size) * block_size + block_size; // ~1 second

                let mut input_buffers = vec![vec![0.0f32; block_size]; 1];
                let mut output_buffers = vec![vec![0.0f32; block_size]; 1];
                let mut output_samples = vec![0.0f32; num_samples];

                // Process the audio block by block
                for n in (0..num_samples).step_by(block_size) {
                    // Fill input with a sine wave
                    for i in 0..block_size {
                        input_buffers[0][i] = (2.0 * PI * (n + i) as f32 / period).sin(); // 0 dBFS
                    }

                    // Process the audio
                    let input_view = vec![input_buffers[0].as_ptr()];
                    let mut output_view = vec![output_buffers[0].as_mut_ptr()];
                    rubberband_live_shift(state, input_view.as_ptr(), output_view.as_mut_ptr());

                    // Save the output samples
                    for i in 0..block_size {
                        output_samples[n + i] = output_buffers[0][i];
                    }
                }

                // Check the frequency by measuring zero-crossings in the last 0.5 seconds
                let last_half_second = num_samples / 2;
                let mut zero_crossing_indices = Vec::new();
                let mut last_sample = output_samples[last_half_second];

                // Collect all zero-crossing indices
                for i in (last_half_second + 1)..num_samples {
                    if output_samples[i] * last_sample < 0.0 {
                        zero_crossing_indices.push(i);
                    }
                    last_sample = output_samples[i];
                }

                // Calculate the mean zero-crossing period (half-period)
                let mut half_periods = Vec::new();
                for i in 1..zero_crossing_indices.len() {
                    half_periods.push(zero_crossing_indices[i] - zero_crossing_indices[i-1]);
                }
                let mean_half_period: f32 = if !half_periods.is_empty() {
                    half_periods.iter().sum::<usize>() as f32 / half_periods.len() as f32
                } else {
                    panic!("No zero-crossings detected in output signal");
                };

                // Expect the calculated period to be half the original
                let mean_period = 2.0 * mean_half_period;
                let expected_period = period / 2.0;

                assert!(
                    (mean_period - expected_period).abs() < 0.1, // tolerance = 0.1 samples (~2.1 us)
                    "Mean period mismatch: expected {:.2} samples, got {:.2} samples", 
                    expected_period, mean_period
                );

                // Clean up
                rubberband_live_delete(state);
            }
        }
    }
}