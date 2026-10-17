//! Art 2 gold items: brushed gold-leaf paint laid over the outgoing clip and
//! wiped away to the incoming one, as stroke data for the kit in
//! `art2_paint.rs`. Positions are uv (0..1, y down), widths fractions of the
//! frame height, times progress. Paint 0 is [`Paint::GOLD`].

use super::art2_paint::{ArtItem, Ease, Paint, Stroke, Style};

const CAT: &str = "Paint & Paper";

fn gold(id: &'static str, name: &'static str, description: &'static str, done: f32, strokes: Vec<Stroke>) -> ArtItem {
    ArtItem { id, name, category: CAT, description, strokes, paints: vec![Paint::GOLD], done }
}

/// A straight brush across the whole frame along `deg` (clockwise from +x,
/// y down), offset `o` (height units) along the normal, half width `hw`.
fn band(deg: f32, o: f32, hw: f32, t0: f32, t1: f32) -> Stroke {
    const A: f32 = 16.0 / 9.0;
    let (s, c) = deg.to_radians().sin_cos();
    let (cx, cy) = (0.5 * A - s * o, 0.5 + c * o);
    let l = 1.2;
    let p = |k: f32| [(cx + c * l * k) / A, cy + s * l * k];
    Stroke::brush(p(-1.0), p(1.0), hw, t0, t1)
}

/// A gold tide rises diagonally over the frame, then a ragged brush edge
/// follows it up with the new clip.
fn gold_tide() -> ArtItem {
    gold(
        "jb.gold_tide",
        "Gold Tide",
        "A sheet of brushed gold rises diagonally from the bottom corner until it fills the frame, then a ragged brush edge sweeps up behind it with the new clip.",
        0.93,
        vec![
            Stroke::sweep_dir(-103.0, -0.78, 0.78, 0.02, 0.42).ease(Ease::Linear).style(Style::Bristle).rag(0.07).widths(0.12, 0.12).paint(0),
            Stroke::sweep_dir(-103.0, -0.8, 0.8, 0.44, 0.92).ease(Ease::Linear).style(Style::Bristle).rag(0.1).widths(0.08, 0.08),
        ],
    )
}

/// A dripping gold curtain sweeps left to right, then the new clip follows.
fn gold_curtain() -> ArtItem {
    gold(
        "jb.gold_curtain",
        "Gold Curtain",
        "A curtain of gold paint with dripping vertical streaks sweeps from left to right over the frame, and the new clip is wiped in behind it the same way.",
        0.92,
        vec![
            Stroke::sweep_dir(0.0, -0.77, 1.0, 0.0, 0.5).ease(Ease::Linear).style(Style::Bristle).rag(0.16).widths(0.03, 0.03).paint(0).seed(3.0),
            Stroke::sweep_dir(0.0, -0.95, 1.0, 0.44, 0.9).ease(Ease::Linear).style(Style::Bristle).rag(0.14).widths(0.03, 0.03).seed(9.0),
        ],
    )
}

/// Crossing gold brush strokes fill the frame, then broad diagonal strokes
/// lay the new clip.
fn gold_cross() -> ArtItem {
    let g = |deg: f32, o: f32, hw: f32, t0: f32, t1: f32| band(deg, o, hw, t0, t1).paint(0).rag(0.1).ease(Ease::Out);
    let b = |deg: f32, o: f32, hw: f32, t0: f32, t1: f32| band(deg, o, hw, t0, t1).rag(0.12).dry(1.025).dry_until(t1 + 0.1).ease(Ease::Out);
    let mut it = gold(
        "jb.gold_cross_strokes",
        "Gold Cross Strokes",
        "Broad gold brush strokes cross the frame in an X and pile up until it is all gold, then streaky diagonal strokes paint the new clip over it.",
        0.687,
        vec![
            g(-30.0, 0.1, 0.3, 0.108, 0.306),
            g(28.0, -0.15, 0.28, 0.09, 0.275),
            g(-30.0, -0.5, 0.35, 0.13, 0.28),
            g(-30.0, 0.6, 0.35, 0.15, 0.285),
            g(28.0, 0.55, 0.35, 0.105, 0.24),
            g(28.0, -0.7, 0.35, 0.258, 0.45),
            b(-20.0, 0.0, 0.32, 0.483, 0.516),
            b(-20.0, -0.5, 0.35, 0.497, 0.56),
            b(-20.0, 0.5, 0.35, 0.587, 0.804),
        ],
    );
    it.paints = vec![Paint::GOLD_SOFT];
    it
}

/// The new clip slides down from the top behind a gilded edge.
fn gold_edge_fall() -> ArtItem {
    let s = |r: f32| Stroke::sweep_dir(93.0, -0.6, 0.62, 0.0, 0.76).ease(Ease::Linear).style(Style::Bristle).rag(r).widths(0.25, 0.25).seed(4.0);
    gold(
        "jb.gilded_edge_fall",
        "Gilded Edge Fall",
        "A softly curving edge with a band of gold glinting along it descends from the top of the frame, uncovering the new clip.",
        0.77,
        vec![s(0.06).paint(0).rim(0.03).soft(0.01).fade_in(0.3, 0.2), s(0.06)],
    )
}

