//! A Rust binding for the Rubber Band audio processing library.
//!
//! The original Rubber Band C++ library provides two different APIs:
//!
//! - `RubberBandStretcher`: A general-purpose time stretching and pitch shifting processor capable
//!   of variable input/output sizes.
//! - `RubberBandLiveShifter`: A real-time pitch shifter (no time stretching) designed for
//!   fixed-size block processing with minimal latency.
//!
//! **This crate currently only implements bindings for the `RubberBandLiveShifter` API.**
//!
//! ## RubberBand Live Shifter
//!
//! The [LiveShifter] struct wraps the `RubberBandLiveShifter` C++ class. It provides
//! realtime-safe pitch shifting suitable for live audio processing or applications where
//! consistent block-based processing is required.
//!
//! Key characteristics:
//!
//! *   **Real-time Pitch Shifting:** Designed specifically for changing pitch without altering tempo.
//! *   **Fixed Block Size:** Processes audio in constant-size blocks. The required block size can be
//!     queried with [LiveShifter::block_size()]. This size is fixed for the lifetime of the shifter.
//! *   **Inherent Latency:** While optimized for speed compared to the general stretcher, it still
//!     introduces a processing delay (typically >50ms depending on configuration). Use
//!     [LiveShifter::start_delay()] to get the exact latency in samples required to align input and output.
//! *   **Configuration:** Options like window size, formant preservation, and channel processing
//!     mode can be configured using the [LiveShifterBuilder]. Note that some options (like window
//!     size and channel mode) cannot be changed after the shifter is built.
//!
//! See the [LiveShifter] and [LiveShifterBuilder] documentation for more details and usage examples.
//!
//! ## Future Work
//!
//! Bindings for the `RubberBandStretcher` API may be added in the future.

use std::sync::atomic::Ordering;
use atomic_float::AtomicF64;
use parking_lot::Mutex;
use thiserror::Error;
use std::sync::atomic::AtomicBool;

use rubberband_sys::{
    rubberband_live_new,
    rubberband_live_delete,
    rubberband_live_set_debug_level,
    rubberband_live_set_pitch_scale,
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
/// Note that this option **cannot** be changed once the [LiveShifter] instance is created.
/// It must be set via the [LiveShifterBuilder].
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
/// This option can be set at any time using [LiveShifter::set_formant_option()] or
/// initially via the [LiveShifterBuilder].
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
/// Note that this option **cannot** be changed once the [LiveShifter] instance is created.
/// It must be set via the [LiveShifterBuilder].
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
/// Provides methods to set options like window size, formant preservation, channel processing mode,
/// and debug level before constructing the `LiveShifter`.
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
    /// Initializes the builder with default options:
    /// - Window: [LiveShifterWindow::Short]
    /// - Formant: [LiveShifterFormant::Shifted]
    /// - Channel Mode: [LiveShifterChannelMode::Apart]
    /// - Debug Level: 0
    ///
    /// # Arguments
    ///
    /// * `sample_rate`: The sample rate of the audio (must be > 0).
    /// * `channels`: The number of channels of the audio (must be > 0).
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
    /// This option **cannot** be changed once the [LiveShifter] instance is created.
    /// Defaults to [LiveShifterWindow::Short].
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
    /// This option can be changed later using [LiveShifter::set_formant_option()].
    /// Defaults to [LiveShifterFormant::Shifted].
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
    /// This option **cannot** be changed once the [LiveShifter] instance is created.
    /// Defaults to [LiveShifterChannelMode::Apart].
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
    /// The default is 0. The higher the level, the more verbose the output.  See the C++
    /// documentation for `RubberBandLiveShifter::setDebugLevel` for details on the levels.
    /// Only level 0 is guaranteed realtime-safe.
    ///
    /// This option cannot be changed after the shifter is built.
    ///
    /// # Arguments
    ///
    /// * `level`: The debug level of the live pitch shifter.
    pub fn debug_level(mut self, level: i32) -> Self {
        self.debug_level = level;
        self
    }

    /// Build the [LiveShifter] with the configured options.
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
            mutex: Mutex::new(()),
            sample_rate: self.sample_rate,
            pitch_scale: AtomicF64::new(1.0),
            pitch_dirty: AtomicBool::new(false),
        }
    }
}

