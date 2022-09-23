use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::getters::*;
use crate::input_handling::*;
use anyhow::Error;
type WriteHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

pub struct Recorder {
	writer: WriteHandle,
	interrupt_handles: InterruptHandles,
	default_config: SupportedStreamConfig,
	user_config: StreamConfig,
	device: Device,
	spec: hound::WavSpec,
	name: String,
	path: PathBuf,
	current_file: String,
}

/// # Recorder
///
/// The `Recorder` struct is used to record audio.
///
/// Use `init()` to initialize the recorder, `record()` to start a continuous recording,
/// and `rec_secs()` to record for a given number of seconds. The Recorder does not
/// need to be reinitialized after a recording is stopped. Calling `record()` or
/// `rec_secs()` again will start a new recording with a new filename according to
/// the time and date.
impl Recorder {

	/// # Init
	///
	/// Initializes the recorder with the given host, sample rate, channel count, and buffer size.
	pub fn init(
		name: String,
		path: PathBuf,
		host: HostId,
		sample_rate: u32,
		channels: u16,
		buffer_size: u32,
	) -> Result<Self, Error> {

		// Create interrupt handles to be used by the stream or batch loop.
		let interrupt_handles = InterruptHandles::new()?;

		// Select requested host.
		let host = get_host(host)?;

		// Set up the input device and stream with the default input config.
		let device = get_device(host)?;

		// Get default config for the device.
		let default_config = get_default_config(&device)?;

		// Override certain fields of the default stream config with the user's config.
		let user_config = get_user_config(sample_rate, channels, buffer_size)?;

		// Get the hound WAV spec for the user's config.
		let spec = get_wav_spec(&default_config, &user_config)?;

		Ok(Self {
			writer: Arc::new(Mutex::new(None)),
			interrupt_handles,
			default_config,
			user_config,
			device,
			spec,
			name,
			path,
			current_file: "".to_string(),
		})
	}

	fn init_writer(&mut self) -> Result<(), Error> {
		let filename = get_filename(&self.name, &self.path);
		self.current_file = filename.clone();
		*self.writer.lock().unwrap() = Some(hound::WavWriter::create(filename, self.spec)?);
		Ok(())
	}

	fn create_stream(&self) -> Result<Stream, Error> {
		let writer = self.writer.clone();
		let config = self.user_config.clone();
		let err_fn = |err| { eprintln!("An error occurred on stream: {}", err); };

		let stream = match self.default_config.sample_format() {
			cpal::SampleFormat::F32 => self.device.build_input_stream(
				&config.into(),
				move |data, _: &_| write_input_data::<f32, f32>(data, &writer),
				err_fn,
			)?,
			cpal::SampleFormat::I16 => self.device.build_input_stream(
				&config.into(),
				move |data, _: &_| write_input_data::<i16, i16>(data, &writer),
				err_fn,
			)?,
			cpal::SampleFormat::U16 => self.device.build_input_stream(
				&config.into(),
				move |data, _: &_| write_input_data::<u16, i16>(data, &writer),
				err_fn,
			)?,
		};
		Ok(stream)
	}

	/// # Record
	///
	/// Start a continuous recording. The recording will be stopped when the
	/// user presses `Ctrl+C`.
	pub fn record(&mut self) -> Result<(), Error> {
		self.init_writer()?;
		let stream = self.create_stream()?;
		stream.play()?;
		println!("REC: {}", self.current_file);
		self.interrupt_handles.stream_wait();
		drop(stream);
		self.writer.lock().unwrap().take().unwrap().finalize()?;
		println!("STOP: {}", self.current_file);
		Ok(())
	}

	/// # Record Seconds
	///
	/// Record for a given number of seconds or until the user presses `Ctrl+C`.
	/// Current batch is finished before stopping.
	pub fn record_secs(&mut self, secs: u64) -> Result<(), Error> {
		self.init_writer()?;
		let stream = self.create_stream()?;
		stream.play()?;
		println!("REC: {}", self.current_file);
		let now = std::time::Instant::now();
		loop {
			std::thread::sleep(std::time::Duration::from_millis(500));
			if now.elapsed().as_secs() >= secs {
				break;
			}
		}
		drop(stream);
		self.writer.lock().unwrap().take().unwrap().finalize()?;
		println!("STOP: {}", self.current_file);
		Ok(())
	}
}

fn write_input_data<T, U>(input: &[T], writer: &WriteHandle)
where
    T: cpal::Sample,
    U: cpal::Sample + hound::Sample,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = cpal::Sample::from(&sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}

pub fn batch_recording(rec: &mut Recorder, secs: u64) -> Result<(), Error> {
	while rec.interrupt_handles.batch_is_running() {
		rec.record_secs(secs)?;
	}
	Ok(())
}

pub fn contiguous_recording(rec: &mut Recorder) -> Result<(), Error> {
	rec.record()?;
	Ok(())
}
