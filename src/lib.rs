use thiserror::Error;
use rubberband_sys::{
    rubberband_live_new,
    rubberband_live_delete,
    rubberband_live_set_debug_level,
    rubberband_live_set_pitch_scale,
    rubberband_live_get_pitch_scale,
    rubberband_live_set_formant_scale,
    rubberband_live_get_formant_scale,
    rubberband_live_set_formant_option,
    rubberband_live_get_start_delay,
    rubberband_live_get_channel_count,
    rubberband_live_get_block_size,
    rubberband_live_shift,
    rubberband_live_reset,
    RubberBandLiveState,
    RubberBandLiveOption,
    RubberBandLiveOptions,
    RubberBandLiveOption_RubberBandLiveOptionWindowShort as OPTION_BITS_WINDOW_SHORT,
    RubberBandLiveOption_RubberBandLiveOptionWindowMedium as OPTION_BITS_WINDOW_MEDIUM,
    RubberBandLiveOption_RubberBandLiveOptionFormantShifted as OPTION_BITS_FORMANT_SHIFTED,
    RubberBandLiveOption_RubberBandLiveOptionFormantPreserved as OPTION_BITS_FORMANT_PRESERVED,
    RubberBandLiveOption_RubberBandLiveOptionChannelsApart as OPTION_BITS_CHANNELS_APART,
    RubberBandLiveOption_RubberBandLiveOptionChannelsTogether as OPTION_BITS_CHANNELS_TOGETHER,
};

/// Window size options for the live pitch shifter.
#[derive(Debug, Clone, Copy)]
pub enum RubberBandLiveShifterWindow {
    /// Short window, which is the default option.
    Short,
    /// Medium window, enabling the read ahead feature in R3 (Live Shifter) engine.
    Medium,
}

/// Formant preservation options for the live pitch shifter.
#[derive(Debug, Clone, Copy)]
pub enum RubberBandLiveShifterFormant {
    /// No formant preservation, formants are shifted with the pitch. Default option.
    Shifted,
    /// With formant preservation, trying to preserve the formant and hence the timbre.
    Preserved,
}

/// Channel processing mode for the live pitch shifter.
#[derive(Debug, Clone, Copy)]
pub enum RubberBandLiveShifterChannelMode {
    /// Process channels independently. Gives the best quality for individual channels but a more
    /// diffuse stereo image. Default option.
    Apart,
    /// Process channels together to preserve stereo image. Gives relatively less stereo space and
    /// width than the default, as well as slightly lower fidelity for individual channel content.
    Together,
}

/// Builder for configuring and creating a RubberBandLiveShifter instance.
pub struct RubberBandLiveShifterBuilder {
    sample_rate: u32,
    channels: u32,
    window: RubberBandLiveShifterWindow,
    formant: RubberBandLiveShifterFormant,
    channel_mode: RubberBandLiveShifterChannelMode,
    debug_level: i32,
}

impl RubberBandLiveShifterBuilder {
    pub fn new(sample_rate: u32, channels: u32) -> Result<Self, RubberBandError> {
        if sample_rate == 0 {
            return Err(RubberBandError::UnsupportedSampleRate(sample_rate));
        }
        if channels == 0 {
            return Err(RubberBandError::UnsupportedChannelCount(channels));
        }
        Ok(Self {
            sample_rate,
            channels,
            window: RubberBandLiveShifterWindow::Short,
            formant: RubberBandLiveShifterFormant::Shifted,
            channel_mode: RubberBandLiveShifterChannelMode::Apart,
            debug_level: 0,
        })
    }

    pub fn window(mut self, window: RubberBandLiveShifterWindow) -> Self {
        self.window = window;
        self
    }

    pub fn formant(mut self, formant: RubberBandLiveShifterFormant) -> Self {
        self.formant = formant;
        self
    }

    pub fn channel_mode(mut self, channel_mode: RubberBandLiveShifterChannelMode) -> Self {
        self.channel_mode = channel_mode;
        self
    }

