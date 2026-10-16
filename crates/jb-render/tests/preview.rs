//! Library preview generator: sample footage, preview projects, card
//! stills and sprite sheets, and the on-disk cache.

mod common;

use jb_core::frame::FrameSource;
use jb_core::TICKS_PER_SECOND;
use jb_render::preview::*;
use jb_render::testing::mean_abs_diff;
use jb_render::{FxKind, Registry};

fn decode(jpeg: &[u8]) -> image::RgbImage {
    image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg).expect("valid jpeg").to_rgb8()
}

fn tile(sheet: &image::RgbImage, layout: SpriteLayout, i: u32) -> Vec<u8> {
    let (x0, y0) = ((i % layout.cols) * layout.frame_w, (i / layout.cols) * layout.frame_h);
    let mut out = Vec::new();
    for y in 0..layout.frame_h {
        for x in 0..layout.frame_w {
            out.extend_from_slice(&sheet.get_pixel(x0 + x, y0 + y).0);
        }
    }
    out
}

#[test]
fn sample_footage_moves_and_differs_between_scenes() {
    let s = SampleSource::new();
    let a0 = s.frame(SAMPLE_A, 0, Some((320, 180))).unwrap().unwrap();
    let a1 = s.frame(SAMPLE_A, TICKS_PER_SECOND, Some((320, 180))).unwrap().unwrap();
    let b0 = s.frame(SAMPLE_B, 0, Some((320, 180))).unwrap().unwrap();
    assert_eq!((a0.width, a0.height), (320, 180));
    assert!(mean_abs_diff(&a0.data, &a1.data) > 2.0, "scene A pans");
    assert!(mean_abs_diff(&a0.data, &b0.data) > 30.0, "A and B contrast");
    assert!(s.frame("other", 0, None).unwrap().is_none());
    // Cool A, warm B.
    let avg = |d: &[u8], c: usize| d.chunks(4).map(|p| p[c] as f64).sum::<f64>() / (d.len() / 4) as f64;
    assert!(avg(&a0.data, 2) > avg(&a0.data, 0));
    assert!(avg(&b0.data, 0) > avg(&b0.data, 2));
}

/// The scenes are resolution independent: a 1080p frame is the same
/// picture as a small one, with finer detail (not an upscale).
#[test]
fn sample_footage_is_drawn_at_the_requested_size() {
    let s = SampleSource::new();
    for id in [SAMPLE_A, SAMPLE_B] {
        let big = s.frame(id, TICKS_PER_SECOND / 2, Some((1920, 1080))).unwrap().unwrap();
        let small = s.frame(id, TICKS_PER_SECOND / 2, Some((480, 270))).unwrap().unwrap();
        assert_eq!((big.width, big.height, small.width, small.height), (1920, 1080, 480, 270));
        // Box-shrink the big frame 4x: it matches the small one.
        let mut shrunk = vec![0u8; 480 * 270 * 4];
        let mut fine = 0.0f64;
        for y in 0..270 {
            for x in 0..480 {
                for c in 0..3 {
                    let px = |dx: usize, dy: usize| big.data[((y * 4 + dy) * 1920 + x * 4 + dx) * 4 + c] as f64;
                    let mut sum = 0.0;
                    for dy in 0..4 {
                        for dx in 0..4 {
                            sum += px(dx, dy);
                        }
                    }
                    let mean = sum / 16.0;
                    shrunk[(y * 480 + x) * 4 + c] = mean.round() as u8;
                    // Detail inside the 4x4 block, beyond film grain.
                    let (l, r) = ((px(0, 0) + px(1, 0)) / 2.0, (px(2, 0) + px(3, 0)) / 2.0);
                    fine += (l - r).abs();
                }
                shrunk[(y * 480 + x) * 4 + 3] = 255;
            }
        }
        let d = mean_abs_diff(&shrunk, &small.data);
        assert!(d < 6.0, "{id}: 1080p and 270p frames differ by {d:.2}");
        let fine = fine / (480.0 * 270.0 * 3.0);
        assert!(fine > 1.5, "{id}: no sub-block detail at 1080p ({fine:.2})");
    }
}

#[test]
fn every_registry_item_has_a_preview_project() {
    let reg = Registry::builtin();
    for info in reg.iter() {
        let p = sample_preview(&reg, &PreviewItem::Fx(info.id.into()), 256, 144).expect(info.id);
        assert_eq!(p.duration, PREVIEW_DURATION);
        assert!(p.still > 0 && p.still < p.duration);
        assert_eq!(p.split_still, info.kind == FxKind::Filter);
    }
    assert!(sample_preview(&reg, &PreviewItem::Fx("nope".into()), 256, 144).is_none());
}

