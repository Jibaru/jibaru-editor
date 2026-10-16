//! [`MediaPool`]: the shared frame and audio source used by preview and
//! export. It owns a bounded LRU of open decoders (many 4K HEVC decoders
//! exhaust memory quickly), keeps a short ring of recent frames per decoder
//! so a prefetching thread and the renderer can share work, and switches to
//! proxies for reduced-size requests when they are available.

use crate::audio::AudioDecoder;
use crate::probe::is_image_path;
use crate::video::{DecodeMode, VideoDecoder};
use anyhow::{Context as _, Result};
use jb_core::frame::{FrameSource, VideoFrame};
use jb_core::model::{MediaKind, Project};
use jb_core::time::{Ticks, TICKS_PER_SECOND};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of open video decoders across all assets.
    pub max_video_decoders: usize,
    /// Maximum decoders for a single asset (the same clip used twice at
    /// different times, e.g. during a transition, would otherwise thrash).
    pub max_decoders_per_asset: usize,
    pub max_audio_decoders: usize,
    /// Recently decoded frames kept per decoder.
    pub frame_ring: usize,
    /// Use hardware decoding when available.
    pub hardware: bool,
    /// Serve proxies for requests with a `max_size` when one is registered.
    pub use_proxies: bool,
    /// Output sample rate of [`MediaPool::audio_samples`].
    pub sample_rate: u32,
    /// Memory budget for decoded still images.
    pub image_cache_bytes: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_video_decoders: 6,
            max_decoders_per_asset: 2,
            max_audio_decoders: 16,
            frame_ring: 8,
            hardware: true,
            use_proxies: true,
            sample_rate: 48_000,
            image_cache_bytes: 512 << 20,
        }
    }
}

#[derive(Debug, Clone)]
struct Asset {
    path: PathBuf,
    kind: Option<MediaKind>,
    proxy: Option<PathBuf>,
}

struct VideoSlot {
    key: String,
    dec: Arc<Mutex<VideoDecoder>>,
    ring: Arc<Mutex<Ring>>,
    position: Option<Ticks>,
    last_used: u64,
}

struct AudioSlot {
    key: String,
    dec: Arc<Mutex<AudioDecoder>>,
    position: Option<Ticks>,
    last_used: u64,
}

/// Recently produced frames of one decoder, for one `max_size`.
#[derive(Default)]
struct Ring {
    size: Option<(u32, u32)>,
    /// Frames with their display duration, in decode order.
    frames: VecDeque<(VideoFrame, Ticks)>,
}

impl Ring {
    fn find(&self, t: Ticks, size: Option<(u32, u32)>) -> Option<VideoFrame> {
        if self.size != size {
            return None;
        }
        self.frames.iter().find(|(f, d)| t >= f.pts && t < f.pts + d).map(|(f, _)| f.clone())
    }

    fn push(&mut self, f: VideoFrame, dur: Ticks, size: Option<(u32, u32)>, cap: usize) {
        if self.size != size {
            self.frames.clear();
            self.size = size;
        }
        if self.frames.back().is_some_and(|b| b.0.pts == f.pts) {
            return;
        }
        if self.frames.back().is_some_and(|b| b.0.pts > f.pts) {
            self.frames.clear();
        }
        self.frames.push_back((f, dur.max(1)));
        while self.frames.len() > cap {
            self.frames.pop_front();
        }
    }
}

type ImageKey = (String, Option<(u32, u32)>);
type VideoCheckout = (Arc<Mutex<VideoDecoder>>, Arc<Mutex<Ring>>);

#[derive(Default)]
struct State {
    assets: HashMap<String, Asset>,
    video: Vec<VideoSlot>,
    audio: Vec<AudioSlot>,
    images: HashMap<ImageKey, (VideoFrame, u64)>,
    image_bytes: usize,
}

/// Counters for diagnostics and tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct PoolStats {
    pub video_decoders_opened: u64,
    pub video_decoders_evicted: u64,
    pub ring_hits: u64,
    pub open_video_decoders: usize,
    pub open_audio_decoders: usize,
}

