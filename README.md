# rubberband-rs

A Rust binding for [Rubber Band](https://breakfastquay.com/rubberband/), a high-quality library for audio time-stretching and pitch-shifting. This crate is still under development.

## Current Status

This crate currently provides a binding for the `RubberBandLiveShifter` API only. The more general `RubberBandStretcher` API, which supports both time-stretching and pitch-shifting, is not yet implemented.

## Features

### Original Rubber Band Features

- High-quality and real-time safe pitch shifting algorithm
- Thread safety for multiple instances in separate threads
- Formant preservation for natural-sounding pitch shifts (without changing the timbre)
- Configurable window size options for different latency/quality trade-offs
- Channel processing modes for stereo image/fidelity trade-offs

### Rust Binding Features

- Safe and idiomatic Rust API
- Builder pattern for easy configuration
- Support setting pitch shift amount in semitones or cents
- Comprehensive error handling
- (TODO) Thread-safe parameter changes while processing

## Usage

```rust
use rubberband::{LiveShifterBuilder, LiveShifterFormant, LiveShifterWindow};

// Create a pitch shifter with formant preservation
let mut shifter = LiveShifterBuilder::new(44100, 2)
    .unwrap()
    .window(LiveShifterWindow::Medium)
    .formant(LiveShifterFormant::Preserved)
    .build();

// Shift up by 3 semitones
shifter.set_pitch_semitone(3.0);

// Process audio blocks
let block_size = shifter.block_size() as usize;
let input = vec![vec![0.0f32; block_size], vec![0.0f32; block_size]];
let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();
let output = shifter.process(&input_slices).unwrap();
```

## Thread Safety Considerations

As a Rust binding, the `LiveShifter` struct inherits the following thread safety properties guaranteed in the original Rubber Band documentation:

1. Multiple instances of `LiveShifter` may be created and used in separate threads concurrently.

2. It is safe to call `set_formant_option()` concurrently with `process()` or `process_into()`.

On the other hand, you still need to take care of the following:

1. For any single instance:

   - You may not call `process()` or `process_into()` more than once concurrently.
   - You may not change the pitch scaling ratio (using `set_pitch_scale()`, `set_pitch_semitone()`, or `set_pitch_cent()`) while a process call is being executed.

2. The thread safety of the following methods has not been mentioned in the original documentation and needs further investigation:

   - `set_formant_scale()`
   - `reset()`

## Performance Considerations

Although `LiveShifter` has a lower latency than the general `RubberBandStretcher`, it is not a low-latency effect, with a delay of about 50 ms between input and output signals depending on configuration. The actual delay can be queried via `start_delay()`

## To-do

- [ ] Ensure the thread-safety of `LiveShifter`.
- [ ] Implement `Stretcher` struct for `RubberBandStretcher` class.
- [ ] Add tests for `Stretcher`.

## License

This project is licensed under the GNU General Public License v2.0 or later. See the [LICENSE](LICENSE) file for details.

Note that this is a binding to the [Rubber Band](https://breakfastquay.com/rubberband/) library, which is also licensed under the GNU General Public License v2.0 or later.