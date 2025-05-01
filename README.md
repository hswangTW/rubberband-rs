# rubberband-rs

A Rust binding for [Rubber Band](https://breakfastquay.com/rubberband/), a high-quality library for audio time-stretching and pitch-shifting.

## Current Status

This crate currently provides bindings for the `RubberBandLiveShifter` API only. The `LiveShifter` provides real-time pitch shifting with fixed block sizes and inherent latency. The bindings aim to be safe and idiomatic Rust.

The more general `RubberBandStretcher` API, which supports both time-stretching and pitch-shifting with variable I/O sizes, is not yet implemented.

## Installation

As this crate is not yet published on crates.io, you need to add it as a git dependency in your `Cargo.toml`. It is highly recommended to pin it to a specific tag or commit hash for reproducible builds:

```toml
[dependencies]
rubberband = { git = "https://github.com/hswangTW/rubberband-rs.git", tag = "v0.2.0" }
```

### Build Requirements

The underlying `rubberband-sys` crate automatically builds the required version of the Rubber Band C++ library using its provided single-file source (`RubberBandSingle.cpp`). This means you do **not** need to install Rubber Band separately, nor do you need CMake or any other build system. However, you will still need a compatible **C++ compiler** (like Clang or GCC) installed on your system for the build process to succeed.

> [!NOTE]
>
> Currently, this crate and its build process have been tested primarily on macOS Sonoma 14.5 (Apple Silicon M3).

## Features

### Original Rubber Band Live Shifter Features

- High-quality real-time pitch shifting algorithm.
- Formant preservation for natural-sounding pitch shifts (preserving timbre).
- Configurable window size options (`Short`, `Medium`) for latency/quality trade-offs.
- Channel processing modes (`Apart`, `Together`) for stereo image/fidelity trade-offs.
- Thread safety allows multiple `LiveShifter` instances to be used concurrently in separate threads.

### Rust Binding Features (`LiveShifter`)

- Safe and idiomatic Rust API (`LiveShifter`, `LiveShifterBuilder`).
- Builder pattern for easy configuration.
- Support setting pitch shift amount in semitones or cents.
- Comprehensive error handling (`RubberBandError`).
- Thread-safe implementation (`Send + Sync`).
  - `set_pitch_scale` can be safely called concurrently with processing. (This may cause a crash in the original library.)
  - Processing calls (`process`, `process_into`) on the *same instance* are made mutually exclusive; concurrent calls will immediately return an `OperationInProgress` error instead of blocking.
  - See the `LiveShifter` documentation's "Thread Safety" section for detailed guarantees.

## Usage

```rust
use rubberband::{LiveShifterBuilder, LiveShifterFormant, LiveShifterWindow};

// Create a pitch shifter for stereo audio at 44.1kHz
// Use medium window and enable formant preservation
let mut shifter = LiveShifterBuilder::new(44100, 2)
    .unwrap()
    .window(LiveShifterWindow::Medium)
    .formant(LiveShifterFormant::Preserved)
    .build();

// Shift up by 3 semitones
shifter.set_pitch_semitone(3.0);

// Get the required block size for processing
let block_size = shifter.block_size() as usize;

// Prepare input buffers (example with dummy data)
let input_ch1: Vec<f32> = vec![0.1; block_size];
let input_ch2: Vec<f32> = vec![-0.1; block_size];
let input_buffers: [&[f32]; 2] = [&input_ch1, &input_ch2];

// Prepare output buffers (pre-allocate for performance)
let mut output_ch1: Vec<f32> = vec![0.0; block_size];
let mut output_ch2: Vec<f32> = vec![0.0; block_size];
let mut output_buffers: [&mut [f32]; 2] = [&mut output_ch1, &mut output_ch2];

// Process audio using pre-allocated output buffers (avoids allocation)
match shifter.process_into(&input_buffers, &mut output_buffers) {
    Ok(()) => { /* output_buffers now contain shifted audio */ }
    Err(e) => eprintln!("Error processing audio: {}", e),
}

// Alternatively, use `process` which allocates the output buffer (convenient but less performant)
// let result = shifter.process(&input_buffers);
// match result {
//     Ok(output_vecs) => { /* output_vecs contains shifted audio */ },
//     Err(e) => eprintln!("Error processing audio: {}", e),
// }
```

## Performance Considerations

Although `LiveShifter` is optimized for lower latency compared to the general `RubberBandStretcher`, it is **not** a zero-latency effect. It introduces a processing delay (typically >50ms depending on configuration) between the input and the corresponding output. The exact delay in samples can be queried via `LiveShifter::start_delay()`. Use this value to compensate for the latency if needed.

For performance-critical code, prefer using `process_into` with pre-allocated buffers to avoid allocations during processing.

## To-do

- [ ] Implement `Stretcher` struct for the `RubberBandStretcher` C++ class.
- [ ] Add comprehensive tests for the `Stretcher` implementation.

## License

This project is licensed under the **GNU General Public License v2.0 or later**. See the [LICENSE](LICENSE) file for details.

Note that this is a binding to the [Rubber Band](https://breakfastquay.com/rubberband/) library, which is also licensed under the GNU General Public License v2.0 or later.