/// Vertical columns of paint close in from both sides, each led by gold.
fn gold_columns() -> ArtItem {
    // (x, upward, t0, duration)
    let cols: [(f32, bool, f32, f32); 9] = [
        (0.06, true, 0.083, 0.188),
        (0.93, false, 0.112, 0.188),
        (0.19, false, 0.188, 0.188),
        (0.8, true, 0.188, 0.188),
        (0.31, true, 0.277, 0.197),
        (0.67, false, 0.288, 0.207),
        (0.41, false, 0.387, 0.207),
        (0.56, true, 0.407, 0.207),
        (0.48, true, 0.455, 0.245),
    ];
    let mut v = Vec::new();
    for (k, &(x, up, t0, dt)) in cols.iter().enumerate() {
        let t1 = t0 + dt;
        let (a, b) = if up { ([x, 1.12], [x, -0.12]) } else { ([x, -0.12], [x, 1.12]) };
        let hw = if k == 8 { 0.09 } else { 0.13 };
        v.push(Stroke::brush(a, b, hw * 1.05, t0 - 0.05, t1 - 0.035).paint(0).dry(0.5).rag(0.15).ease(Ease::Out).seed(k as f32 * 3.1));
        v.push(Stroke::brush(a, b, hw, t0, t1).rag(0.12).ease(Ease::Out).seed(k as f32 * 3.1 + 1.0));
    }
    let mut it = gold("jb.gold_led_columns", "Gold-Led Columns", "Vertical brush strokes, each led by a streak of gold, close in from both sides toward the middle until the new clip fills the frame.", 0.58, v);
    it.paints = vec![Paint { alpha: 0.75, ..Paint::GOLD }];
    it
}

/// Slanted gold strokes flash in from right to left; the new clip follows them.
fn gold_slash_run() -> ArtItem {
    let g = |x: f32, hw: f32, t0: f32, t1: f32| Stroke::brush([x - 0.06, 1.12], [x + 0.06, -0.12], hw, t0, t1).paint(0).rag(0.12).dry(0.3).ease(Ease::Out);
    gold(
        "jb.gold_slash_run",
        "Gold Slash Run",
        "Tall slanted gold brush strokes flash in from right to left across the frame, each one turning into the new clip as the edge passes it.",
        0.42,
        vec![
            g(0.92, 0.14, 0.0, 0.06),
            g(0.68, 0.2, 0.05, 0.1),
            g(0.43, 0.15, 0.16, 0.22),
            g(0.12, 0.13, 0.2, 0.26),
            g(0.24, 0.16, 0.27, 0.32),
            Stroke::sweep_dir(197.0, -0.95, 0.95, 0.02, 0.41).knots(0.13, 0.18, 0.27, 0.6).ease(Ease::Out).style(Style::Bristle).rag(0.08).widths(0.05, 0.05),
        ],
    )
}

/// A torn, gilded edge runs diagonally from the top-left corner.
fn gold_torn_edge() -> ArtItem {
    let s = || Stroke::sweep_dir(56.0, -0.96, 1.0, 0.04, 0.67).ease(Ease::Linear).style(Style::Torn).rag(0.1).widths(0.2, 0.2).seed(7.0);
    gold(
        "jb.gilded_torn_edge",
        "Gilded Torn Edge",
        "An irregular torn edge trimmed with flecks of gold travels diagonally from the top-left corner to the bottom-right one, revealing the new clip.",
        0.68,
        vec![s().paint(0).rim(0.025).mottle(0.5), s()],
    )
}

/// A gold vortex closes over the frame like an iris, then shrinks away
/// inside the new clip.
fn gold_vortex() -> ArtItem {
    let c = [0.45, 0.55];
    gold(
        "jb.gold_vortex",
        "Gold Vortex",
        "Swirling gold closes in from the edges like an iris over the old clip, then the new clip opens around the gold swirl until it shrinks away in the middle.",
        0.93,
        vec![
            Stroke::blot(c, 0.85, 0.0, 0.0, 0.5).stretch(1.6, 1.0).invert().ease(Ease::Linear).rag(0.35).paint(0).seed(2.0),
            Stroke::blot(c, 1.25, 0.0, 0.3, 0.92).stretch(1.6, 1.0).invert().knots(0.53, 0.72, 0.8, 0.86).ease(Ease::Out).rag(0.35).seed(5.0),
        ],
    )
}

/// Library listing, in display order.
pub const LIB_IDS: &[&str] = &[
    "jb.gold_tide",
    "jb.gold_curtain",
    "jb.gold_cross_strokes",
    "jb.gilded_edge_fall",
    "jb.gold_led_columns",
    "jb.gold_slash_run",
    "jb.gilded_torn_edge",
    "jb.gold_vortex",
];

pub fn items() -> Vec<ArtItem> {
    vec![gold_tide(), gold_curtain(), gold_cross(), gold_edge_fall(), gold_columns(), gold_slash_run(), gold_torn_edge(), gold_vortex()]
}