    pub fn debug_level(mut self, level: i32) -> Self {
        self.debug_level = level;
        self
    }

    pub fn build(self) -> RubberBandLiveShifter {
        let mut options: RubberBandLiveOption = 0; // Default options
        match self.window {
            RubberBandLiveShifterWindow::Short => options |= OPTION_BITS_WINDOW_SHORT,
            RubberBandLiveShifterWindow::Medium => options |= OPTION_BITS_WINDOW_MEDIUM,
        }
        match self.formant {
            RubberBandLiveShifterFormant::Shifted => options |= OPTION_BITS_FORMANT_SHIFTED,
            RubberBandLiveShifterFormant::Preserved => options |= OPTION_BITS_FORMANT_PRESERVED,
        }
        match self.channel_mode {
            RubberBandLiveShifterChannelMode::Apart => options |= OPTION_BITS_CHANNELS_APART,
            RubberBandLiveShifterChannelMode::Together => options |= OPTION_BITS_CHANNELS_TOGETHER,
        }

        let state: RubberBandLiveState = unsafe {
            let state = rubberband_live_new(
                self.sample_rate,
                self.channels,
                options as RubberBandLiveOptions,
            );
            rubberband_live_set_debug_level(state, self.debug_level);
            state
        };

        RubberBandLiveShifter {
            state,
            sample_rate: self.sample_rate,
        }
    }
}

/// A real-time pitch shifter using the RubberBand audio processing library.
///
/// This is a wrapper around the RubberBandLiveShifter, providing realtime-safe pitch shifting with
/// optional formant preservation and several other options.
pub struct RubberBandLiveShifter {
    state: RubberBandLiveState,
    sample_rate: u32,
}

/// Error types for RubberBandLiveShifter operations
#[derive(Debug, Error)]
pub enum RubberBandError {
    #[error("Unsupported sample rate: {0}")]
    UnsupportedSampleRate(u32),
    #[error("Unsupported channel count: {0}")]
    UnsupportedChannelCount(u32),
    #[error("Inconsistent channel count: expected {expected}, got {actual}")]
    InconsistentChannelCount {
        expected: usize,
        actual: usize,
    },
    #[error("Inconsistent block size for channel {channel}: expected {expected}, got {actual}")]
    InconsistentBlockSize {
        channel: usize,
        expected: usize,
        actual: usize,
    },
}

impl RubberBandLiveShifter {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn set_pitch_scale(&mut self, scale: f64) {
        unsafe {
            rubberband_live_set_pitch_scale(self.state, scale);
        }
    }

    pub fn pitch_scale(&self) -> f64 {
        unsafe {
            rubberband_live_get_pitch_scale(self.state)
        }
    }

    pub fn set_formant_scale(&mut self, scale: f64) {
        unsafe {
            rubberband_live_set_formant_scale(self.state, scale);
        }
    }

    pub fn formant_scale(&self) -> f64 {
        unsafe {
            rubberband_live_get_formant_scale(self.state)
        }
    }

    pub fn set_formant_option(&mut self, option: RubberBandLiveShifterFormant) {
        let option_bits = match option {
            RubberBandLiveShifterFormant::Shifted => OPTION_BITS_FORMANT_SHIFTED,
            RubberBandLiveShifterFormant::Preserved => OPTION_BITS_FORMANT_PRESERVED,
        };
        unsafe {
            rubberband_live_set_formant_option(
                self.state,
                option_bits as RubberBandLiveOptions,
            );
        }
    }

    pub fn start_delay(&self) -> u32 {
        unsafe {
            rubberband_live_get_start_delay(self.state)
        }
    }

    pub fn channel_count(&self) -> u32 {
        unsafe {
            rubberband_live_get_channel_count(self.state)
        }
    }

    pub fn block_size(&self) -> u32 {
        unsafe {
            rubberband_live_get_block_size(self.state)
        }
    }

