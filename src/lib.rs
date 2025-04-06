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

/// Window size options for [LiveShifter].
///
/// Note that this option cannot be changed once the [LiveShifter] instance is created.
///
/// # Examples
///
/// ```
/// use rubberband::{LiveShifterBuilder, LiveShifterWindow};
///
/// let mut shifter = LiveShifterBuilder::new(44100, 1)
///     .unwrap()
///     .window(LiveShifterWindow::Medium)
///     .build();
/// ```
#[derive(Debug, Clone, Copy)]
pub enum LiveShifterWindow {
    /// Short window, which is the default option.
    Short,
    /// Medium window, enabling the read ahead feature in R3 (Live Shifter) engine.
    Medium,
}

/// Formant preservation options for [LiveShifter].
///
/// This option can be set at any time.
///
/// # Examples
///
/// ```
/// use rubberband::{LiveShifterBuilder, LiveShifterFormant};
///
/// let mut shifter = LiveShifterBuilder::new(44100, 1)
///     .unwrap()
///     .formant(LiveShifterFormant::Preserved)
///     .build();
///
/// // Change the formant option
/// shifter.set_formant_option(LiveShifterFormant::Shifted);
/// ```
#[derive(Debug, Clone, Copy)]
pub enum LiveShifterFormant {
    /// No formant preservation, formants are shifted with the pitch. Default option.
    Shifted,
    /// With formant preservation, trying to preserve the formant and hence the timbre.
    Preserved,
}

/// Channel processing mode for [LiveShifter].
///
/// This option cannot be changed once the [LiveShifter] instance is created.
///
/// # Examples
///
/// ```
/// use rubberband::{LiveShifterBuilder, LiveShifterChannelMode};
///
/// let mut shifter = LiveShifterBuilder::new(44100, 1)
///     .unwrap()
///     .channel_mode(LiveShifterChannelMode::Together)
///     .build();
/// ```
#[derive(Debug, Clone, Copy)]
pub enum LiveShifterChannelMode {
    /// Process channels independently. Gives the best quality for individual channels but a more
    /// diffuse stereo image. Default option.
    Apart,
    /// Process channels together to preserve stereo image. Gives relatively less stereo space and
    /// width than the default, as well as slightly lower fidelity for individual channel content.
    Together,
}

/// Builder for configuring and creating a [LiveShifter] instance.
///
/// # Examples
///
/// ```
/// use rubberband::{
///     LiveShifterBuilder,
///     LiveShifterWindow,
///     LiveShifterFormant,
///     LiveShifterChannelMode,
/// };
///
/// let mut shifter = LiveShifterBuilder::new(44100, 1)
///     .unwrap()
///     .window(LiveShifterWindow::Medium)
///     .formant(LiveShifterFormant::Preserved)
///     .channel_mode(LiveShifterChannelMode::Apart)
///     .debug_level(1)
///     .build();
/// ```
pub struct LiveShifterBuilder {
    /// The sample rate of the audio.
    sample_rate: u32,
    /// The number of channels of the audio.
    channels: u32,
    /// The window size option of the live pitch shifter.
    window: LiveShifterWindow,
    /// The formant preservation option of the live pitch shifter.
    formant: LiveShifterFormant,
    /// The channel processing mode of the live pitch shifter.
    channel_mode: LiveShifterChannelMode,
    /// The debug level of the live pitch shifter.
    debug_level: i32,
}

