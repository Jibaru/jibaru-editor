//! Preview proxies. Camera proxies (GoPro `.LRV`, DJI `.LRF`) are used as
//! soon as media is registered; large videos without one get a 540p proxy
//! generated in the background by `jb-media` (one at a time, paused while
//! the preview is playing so it never steals decoder time from playback).
//! Status changes are reported through a callback (`proxy-status` events).

use jb_core::{MediaKind, Project};
use jb_media::{MediaCache, MediaPool, ProxyOptions};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProxyState {
    Queued,
    Generating,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxyStatus {
    pub media_id: String,
    pub path: String,
    pub state: ProxyState,
    /// 0..1 while generating.
    pub progress: f32,
    pub proxy: Option<String>,
    /// "camera" for .LRV/.LRF files, "generated" for jb-media proxies.
    pub source: Option<&'static str>,
    pub error: Option<String>,
}

struct Job {
    media_id: String,
    path: PathBuf,
}

struct Inner {
    cache: Arc<MediaCache>,
    pool: Arc<MediaPool>,
    queue: Mutex<VecDeque<Job>>,
    cv: Condvar,
    status: Mutex<HashMap<String, ProxyStatus>>,
    busy: Box<dyn Fn() -> bool + Send + Sync>,
    emit: Box<dyn Fn(&ProxyStatus) + Send + Sync>,
}

pub struct Proxies {
    inner: Arc<Inner>,
}

/// Videos larger than this get a generated proxy.
fn needs_proxy(w: u32, h: u32) -> bool {
    (w as u64) * (h as u64) > 1920 * 1080
}

impl Proxies {
    pub fn new(
        cache: Arc<MediaCache>,
        pool: Arc<MediaPool>,
        busy: impl Fn() -> bool + Send + Sync + 'static,
        emit: impl Fn(&ProxyStatus) + Send + Sync + 'static,
    ) -> Self {
        let inner = Arc::new(Inner {
            cache,
            pool,
            queue: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
            status: Mutex::new(HashMap::new()),
            busy: Box::new(busy),
            emit: Box::new(emit),
        });
        let i = inner.clone();
        std::thread::Builder::new().name("jb-proxies".into()).spawn(move || worker(i)).expect("spawn proxies");
        Self { inner }
    }

    pub fn statuses(&self) -> Vec<ProxyStatus> {
        self.inner.status.lock().unwrap().values().cloned().collect()
    }

    /// Registers proxies for `project` and queues generation for large
    /// videos used on the timeline that have none.
    pub fn register(&self, project: &Project) {
        let used: HashSet<&str> = project
            .sequences
            .iter()
            .flat_map(|s| s.tracks.iter())
            .flat_map(|t| t.clips.iter())
            .filter_map(|c| match &c.source {
                jb_core::ClipSource::Media { media_id } => Some(media_id.as_str()),
                _ => None,
            })
            .collect();
        let mut changed = Vec::new();
        {
            let mut st = self.inner.status.lock().unwrap();
            let mut q = self.inner.queue.lock().unwrap();
            for m in project.media.iter().filter(|m| m.kind == MediaKind::Video) {
                if let Some(s) = st.get(&m.id) {
                    if s.path == m.path {
                        if let Some(p) = &s.proxy {
                            self.inner.pool.set_proxy(&m.id, Some(PathBuf::from(p)));
                        }
                        continue;
                    }
                }
                let path = Path::new(&m.path);
                let ready = |proxy: PathBuf, source| ProxyStatus {
                    media_id: m.id.clone(),
                    path: m.path.clone(),
                    state: ProxyState::Ready,
                    progress: 1.0,
                    proxy: Some(proxy.to_string_lossy().into_owned()),
                    source: Some(source),
                    error: None,
                };
                let status = if let Some(p) = jb_media::cache::camera_proxy(path) {
                    ready(p, "camera")
                } else if let Some(p) = self.inner.cache.existing_proxy(path) {
                    ready(p, "generated")
                } else if used.contains(m.id.as_str()) && needs_proxy(m.width, m.height) && path.exists() {
                    q.push_back(Job { media_id: m.id.clone(), path: path.to_path_buf() });
                    ProxyStatus {
                        media_id: m.id.clone(),
                        path: m.path.clone(),
                        state: ProxyState::Queued,
                        progress: 0.0,
                        proxy: None,
                        source: None,
                        error: None,
                    }
                } else {
                    continue;
                };
                if let Some(p) = &status.proxy {
                    self.inner.pool.set_proxy(&m.id, Some(PathBuf::from(p)));
                }
                st.insert(m.id.clone(), status.clone());
                changed.push(status);
            }
        }
        for s in &changed {
            (self.inner.emit)(s);
        }
        self.inner.cv.notify_all();
    }
}

fn worker(inner: Arc<Inner>) {
    loop {
        let job = {
            let mut q = inner.queue.lock().unwrap();
            loop {
                if let Some(j) = q.pop_front() {
                    break j;
                }
                q = inner.cv.wait(q).unwrap();
            }
        };
        let set = |f: &dyn Fn(&mut ProxyStatus)| {
            let snapshot = {
                let mut st = inner.status.lock().unwrap();
                let Some(s) = st.get_mut(&job.media_id) else { return };
                f(s);
                s.clone()
            };
            (inner.emit)(&snapshot);
        };
        // The media may have been removed or replaced meanwhile.
        let current = inner.status.lock().unwrap().get(&job.media_id).map(|s| s.path.clone());
        if current.as_deref() != Some(job.path.to_string_lossy().as_ref()) {
            continue;
        }
        set(&|s| s.state = ProxyState::Generating);
        let mut last = Instant::now();
        let opts = ProxyOptions { height: 540, ..Default::default() };
        let result = inner.cache.ensure_proxy(&job.path, &opts, &mut |p| {
            // Yield to playback: decoders and the GPU are busier than us.
            while (inner.busy)() {
                std::thread::sleep(Duration::from_millis(150));
            }
            if last.elapsed() >= Duration::from_millis(400) {
                last = Instant::now();
                set(&|s| s.progress = p);
            }
            true
        });
        match result {
            Ok(p) => {
                inner.pool.set_proxy(&job.media_id, Some(p.clone()));
                set(&|s| {
                    s.state = ProxyState::Ready;
                    s.progress = 1.0;
                    s.proxy = Some(p.to_string_lossy().into_owned());
                    s.source = Some("generated");
                });
            }
            Err(e) => {
                eprintln!("proxy for {}: {e:#}", job.path.display());
                set(&|s| {
                    s.state = ProxyState::Failed;
                    s.error = Some(format!("{e:#}"));
                });
            }
        }
    }
}