pub struct MediaPool {
    config: PoolConfig,
    state: Mutex<State>,
    clock: AtomicU64,
    opened: AtomicU64,
    evicted: AtomicU64,
    ring_hits: AtomicU64,
}

impl MediaPool {
    pub fn new(config: PoolConfig) -> Self {
        crate::init();
        Self {
            config,
            state: Mutex::new(State::default()),
            clock: AtomicU64::new(0),
            opened: AtomicU64::new(0),
            evicted: AtomicU64::new(0),
            ring_hits: AtomicU64::new(0),
        }
    }

    /// A pool configured for `project` with all its media registered.
    pub fn for_project(project: &Project, mut config: PoolConfig) -> Self {
        config.sample_rate = project.settings.sample_rate;
        let pool = Self::new(config);
        pool.sync(project);
        pool
    }

    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// Registers (or re-points) a media id to a file.
    pub fn add(&self, media_id: &str, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        let mut st = self.state.lock().unwrap();
        let changed = st.assets.get(media_id).is_none_or(|a| a.path != path);
        if changed {
            let proxy = st.assets.get(media_id).and_then(|a| a.proxy.clone());
            st.assets.insert(media_id.to_string(), Asset { path, kind: None, proxy });
            Self::drop_decoders(&mut st, media_id);
        }
    }

    /// Registers all media of `project`.
    pub fn sync(&self, project: &Project) {
        for m in &project.media {
            self.add(&m.id, &m.path);
            let mut st = self.state.lock().unwrap();
            if let Some(a) = st.assets.get_mut(&m.id) {
                a.kind = Some(m.kind);
            }
        }
    }

    /// Uses `proxy` (same timing, lower resolution) for preview requests of
    /// `media_id`. `None` removes it.
    pub fn set_proxy(&self, media_id: &str, proxy: Option<PathBuf>) {
        let mut st = self.state.lock().unwrap();
        if let Some(a) = st.assets.get_mut(media_id) {
            a.proxy = proxy;
        }
        let key = proxy_key(media_id);
        st.video.retain(|s| s.key != key);
    }

    /// Closes every decoder (e.g. when the app goes idle).
    pub fn clear(&self) {
        let mut st = self.state.lock().unwrap();
        st.video.clear();
        st.audio.clear();
        st.images.clear();
        st.image_bytes = 0;
    }

    pub fn stats(&self) -> PoolStats {
        let st = self.state.lock().unwrap();
        PoolStats {
            video_decoders_opened: self.opened.load(Ordering::Relaxed),
            video_decoders_evicted: self.evicted.load(Ordering::Relaxed),
            ring_hits: self.ring_hits.load(Ordering::Relaxed),
            open_video_decoders: st.video.len(),
            open_audio_decoders: st.audio.len(),
        }
    }

    fn drop_decoders(st: &mut State, media_id: &str) {
        let pk = proxy_key(media_id);
        st.video.retain(|s| s.key != media_id && s.key != pk);
        st.audio.retain(|s| s.key != media_id);
        st.images.retain(|k, _| k.0 != media_id);
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }

    fn asset(&self, media_id: &str) -> Result<Asset> {
        self.state.lock().unwrap().assets.get(media_id).cloned().with_context(|| format!("unknown media id {media_id}"))
    }