impl LiveShifterBuilder {
    /// Create a new LiveShifterBuilder.
    ///
    /// # Arguments
    ///
    /// * `sample_rate`: The sample rate of the audio.
    /// * `channels`: The number of channels of the audio.
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
            window: LiveShifterWindow::Short,
            formant: LiveShifterFormant::Shifted,
            channel_mode: LiveShifterChannelMode::Apart,
            debug_level: 0,
        })
    }

    /// Set the window size option of [LiveShifter].
    ///
    /// This option cannot be changed once the [LiveShifter] instance is created.
    ///
    /// # Arguments
    ///
    /// * `window`: The window size option of the live pitch shifter.
    pub fn window(mut self, window: LiveShifterWindow) -> Self {
        self.window = window;
        self
    }

    /// Set the formant preservation option of [LiveShifter].
    ///
    /// This option can be changed even after the [LiveShifter] instance is created.
    ///
    /// # Arguments
    ///
    /// * `formant`: The formant preservation option of the live pitch shifter.
    pub fn formant(mut self, formant: LiveShifterFormant) -> Self {
        self.formant = formant;
        self
    }

    /// Set the channel processing mode of the live pitch shifter.
    ///
    /// This option cannot be changed once the [LiveShifter] instance is created.
    ///
    /// # Arguments
    ///
    /// * `channel_mode`: The channel processing mode of the live pitch shifter.
    pub fn channel_mode(mut self, channel_mode: LiveShifterChannelMode) -> Self {
        self.channel_mode = channel_mode;
        self
    }

    /// Set the debug level of the live pitch shifter.
    ///
    /// For more information, see the documentation of [LiveShifter::set_debug_level].
    ///
    /// # Arguments
    ///
    /// * `level`: The debug level of the live pitch shifter.
    pub fn debug_level(mut self, level: i32) -> Self {
        self.debug_level = level;
        self
    }

    /// Build the [LiveShifter] with the given options.
    ///
    /// # Returns
    ///
    /// A new [LiveShifter] instance.
    pub fn build(self) -> LiveShifter {
        let mut options: RubberBandLiveOption = 0; // Default options
        match self.window {
            LiveShifterWindow::Short => options |= OPTION_BITS_WINDOW_SHORT,
            LiveShifterWindow::Medium => options |= OPTION_BITS_WINDOW_MEDIUM,
        }
        match self.formant {
            LiveShifterFormant::Shifted => options |= OPTION_BITS_FORMANT_SHIFTED,
            LiveShifterFormant::Preserved => options |= OPTION_BITS_FORMANT_PRESERVED,
        }
        match self.channel_mode {
            LiveShifterChannelMode::Apart => options |= OPTION_BITS_CHANNELS_APART,
            LiveShifterChannelMode::Together => options |= OPTION_BITS_CHANNELS_TOGETHER,
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

        LiveShifter {
            state,
            process_mutex: std::sync::Mutex::new(()),
            sample_rate: self.sample_rate,
        }
    }
}

/// A real-time pitch shifter using the RubberBand audio processing library.
///
/// This is a wrapper around the C++ `RubberBandLiveShifter`, providing realtime-safe pitch
/// shifting with several options like formant preservation. The shifter API is much simpler than
/// the general RubberBand stretcher, which is capable of not only pitch shifting but also time
/// stretching.
///
/// [LiveShifter] accepts a fixed number of sample frames on each call and always returns exactly
/// the same number of sample frames. The number of frames is fixed for the lifetime of the shifter
/// and can be queried using [Self::block_size()].
///
/// While this shifter provides a shorter processing delay than the general stretcher, it is still
/// not a low-latency effect, with a delay around 50 ms between input and output signals depending
/// on configuration. The actual delay can be queried via [Self::start_delay()].
///
/// You should create the [LiveShifter] instance using the [LiveShifterBuilder].
///
/// # Thread Safety
///
/// Multiple instances of [LiveShifter] may be created and used in separate threads concurrently.
/// However, for any single instance:
///
/// - You may not call [Self::process()] or [Self::process_into()] more than once concurrently.
/// - You may not change the pitch scaling ratio (e.g., using [Self::set_pitch_scale()]) while a
///   process call is being executed.
///
/// # Examples
///
/// ```
/// use rubberband::LiveShifterBuilder;
///
/// let mut shifter = LiveShifterBuilder::new(44100, 1).unwrap().build();
/// ```
pub struct LiveShifter {
    state: *mut rubberband_sys::RubberBandLiveState_,
    process_mutex: std::sync::Mutex<()>,
    sample_rate: u32,
}

