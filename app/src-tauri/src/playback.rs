//! Real-time preview: the native engine renders the main sequence with
//! `jb-render` (effects, transitions and titles included) from frames
//! decoded by `jb-media`, and mixes audio with `jb_media::mixer` to a `cpal`
//! output stream.
//!
//! Threads:
//!
//! ```text
//! audio   mix ahead into the device FIFO; frames consumed = master clock
//! video   pick the frame for the clock (+ render lead), render, publish
//! decode  prefetch the next frames' sources so `video` finds them decoded
//! ```
//!
//! Shuttle (J/K/L: reverse, 0.5x, 2x, 4x, 8x) keeps audio as the master
//! clock: `jb_media::shuttle` plays the normal-speed mix time-stretched
//! (WSOLA), so speeds other than 1x keep the original pitch, like a
//! pitch-corrected NLE shuttle rather than a tape; reverse plays the grains
//! backwards. The device's consumed frames times the rate is the timeline
//! clock.
//!
//! Dropped-frame strategy: the video thread always renders the frame due at
//! the clock, skipping any it fell behind on, so A/V sync never drifts and a
//! slow frame only lowers the frame rate. When paused, the latest seek target
//! wins (intermediate scrub positions are skipped).
//!
//! Frames reach the webview through the `jb://localhost/live` route (long
//! poll, raw RGBA; see [`Playback::wait_frame`]).

use crate::audio_out::AudioShared;
use jb_core::eval::{evaluate, Layer, LayerContent};
use jb_core::{Project, Rational, Ticks, TICKS_PER_SECOND};
use jb_media::shuttle::Shuttle;
use jb_media::MediaPool;
use jb_render::Renderer;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Samples mixed per audio block.
const AUDIO_BLOCK: usize = 1024;
/// How far ahead audio is mixed, in seconds.
const AUDIO_AHEAD: f64 = 0.12;
/// Frames decoded ahead of the video thread.
const PREFETCH_FRAMES: i64 = 3;
/// Length of the audio snippet played when scrubbing, in seconds.
const SCRUB_SECONDS: f64 = 0.07;

/// A rendered preview frame.
pub struct LiveFrame {
    pub seq: u64,
    /// Timeline time of the frame.
    pub time: Ticks,
    pub width: u32,
    pub height: u32,
    /// Tightly packed RGBA8 (sRGB, straight alpha).
    pub rgba: Arc<Vec<u8>>,
    pub playing: bool,
}

struct Ctl {
    project: Arc<Project>,
    revision: u64,
    playing: bool,
    rate: f64,
    /// Timeline time at the clock anchor.
    anchor_time: Ticks,
    anchor_wall: Instant,
    /// Audio frames the device had played at the anchor.
    anchor_played: u64,
    audio_clock: bool,
    /// Time shown while paused.
    target: Ticks,
    size: (u32, u32),
    /// Bumped on every play/pause/seek, so threads drop stale work.
    generation: u64,
    scrub: Option<Ticks>,
    quit: bool,
}

#[derive(Default)]
struct Published {
    seq: u64,
    frame: Option<Arc<LiveFrame>>,
}

#[derive(Default, Clone, Serialize)]
pub struct Stats {
    /// Frames rendered since playback started.
    pub rendered: u64,
    /// Frames skipped to keep up with the clock.
    pub dropped: u64,
    /// Rendered frames per second over the last second of playback.
    pub fps: f64,
    /// Average render time (compose + readback) in milliseconds.
    pub render_ms: f64,
    pub audio_underruns: u64,
    pub audio_clock: bool,
    /// RMS level of the audio queued recently (before the master volume).
    pub audio_rms: f64,
    pub width: u32,
    pub height: u32,
}

struct Inner {
    pool: Arc<MediaPool>,
    audio: Option<Arc<AudioShared>>,
    ctl: Mutex<Ctl>,
    wake: Condvar,
    out: Mutex<Published>,
    out_cv: Condvar,
    stats: Mutex<Stats>,
    on_end: Box<dyn Fn(Ticks) + Send + Sync>,
}

#[derive(Clone)]
pub struct Playback {
    inner: Arc<Inner>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn secs(t: f64) -> Ticks {
    (t * TICKS_PER_SECOND as f64).round() as Ticks
}

impl Inner {
    /// Current playback time.
    fn clock(&self, c: &Ctl) -> Ticks {
        if !c.playing {
            return c.target;
        }
        if c.audio_clock {
            if let Some(a) = &self.audio {
                let played = a.played.load(Ordering::Relaxed).saturating_sub(c.anchor_played);
                let lat = a.latency_us.load(Ordering::Relaxed) as f64 / 1e6;
                let s = (played as f64 / a.sample_rate as f64 - lat).max(0.0);
                return c.anchor_time + secs(s * c.rate);
            }
        }
        c.anchor_time + secs(c.anchor_wall.elapsed().as_secs_f64() * c.rate)
    }