#[test]
fn footage_preview_wraps_the_timeline_with_the_effect() {
    let reg = Registry::builtin();
    let base = sample_preview(&reg, &PreviewItem::Fx("jb.dissolve".into()), 640, 360).unwrap().project;
    let p = footage_preview(&reg, &base, TICKS_PER_SECOND / 2, "jb.gaussian_blur").expect("footage preview");
    let main = p.project.main();
    assert_ne!(main.id, base.main_sequence);
    let clip = &main.tracks[0].clips[0];
    assert_eq!(clip.source_in, TICKS_PER_SECOND / 2);
    assert_eq!(clip.effects[0].kind, "jb.gaussian_blur");
    // Shorter than 5 s when the timeline ends earlier.
    assert_eq!(p.duration, base.main().duration() - TICKS_PER_SECOND / 2);
    // Transitions are previewed on the sample footage instead.
    assert!(footage_preview(&reg, &base, 0, "jb.dissolve").is_none());
}

#[test]
fn renders_cards_and_caches_them() {
    let Some(mut r) = common::renderer(64, 36) else { return };
    let samples = SampleSource::new();
    let layout = CARD_LAYOUT;

    // Transition: the sprite goes from A to B, the still is in between.
    let card = render_card(&mut r, &samples, &PreviewItem::Fx("jb.dissolve".into()), layout).unwrap();
    let sheet = decode(&card.sprite);
    assert_eq!(sheet.dimensions(), layout.sheet_size());
    let first = tile(&sheet, layout, 0);
    let last = tile(&sheet, layout, layout.frames - 1);
    let mid = decode(&card.still).into_raw();
    assert_eq!(mid.len(), first.len());
    assert!(mean_abs_diff(&first, &last) > 30.0, "A becomes B");
    assert!(mean_abs_diff(&mid, &first) > 8.0 && mean_abs_diff(&mid, &last) > 8.0, "still mid-transition");

    // A dip to black is never shown as a black still.
    let card = render_card(&mut r, &samples, &PreviewItem::Fx("jb.fade_black".into()), layout).unwrap();
    assert!(luma_spread(&rgba(&decode(&card.still))) > 10.0);

    // Filters: original on the left, filtered on the right.
    let card = render_card(&mut r, &samples, &PreviewItem::Fx("jb.bw".into()), layout)
        .or_else(|_| {
            let reg = Registry::builtin();
            let id = reg.of_kind(FxKind::Filter).next().unwrap().id;
            render_card(&mut r, &samples, &PreviewItem::Fx(id.into()), layout)
        })
        .unwrap();
    assert_eq!(decode(&card.still).dimensions(), (layout.frame_w, layout.frame_h));

    // Titles render their text.
    let title = sample_titles().remove(0).1;
    let with = render_card(&mut r, &samples, &PreviewItem::Title(title.clone()), layout).unwrap();
    let mut empty = title;
    empty.text = String::new();
    let without = render_card(&mut r, &samples, &PreviewItem::Title(empty), layout).unwrap();
    let (a, b) = (decode(&with.still).into_raw(), decode(&without.still).into_raw());
    let changed = a.iter().zip(&b).filter(|(x, y)| (**x as i32 - **y as i32).abs() > 60).count();
    assert!(changed > 100, "title text drawn ({changed} values changed)");

    // Footage previews show the timeline frame with the effect applied.
    let reg = Registry::builtin();
    let base = sample_preview(&reg, &PreviewItem::Fx("jb.dissolve".into()), 64, 36).unwrap().project;
    let fp = footage_preview(&reg, &base, 0, "jb.gaussian_blur").unwrap().project;
    r.resize(64, 36);
    let mut at =
        |p: &jb_core::Project, t| r.render(p, &jb_core::eval::evaluate(p, p.main(), t), &samples).data.to_vec();
    let mut r2 = common::renderer(64, 36).unwrap();
    let plain = r2.render(&base, &jb_core::eval::evaluate(&base, base.main(), 0), &samples).data.to_vec();
    let blurred = at(&fp, 0);
    let d = mean_abs_diff(&plain, &blurred);
    assert!(d > 0.5 && d < 40.0, "same footage, blurred (diff {d})");

    // Cache: rendered once, stored under the renderer version.
    let root = std::env::temp_dir().join(format!("jb-preview-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("old-version")).unwrap();
    let cache = PreviewCache::new(&root);
    assert!(cache.dir.ends_with(PREVIEW_VERSION));
    let item = PreviewItem::Fx("jb.dissolve".into());
    assert!(!cache.has(&item));
    let (still, sprite) = cache.ensure(&mut r, &samples, &item).unwrap();
    assert!(cache.has(&item) && still.is_file() && sprite.is_file());
    let t = std::fs::metadata(&sprite).unwrap().modified().unwrap();
    cache.ensure(&mut r, &samples, &item).unwrap();
    assert_eq!(std::fs::metadata(&sprite).unwrap().modified().unwrap(), t, "cached, not re-rendered");
    cache.prune_other_versions();
    assert!(!root.join("old-version").exists());
    assert!(cache.size() > 0);
    std::fs::remove_dir_all(&root).unwrap();
}

fn rgba(img: &image::RgbImage) -> Vec<u8> {
    img.pixels().flat_map(|p| [p.0[0], p.0[1], p.0[2], 255]).collect()
}