/// Error types for this crate.
#[derive(Debug, Error)]
pub enum RubberBandError {
    /// The sample rate must be greater than 0.
    #[error("Unsupported sample rate: {0}")]
    UnsupportedSampleRate(u32),

    /// The number of channels must be greater than 0.
    #[error("Unsupported channel count: {0}")]
    UnsupportedChannelCount(u32),

    /// The number of input/output channels must match the shifter's channel count.
    #[error("Inconsistent channel count: expected {expected}, got {actual}")]
    InconsistentChannelCount {
        expected: usize,
        actual: usize,
    },

    /// Each channel must have exactly the same number of samples as the shifter's block size.
    #[error("Inconsistent block size for channel {channel}: expected {expected}, got {actual}")]
    InconsistentBlockSize {
        channel: usize,
        expected: usize,
        actual: usize,
    },
}

impl LiveShifter {
    /// Get the sample rate of the [LiveShifter].
    ///
    /// # Returns
    ///
    /// The sample rate of the [LiveShifter].
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Set the pitch scale of the [LiveShifter].
    ///
    /// The pitch scale is the ratio of the target frequency to the source frequency. For example:
    ///
    /// - A ratio of 2.0 shifts up by one octave
    /// - A ratio of 0.5 shifts down by one octave
    /// - A ratio of 1.0 leaves the pitch unaffected
    ///
    /// You should not call this function concurrently with either [Self::process()] or
    /// [Self::process_into()].
    ///
    /// # Arguments
    ///
    /// * `scale`: The pitch scale of the [LiveShifter].
    ///
    /// # Examples
    ///
    /// ```
    /// use rubberband::LiveShifterBuilder;
    ///
    /// let mut shifter = LiveShifterBuilder::new(44100, 1).unwrap().build();
    ///
    /// // Shift up by one octave
    /// shifter.set_pitch_scale(2.0);
    /// ```
    pub fn set_pitch_scale(&self, scale: f64) {
        unsafe {
            rubberband_live_set_pitch_scale(self.state, scale);
        }
    }

    /// Get the pitch scale of the [LiveShifter].
    ///
    /// The pitch scale is the ratio of the target frequency to the source frequency. For example:
    ///
    /// - A ratio of 2.0 means shifted up by one octave
    /// - A ratio of 0.5 means shifted down by one octave
    /// - A ratio of 1.0 means no pitch shift
    ///
    /// # Returns
    ///
    /// The pitch scale of the [LiveShifter].
    ///
    /// # Examples
    ///
    /// ```
    /// use rubberband::LiveShifterBuilder;
    ///
    /// let mut shifter = LiveShifterBuilder::new(44100, 1).unwrap().build();
    ///
    /// // Initially no pitch shift
    /// assert_eq!(shifter.pitch_scale(), 1.0);
    ///
    /// // Shift up by one octave
    /// shifter.set_pitch_scale(2.0);
    /// assert_eq!(shifter.pitch_scale(), 2.0);
    /// ```
    pub fn pitch_scale(&self) -> f64 {
        unsafe {
            rubberband_live_get_pitch_scale(self.state)
        }
    }

    /// Set the pitch shift in semitones.
    ///
    /// A positive value shifts the pitch up, while a negative value shifts it down.
    /// One semitone is 1/12th of an octave.
    ///
    /// You should not call this function concurrently with either [Self::process()] or
    /// [Self::process_into()].
    ///
    /// # Arguments
    ///
    /// * `semitones`: The number of semitones to shift the pitch by. Positive values shift up,
    ///                negative values shift down.
    ///
    /// # Examples
    ///
    /// ```
    /// use rubberband::LiveShifterBuilder;
    ///
    /// let mut shifter = LiveShifterBuilder::new(44100, 1).unwrap().build();
    ///
    /// // Shift up by one octave (12 semitones)
    /// shifter.set_pitch_semitone(12.0);
    ///
    /// // Shift down by one semitone
    /// shifter.set_pitch_semitone(-1.0);
    /// ```
    pub fn set_pitch_semitone(&self, semitones: f64) {
        let scale = 2.0f64.powf(semitones / 12.0);
        self.set_pitch_scale(scale);
    }

