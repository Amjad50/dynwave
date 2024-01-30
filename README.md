# dynwave
[![Build status](https://github.com/Amjad50/dynwave/workflows/CI/badge.svg)](https://actions-badge.atrox.dev/Amjad50/dynwave/goto)
[![dependency status](https://deps.rs/repo/github/Amjad50/dynwave/status.svg)](https://deps.rs/repo/github/Amjad50/dynwave)
[![license](https://img.shields.io/github/license/Amjad50/dynwave)](./LICENSE)
[![Crates.io dynwave](https://img.shields.io/crates/v/dynwave)](https://crates.io/crates/dynwave)
[![docs.rs dynwave](https://img.shields.io/docsrs/dynwave/latest)](https://docs.rs/dynwave/latest/dynwave/)

dynwave is a dynamic audio player based on fixed samples stream, written in Rust.

The purpose of this is to implement a cross platform audio player that plays audio samples stream generated and plays it real-time.

This works as a fusion between [rubato](https://crates.io/crates/rubato) and [cpal](https://crates.io/crates/cpal).

This is useful for emulators for example, where an emulation loop will be like this:
1) Run emulation for a frame.
2) Extract the collected audio samples for that frame.
3) Queue the samples for playing (using `dynwave`).
4) Take video frame and display it.
5) Repeat.

## Getting Started

You can use `dynwave` to play audio streams for your Rust projects.

### Cargo
Add it as a dependency in your `Cargo.toml` file:
```sh
cargo add dynwave
```

### Example usage
```rust
use dynwave::{AudioPlayer, BufferSize};

let mut player = AudioPlayer::<f32>::new(44100, BufferSize::OneSecond).unwrap();

// Start playing the audio
player.play().unwrap();

// generate audio samples (can be done in a emulation loop for example)
let samples = generate_samples();
player.queue(&samples);

// pause the audio
player.pause().unwrap();
```

## Minimum Supported Rust Version (MSRV)
The minimum supported Rust version for this crate is `1.62.0`.

## Contributing

Contributions are welcome, please open an issue or a PR if you have any suggestions or ideas.

Make sure to:
- Run `cargo fmt`.
- Run `cargo clippy`.
- Run `cargo test`.

## Projects using `dynwave`
> If you are using `dynwave` in your project, please open a PR to add it here.

| Project | Description |
| ------- | ----------- |
| [mizu](https://github.com/Amjad50/mizu) | A GameBoy emulator written in Rust (this is actually were this library originiated https://github.com/Amjad50/mizu/issues/11)|
| [trapezoid](https://github.com/Amjad50/trapezoid) | PSX emulator powered with Vulkan and Rust |

## License
This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details