    /// Video frame of `media_id` at `t`; see [`FrameSource::frame`].
    pub fn video_frame(&self, media_id: &str, t: Ticks, max_size: Option<(u32, u32)>) -> Result<Option<VideoFrame>> {
        let asset = self.asset(media_id)?;
        let is_image = asset.kind == Some(MediaKind::Image) || (asset.kind.is_none() && is_image_path(&asset.path));
        if is_image {
            return self.image_frame(media_id, &asset.path, max_size).map(Some);
        }
        if asset.kind == Some(MediaKind::Audio) {
            return Ok(None);
        }
        let (key, path) = match (&asset.proxy, max_size) {
            (Some(p), Some(_)) if self.config.use_proxies => (proxy_key(media_id), p.clone()),
            _ => (media_id.to_string(), asset.path.clone()),
        };
        let (dec, ring) = self.checkout_video(&key, &path, t, max_size)?;
        if let Some(f) = ring.lock().unwrap().find(t, max_size) {
            self.ring_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(Some(f));
        }
        let mut d = dec.lock().unwrap();
        // Another thread may have produced it while we waited.
        if let Some(f) = ring.lock().unwrap().find(t, max_size) {
            self.ring_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(Some(f));
        }
        let frame = d.frame_at(t, max_size, DecodeMode::Accurate)?;
        let pos = d.position();
        let fd = d.last_duration();
        let at_end = d.at_end();
        drop(d);
        if let Some(f) = &frame {
            // The held last frame is not cached: its range is open-ended.
            if !at_end {
                ring.lock().unwrap().push(f.clone(), fd, max_size, self.config.frame_ring.max(1));
            }
        }
        let mut st = self.state.lock().unwrap();
        if let Some(s) = st.video.iter_mut().find(|s| Arc::ptr_eq(&s.dec, &dec)) {
            s.position = pos;
        }
        Ok(frame)
    }

