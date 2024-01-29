pub mod error;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SizedSample,
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

pub struct AudioPlayer<T: Sample> {
    buffer_producer: HeapProducer<T>,
    resampler: Option<AudioResampler<T>>,
    output_stream: cpal::Stream,
}

impl<T: Sample + SizedSample> AudioPlayer<T> {
    pub fn new(sample_rate: u32) -> Result<Self, AudioPlayerError> {
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

        let (output_sample_rate, resampler) = if found_conf {
            (sample_rate, None)
        } else {
            let def_conf = output_device.default_output_config()?;

            if def_conf.channels() != 2 || def_conf.sample_format() != T::FORMAT {
                eprintln!("No supported configuration found for audio device, please open an issue in github `Amjad50/dynwave`\n\
                      list of supported configurations: {:#?}", conf);
                return Err(AudioPlayerError::DualChannelNotSupported);
            }

            (
                def_conf.sample_rate(),
                Some(AudioResampler::new(
                    sample_rate.0 as usize,
                    def_conf.sample_rate().0 as usize,
                )?),
            )
        };

        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: output_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        // Limiting the number of samples in the buffer is better to minimize
        // audio delay in emulation, this is because emulation speed
        // does not 100% match audio playing speed (44100Hz).
        // The buffer holds only audio for 1/4 second, which is good enough for delays,
        // It can be reduced more, but it might cause noise(?) for slower machines
        // or if any CPU intensive process started while the emulator is running
        let buffer = HeapRb::new(output_sample_rate.0 as usize / 2);
        let (buffer_producer, mut buffer_consumer) = buffer.split();

        let output_data_fn = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for sample in data {
                *sample = buffer_consumer.pop().unwrap_or(T::EQUILIBRIUM);
            }
        };

        let output_stream = output_device
            .build_output_stream(&config, output_data_fn, Self::err_fn, None)
            .expect("failed to build an output audio stream");

        Ok(Self {
            buffer_producer,
            output_stream,
            resampler,
        })
    }

    /// Start the player
    pub fn play(&self) -> Result<(), PlayError> {
        self.output_stream.play().map_err(|e| e.into())
    }

    /// Pause the player
    pub fn pause(&self) -> Result<(), PlayError> {
        self.output_stream.pause().map_err(|e| e.into())
    }

    pub fn queue(&mut self, data: &[T]) {
        let Some(resampler) = &mut self.resampler else {
            // no resampling
            self.buffer_producer.push_slice(data);
            return;
        };

        resampler.resample_into_producer(data, &mut self.buffer_producer);
    }

    fn err_fn(err: cpal::StreamError) {
        eprintln!("an error occurred on audio stream: {}", err);
    }
}
