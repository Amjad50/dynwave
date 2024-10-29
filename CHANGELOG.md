 Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2024-10-28
### Fixed
- Add support for hardwares that may not have the pcm format we want, by converting the audio stream to the supported format.

### Changed
- Updated cpal to 0.15.3
- Updated rubato to 0.16
- Updated ringbuf to 0.4

## [0.1.0] - 2024-01-29
### Added
- Initial implementation of the library, supporting dynamic resampling of audio stream.

[Unreleased]: https://github.com/Amjad50/dynwave/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Amjad50/dynwave/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Amjad50/dynwave/compare/0b4d33e...v0.1.0