/// A real-time pitch shifter using the RubberBand audio processing library.
///
/// This struct wraps the C++ `RubberBandLiveShifter`, providing realtime-safe pitch shifting with
/// options like formant preservation. It processes audio in fixed-size blocks, which can be
/// determined by [block_size()](Self::block_size()).
///
/// While optimized for lower latency compared to the general RubberBand stretcher, it still
/// introduces a delay. Use [start_delay()](Self::start_delay()) to query this latency.
///
/// Create instances using the [LiveShifterBuilder].
///
/// # Thread Safety
///
/// > TL;DR:
/// > - This wrapper guarantees that it is safe to call any method concurrently with
/// [process](Self::process()) or [process_into](Self::process_into()) on the same instance.
/// > - It is generally safe to call other methods concurrently, but it is not guaranteed.
///
/// This type implements `Send` and `Sync`.
///
/// The thread safety relies on a combination of features from the underlying C++ library and
/// synchronization primitives added in this Rust wrapper.
///
/// - **Instance Creation:** Multiple instances can be created and used concurrently in different
///   threads, as guaranteed by the C++ library.
/// - **Processing (`process`, `process_into`):** The underlying C++ `shift` function is **not**
///   safe for concurrent calls on the same instance. This wrapper uses an internal `Mutex` to
///   ensure that only one call to `process`, `process_into`, `reset`, or `start_delay` can execute
///   at a time on a single `LiveShifter` instance. Concurrent calls will block or return
///   [`OperationInProgress`](RubberBandError::OperationInProgress).
/// - **Pitch Changes (`set_pitch_scale`, `set_pitch_semitone`, `set_pitch_cent`):** The C++
///   `setPitchScale` function is **not** safe to call concurrently with `shift`. This wrapper
///   uses atomic variables to store the desired pitch scale immediately without locking the main
///   mutex, making these Rust methods safe to call concurrently. The new pitch scale will not
///   take effect until the next `process_into` or `start_delay` call.
/// - **Formant Changes (`set_formant_scale`, `set_formant_option`):** The underlying C++ library
///   guarantees that `setFormantScale` and `setFormantOption` are safe to call concurrently with
///   processing. Therefore, these Rust methods can also be called concurrently.
/// - **State Query:**
///   - `pitch_scale`: The thread-safety is guaranteed by this Rust wrapper.
///   - `start_delay`: The thread-safety is guaranteed by this Rust wrapper, but it may cause the
///     processing call to fail (gracefully) if called concurrently.
///   - `formant_scale`, `channel_count`, `block_size`, etc.: Thread-safe in the C++ library.
/// - **State Reset (`reset`):** These methods acquire the same internal mutex as the
///   processing methods to ensure safe state modification or query, and are subject to the same
///   concurrency limitations as `process`.
///
/// # Examples
///
/// ```
/// use rubberband::LiveShifterBuilder;
///
/// // Create a shifter for stereo audio at 48kHz
/// let mut shifter = LiveShifterBuilder::new(48000, 2).unwrap().build();
///
/// // Set pitch shift up by 2 semitones
/// shifter.set_pitch_semitone(2.0);
///
/// // Get required block size
/// let block_size = shifter.block_size() as usize;
///
/// // Prepare input and output buffers (example with dummy data)
/// let input_ch1: Vec<f32> = vec![0.1; block_size];
/// let input_ch2: Vec<f32> = vec![-0.1; block_size];
/// let input_buffers: [&[f32]; 2] = [&input_ch1, &input_ch2];
///
/// let mut output_ch1: Vec<f32> = vec![0.0; block_size];
/// let mut output_ch2: Vec<f32> = vec![0.0; block_size];
/// let mut output_buffers: [&mut [f32]; 2] = [&mut output_ch1, &mut output_ch2];
///
/// // Process the audio
/// assert!(shifter.process_into(&input_buffers, &mut output_buffers).is_ok());
/// // Output buffers now contain the shifted audio
/// ```
pub struct LiveShifter {
    state: *mut rubberband_sys::RubberBandLiveState_,
    mutex: Mutex<()>,
    sample_rate: u32,
    pitch_scale: AtomicF64,
    pitch_dirty: AtomicBool,
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

