//! Dynamic audio player based on fixed samples stream
//!
//! This crate provides a dynamic audio player that can play audio samples stream coming
//! from an external generating source, such as an emulator.
//!
//! The [`AudioPlayer`] acts as an audio stream player that will play the samples as they come.
//! And will resample the audio if the generated sample rate is not supported by the audio device,
//!
//! # Supported sample types
//! For now, we rely on [`rubato`] crate for resampling, it has the trait [`Sample`] that is implemented for:
//! - [`f32`]
//! - [`f64`]
//!
//! # Example
//!
//! Here's an example of how to use the `AudioPlayer`:
//! ```rust,no_run
//! # use dynwave::{AudioPlayer, BufferSize};
//! // create a buffer, that can hold 1 second worth of samples
//! // (base it depend on how fast you generate samples, less buffer is better for latency)
//! let mut player = AudioPlayer::<f32>::new(44100, BufferSize::OneSecond).unwrap();
//!
//! // Start playing the audio
//! player.play().unwrap();
//!
//! // generate audio samples (can be done in a emulation loop for example)
//! let samples = generate_samples();
//! player.queue(&samples);
//!
//! // pause the audio
//! player.pause().unwrap();
//!
//! # fn generate_samples() -> Vec<f32> {
//! #     vec![0.0; 1]
//! # }
//! ```
pub mod error;
mod utils;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SizedSample,
};
use error::{AudioPlayerError, PlayError};
use ringbuf::{HeapProducer, HeapRb};
use rubato::{FftFixedInOut, Resampler, Sample};

struct AudioResampler<T: Sample> {
    resampler: FftFixedInOut<T>,
    pre_resampled_buffer: Vec<T>,
    pre_resampled_split_buffers: [Vec<T>; 2],
    resample_process_buffers: [Vec<T>; 2],
    resampled_buffer: Vec<T>,
}

impl<T: Sample + SizedSample> AudioResampler<T> {
    fn new(input_rate: usize, output_rate: usize) -> Result<Self, AudioPlayerError> {
        let resampler = FftFixedInOut::<T>::new(
            input_rate,
            output_rate,
            // the number of samples for one video frame in 60 FPS
            input_rate / 60,
            2,
        )?;

        Ok(Self {
            resampler,
            pre_resampled_buffer: Vec::new(),
            pre_resampled_split_buffers: [Vec::new(), Vec::new()],
            resample_process_buffers: [Vec::new(), Vec::new()],
            resampled_buffer: Vec::new(),
        })
    }

    fn resample_into_producer(&mut self, data: &[T], producer: &mut HeapProducer<T>) {
        // helper method to split channels into separate vectors
        fn read_frames<T: Copy>(inbuffer: &[T], n_frames: usize, outputs: &mut [Vec<T>]) {
            for output in outputs.iter_mut() {
                output.clear();
                output.reserve(n_frames);
            }
            let mut value: T;
            let mut inbuffer_iter = inbuffer.iter();
            for _ in 0..n_frames {
                for output in outputs.iter_mut() {
                    value = *inbuffer_iter.next().unwrap();
                    output.push(value);
                }
            }
        }

        /// Helper to merge channels into a single vector
        /// the number of channels is the size of `waves` slice
        fn write_frames<T: Copy>(waves: &[Vec<T>], outbuffer: &mut Vec<T>) {
            let nbr = waves[0].len();
            for frame in 0..nbr {
                for wave in waves.iter() {
                    outbuffer.push(wave[frame]);
                }
            }
        }

        self.pre_resampled_buffer.extend_from_slice(data);
        // finish all the frames, as sometimes after appending many data
        // we might get 2 loops worth of unprocessed audio
        loop {
            let frames = self.resampler.input_frames_next();

            if self.pre_resampled_buffer.len() < frames * 2 {
                return;
            }

            // only read the needed frames
            read_frames(
                &self.pre_resampled_buffer,
                frames,
                &mut self.pre_resampled_split_buffers,
            );

            self.resample_process_buffers[0].clear();
            self.resample_process_buffers[0].clear();

            let output_frames = self.resampler.output_frames_next();
            self.resample_process_buffers[0].resize(output_frames, T::EQUILIBRIUM);
            self.resample_process_buffers[1].resize(output_frames, T::EQUILIBRIUM);

            self.resampler
                .process_into_buffer(
                    &self.pre_resampled_split_buffers,
                    &mut self.resample_process_buffers,
                    None,
                )
                .unwrap();

            // resample
            if self.resampled_buffer.len() < output_frames * 2 {
                self.resampled_buffer
                    .reserve(output_frames * 2 - self.resampled_buffer.len());
            }
            self.resampled_buffer.clear();
            write_frames(&self.resample_process_buffers, &mut self.resampled_buffer);

            producer.push_slice(&self.resampled_buffer);

            self.pre_resampled_buffer = self.pre_resampled_buffer.split_off(frames * 2);
        }
    }
}