    fn audio_ok(&self) -> bool {
        self.audio.as_ref().is_some_and(|a| a.alive.load(Ordering::Relaxed))
    }
}

impl Playback {
    pub fn new(
        pool: Arc<MediaPool>,
        audio: Option<Arc<AudioShared>>,
        project: Project,
        on_end: impl Fn(Ticks) + Send + Sync + 'static,
    ) -> Self {
        let inner = Arc::new(Inner {
            pool,
            audio,
            ctl: Mutex::new(Ctl {
                project: Arc::new(project),
                revision: 0,
                playing: false,
                rate: 1.0,
                anchor_time: 0,
                anchor_wall: Instant::now(),
                anchor_played: 0,
                audio_clock: false,
                target: 0,
                size: (960, 540),
                generation: 0,
                scrub: None,
                quit: false,
            }),
            wake: Condvar::new(),
            out: Mutex::new(Published::default()),
            out_cv: Condvar::new(),
            stats: Mutex::new(Stats::default()),
            on_end: Box::new(on_end),
        });
        let i = inner.clone();
        std::thread::Builder::new().name("jb-play-video".into()).spawn(move || video_thread(i)).expect("spawn video");
        let i = inner.clone();
        std::thread::Builder::new()
            .name("jb-play-decode".into())
            .spawn(move || prefetch_thread(i))
            .expect("spawn decode");
        if inner.audio.is_some() {
            let i = inner.clone();
            std::thread::Builder::new()
                .name("jb-play-audio".into())
                .spawn(move || audio_thread(i))
                .expect("spawn audio");
        }
        Self { inner }
    }

    /// Replaces the project being previewed (after every edit).
    pub fn set_project(&self, project: Project) {
        self.inner.pool.sync(&project);
        let mut c = lock(&self.inner.ctl);
        c.project = Arc::new(project);
        c.revision += 1;
        drop(c);
        self.inner.wake.notify_all();
    }

    pub fn play(&self, time: Ticks, rate: f64) {
        let mut c = lock(&self.inner.ctl);
        let duration = c.project.main().duration();
        c.playing = true;
        c.rate = if rate == 0.0 { 1.0 } else { rate };
        c.anchor_time = time.clamp(0, duration.max(0));
        c.anchor_wall = Instant::now();
        // Audio is the master clock at every speed: shuttle audio is
        // time-stretched (see `audio_thread`).
        c.audio_clock = self.inner.audio_ok();
        if let Some(a) = &self.inner.audio {
            a.clear();
            c.anchor_played = a.played.load(Ordering::Relaxed);
        }
        c.generation += 1;
        c.scrub = None;
        drop(c);
        *lock(&self.inner.stats) = Stats::default();
        self.inner.wake.notify_all();
    }

    /// Pauses and returns the time playback stopped at.
    pub fn pause(&self) -> Ticks {
        let mut c = lock(&self.inner.ctl);
        let now = self.inner.clock(&c);
        if c.playing {
            c.playing = false;
            c.target = now;
            c.generation += 1;
        }
        if let Some(a) = &self.inner.audio {
            a.clear();
        }
        drop(c);
        self.inner.wake.notify_all();
        now
    }

    /// Shows `time` while paused; `scrub_audio` plays a short audio snippet.
    pub fn seek(&self, time: Ticks, scrub_audio: bool) {
        let mut c = lock(&self.inner.ctl);
        if c.playing {
            let rate = c.rate;
            drop(c);
            return self.play(time, rate);
        }
        c.target = time.max(0);
        c.generation += 1;
        c.scrub = scrub_audio.then_some(c.target);
        drop(c);
        self.inner.wake.notify_all();
    }

    pub fn set_size(&self, width: u32, height: u32) {
        let mut c = lock(&self.inner.ctl);
        c.size = ((width.clamp(64, 3840)) & !1, (height.clamp(36, 2160)) & !1);
        drop(c);
        self.inner.wake.notify_all();
    }

    pub fn set_volume(&self, v: f32) {
        if let Some(a) = &self.inner.audio {
            a.set_volume(v);
        }
    }

    pub fn is_playing(&self) -> bool {
        lock(&self.inner.ctl).playing
    }

    pub fn stats(&self) -> Stats {
        let mut s = lock(&self.inner.stats).clone();
        if let Some(a) = &self.inner.audio {
            s.audio_underruns = a.underruns.load(Ordering::Relaxed);
        }
        s.audio_clock = lock(&self.inner.ctl).audio_clock;
        s
    }

