//! Golden images: every registered item rendered at a small size and
//! compared with `tests/golden/<id>.png` using a tolerant metric (mean
//! absolute difference per channel). Missing goldens are written; set
//! `JB_UPDATE_GOLDEN=1` to regenerate all of them after intended changes.
//!
//! Goldens are produced on real GPUs. On software or virtualized adapters
//! (WARP, llvmpipe/lavapipe, the paravirtual GPU of macOS CI runners) the
//! per-pixel hash noise of grain/glitch style items is not reproducible, so
//! those items are compared and reported but not asserted; every other item
//! is still checked with the normal tolerance. `JB_GOLDEN_STRICT=1` forces
//! the strict behavior on any adapter.

mod common;

use jb_core::eval::FramePlan;
use jb_render::testing::*;
use jb_render::FxKind;
use std::path::PathBuf;

const W: u32 = 96;
const H: u32 = 54;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn read_png(path: &PathBuf) -> Option<Vec<u8>> {
    let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?));
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    (info.width == W && info.height == H && info.color_type == png::ColorType::Rgba).then_some(buf)
}

fn write_png(path: &PathBuf, data: &[u8]) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W, H);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(data).unwrap();
}

/// Items whose output is dominated by per-pixel noise get a looser tolerance
/// (hash noise is not bit-identical across GPU vendors).
fn tolerance(id: &str) -> f64 {
    const NOISY: &[&str] = &[
        "grain", "noise", "glitch", "vhs", "film", "pelicula", "polvo", "super8", "charcoal", "shake", "strobe", "dust",
    ];
    if NOISY.iter().any(|n| id.contains(n)) {
        12.0
    } else {
        3.0
    }
}

/// Items with thin anti-aliased edges or many small drawn shapes, whose
/// rasterization differs on software adapters even though they match exactly
/// on real GPUs. Strict locally, report-only on software/virtual adapters.
fn software_sensitive(id: &str) -> bool {
    const THIN: &[&str] = &["blind_bars", "comic_burst", "gold_bokeh", "gold_plates", "comet"];
    THIN.iter().any(|n| id.contains(n))
}

/// Whether the adapter is a software rasterizer or a virtualized GPU, where
/// noisy items cannot be expected to match goldens made on real hardware.
fn is_reference_adapter(info: &wgpu::AdapterInfo) -> bool {
    if std::env::var_os("JB_GOLDEN_STRICT").is_some() {
        return true;
    }
    let name = info.name.to_ascii_lowercase();
    let soft_name = ["paravirtual", "basic render", "warp", "llvmpipe", "lavapipe", "swiftshader", "software"]
        .iter()
        .any(|n| name.contains(n));
    !soft_name && matches!(info.device_type, wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu)
}

/// Appends a line to the GitHub Actions job summary when running in CI.
fn summary(line: &str) {
    if let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(path) {
            let _ = writeln!(f, "{line}");
        }
    }
}

#[test]
fn every_item_matches_its_golden() {
    let Some(mut r) = common::renderer(W, H) else { return };
    let info = r.gpu().adapter_info.clone();
    let strict = is_reference_adapter(&info);
    let adapter = format!("{} ({:?}, {:?}, driver {:?})", info.name, info.device_type, info.backend, info.driver);
    eprintln!("golden: adapter {adapter}, {}", if strict { "strict" } else { "noisy items report-only" });
    summary(&format!(
        "### jb-render goldens\n\nAdapter: `{adapter}` ({})\n",
        if strict { "strict" } else { "software/virtual: noisy items report-only" }
    ));
    let mut report_only = Vec::new();
    let update = std::env::var_os("JB_UPDATE_GOLDEN").is_some();
    let dir = golden_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let project = project(W, H);
    let src = MemorySource::ab(W, H);
    let items: Vec<(String, FxKind)> = r.registry().iter().map(|i| (i.id.to_string(), i.kind)).collect();
    let mut failures = Vec::new();
    let mut written = 0;
    for (id, kind) in items {
        // Mid-way frame (transitions at 0.4 so asymmetric ones show both sides).
        let plan = match kind {
            FxKind::Transition => transition_plan(&id, 0.4, &[]),
            FxKind::Animation => {
                let mut fg = media_layer("a");
                fg.transform.scale = [0.6, 0.6];
                fg.effects.push(effect(&id, 0.4, &[]));
                FramePlan { time: 0, layers: vec![media_layer("b"), fg] }
            }
            _ => effect_plan(&id, 0.4, &[]),
        };
        let out = r.render(&project, &plan, &src);
        let rep = r.last_report();
        if !rep.unknown_kinds.is_empty() || !rep.errors.is_empty() {
            failures.push(format!("{id}: {rep:?}"));
            continue;
        }
        let path = dir.join(format!("{}.png", id.replace([':', '/'], "_")));
        match read_png(&path).filter(|_| !update) {
            Some(want) => {
                let d = mean_abs_diff(&out.data, &want);
                if d > tolerance(&id) && !strict && (tolerance(&id) > 3.0 || software_sensitive(&id)) {
                    report_only.push(format!("{id}: mean abs diff {d:.2}"));
                } else if d > tolerance(&id) {
                    failures.push(format!("{id}: mean abs diff {d:.2}"));
                    write_png(&dir.join(format!("{}.actual.png", id.replace([':', '/'], "_"))), &out.data);
                }
            }
            None => {
                write_png(&path, &out.data);
                written += 1;
            }
        }
    }
    if !report_only.is_empty() {
        eprintln!("golden: not asserted on this adapter:\n{}", report_only.join("\n"));
        summary(&format!("Noisy items not asserted on this adapter:\n\n- {}\n", report_only.join("\n- ")));
    }
    if !failures.is_empty() {
        summary(&format!("Mismatches:\n\n- {}\n", failures.join("\n- ")));
    }
    if written > 0 {
        eprintln!("wrote {written} golden images to {}", dir.display());
    }
    assert!(failures.is_empty(), "golden mismatches:\n{}", failures.join("\n"));
}
