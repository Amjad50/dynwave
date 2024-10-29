use cpal::{Data, FromSample, Sample, SampleFormat, SizedSample};
use ringbuf::{traits::Consumer, HeapCons};

// Type alias for the processing function - matches the required callback signature
type ProcessingFn = Box<dyn FnMut(&mut Data, &cpal::OutputCallbackInfo) + Send + 'static>;

// Function to create the appropriate processing function based on format
pub fn create_output_processor<T>(
    format: SampleFormat,
    mut buffer_consumer: HeapCons<T>,
) -> ProcessingFn
where
    T: Sample + SizedSample + Send + 'static,

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
    match format {
        SampleFormat::I8 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<i8>().expect("Valid format") {
                *sample = i8::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::I16 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<i16>().expect("Valid format") {
                *sample = i16::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::I32 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<i32>().expect("Valid format") {
                *sample = i32::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::I64 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<i64>().expect("Valid format") {
                *sample = i64::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::U8 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<u8>().expect("Valid format") {
                *sample = u8::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::U16 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<u16>().expect("Valid format") {
                *sample = u16::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::U32 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<u32>().expect("Valid format") {
                *sample = u32::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::U64 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<u64>().expect("Valid format") {
                *sample = u64::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::F32 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<f32>().expect("Valid format") {
                *sample = f32::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        SampleFormat::F64 => Box::new(move |data, _| {
            for sample in data.as_slice_mut::<f64>().expect("Valid format") {
                *sample = f64::from_sample(buffer_consumer.try_pop().unwrap_or(T::EQUILIBRIUM));
            }
        }),
        e => panic!("Format {e:?} isn't supported"),
    }
}
