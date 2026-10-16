//! Audio output through `cpal`. The device callback drains a small FIFO of
//! interleaved stereo f32 samples that the playback engine fills ahead of
//! time; the number of samples the device has consumed is the playback
//! master clock (see `playback.rs`).
//!
//! The `cpal::Stream` lives on its own thread because it is not `Send` on
//! every platform; everything else talks to it through [`AudioShared`].

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, OutputCallbackInfo, SizedSample};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// State shared between the device callback and the playback engine.
pub struct AudioShared {
    /// Interleaved stereo samples waiting to be played.
    pub fifo: Mutex<VecDeque<f32>>,
    /// Stereo frames taken from `fifo` by the device since it was opened.
    pub played: AtomicU64,
    /// Device output latency (callback to DAC) in microseconds.
    pub latency_us: AtomicU64,
    /// Master volume as f32 bits.
    volume: AtomicU32,
    /// Set when the device callback found the FIFO empty while it expected data.
    pub underruns: AtomicU64,
    pub sample_rate: u32,
    /// False once the stream reported a fatal error.
    pub alive: AtomicBool,
}

impl AudioShared {
    pub fn set_volume(&self, v: f32) {
        self.volume.store(v.clamp(0.0, 4.0).to_bits(), Ordering::Relaxed);
    }

    fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    /// Stereo frames queued but not yet handed to the device.
    pub fn queued(&self) -> usize {
        self.fifo.lock().map(|f| f.len() / 2).unwrap_or(0)
    }

    pub fn clear(&self) {
        if let Ok(mut f) = self.fifo.lock() {
            f.clear();
        }
    }

    pub fn push(&self, samples: &[f32]) {
        if let Ok(mut f) = self.fifo.lock() {
            f.extend(samples.iter().copied());
        }
    }
}

/// Opens the default output device. Returns `None` when there is no usable
/// device (playback then runs on the wall clock, silently).
pub fn open() -> Option<Arc<AudioShared>> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<Arc<AudioShared>>>();
    std::thread::Builder::new()
        .name("jb-audio-out".into())
        .spawn(move || {
            let built = (|| -> anyhow::Result<(cpal::Stream, Arc<AudioShared>)> {
                let host = cpal::default_host();
                let device = host.default_output_device().ok_or_else(|| anyhow::anyhow!("no output device"))?;
                let supported = device.default_output_config()?;
                let format = supported.sample_format();
                let config: cpal::StreamConfig = supported.into();
                let shared = Arc::new(AudioShared {
                    fifo: Mutex::new(VecDeque::with_capacity(48_000)),
                    played: AtomicU64::new(0),
                    latency_us: AtomicU64::new(0),
                    volume: AtomicU32::new(1f32.to_bits()),
                    underruns: AtomicU64::new(0),
                    sample_rate: config.sample_rate,
                    alive: AtomicBool::new(true),
                });
                let stream = match format {
                    cpal::SampleFormat::F32 => build::<f32>(&device, &config, shared.clone())?,
                    cpal::SampleFormat::I16 => build::<i16>(&device, &config, shared.clone())?,
                    cpal::SampleFormat::I32 => build::<i32>(&device, &config, shared.clone())?,
                    cpal::SampleFormat::U16 => build::<u16>(&device, &config, shared.clone())?,
                    cpal::SampleFormat::F64 => build::<f64>(&device, &config, shared.clone())?,
                    other => anyhow::bail!("unsupported sample format {other}"),
                };
                stream.play()?;
                Ok((stream, shared))
            })();
            match built {
                Ok((stream, shared)) => {
                    let _ = tx.send(Some(shared.clone()));
                    // Keep the stream alive for the lifetime of the app.
                    while shared.alive.load(Ordering::Relaxed) {
                        std::thread::park_timeout(std::time::Duration::from_secs(1));
                    }
                    drop(stream);
                }
                Err(e) => {
                    eprintln!("audio output unavailable: {e:#}");
                    let _ = tx.send(None);
                }
            }
        })
        .ok()?;
    rx.recv().ok().flatten()
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: Arc<AudioShared>,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let err_shared = shared.clone();
    let stream = device.build_output_stream(
        *config,
        move |data: &mut [T], info: &OutputCallbackInfo| {
            let ts = info.timestamp();
            let lat = ts.playback.duration_since(ts.callback);
            shared.latency_us.store(lat.as_micros() as u64, Ordering::Relaxed);
            let vol = shared.volume();
            let frames = data.len() / channels.max(1);
            let mut taken = 0u64;
            // Never block the real-time thread: if the engine holds the lock, play silence.
            match shared.fifo.try_lock() {
                Ok(mut fifo) => {
                    for frame in data.chunks_mut(channels.max(1)) {
                        let (l, r) = if fifo.len() >= 2 {
                            taken += 1;
                            (fifo.pop_front().unwrap_or(0.0), fifo.pop_front().unwrap_or(0.0))
                        } else {
                            (0.0, 0.0)
                        };
                        for (c, s) in frame.iter_mut().enumerate() {
                            let v = match (channels, c) {
                                (1, _) => (l + r) * 0.5,
                                (_, 0) => l,
                                (_, 1) => r,
                                _ => 0.0,
                            };
                            *s = T::from_sample(v * vol);
                        }
                    }
                }
                Err(_) => {
                    for s in data.iter_mut() {
                        *s = T::from_sample(0.0);
                    }
                }
            }
            if taken < frames as u64 && taken > 0 {
                shared.underruns.fetch_add(1, Ordering::Relaxed);
            }
            shared.played.fetch_add(taken, Ordering::Relaxed);
        },
        move |e| {
            eprintln!("audio stream error: {e}");
            if matches!(e.kind(), cpal::ErrorKind::DeviceNotAvailable | cpal::ErrorKind::HostUnavailable) {
                err_shared.alive.store(false, Ordering::Relaxed);
            }
        },
        None,
    )?;
    Ok(stream)
}