    /// Get the current pitch shift in semitones.
    ///
    /// A positive value indicates pitch shifted up, while a negative value indicates pitch shifted
    /// down. One semitone is 1/12th of an octave.
    ///
    /// # Returns
    ///
    /// The current pitch shift in semitones.
    ///
    /// # Examples
    ///
    /// ```
    /// use approx::assert_abs_diff_eq;
    /// use rubberband::LiveShifterBuilder;
    ///
    /// let mut shifter = LiveShifterBuilder::new(44100, 1).unwrap().build();
    ///
    /// // Initially no pitch shift
    /// assert_eq!(shifter.pitch_semitone(), 0.0);
    ///
    /// // Set pitch shift to one octave up
    /// shifter.set_pitch_semitone(12.0);
    /// assert_abs_diff_eq!(shifter.pitch_semitone(), 12.0, epsilon = 1e-6);
    /// ```
    pub fn pitch_semitone(&self) -> f64 {
        // Convert pitch ratio to semitones: semitones = 12 * log2(ratio)
        12.0 * self.pitch_scale().log2()
    }

    /// Set the pitch shift in cents.
    ///
    /// A positive value shifts the pitch up, while a negative value shifts it down.
    /// One cent is 1/100th of a semitone, or 1/1200th of an octave.
    ///
    /// You should not call this function concurrently with either [Self::process()] or
    /// [Self::process_into()].
    ///
    /// # Arguments
    ///
    /// * `cents`: The number of cents to shift the pitch by. Positive values shift up,
    ///            negative values shift down.
    ///
    /// # Examples
    ///
    /// ```
    /// use rubberband::LiveShifterBuilder;
    ///
    /// let mut shifter = LiveShifterBuilder::new(44100, 1).unwrap().build();
    ///
    /// // Fine-tune up by 5 cents
    /// shifter.set_pitch_cent(5.0);
    ///
    /// // Fine-tune down by 2 cents
    /// shifter.set_pitch_cent(-2.0);
    /// ```
    pub fn set_pitch_cent(&self, cents: f64) {
        // Convert cents to pitch ratio: ratio = 2^(cents/1200)
        let scale = 2.0f64.powf(cents / 1200.0);
        self.set_pitch_scale(scale);
    }

    /// Get the current pitch shift in cents.
    ///
    /// A positive value indicates pitch shifted up, while a negative value indicates pitch shifted
    /// down. One cent is 1/100th of a semitone, or 1/1200th of an octave.
    ///
    /// # Returns
    ///
    /// The current pitch shift in cents.
    ///
    /// # Examples
    ///
    /// ```
    /// use approx::assert_abs_diff_eq;
    /// use rubberband::LiveShifterBuilder;
    ///
    /// let mut shifter = LiveShifterBuilder::new(44100, 1).unwrap().build();
    ///
    /// // Fine-tune up by 5 cents
    /// shifter.set_pitch_cent(105.0);
    /// assert_abs_diff_eq!(shifter.pitch_cent(), 105.0, epsilon = 1e-6);
    /// ```
    pub fn pitch_cent(&self) -> f64 {
        // Convert pitch ratio to cents: cents = 1200 * log2(ratio)
        1200.0 * self.pitch_scale().log2()
    }

    // TODO Check if it is safe to call `set_pitch_scale` concurrently with `shift` (RubberBand
    //      docs don't mention this). Considering the fact that `set_formant_option` is thread-safe,
    //      this is probably safe, too.