/// The `BufferSize` enum represents the amount of audio samples that can be stored in the buffer.
/// Limiting the number of samples in the buffer is crucial for minimizing audio delay in audio playing.
///
/// We will use `emulation` as an example to refer to the process of generating audio samples.
///
/// minimizing the buffer size will help minimize audio delay such as audio coming from an emulator.
/// This is due to the fact that emulation speed does not always perfectly
/// match the audio playing speed (e.g., 44100Hz).
///
/// A smaller buffer size can help maintain better synchronization,
/// but it may cause noise or other issues on slower machines.
/// This can occur if the emulation process is slow, or if a CPU-intensive
/// process starts while the emulator is running.
#[derive(Debug, Clone, Copy, Default)]
pub enum BufferSize {
    #[default]
    /// 1/4 second worth of samples
    QuarterSecond,
    /// 1/2 second worth of samples
    HalfSecond,
    /// 1 second worth of samples
    OneSecond,
    /// Number of samples to store
    /// Be careful, here you have to calculate based on the sample rate manually
    Samples(usize),
}

impl BufferSize {
    /// Returns the number of samples in the buffer
    #[inline]
    #[must_use]
    fn store_for_samples(&self, sample_rate: usize, channels: usize) -> usize {
        match self {
            Self::QuarterSecond => sample_rate / 4 * channels,
            Self::HalfSecond => sample_rate / 2 * channels,
            Self::OneSecond => sample_rate * channels,
            Self::Samples(alternative_samples) => *alternative_samples,
        }
    }
}

/// The `AudioPlayer` struct represents an audio player that can play audio samples stream
/// coming from an external generating source, such as an emulator.
///
/// The `AudioPlayer` may resample the audio if the generated sample rate is not supported by the audio device,
/// which may cause a slight performance hit due to the resampling process. If the machine supports the input sample rate,
/// no resampling will be done, and the audio samples will be used as is.
///
/// # Example
///
/// Here's an example of how to use the `AudioPlayer`:
/// ```rust,no_run
/// # use dynwave::{AudioPlayer, BufferSize};
/// // create a buffer, that can hold 1 second worth of samples
/// // (base it depend on how fast you generate samples, less buffer is better for latency)
/// let mut player = AudioPlayer::<f32>::new(44100, BufferSize::OneSecond).unwrap();
///
/// // Start playing the audio
/// player.play().unwrap();
///
/// // generate audio samples (can be done in a emulation loop for example)
/// let samples = generate_samples();
/// player.queue(&samples);
///
/// // pause the audio
/// player.pause().unwrap();
///
/// # fn generate_samples() -> Vec<f32> {
/// #     vec![0.0; 1]
/// # }
/// ```
pub struct AudioPlayer<T: Sample> {
    buffer_producer: HeapProducer<T>,
    resampler: Option<AudioResampler<T>>,
    output_stream: cpal::Stream,
}

