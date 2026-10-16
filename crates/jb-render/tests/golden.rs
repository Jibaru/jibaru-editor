//! Golden images: every registered item rendered at a small size and
//! compared with `tests/golden/<id>.png` using a tolerant metric (mean
//! absolute difference per channel). Missing goldens are written; set
//! `JB_UPDATE_GOLDEN=1` to regenerate all of them after intended changes.

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

#[test]
fn every_item_matches_its_golden() {
    let Some(mut r) = common::renderer(W, H) else { return };
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
                if d > tolerance(&id) {
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
    if written > 0 {
        eprintln!("wrote {written} golden images to {}", dir.display());
    }
    assert!(failures.is_empty(), "golden mismatches:\n{}", failures.join("\n"));
}