    /// Set the formant scale of the [LiveShifter].
    ///
    /// This sets a pitch scale for the vocal formant envelope separately from the overall pitch scale.
    /// By default (when set to 0.0), the scale is calculated automatically:
    ///
    /// - If formant preservation is enabled, it will be treated as 1.0 / the pitch scale
    /// - If formant shifting is enabled, it will be treated as 1.0
    ///
    /// Setting this to a value other than 0.0 will override the automatic behavior and force formant
    /// shifting regardless of the formant preservation option.
    ///
    /// This function is provided for special effects only. You do not need to call it for ordinary
    /// pitch shifting - just use the formant preservation option as appropriate.
    ///
    /// # Arguments
    ///
    /// * `scale`: The formant scale of the [LiveShifter].
    pub fn set_formant_scale(&self, scale: f64) {
        unsafe {
            rubberband_live_set_formant_scale(self.state, scale);
        }
    }

    /// Get the formant scale of the [LiveShifter].
    ///
    /// Returns the last formant scaling ratio that was set with `set_formant_scale()`, or 0.0 if
    /// the default automatic scaling is in effect.
    ///
    /// # Returns
    ///
    /// The formant scale of the [LiveShifter].
    pub fn formant_scale(&self) -> f64 {
        unsafe {
            rubberband_live_get_formant_scale(self.state)
        }
    }

    // TODO According to the RubberBand docs, it is safe to call `set_formant_option` concurrently
    //      with `shift`.

    /// Set the formant option of the [LiveShifter].
    ///
    /// It is safe to call this function even if the [Self::process()] or [Self::process_into()] is
    /// running.
    ///
    /// # Arguments
    ///
    /// * `option`: The formant option of the [LiveShifter].
    ///
    /// # Examples
    ///
    /// ```
    /// use rubberband::{LiveShifterBuilder, LiveShifterFormant};
    ///
    /// let mut shifter = LiveShifterBuilder::new(44100, 1).unwrap().build();
    ///
    /// // Change the formant option
    /// shifter.set_formant_option(LiveShifterFormant::Preserved);
    /// ```
    pub fn set_formant_option(&self, option: LiveShifterFormant) {
        let option_bits = match option {
            LiveShifterFormant::Shifted => OPTION_BITS_FORMANT_SHIFTED,
            LiveShifterFormant::Preserved => OPTION_BITS_FORMANT_PRESERVED,
        };
        unsafe {
            rubberband_live_set_formant_option(
                self.state,
                option_bits as RubberBandLiveOptions,
            );
        }
    }

    /// Get the start delay (in samples) of the [LiveShifter].
    ///
    /// This is the number of samples that one should discard at the start of the output, in order
    /// to align the output with the input.
    ///
    /// # Returns
    ///
    /// The start delay of the [LiveShifter].
    pub fn start_delay(&self) -> u32 {
        unsafe {
            rubberband_live_get_start_delay(self.state)
        }
    }

    /// Get the number of channels of the [LiveShifter].
    ///
    /// # Returns
    ///
    /// The number of channels of the [LiveShifter].
    pub fn channel_count(&self) -> u32 {
        unsafe {
            rubberband_live_get_channel_count(self.state)
        }
    }

    /// Get the block size (in samples) of the [LiveShifter].
    ///
    /// # Returns
    ///
    /// The block size of the [LiveShifter].
    pub fn block_size(&self) -> u32 {
        unsafe {
            rubberband_live_get_block_size(self.state)
        }
    }

