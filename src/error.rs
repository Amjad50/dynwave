//! Error types for the audio player.
//!
//! The error types are defined in this module. The error types are used in the [`AudioPlayer`](crate::AudioPlayer).

use std::{error::Error, fmt};

use cpal::{
    BackendSpecificError, BuildStreamError, DefaultStreamConfigError, PauseStreamError,
    PlayStreamError, SupportedStreamConfigsError,
};
use rubato::ResamplerConstructionError;

/// The `AudioPlayerError` enum represents the possible errors that can occur when constructing [`AudioPlayer`](crate::AudioPlayer).
///
/// Most are related to issue with the device or the API when trying to use the device.
#[derive(Debug)]
pub enum AudioPlayerError {
    /// From [cpal]: No output device was found.
    NoOutputDevice,
    /// The device doesn't support dual channel which is what's supported for now here.
    DualChannelNotSupported,
    /// From [cpal]: The device associated with the stream is no longer available.
    DeviceNotAvailable,
    /// From [cpal]: See the [`BackendSpecificError`] docs for more information about this error variant.
    DeviceBackendSpecificError(BackendSpecificError),
    /// From [cpal]: Returned if e.g. the default input format was requested on an output-only audio device
    StreamTypeNotSupported,
    /// From [cpal]: We called something the C-Layer API did not understand
    StreamConfigInvalidArgument,
    /// From [cpal]: Occurs if adding a new Stream ID would cause an integer overflow.
    StreamIdOverflow,
    /// From [cpal]: The specified stream configuration is not supported by the device (this should not happen as we will perform resampling if needed)
    StreamConfigNotSupported,
    /// From [rubato]: Error while constructing the resampler.
    ResamplerConstructionError(ResamplerConstructionError),
}

impl Error for AudioPlayerError {}

impl fmt::Display for AudioPlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOutputDevice => write!(f, "No output device found"),
            Self::DualChannelNotSupported => write!(f, "Dual channel not supported"),
            Self::DeviceNotAvailable => write!(f, "Device not available"),
            Self::DeviceBackendSpecificError(err) => {
                write!(f, "Device backend specific error: {}", err)
            }
            Self::StreamTypeNotSupported => write!(f, "Stream type not supported"),
            Self::StreamConfigInvalidArgument => write!(f, "Stream config invalid argument"),
            Self::StreamIdOverflow => write!(f, "Stream id overflow"),
            Self::StreamConfigNotSupported => write!(f, "Stream config not supported"),
            Self::ResamplerConstructionError(err) => {
                write!(f, "Resampler construction error: {}", err)
            }
        }
    }
}

impl From<SupportedStreamConfigsError> for AudioPlayerError {
    fn from(e: SupportedStreamConfigsError) -> Self {
        match e {
            SupportedStreamConfigsError::DeviceNotAvailable => Self::DeviceNotAvailable,
            SupportedStreamConfigsError::InvalidArgument => Self::StreamConfigInvalidArgument,
            SupportedStreamConfigsError::BackendSpecific { err } => {
                Self::DeviceBackendSpecificError(err)
            }
        }
    }
}

impl From<DefaultStreamConfigError> for AudioPlayerError {
    fn from(e: DefaultStreamConfigError) -> Self {
        match e {
            DefaultStreamConfigError::DeviceNotAvailable => Self::DeviceNotAvailable,
            DefaultStreamConfigError::StreamTypeNotSupported => Self::StreamTypeNotSupported,
            DefaultStreamConfigError::BackendSpecific { err } => {
                Self::DeviceBackendSpecificError(err)
            }
        }
    }
}

impl From<BuildStreamError> for AudioPlayerError {
    fn from(e: BuildStreamError) -> Self {
        match e {
            BuildStreamError::DeviceNotAvailable => Self::DeviceNotAvailable,
            BuildStreamError::StreamConfigNotSupported => Self::StreamConfigNotSupported,
            BuildStreamError::InvalidArgument => Self::StreamConfigInvalidArgument,
            BuildStreamError::StreamIdOverflow => Self::StreamIdOverflow,
            BuildStreamError::BackendSpecific { err } => Self::DeviceBackendSpecificError(err),
        }
    }
}

impl From<ResamplerConstructionError> for AudioPlayerError {
    fn from(e: ResamplerConstructionError) -> Self {
        Self::ResamplerConstructionError(e)
    }
}

/// The `PlayError` enum represents the possible errors that can occur when playing/pausing audio.
///
/// These errors might occure if the state of the device changed between calls to
/// [`AudioPlayer::play()`](crate::AudioPlayer::play) and [`AudioPlayer::pause()`](crate::AudioPlayer::pause).
/// I.e. if the device became unavailable for example.
#[derive(Debug)]
pub enum PlayError {
    /// From [cpal]: The device associated with the stream is no longer available.
    DeviceNotAvailable,
    /// From [cpal]: See the [`BackendSpecificError`] docs for more information about this error variant.
    DeviceBackendSpecificError(BackendSpecificError),
}

impl Error for PlayError {}

impl fmt::Display for PlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotAvailable => write!(f, "Device not available"),
            Self::DeviceBackendSpecificError(err) => {
                write!(f, "Device backend specific error: {}", err)
            }
        }
    }
}

impl From<PlayStreamError> for PlayError {
    fn from(e: PlayStreamError) -> Self {
        match e {
            PlayStreamError::DeviceNotAvailable => Self::DeviceNotAvailable,
            PlayStreamError::BackendSpecific { err } => Self::DeviceBackendSpecificError(err),
        }
    }
}

impl From<PauseStreamError> for PlayError {
    fn from(e: PauseStreamError) -> Self {
        match e {
            PauseStreamError::DeviceNotAvailable => Self::DeviceNotAvailable,
            PauseStreamError::BackendSpecific { err } => Self::DeviceBackendSpecificError(err),
        }
    }
}