impl<T: Sample + SizedSample> AudioPlayer<T>
where
    // sadly, cpal uses macro to generate those, and there is no auto way
    // to use the type system to, even though it seems that it makes sense
    // to have `T : FromSample<W> where W: SizedSample`?
    i8: FromSample<T>,
    i16: FromSample<T>,
    i32: FromSample<T>,
    i64: FromSample<T>,
    u8: FromSample<T>,
    u16: FromSample<T>,
    u32: FromSample<T>,
    u64: FromSample<T>,
    f32: FromSample<T>,
    f64: FromSample<T>,
{
    /// Creates a new instance of `AudioPlayer`.
    ///
    /// # Parameters
    /// * `sample_rate`: The sample rate of the audio player in Hz. Common values are `44100` or `48000`.
    /// * `buffer_size`: The size of the buffer that will store the audio samples. See [`BufferSize`] for options.
    ///
    /// # Returns
    /// Might return an `Error` if:
    /// - No output device is found
    /// - The output device does not support dual channel
    /// - Some error happened with the device backend
    /// - Could not create the audio stream
    ///
    /// Check [`AudioPlayerError`] for more information about the possible errors.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use dynwave::{AudioPlayer, BufferSize};
    /// let sample_rate = 44100;
    /// let buffer_size = BufferSize::HalfSecond;
    /// let player = AudioPlayer::<f32>::new(sample_rate, buffer_size).unwrap();
    /// ```
    ///
    /// This example creates a new `AudioPlayer` with a sample rate of 44100 Hz and a buffer size of half a second.
    pub fn new(sample_rate: u32, buffer_size: BufferSize) -> Result<Self, AudioPlayerError> {
        let host = cpal::default_host();
        let output_device = host
            .default_output_device()
            .ok_or(AudioPlayerError::NoOutputDevice)?;

        let sample_rate = cpal::SampleRate(sample_rate);

        let conf = output_device
            .supported_output_configs()?
            .collect::<Vec<_>>();

        let mut found_conf = false;

        for c in &conf {
            // must have 2 channels and <T> format
            // (almost all? devices will have at least one configuration with these)
            if c.channels() == 2
                && c.sample_format() == T::FORMAT
                && c.min_sample_rate() <= sample_rate
                && c.max_sample_rate() >= sample_rate
            {
                found_conf = true;
                break;
            }
        }

        let (output_sample_rate, output_format, resampler) = if found_conf {
            (sample_rate, T::FORMAT, None)
        } else {
            // second time, try to find something that is 2 channels, but format and sample range can
            // be different, match with highest value
            let mut max_match = 0;
            let mut matched_conf = None;
            for c in &conf {
                let mut curr_match = 0;
                if c.channels() == 2 {
                    curr_match += 1;
                    if c.sample_format() == T::FORMAT {
                        curr_match += 3;
                    }
                    if c.min_sample_rate() <= sample_rate && c.max_sample_rate() >= sample_rate {
                        curr_match += 2;
                    }
                }
                if curr_match > max_match {
                    max_match = curr_match;
                    matched_conf = Some(c);
                }
            }

            let used_conf = match matched_conf {
                Some(conf) => conf
                    .try_with_sample_rate(sample_rate)
                    .unwrap_or_else(|| conf.with_max_sample_rate()),
                None => output_device.default_output_config()?,
            };

            if used_conf.channels() != 2 {
                eprintln!("No supported configuration found for audio device, please open an issue in github `Amjad50/dynwave`\n\
                      list of supported configurations: {:#?}", conf);
                return Err(AudioPlayerError::DualChannelNotSupported);
            }

            (
                used_conf.sample_rate(),
                used_conf.sample_format(),
                Some(AudioResampler::new(
                    sample_rate.0 as usize,
                    used_conf.sample_rate().0 as usize,
                )?),
            )
        };

        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: output_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let ring_buffer_len = buffer_size.store_for_samples(output_sample_rate.0 as usize, 2);
        let buffer = HeapRb::new(ring_buffer_len);
        let (buffer_producer, buffer_consumer) = buffer.split();

        let output_data_fn = utils::create_output_processor(output_format, buffer_consumer);

        let output_stream = output_device.build_output_stream_raw(
            &config,
            output_format,
            output_data_fn,
            Self::err_fn,
            None,
        )?;

        Ok(Self {
            buffer_producer,
            output_stream,
            resampler,
        })
    }

    /// Start the player
    ///
    /// If the player is playing and if the buffer is emptied (played until finished without adding more data), popping sound might be heard.
    ///
    /// Might return an `Error` if:
    /// - The device associated with the stream is no longer available
    /// - Some error happened with the device backend
    ///
    /// Check [`PlayError`] for more information about the possible errors.
    pub fn play(&self) -> Result<(), PlayError> {
        self.output_stream.play().map_err(|e| e.into())
    }

    /// Pause the player
    ///
    /// Might return an `Error` if:
    /// - The device associated with the stream is no longer available
    /// - Some error happened with the device backend
    ///
    /// Check [`PlayError`] for more information about the possible errors.
    pub fn pause(&self) -> Result<(), PlayError> {
        self.output_stream.pause().map_err(|e| e.into())
    }

    /// Queues audio samples to be played.
    ///
    /// The `queue` function takes a slice of audio samples and adds them to the buffer. If a `resampler` is present,
    /// it resamples the audio data before adding it to the buffer.
    ///
    /// If the buffer is full, the function will drop the audio samples that don't fit in the buffer and won't block.
    ///
    /// If the player is playing, the audio samples will be played immediately, and if the buffer is emptied, popping sound might be heard.
    ///
    /// # Parameters
    /// * `data`: A slice of audio samples to be played.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use dynwave::{AudioPlayer, BufferSize};
    /// let sample_rate = 44100;
    /// let buffer_size = BufferSize::HalfSecond;
    /// let mut player = AudioPlayer::new(sample_rate, buffer_size).unwrap();
    /// let samples = vec![0.5, 0.7, 0.9, 1.0, 0.9, 0.7, 0.5, 0.3, 0.1];
    /// player.queue(&samples);
    /// ```
    /// This example creates a new `AudioPlayer` with a sample rate of 44100 Hz and a buffer size of half a second, queues some audio samples, and then starts playing the audio.
    pub fn queue(&mut self, data: &[T]) {
        if let Some(resampler) = &mut self.resampler {
            resampler.resample_into_producer(data, &mut self.buffer_producer);
        } else {
            // no resampling
            self.buffer_producer.push_slice(data);
        }
    }

    fn err_fn(err: cpal::StreamError) {
        eprintln!("an error occurred on audio stream: {}", err);
    }
}