    fn checkout_video(
        &self,
        key: &str,
        path: &Path,
        t: Ticks,
        max_size: Option<(u32, u32)>,
    ) -> Result<VideoCheckout> {
        {
            let mut st = self.state.lock().unwrap();
            let now = self.tick();
            // 1. A decoder whose ring already holds the frame.
            // 2. A decoder positioned shortly before `t`.
            let mut best: Option<(usize, Ticks)> = None;
            let mut count = 0;
            for (i, s) in st.video.iter().enumerate().filter(|(_, s)| s.key == key) {
                count += 1;
                if s.ring.lock().unwrap().find(t, max_size).is_some() {
                    best = Some((i, -1));
                    break;
                }
                if let Some(p) = s.position {
                    let d = t - p;
                    if (0..=TICKS_PER_SECOND).contains(&d) && best.is_none_or(|(_, bd)| d < bd) {
                        best = Some((i, d));
                    }
                }
            }
            let reuse = match best {
                Some((i, _)) => Some(i),
                None if count >= self.config.max_decoders_per_asset.max(1) => st
                    .video
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.key == key)
                    .min_by_key(|(_, s)| s.last_used)
                    .map(|(i, _)| i),
                None => None,
            };
            if let Some(i) = reuse {
                let s = &mut st.video[i];
                s.last_used = now;
                return Ok((s.dec.clone(), s.ring.clone()));
            }
        }
        // Open outside the lock: this takes a few milliseconds.
        let dec = VideoDecoder::open(path, self.config.hardware)?;
        self.opened.fetch_add(1, Ordering::Relaxed);
        let slot = VideoSlot {
            key: key.to_string(),
            dec: Arc::new(Mutex::new(dec)),
            ring: Arc::new(Mutex::new(Ring::default())),
            position: None,
            last_used: self.tick(),
        };
        let out = (slot.dec.clone(), slot.ring.clone());
        let mut st = self.state.lock().unwrap();
        st.video.push(slot);
        while st.video.len() > self.config.max_video_decoders.max(1) {
            // Evict the least recently used decoder that nobody is using.
            let victim = st
                .video
                .iter()
                .enumerate()
                .filter(|(_, s)| Arc::strong_count(&s.dec) == 1)
                .min_by_key(|(_, s)| s.last_used)
                .map(|(i, _)| i);
            match victim {
                Some(i) => {
                    st.video.remove(i);
                    self.evicted.fetch_add(1, Ordering::Relaxed);
                }
                None => break,
            }
        }
        Ok(out)
    }

    fn image_frame(&self, media_id: &str, path: &Path, max_size: Option<(u32, u32)>) -> Result<VideoFrame> {
        let key = (media_id.to_string(), max_size);
        {
            let mut st = self.state.lock().unwrap();
            let now = self.tick();
            if let Some((f, used)) = st.images.get_mut(&key) {
                *used = now;
                return Ok(f.clone());
            }
        }
        let frame = crate::image::load_image(path, max_size)?;
        let mut st = self.state.lock().unwrap();
        st.image_bytes += frame.data.len();
        let now = self.tick();
        st.images.insert(key, (frame.clone(), now));
        while st.image_bytes > self.config.image_cache_bytes && st.images.len() > 1 {
            let oldest = st.images.iter().min_by_key(|(_, (_, u))| *u).map(|(k, _)| k.clone()).unwrap();
            if let Some((f, _)) = st.images.remove(&oldest) {
                st.image_bytes -= f.data.len();
            }
        }
        Ok(frame)
    }

    /// `len` stereo frames of `media_id` starting at source time `start`, at
    /// the pool's sample rate, interleaved f32. Silence outside the audio.
    pub fn audio_samples(&self, media_id: &str, start: Ticks, len: usize) -> Result<Vec<f32>> {
        let asset = self.asset(media_id)?;
        if asset.kind == Some(MediaKind::Image) || is_image_path(&asset.path) {
            return Ok(vec![0.0; len * crate::audio::CHANNELS]);
        }
        let dec = self.checkout_audio(media_id, &asset.path, start)?;
        let mut d = dec.lock().unwrap();
        let out = d.read(start, len)?;
        let pos = d.position();
        drop(d);
        let mut st = self.state.lock().unwrap();
        if let Some(s) = st.audio.iter_mut().find(|s| Arc::ptr_eq(&s.dec, &dec)) {
            s.position = pos;
        }
        Ok(out)
    }

    fn checkout_audio(&self, key: &str, path: &Path, t: Ticks) -> Result<Arc<Mutex<AudioDecoder>>> {
        {
            let mut st = self.state.lock().unwrap();
            let now = self.tick();
            let mut best: Option<(usize, Ticks)> = None;
            let mut count = 0;
            for (i, s) in st.audio.iter().enumerate().filter(|(_, s)| s.key == key) {
                count += 1;
                if let Some(p) = s.position {
                    let d = (t - p).abs();
                    if d <= TICKS_PER_SECOND && best.is_none_or(|(_, bd)| d < bd) {
                        best = Some((i, d));
                    }
                }
            }
            let reuse = best.map(|b| b.0).or_else(|| {
                (count >= self.config.max_decoders_per_asset.max(1))
                    .then(|| st.audio.iter().enumerate().filter(|(_, s)| s.key == key).min_by_key(|(_, s)| s.last_used))
                    .flatten()
                    .map(|(i, _)| i)
            });
            if let Some(i) = reuse {
                st.audio[i].last_used = now;
                return Ok(st.audio[i].dec.clone());
            }
        }
        let dec = Arc::new(Mutex::new(AudioDecoder::open(path, self.config.sample_rate)?));
        let mut st = self.state.lock().unwrap();
        let now = self.tick();
        st.audio.push(AudioSlot { key: key.to_string(), dec: dec.clone(), position: None, last_used: now });
        while st.audio.len() > self.config.max_audio_decoders.max(1) {
            let victim = st
                .audio
                .iter()
                .enumerate()
                .filter(|(_, s)| Arc::strong_count(&s.dec) == 1)
                .min_by_key(|(_, s)| s.last_used)
                .map(|(i, _)| i);
            match victim {
                Some(i) => {
                    st.audio.remove(i);
                }
                None => break,
            }
        }
        Ok(dec)
    }
}

fn proxy_key(media_id: &str) -> String {
    format!("{media_id}\u{0}proxy")
}

impl FrameSource for MediaPool {
    fn frame(&self, media_id: &str, source_time: Ticks, max_size: Option<(u32, u32)>) -> Result<Option<VideoFrame>> {
        self.video_frame(media_id, source_time, max_size)
    }
}