    /// An operation (process or reset) is already in progress.
    #[error("Operation (process or reset) already in progress")]
    OperationInProgress,
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
    /// The pitch scale is the ratio of the target frequency to the source frequency (e.g., 2.0 for
    /// one octave up, 0.5 for one octave down, 1.0 for no change).
    ///
    /// This method uses atomic operations and is safe to call concurrently with processing or
    /// other methods. The change will take effect on the next processing call.
    ///
    /// # Arguments
    ///
    /// * `scale`: The desired pitch scale (ratio).
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
        self.pitch_scale.store(scale, Ordering::Relaxed);
        self.pitch_dirty.store(true, Ordering::Relaxed);
    }

    /// Get the current target pitch scale of the [LiveShifter].
    ///
    /// Note that the actual pitch scale applied during processing might slightly lag if
    /// `set_pitch_scale` was called very recently from another thread.
    ///
    /// # Returns
    ///
    /// The current target pitch scale ratio.
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
        self.pitch_scale.load(Ordering::Relaxed)
    }

    /// Set the pitch shift in semitones.
    ///
    /// A positive value shifts the pitch up, a negative value shifts it down.
    /// This is a convenience method that calculates the appropriate scale factor and calls
    /// [set_pitch_scale()](Self::set_pitch_scale()).
    ///
    /// This method uses atomic operations internally (via `set_pitch_scale`) and is safe to call
    /// concurrently with processing or other methods.
    ///
    /// # Arguments
    ///
    /// * `semitones`: The number of semitones to shift by.
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
    /// Calculates the shift based on the current value returned by [pitch_scale()](Self::pitch_scale()).
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
    /// A positive value shifts the pitch up, a negative value shifts it down (100 cents = 1 semitone).
    /// This is a convenience method that calculates the appropriate scale factor and calls
    /// [set_pitch_scale()](Self::set_pitch_scale()).
    ///
    /// This method uses atomic operations internally (via `set_pitch_scale`) and is safe to call
    /// concurrently with processing or other methods.
    ///
    /// # Arguments
    ///
    /// * `cents`: The number of cents to shift by.
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
    /// Calculates the shift based on the current value returned by [pitch_scale()](Self::pitch_scale()).
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

    /// Set the formant scale of the [LiveShifter].
    ///
    /// This adjusts the vocal formant envelope independently of the main pitch scale.
    ///
    /// - A value of `0.0` (the default) enables automatic formant scaling based on the
    ///   [LiveShifterFormant] option:
    ///   - `Preserved`: Scale is `1.0 / pitch_scale`.
    ///   - `Shifted`: Scale is `1.0`.
    /// - Setting any other value forces formant shifting with that specific scale, overriding the
    ///   `LiveShifterFormant` option.
    ///
    /// This is typically used for special effects. For standard formant preservation, use
    /// [LiveShifterBuilder::formant()] or [set_formant_option()](Self::set_formant_option()) instead.
    ///
    /// This method is thread-safe and can be called concurrently with processing.
    ///
    /// # Arguments
    ///
    /// * `scale`: The desired formant scale, or `0.0` for automatic behavior.
    pub fn set_formant_scale(&self, scale: f64) {
        unsafe {
            rubberband_live_set_formant_scale(self.state, scale);
        }
    }

    /// Get the currently set formant scale of the [LiveShifter].
    ///
    /// Returns `0.0` if automatic scaling (based on the [LiveShifterFormant] option) is active.
    /// Otherwise, returns the value explicitly set by [set_formant_scale()](Self::set_formant_scale()).
    ///
    /// This method is thread-safe.
    ///
    /// # Returns
    ///
    /// The explicitly set formant scale, or `0.0` for automatic.
    pub fn formant_scale(&self) -> f64 {
        unsafe {
            rubberband_live_get_formant_scale(self.state)
        }
    }

    /// Set the formant preservation option of the [LiveShifter].
    ///
    /// Allows changing whether formants are shifted with the pitch or preserved after the
    /// shifter has been created.
    ///
    /// This method is thread-safe and can be called concurrently with processing.
    ///
    /// # Arguments
    ///
    /// * `option`: The desired [LiveShifterFormant] option.
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

    /// Get the start delay (in samples per channel) of the [LiveShifter].
    ///
    /// This indicates how many samples should be discarded from the beginning of the output
    /// to align it temporally with the input signal. The delay depends on the sample rate,
    /// window settings, and the pitch scale.
    ///
    /// **Note:** This method acquires the internal processing lock. Calling it concurrently with
    /// [process()](Self::process()) or [process_into()](Self::process_into()) on the same instance
    /// will block or may cause the processing call to fail with [RubberBandError::OperationInProgress].
    /// It's best to call this when the shifter is idle or from the same thread that calls process.
    ///
    /// # Returns
    ///
    /// The start delay in samples per channel.
    pub fn start_delay(&self) -> u32 {
        let _guard = self.mutex.lock();
        unsafe {
            if self.pitch_dirty.load(Ordering::Relaxed) {
                rubberband_live_set_pitch_scale(self.state, self.pitch_scale.load(Ordering::Relaxed));
                self.pitch_dirty.store(false, Ordering::Relaxed);
            }
            rubberband_live_get_start_delay(self.state)
        }
    }

    /// Get the number of channels the [LiveShifter] was configured for.
    ///
    /// This method is thread-safe.
    ///
    /// # Returns
    ///
    /// The number of audio channels.
    pub fn channel_count(&self) -> u32 {
        unsafe {
            rubberband_live_get_channel_count(self.state)
        }
    }

    /// Get the required block size (in samples per channel) for processing.
    ///
    /// Both [process()](Self::process()) and [process_into()](Self::process_into()) require input
    /// buffers and produce output buffers of exactly this size for each channel.
    /// This value is fixed for the lifetime of the shifter instance.
    ///
    /// This method is thread-safe.
    ///
    /// # Returns
    ///
    /// The required block size in samples per channel.
    pub fn block_size(&self) -> u32 {
        unsafe {
            rubberband_live_get_block_size(self.state)
        }
    }

    /// Process a single block of audio samples, allocating and returning the output.
    ///
    /// This is a convenience wrapper around [process_into()](Self::process_into()).
    ///
    /// # Arguments
    ///
    /// * `input`: A slice of slices (`&[&[f32]]`), where each inner slice represents one channel
    ///            of audio data.
    ///   - The number of inner slices must equal [channel_count()](Self::channel_count()).
    ///   - The length of each inner slice must equal [block_size()](Self::block_size()).
    ///
    /// # Returns
    ///
    /// A `Vec<Vec<f32>>` containing the processed audio data, with the same channel count and
    /// block size as the input.
    ///
    /// # Errors
    ///
    /// Returns [RubberBandError] if:
    /// - Input channel count or block size is incorrect ([`InconsistentChannelCount`](RubberBandError::InconsistentChannelCount), [`InconsistentBlockSize`](RubberBandError::InconsistentBlockSize)).
    /// - A concurrent call to `process`, `process_into`, `reset`, or `start_delay` is in progress
    ///   on the same instance ([`OperationInProgress`](RubberBandError::OperationInProgress)).
    pub fn process(&self, input: &[&[f32]]) -> Result<Vec<Vec<f32>>, RubberBandError> {
        let mut output = vec![vec![0.0; input[0].len()]; input.len()];
        let mut output_slices: Vec<&mut [f32]> = output
            .iter_mut()
            .map(|slice| slice.as_mut_slice())
            .collect();
        self.process_into(input, &mut output_slices)?;
        Ok(output)
    }

    /// Process a single block of audio samples using pre-allocated output buffers.
    ///
    /// This is the primary processing method and avoids allocations. It wraps the underlying
    /// `shift` C++ method, adding checks and handling pitch scale updates.
    ///
    /// The input and output buffers must not alias or overlap.
    ///
    /// # Arguments
    ///
    /// * `input`: A slice of slices (`&[&[f32]]`) representing the input audio block.
    ///   - Must have `channel_count` inner slices.
    ///   - Each inner slice must have `block_size` samples.
    /// * `output`: A mutable slice of mutable slices (`&mut [&mut [f32]]`) for the output.
    ///   - Must have `channel_count` inner slices.
    ///   - Each inner slice must have `block_size` samples. The contents will be overwritten.
    ///
    /// # Errors
    ///
    /// Returns [RubberBandError] if:
    /// - Input/output channel count or block size is incorrect ([`InconsistentChannelCount`](RubberBandError::InconsistentChannelCount), [`InconsistentBlockSize`](RubberBandError::InconsistentBlockSize)).
    /// - A concurrent call to `process`, `process_into`, `reset`, or `start_delay` is in progress
    ///   on the same instance ([`OperationInProgress`](RubberBandError::OperationInProgress)).
    pub fn process_into(&self, input: &[&[f32]], output: &mut [&mut [f32]]) -> Result<(), RubberBandError> {
        // The underlying C++ implementation does not allow concurrent calls to `shift()`.
        let _guard = self.mutex.try_lock();
        if _guard.is_none() {
            return Err(RubberBandError::OperationInProgress);
        }

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
            if self.pitch_dirty.load(Ordering::Relaxed) {
                rubberband_live_set_pitch_scale(self.state, self.pitch_scale.load(Ordering::Relaxed));
                self.pitch_dirty.store(false, Ordering::Relaxed);
            }
            rubberband_live_shift(
                self.state,
                input_ptrs.as_ptr(),
                output_ptrs.as_ptr(),
            );
        }

        Ok(())
    }

    /// Reset the internal state of the [LiveShifter].
    ///
    /// This clears the internal buffers and history, effectively making the shifter behave as if
    /// it were newly created, but retaining all parameter settings (pitch scale, formant options, etc.).
    ///
    /// **Note:** This method acquires the internal processing lock. Calling it concurrently with
    /// [process()](Self::process()) or [process_into()](Self::process_into()) on the same instance
    /// will block.
    pub fn reset(&self) {
        let _guard = self.mutex.lock();
        unsafe {
            rubberband_live_reset(self.state);
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

