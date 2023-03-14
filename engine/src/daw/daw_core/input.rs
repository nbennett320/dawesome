use rodio::{
  Decoder,
  cpal::{
    self,
    Sample,
    traits::{
      DeviceTrait,
      HostTrait,
      StreamTrait,
    }
  },
};
use symphonia::core::conv::FromSample;
use std::{
  any::{
    Any,
    TypeId
  },
  sync::{
    Arc,
    Mutex, 
    atomic::{
      AtomicBool,
      Ordering,
    },
  },
  io::{
    BufWriter,
  },
  fs::{
    File,
  },
  time::{
    self,
    SystemTime,
  },
  thread,
};


fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
  if TypeId::of::<f32>() == format.type_id() {
    hound::SampleFormat::Float
  } else {
    hound::SampleFormat::Int
  }
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
  hound::WavSpec {
    channels: config.channels() as _,
    sample_rate: config.sample_rate().0 as _,
    bits_per_sample: (config.sample_format().sample_size() * 8) as _,
    sample_format: sample_format(config.sample_format()),
  }
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
  T: Sample,
  U: Sample + hound::Sample + FromSample<T>,
{
  if let Ok(mut guard) = writer.try_lock() {
    if let Some(writer) = guard.as_mut() {
      for &sample in input.iter() {
        let sample: U = U::from_sample(sample);
        writer.write_sample(sample).ok();
      }
    }
  }
}

pub fn record_input(recording: bool) {
  let host = cpal::default_host();
  let device = host.default_input_device().unwrap();

  println!("input device: {}", device.name().unwrap());

  let config = device.default_input_config().unwrap();

  println!("input config: {:?}", config);

  let timestamp = SystemTime::now().duration_since(time::UNIX_EPOCH);

  let path: String = format!("{}/assets/recorded-{}.wav", env!("CARGO_MANIFEST_DIR"), timestamp.unwrap().as_millis());
  let spec = wav_spec_from_config(&config);
  let writer = hound::WavWriter::create(&path, spec).unwrap();
  let writer = Arc::new(Mutex::new(Some(writer)));

  println!("recording started");

  let writer2 = writer.clone();

  let err_fn = move |err| {
    eprintln!("an error occurred on stream: {}", err);
  };

  let stream = match config.sample_format() {
    cpal::SampleFormat::I16 => device.build_input_stream(
      &config.into(),
      move |data, _: &_| write_input_data::<i16, i16>(data, &writer2),
      err_fn
    ).unwrap(),
    cpal::SampleFormat::F32 => device.build_input_stream(
      &config.into(),
      move |data, _: &_| write_input_data::<f32, f32>(data, &writer2),
      err_fn
    ).unwrap(),
    cpal::SampleFormat::U16 => todo!(),
  };

  stream.play().unwrap();

  loop {
    if recording {
      break;
    }

    thread::sleep(std::time::Duration::from_secs(1));
  }

  drop(stream);
  writer.lock().unwrap().take().unwrap().finalize().unwrap();
  println!("Recording {} complete!", &path);
}