    /// Process a single block of audio samples.
    ///
    /// An output buffer will be automatically allocated and returned.
    ///
    /// Arguments:
    ///
    /// - `input`: A slice of slices of floats, each containing a contiguous block of audio
    ///            samples for a single channel. The number of channels must be equal to the number
    ///            of channels of the RubberBandLiveShifter. Each channel must have the same number
    ///            of samples.
    ///
    /// Returns:
    ///
    /// A vector of vectors of floats, each containing a contiguous block of audio samples for a
    /// single channel.
    pub fn process(&mut self, input: &[&[f32]]) -> Result<Vec<Vec<f32>>, RubberBandError> {
        let mut output = vec![vec![0.0; input[0].len()]; input.len()];
        let mut output_slices: Vec<&mut [f32]> = output
            .iter_mut()
            .map(|slice| slice.as_mut_slice())
            .collect();
        self.process_into(input, &mut output_slices)?;
        Ok(output)
    }

    /// Process a single block of audio samples with pre-allocated buffers.
    ///
    /// The input and output buffers must not alias one another or overlap.
    ///
    /// Arguments:
    ///
    /// - `input`: A slice of slices of floats, each containing a contiguous block of audio
    ///            samples for a single channel. The number of channels must be equal to the number
    ///            of channels of the RubberBandLiveShifter. Each channel must have the same number
    ///            of samples.
    /// - `output`: A slice of slices of floats, should have the same shape as the input.
    pub fn process_into(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) -> Result<(), RubberBandError> {
        let channel_count = self.channel_count() as usize;
        if input.len() != channel_count {
            return Err(RubberBandError::InconsistentChannelCount {
                expected: channel_count,
                actual: input.len(),
            });
        }
        if output.len() != channel_count {
            return Err(RubberBandError::InconsistentChannelCount {
                expected: channel_count,
                actual: output.len(),
            });
        }

        let block_size = self.block_size() as usize;
        for ch in 0..channel_count {
            if input[ch].len() != block_size {
                return Err(RubberBandError::InconsistentBlockSize {
                    channel: ch,
                    expected: block_size,
                    actual: input[ch].len(),
                });
            }
            if output[ch].len() != block_size {
                return Err(RubberBandError::InconsistentBlockSize {
                    channel: ch,
                    expected: block_size,
                    actual: output[ch].len(),
                });
            }
        }

        let input_ptrs: Vec<*const f32> = input
            .iter()
            .map(|slice| slice.as_ptr())
            .collect();
        let output_ptrs: Vec<*mut f32> = output
            .iter_mut()
            .map(|slice| slice.as_mut_ptr())
            .collect();

        unsafe {
            rubberband_live_shift(
                self.state,
                input_ptrs.as_ptr(),
                output_ptrs.as_ptr(),
            );
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        unsafe {
            rubberband_live_reset(self.state);
        }
    }

    pub fn set_debug_level(&mut self, level: i32) {
        unsafe {
            rubberband_live_set_debug_level(self.state, level);
        }
    }
}

impl Drop for RubberBandLiveShifter {
    fn drop(&mut self) {
        unsafe { rubberband_live_delete(self.state) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic_creation() {
        let shifter = RubberBandLiveShifterBuilder::new(44100, 2)
            .unwrap()
            .build();

        assert_eq!(shifter.sample_rate(), 44100);
        assert_eq!(shifter.channel_count(), 2);
        assert!(shifter.block_size() > 0);
    }

    #[test]
    fn test_builder_invalid_params() {
        assert!(RubberBandLiveShifterBuilder::new(0, 2).is_err());
        assert!(RubberBandLiveShifterBuilder::new(44100, 0).is_err());
    }

    /// Check if the window option works as expected, by comparing the start delay values with the
    /// ones obtained with the C API.
    #[test]
    fn test_builder_window_option() {
        // Test start delay values for different sample rates and window options
        fn check_start_delay(sample_rate: u32, window: RubberBandLiveShifterWindow, expected_delay: u32) {
            let shifter = RubberBandLiveShifterBuilder::new(sample_rate, 1)
                .unwrap()
                .window(window)
                .build();
            assert_eq!(shifter.start_delay(), expected_delay);
        }

        // Test common sample rates with Short window
        check_start_delay(44100, RubberBandLiveShifterWindow::Short, 2112);
        check_start_delay(48000, RubberBandLiveShifterWindow::Short, 2112);
        check_start_delay(96000, RubberBandLiveShifterWindow::Short, 4160);

        // Test common sample rates with Medium window
        check_start_delay(44100, RubberBandLiveShifterWindow::Medium, 2624);
        check_start_delay(48000, RubberBandLiveShifterWindow::Medium, 2624);
        check_start_delay(96000, RubberBandLiveShifterWindow::Medium, 5184);
    }

    #[test]
    fn test_block_size() {
        // The block size should be fixed at 512 frames (samples per channel), independent of the
        // sample rate.
        for sample_rate in [16000, 44100, 48000, 96000, 192000] {
            let shifter = RubberBandLiveShifterBuilder::new(sample_rate, 1)
                .unwrap()
                .build();
            assert_eq!(shifter.block_size(), 512);
        }
    }

    #[test]
    fn test_pitch_scale() {
        let mut shifter = RubberBandLiveShifterBuilder::new(44100, 1)
            .unwrap()
            .build();

        // Test default pitch scale
        assert_eq!(shifter.pitch_scale(), 1.0);

        // Test setting and getting pitch scale
        shifter.set_pitch_scale(2.0);
        assert_eq!(shifter.pitch_scale(), 2.0);
    }

    #[test]
    fn test_formant_scale() {
        let mut shifter = RubberBandLiveShifterBuilder::new(44100, 1)
            .unwrap()
            .build();

        // Test default formant scale (formant preservation disabled)
        assert_eq!(shifter.formant_scale(), 0.0);

        // Test setting and getting formant scale
        shifter.set_formant_scale(1.5);
        assert_eq!(shifter.formant_scale(), 1.5);
    }

    #[test]
    fn test_process_invalid_channels() {
        let mut shifter = RubberBandLiveShifterBuilder::new(44100, 2)
            .unwrap()
            .build();

        let block_size = shifter.block_size() as usize;
        let input = vec![vec![0.0f32; block_size]];  // Only 1 channel for 2-channel shifter
        let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();

        assert!(matches!(
            shifter.process(&input_slices),
            Err(RubberBandError::InconsistentChannelCount { .. })
        ));
    }

    #[test]
    fn test_process_invalid_block_size() {
        let mut shifter = RubberBandLiveShifterBuilder::new(44100, 1)
            .unwrap()
            .build();

        let wrong_size = 64;  // Using arbitrary small size
        let input = vec![vec![0.0f32; wrong_size]];
        let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();

        assert!(matches!(
            shifter.process(&input_slices),
            Err(RubberBandError::InconsistentBlockSize { .. })
        ));
    }

    #[test]
    fn test_process_valid_input() {
        let mut shifter = RubberBandLiveShifterBuilder::new(44100, 1)
            .unwrap()
            .build();

        let block_size = shifter.block_size() as usize;
        let input = vec![vec![0.5f32; block_size]];
        let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();

        let result = shifter.process(&input_slices);
        assert!(result.is_ok());

        let output: Vec<Vec<f32>> = result.unwrap();
        assert_eq!(output.len(), 1);  // One channel
        assert_eq!(output[0].len(), block_size);
    }

    #[test]
    fn test_process_into() {
        let mut shifter = RubberBandLiveShifterBuilder::new(44100, 2)
            .unwrap()
            .build();

        let block_size = shifter.block_size() as usize;
        let input = vec![vec![0.5f32; block_size], vec![0.3f32; block_size]];
        let mut output = vec![vec![0.0f32; block_size], vec![0.0f32; block_size]];

        let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();
        let mut output_slices: Vec<&mut [f32]> = output.iter_mut().map(|v| v.as_mut_slice()).collect();

        assert!(shifter.process_into(&input_slices, &mut output_slices).is_ok());
    }

    #[test]
    fn test_reset() {
        let mut shifter = RubberBandLiveShifterBuilder::new(44100, 1)
            .unwrap()
            .build();

        // Process several blocks to cover the start delay
        let block_size = shifter.block_size();
        let start_delay = shifter.start_delay();
        let blocks_for_delay = (start_delay + block_size - 1) / block_size;

        let input = vec![vec![0.5f32; block_size as usize]];
        let mut output = vec![vec![0.0f32; block_size as usize]; 1];
        let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();

        for _ in 0..blocks_for_delay {
            let mut output_slices: Vec<&mut [f32]> = output.iter_mut().map(|v| v.as_mut_slice()).collect();
            shifter.process_into(&input_slices, &mut output_slices).unwrap();
        }
        assert!(!output[0].iter().all(|x| *x == 0.0));

        // After reset, the internal state is cleared and the output should be all zeros
        shifter.reset();
        {
            let mut output_slices: Vec<&mut [f32]> = output.iter_mut().map(|v| v.as_mut_slice()).collect();
            shifter.process_into(&input_slices, &mut output_slices).unwrap();
        }
        assert!(output[0].iter().all(|x| *x == 0.0));
    }

    #[test]
    fn test_pitch_shift_frequency() {
        use std::f32::consts::PI;
        let sample_rate: u32 = 44100;

        // Set the pitch scale to 2.0 (one octave up)
        let mut shifter = RubberBandLiveShifterBuilder::new(sample_rate, 1)
            .unwrap()
            .build();
        shifter.set_pitch_scale(2.0);

        // Calculate number of blocks needed to cover start delay plus some extra blocks for measurement
        let block_size = shifter.block_size() as usize;
        let start_delay = shifter.start_delay() as usize;
        let blocks_for_delay = (start_delay + block_size - 1) / block_size; // Round up division
        let measurement_blocks = 5; // Number of blocks to use for frequency measurement
        let total_blocks = blocks_for_delay + measurement_blocks;

        let mut processed_samples = Vec::with_capacity(block_size * total_blocks);

        // Process a 440Hz sine wave (A4 note)
        let frequency = 440.0;
        let omega = 2.0 * PI * frequency / sample_rate as f32;

        for block in 0..total_blocks {
            let mut input = vec![0.0f32; block_size];
            for i in 0..block_size {
                let n = block * block_size + i;
                input[i] = (omega * n as f32).sin();
            }
            let input_slice = &input[..];
            let output = shifter.process(&[input_slice]).unwrap();
            processed_samples.extend_from_slice(&output[0]);
        }

        // Count the zero-crossings in the measurement blocks
        let start_idx = blocks_for_delay * block_size;
        let end_idx = start_idx + (measurement_blocks * block_size);

        let mut first_zero_crossing = None;
        let mut last_zero_crossing = None;
        let mut zero_crossings = 0;

        for i in start_idx..end_idx {
            if processed_samples[i-1].signum() != processed_samples[i].signum() {
                if first_zero_crossing.is_none() {
                    first_zero_crossing = Some(i);
                }
                last_zero_crossing = Some(i);
                zero_crossings += 1;
            }
        }

        // Calculate frequency with the samples between first and last zero crossings
        if let (Some(first), Some(last)) = (first_zero_crossing, last_zero_crossing) {
            let total_samples = (last - first) as f32;
            let measured_frequency = ((zero_crossings - 1) as f32 / 2.0) * sample_rate as f32 / total_samples;

            // The measured frequency should be approximately 2x the input frequency
            let expected_frequency = frequency * 2.0;
            let error_cents = 1200.0 * (measured_frequency / expected_frequency).log2();
            let tolerance = 50.0; // 50 cents = 0.5 semitone
            assert!(error_cents.abs() < tolerance, "Frequency error too large: {} cents", error_cents);
        } else {
            panic!("No zero crossings found in the measurement interval");
        }
    }
}