    /// Waits up to `timeout` for a frame newer than `after`.
    pub fn wait_frame(&self, after: u64, timeout: Duration) -> Option<Arc<LiveFrame>> {
        let deadline = Instant::now() + timeout;
        let mut out = lock(&self.inner.out);
        loop {
            if out.seq > after {
                return out.frame.clone();
            }
            let left = deadline.checked_duration_since(Instant::now())?;
            out = self.inner.out_cv.wait_timeout(out, left).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}

fn prefetch(layers: &[Layer], pool: &MediaPool, max: Option<(u32, u32)>) {
    for l in layers {
        match &l.content {
            LayerContent::Media { media_id, source_time } => {
                let _ = pool.video_frame(media_id, *source_time, max);
            }
            LayerContent::Nested(p) => prefetch(&p.layers, pool, max),
            _ => {}
        }
        if let Some(tr) = &l.transition {
            prefetch(std::slice::from_ref(&tr.incoming), pool, max);
        }
    }
}

/// Output size that fits `size` with the project's aspect ratio.
fn fit_size(p: &Project, size: (u32, u32)) -> (u32, u32) {
    let (pw, ph) = (p.settings.width.max(2) as f64, p.settings.height.max(2) as f64);
    let k = (size.0 as f64 / pw).min(size.1 as f64 / ph).min(1.0);
    let w = ((pw * k).round() as u32).max(2) & !1;
    let h = ((ph * k).round() as u32).max(2) & !1;
    (w, h)
}

fn frame_rate(p: &Project) -> Rational {
    let r = p.settings.frame_rate;
    if r.num <= 0 || r.den <= 0 {
        Rational::new(30, 1)
    } else {
        r
    }
}

fn video_thread(inner: Arc<Inner>) {
    let mut renderer: Option<Renderer> = None;
    // What was last rendered: (generation, revision, frame index, size).
    let mut last: Option<(u64, u64, i64, (u32, u32))> = None;
    let mut last_index: Option<i64> = None;
    let mut render_ms = 8.0f64;
    let mut window: (Instant, u64) = (Instant::now(), 0);
    loop {
        let mut c = lock(&inner.ctl);
        if c.quit {
            return;
        }
        let project = c.project.clone();
        let fr = frame_rate(&project);
        let size = fit_size(&project, c.size);
        let duration = project.main().duration();
        let now = inner.clock(&c);
        let (index, playing) = if c.playing {
            // Forward stops at the end, reverse shuttle at the start.
            if now >= duration || (c.rate < 0.0 && now <= 0) {
                c.playing = false;
                c.target = if c.rate < 0.0 { 0 } else { (duration - 1).max(0) };
                c.generation += 1;
                let end = c.target;
                drop(c);
                if let Some(a) = &inner.audio {
                    a.clear();
                }
                (inner.on_end)(end);
                continue;
            }
            let lead = secs(render_ms / 1000.0 * c.rate.abs());
            let t = if c.rate >= 0.0 { now + lead } else { now - lead };
            (fr.frame_at(t.clamp(0, (duration - 1).max(0))), true)
        } else {
            (fr.frame_at(c.target), false)
        };
        let key = (if playing { 0 } else { c.generation }, c.revision, index, size);
        if last == Some(key) {
            if playing {
                // Sleep until the next frame is due.
                let next = fr.frame_start(index + 1);
                let wait = ((next - now) as f64 / TICKS_PER_SECOND as f64 / c.rate.abs().max(0.1)).clamp(0.001, 0.05);
                let _ = inner.wake.wait_timeout(c, Duration::from_secs_f64(wait));
            } else {
                let _ = inner.wake.wait_timeout(c, Duration::from_millis(250));
            }
            continue;
        }
        drop(c);

        let time = fr.frame_start(index);
        if renderer.is_none() {
            match Renderer::new(size.0, size.1) {
                Ok(r) => renderer = Some(r),
                Err(e) => {
                    eprintln!("preview renderer unavailable: {e}");
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
            }
        }
        let r = renderer.as_mut().expect("renderer");
        if r.size() != size {
            r.resize(size.0, size.1);
        }
        r.options.source_max_size = Some(size);
        let started = Instant::now();
        let plan = evaluate(&project, project.main(), time);
        let frame = r.render(&project, &plan, inner.pool.as_ref());
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        render_ms = render_ms * 0.85 + ms * 0.15;

        {
            let mut s = lock(&inner.stats);
            if playing {
                s.rendered += 1;
                if let Some(li) = last_index {
                    let step = (index - li).abs();
                    if step > 1 {
                        s.dropped += (step - 1) as u64;
                    }
                }
                window.1 += 1;
                let el = window.0.elapsed().as_secs_f64();
                if el >= 1.0 {
                    s.fps = window.1 as f64 / el;
                    window = (Instant::now(), 0);
                }
            } else {
                window = (Instant::now(), 0);
            }
            s.render_ms = render_ms;
            s.width = size.0;
            s.height = size.1;
        }
        last_index = playing.then_some(index);
        last = Some(key);
        let mut out = lock(&inner.out);
        out.seq += 1;
        out.frame = Some(Arc::new(LiveFrame {
            seq: out.seq,
            time,
            width: frame.width,
            height: frame.height,
            rgba: frame.data.clone(),
            playing,
        }));
        drop(out);
        inner.out_cv.notify_all();
    }
}

fn prefetch_thread(inner: Arc<Inner>) {
    let mut done: Option<(u64, u64, i64)> = None;
    loop {
        let c = lock(&inner.ctl);
        if c.quit {
            return;
        }
        if !c.playing || c.rate < 0.0 {
            let _ = inner.wake.wait_timeout(c, Duration::from_millis(200));
            continue;
        }
        let project = c.project.clone();
        let fr = frame_rate(&project);
        let size = fit_size(&project, c.size);
        let base = fr.frame_at(inner.clock(&c));
        let key = (c.generation, c.revision);
        drop(c);
        let from = match done {
            Some((g, r, i)) if (g, r) == key && i >= base => i + 1,
            _ => base + 1,
        };
        let to = base + PREFETCH_FRAMES;
        if from > to {
            std::thread::sleep(Duration::from_millis(4));
            continue;
        }
        let duration = project.main().duration();
        for i in from..=to {
            let t = fr.frame_start(i);
            if t >= duration {
                break;
            }
            let plan = evaluate(&project, project.main(), t);
            prefetch(&plan.layers, &inner.pool, Some(size));
            done = Some((key.0, key.1, i));
        }
    }
}

fn audio_thread(inner: Arc<Inner>) {
    let audio = inner.audio.clone().expect("audio device");
    let rate = audio.sample_rate;
    let mut generation = u64::MAX;
    // Output frames queued since the clock anchor, and the shuttle that maps
    // them to the timeline (with the project revision its cache belongs to).
    let mut played: i64 = 0;
    let mut shuttle: Option<(u64, Shuttle)> = None;
    let ahead = (AUDIO_AHEAD * rate as f64) as usize;
    loop {
        let mut c = lock(&inner.ctl);
        if c.quit {
            return;
        }
        if c.generation != generation {
            generation = c.generation;
            played = 0;
            shuttle = None;
        }
        let project = c.project.clone();
        if c.playing && c.audio_clock {
            if audio.queued() >= ahead {
                let _ = inner.wake.wait_timeout(c, Duration::from_millis(4));
                continue;
            }
            let revision = c.revision;
            let origin = jb_media::audio::ticks_to_samples(c.anchor_time, rate);
            let (rev, s) = shuttle.get_or_insert_with(|| (revision, Shuttle::new(origin, c.rate)));
            if *rev != revision {
                // Edited while playing: same time mapping, fresh audio.
                s.invalidate();
                *rev = revision;
            }
            drop(c);
            let block = match s.render(&project, project.main(), inner.pool.as_ref(), played, AUDIO_BLOCK) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("audio mix: {e:#}");
                    vec![0.0; AUDIO_BLOCK * 2]
                }
            };
            // Only queue it if nothing was seeked meanwhile.
            let c = lock(&inner.ctl);
            if c.generation == generation && c.playing {
                audio.push(&block);
                played += AUDIO_BLOCK as i64;
                drop(c);
                let ms = block.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / block.len().max(1) as f64;
                let mut s = lock(&inner.stats);
                s.audio_rms = s.audio_rms * 0.9 + ms.sqrt() * 0.1;
            }
        } else if let Some(t) = c.scrub.take() {
            drop(c);
            let n = (SCRUB_SECONDS * rate as f64) as usize;
            let start = jb_media::audio::ticks_to_samples(t, rate);
            if let Ok(mut s) = jb_media::mixer::mix(&project, project.main(), start, n, inner.pool.as_ref()) {
                // Short fades so snippets do not click.
                let fade = (n / 6).max(1);
                for i in 0..n {
                    let g = (i.min(n - 1 - i) as f32 / fade as f32).min(1.0);
                    s[i * 2] *= g;
                    s[i * 2 + 1] *= g;
                }
                let c = lock(&inner.ctl);
                if !c.playing {
                    audio.clear();
                    audio.push(&s);
                }
            }
        } else {
            let _ = inner.wake.wait_timeout(c, Duration::from_millis(100));
        }
    }
}
