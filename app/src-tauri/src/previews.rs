//! Library card previews (see `jb_render::preview`): a still and a hover
//! sprite sheet per Transitions/Effects/Filters/Animation/Titles item,
//! rendered on a dedicated GPU renderer and cached on disk under the app
//! cache directory, in a folder named after the renderer version (older
//! versions are deleted at startup).
//!
//! Requests from the webview (`jb://localhost/fxpreview`) are urgent and
//! served newest first, so the cards currently scrolled into view come
//! before ones that scrolled past. When nothing is requested and playback
//! is idle, the rest of the catalog is rendered in the background.

use jb_core::{Project, Ticks, TitleData};
use jb_render::preview::{
    footage_preview, sample_preview, PreviewCache, PreviewItem, SampleSource, CARD_LAYOUT, PREVIEW_VERSION,
};
use jb_render::{Registry, Renderer};
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// What the webview asks a preview for.
#[derive(Debug, Clone, Deserialize)]
pub struct PreviewRequest {
    /// Registry id (effects, filters, transitions, animations).
    #[serde(default)]
    pub fx: Option<String>,
    /// Title template data.
    #[serde(default)]
    pub title: Option<TitleData>,
}

impl PreviewRequest {
    pub fn item(&self) -> Option<PreviewItem> {
        match (&self.fx, &self.title) {
            (Some(id), _) => Some(PreviewItem::Fx(id.clone())),
            (None, Some(t)) => Some(PreviewItem::Title(t.clone())),
            _ => None,
        }
    }
}

pub fn registry() -> &'static Registry {
    static R: OnceLock<Registry> = OnceLock::new();
    R.get_or_init(Registry::builtin)
}

#[derive(Default)]
struct Queue {
    /// Requested by the webview; the last one is served first.
    urgent: Vec<PreviewItem>,
    background: VecDeque<PreviewItem>,
    failed: HashSet<String>,
}

struct Inner {
    cache: PreviewCache,
    queue: Mutex<Queue>,
    /// Wakes the worker.
    work: Condvar,
    /// Signals a finished item.
    done: Condvar,
    busy: Box<dyn Fn() -> bool + Send + Sync>,
}

#[derive(Clone)]
pub struct Previews {
    inner: Arc<Inner>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone, Copy)]
pub enum Part {
    Still,
    Sprite,
}

impl Previews {
    /// `root` is the cache folder; `busy` pauses background work (playback).
    pub fn new(root: PathBuf, busy: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        let cache = PreviewCache::new(root);
        let c = cache.clone();
        std::thread::Builder::new()
            .name("jb-previews-prune".into())
            .spawn(move || c.prune_other_versions())
            .expect("spawn prune");
        let inner = Arc::new(Inner {
            cache,
            queue: Mutex::new(Queue::default()),
            work: Condvar::new(),
            done: Condvar::new(),
            busy: Box::new(busy),
        });
        let i = inner.clone();
        std::thread::Builder::new().name("jb-previews".into()).spawn(move || worker(i)).expect("spawn previews");
        Self { inner }
    }

    /// Queues every registry item for background rendering.
    pub fn prefetch_catalog(&self) {
        let mut q = lock(&self.inner.queue);
        for i in registry().iter() {
            let item = PreviewItem::Fx(i.id.to_string());
            if !self.inner.cache.has(&item) {
                q.background.push_back(item);
            }
        }
        drop(q);
        self.inner.work.notify_all();
    }

    /// The still or sprite JPEG of `item`, rendering it first when needed.
    pub fn get(&self, item: &PreviewItem, part: Part, timeout: Duration) -> anyhow::Result<Vec<u8>> {
        let (still, sprite) = self.inner.cache.paths(item);
        let path = match part {
            Part::Still => still,
            Part::Sprite => sprite,
        };
        if let Ok(b) = std::fs::read(&path) {
            return Ok(b);
        }
        let key = item.key();
        let deadline = Instant::now() + timeout;
        let mut q = lock(&self.inner.queue);
        q.urgent.retain(|i| i != item);
        q.urgent.push(item.clone());
        self.inner.work.notify_all();
        loop {
            if self.inner.cache.has(item) {
                drop(q);
                return Ok(std::fs::read(&path)?);
            }
            if q.failed.contains(&key) {
                anyhow::bail!("cannot render the preview of {key}");
            }
            let left = deadline.checked_duration_since(Instant::now()).ok_or_else(|| anyhow::anyhow!("timed out"))?;
            q = self.inner.done.wait_timeout(q, left).unwrap_or_else(|e| e.into_inner()).0;
        }
    }

    pub fn info() -> serde_json::Value {
        serde_json::json!({ "version": PREVIEW_VERSION, "layout": CARD_LAYOUT })
    }
}

fn worker(inner: Arc<Inner>) {
    let samples = SampleSource::new();
    let mut renderer: Option<Renderer> = None;
    loop {
        let item = {
            let mut q = lock(&inner.queue);
            loop {
                if let Some(i) = q.urgent.pop() {
                    break i;
                }
                if !(inner.busy)() {
                    if let Some(i) = q.background.pop_front() {
                        break i;
                    }
                }
                q = inner.work.wait_timeout(q, Duration::from_millis(500)).unwrap_or_else(|e| e.into_inner()).0;
            }
        };
        if inner.cache.has(&item) {
            inner.done.notify_all();
            continue;
        }
        if renderer.is_none() {
            match Renderer::new(CARD_LAYOUT.frame_w, CARD_LAYOUT.frame_h) {
                Ok(r) => renderer = Some(r),
                Err(e) => {
                    eprintln!("library previews unavailable: {e}");
                    lock(&inner.queue).failed.insert(item.key());
                    inner.done.notify_all();
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            }
        }
        let r = renderer.as_mut().expect("renderer");
        if let Err(e) = inner.cache.ensure(r, &samples, &item) {
            eprintln!("preview {}: {e:#}", item.key());
            lock(&inner.queue).failed.insert(item.key());
        }
        inner.done.notify_all();
        // Leave the GPU to the player between background items.
        if lock(&inner.queue).urgent.is_empty() {
            std::thread::sleep(Duration::from_millis(15));
        }
    }
}

/// The project the player shows for a library item: effects and filters
/// on the timeline footage at `at` (Filmora), otherwise the sample footage.
pub fn player_project(req: &PreviewRequest, timeline: &Project, at: Ticks) -> Option<(Project, Ticks)> {
    let item = req.item()?;
    if let PreviewItem::Fx(id) = &item {
        if let Some(p) = footage_preview(registry(), timeline, at, id) {
            return Some((p.project, p.duration));
        }
    }
    let p = sample_preview(registry(), &item, 1920, 1080)?;
    Some((p.project, p.duration))
}