    /// Process a single block of audio samples.
    ///
    /// An output buffer will be automatically allocated and returned.
    ///
    /// # Arguments
    ///
    /// * `input`: A slice of slices of floats, each containing a contiguous block of audio
    ///            samples for a single channel. The number of channels must be equal to the number
    ///            of channels of the [LiveShifter]. Each channel must have the same number of
    ///            samples.
    ///
    /// # Returns
    ///
    /// A vector of vectors of floats, each containing a contiguous block of audio samples for a
    /// single channel. The number of samples per channel will be equal to `block_size()`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The number of input channels doesn't match the shifter's channel count
    /// - The number of samples in any channel doesn't match the shifter's block size
    pub fn process(&self, input: &[&[f32]]) -> Result<Vec<Vec<f32>>, RubberBandError> {
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
    /// # Arguments
    ///
    /// * `input`: A slice of slices of floats, each containing a contiguous block of audio
    ///            samples for a single channel. The number of channels must be equal to the number
    ///            of channels of the [LiveShifter]. Each channel must have the same number of
    ///            samples.
    ///
    /// * `output`: A slice of slices of floats, should have the same shape as the input. Each
    ///             channel must have exactly the same number of samples as the shifter's block size.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The number of input/output channels doesn't match the shifter's channel count
    /// - The number of samples in any channel doesn't match the shifter's block size
    pub fn process_into(&self, input: &[&[f32]], output: &mut [&mut [f32]]) -> Result<(), RubberBandError> {
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

    // TODO Check if it is safe to call `reset` concurrently with `shift`.

    /// Reset the internal state of the [LiveShifter].
    ///
    /// Note that this function would not affect the parameters. Instead, it just make the live
    /// shifter forget the previous input and output.
    pub fn reset(&self) {
        unsafe {
            rubberband_live_reset(self.state);
        }
    }

    /// Set the debug level of the [LiveShifter].
    ///
    /// According to the RubberBand documentation, the supported values are:
    ///
    /// - 0: Report errors only.
    /// - 1: Report some information on construction and ratio change. Nothing is reported during
    ///   normal processing unless something changes.
    /// - 2: Report a significant amount of information about ongoing calculations during normal
    ///   processing.
    ///
    /// Note that only level 0 is realtime-safe.
    ///
    /// # Arguments
    ///
    /// * `level`: The debug level of the live pitch shifter.
    pub fn set_debug_level(&self, level: i32) {
        unsafe {
            rubberband_live_set_debug_level(self.state, level);
        }
    }
}

impl Drop for LiveShifter {
    fn drop(&mut self) {
        unsafe { rubberband_live_delete(self.state) };
    }
}

unsafe impl Send for LiveShifter {}
unsafe impl Sync for LiveShifter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_invalid_params() {
        assert!(LiveShifterBuilder::new(0, 2).is_err());
        assert!(LiveShifterBuilder::new(44100, 0).is_err());
    }

    /// Check if the window option works as expected, by comparing the start delay values with the
    /// ones obtained with the C API.
    #[test]
    fn test_builder_window_option() {
        // Test start delay values for different sample rates and window options
        fn check_start_delay(sample_rate: u32, window: LiveShifterWindow, expected_delay: u32) {
            let shifter = LiveShifterBuilder::new(sample_rate, 1)
                .unwrap()
                .window(window)
                .build();
            assert_eq!(shifter.start_delay(), expected_delay);
        }

        // Test common sample rates with Short window
        check_start_delay(44100, LiveShifterWindow::Short, 2112);
        check_start_delay(48000, LiveShifterWindow::Short, 2112);
        check_start_delay(96000, LiveShifterWindow::Short, 4160);

        // Test common sample rates with Medium window
        check_start_delay(44100, LiveShifterWindow::Medium, 2624);
        check_start_delay(48000, LiveShifterWindow::Medium, 2624);
        check_start_delay(96000, LiveShifterWindow::Medium, 5184);
    }

    #[test]
    fn test_block_size() {
        // The block size should be fixed at 512 frames (samples per channel), independent of the
        // sample rate.
        for sample_rate in [16000, 44100, 48000, 96000, 192000] {
            let shifter = LiveShifterBuilder::new(sample_rate, 1)
                .unwrap()
                .build();
            assert_eq!(shifter.block_size(), 512);
        }
    }

    #[test]
    fn test_process_invalid_channels() {
        let shifter = LiveShifterBuilder::new(44100, 2)
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
        let shifter = LiveShifterBuilder::new(44100, 1)
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
        let shifter = LiveShifterBuilder::new(44100, 1)
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
        let shifter = LiveShifterBuilder::new(44100, 2)
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
        let shifter = LiveShifterBuilder::new(44100, 1)
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
        let shifter = LiveShifterBuilder::new(sample_rate, 1)
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

