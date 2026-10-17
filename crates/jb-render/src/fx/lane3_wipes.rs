//! Shape and line wipes fitted on Filmora's exports: the new clip is revealed
//! through a parametric mask whose threshold (and edge width) follows a
//! keyframe table fitted frame by frame.

use super::data as D;
use super::kit::{edge_wipe, ramp_wipe, register_items, table, Item};
use crate::registry::Registry;

fn items() -> Vec<Item> {
    vec![
        Item::new(
            "jb.line_erase_down",
            "Line Erase Down",
            "Wipes",
            "A hard horizontal edge sweeps down revealing the new clip: it jumps in quickly, slows to a crawl \
             through the middle and speeds off at the bottom. Fitted on Filmora's export of Erase Line Down.",
            edge_wipe("uv.y", D::ERASE_DOWN),
        ),
        Item::new(
            "jb.line_erase_right",
            "Line Erase Right",
            "Wipes",
            "A hard vertical edge sweeps left to right revealing the new clip, fast at both ends and slow in the \
             middle. Fitted on Filmora's export of Erase Line Right.",
            edge_wipe("uv.x", D::ERASE_RIGHT),
        ),
        Item::new(
            "jb.sides_merge",
            "Sides Merge",
            "Wipes",
            "The new clip closes in from the left and right edges towards a vertical centre line. Fitted on \
             Filmora's export of Col Merge.",
            edge_wipe("1.0 - 2.0 * abs(uv.x - 0.5)", D::COL_MERGE),
        ),
        Item::new(
            "jb.chalk_swipe_left",
            "Chalk Swipe Left",
            "Wipes",
            "A slightly slanted edge sweeps from right to left in stop-motion steps (about 12 per second), as \
             if wiped by hand. Fitted on Filmora's export of Chalk Swipe Left.",
            edge_wipe("(1.0 - uv.x) - 0.08 * uv.y", D::CHALK_SWIPE),
        ),
        Item::new(
            "jb.diagonal_dissolve",
            "Diagonal Dissolve",
            "Dissolve",
            "A soft diagonal front crosses from the top-left corner to the bottom-right, dissolving into the new \
             clip; the front widens as it goes. Fitted on Filmora's export of Dissolve Skew 01.",
            ramp_wipe("(uv.x * A + uv.y) / (A + 1.0)", D::SKEW_DISSOLVE),
        ),
        Item::new(
            "jb.blind_bars",
            "Blind Bars",
            "Wipes",
            "A band of thin vertical slats sweeps from left to right: inside the band each slat flips to the new \
             clip at its own moment and dark gaps open between slats. Front position and band width fitted on \
             Filmora's export of Blind 1.",
            format!("{}\n{BLIND_BARS}", table("g", D::BLIND_BARS)),
        ),
        Item::new(
            "jb.slice_slide",
            "Slice Slide",
            "Motion",
            "The frame is cut into ten horizontal strips of uneven height that each slide one full mirrored \
             period to the left with their own easing (from ease-in to strong ease-out) while the clips \
             cross-dissolve. Strip edges, easings and timing fitted on Filmora's export of Horizontal Slice.",
            SLICE_SLIDE.to_string(),
        ),
        Item::new(
            "jb.bar_cutout",
            "Bar Cutout",
            "Wipes",
            "The new clip, shrunk to 80%, appears through horizontal bars that open from the centre row outwards \
             and grow sideways to uneven lengths; the bars merge into its rectangle, which then grows to full \
             frame. Timing estimated from Filmora's export of Horizontal Cutout 1.",
            BAR_CUTOUT.to_string(),
        ),
    ]
}

/// Horizontal Cutout 1: the new clip, scaled to `s` (0.8 until p = 0.83, then
/// easing to 1), shows through 15 horizontal bars over its rectangle; bar
/// `k` opens at `0.17 + 0.045 |k - 7|` (centre first) and grows sideways from
/// the centre to a hashed length within 0.12, gaps close from p = 0.8.
const BAR_CUTOUT: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let s = mix(0.8, 1.0, smoothstep(0.83, 0.97, p));
    let q = (uv - 0.5) / s + 0.5;
    if (!in01(q)) { return sa(uv); }
    let fk = q.y * 15.0;
    let k = floor(fk);
    let d = abs(k - 7.0);
    let t0 = 0.17 + 0.045 * d;
    let grow = smoothstep(0.0, 1.0, clamp((p - t0) / 0.12, 0.0, 1.0));
    let full = smoothstep(0.78, 0.84, p);
    let len = mix(grow * (0.55 + 0.4 * hash11(k * 3.7 + 1.0)), 1.0, full);
    let gap = 0.12 * (1.0 - full);
    let inside = abs(q.x - 0.5) <= 0.5 * len && abs(fract(fk) - 0.5) <= 0.5 - gap;
    if (inside && grow > 0.0) { return sb(q); }
    return sa(uv);
}
"#;

/// Horizontal Slice: strip `k` spans `v` in `[SL_EDGES[k], SL_EDGES[k+1])` and
/// moves by `-2 F_k(t)` frame widths (mirrored repeat), `t = (p - 0.25) / 0.5`,
/// `F = 1 - (1 - t)^e` for `e >= 1`, `t^(1/e)` below; the clips cross-dissolve
/// with `smoothstep(0.3, 0.62, p)`. Fitted per strip: every strip travels
/// between frames 15 and 45 of the 60, with residual < 0.01 frame widths.
const SLICE_SLIDE: &str = r#"
var<private> SL_EDGES: array<vec4f, 11> = array<vec4f, 11>(
    vec4f(0.0), vec4f(0.0972), vec4f(0.1389), vec4f(0.287), vec4f(0.3657), vec4f(0.5463),
    vec4f(0.7083), vec4f(0.8009), vec4f(0.8472), vec4f(0.9213), vec4f(1.0001));
var<private> SL_EXP: array<vec4f, 10> = array<vec4f, 10>(
    vec4f(1.0), vec4f(0.63), vec4f(2.45), vec4f(1.0), vec4f(1.5), vec4f(2.1), vec4f(1.07), vec4f(1.77),
    vec4f(0.94), vec4f(1.96));
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let t = clamp((p - 0.25) / 0.5, 0.0, 1.0);
    var k = 0;
    for (var i = 0; i < 10; i = i + 1) {
        if (uv.y >= SL_EDGES[i].x) { k = i; }
    }
    let e = SL_EXP[k].x;
    var f = pow(t, 1.0 / e);
    if (e >= 1.0) { f = 1.0 - pow(1.0 - t, e); }
    let q = l3_mirror(vec2f(uv.x + 2.0 * f, uv.y));
    return l3_mix(sa(q), sb(q), smoothstep(0.3, 0.62, p));
}
"#;

/// Slat band of [`D::BLIND_BARS`]: slats 1/96 of the width, grouped in runs of
/// one to three by a hash; each run flips when the band's local position
/// passes its own threshold; a dark gap marks some run boundaries.
const BLIND_BARS: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let k = kf_g(prog());
    let bw = max(k.y, 0.02);
    let local = 0.5 + (k.x - uv.x) / bw;
    if (local >= 1.0) { return sb(uv); }
    if (local <= 0.0) { return sa(uv); }
    let cell = floor(uv.x * 96.0);
    let run = floor(cell / 3.0) * 3.0 + floor(hash11(floor(cell / 3.0) + 7.0) * 2.0);
    let id = select(run, cell, cell < run);
    let thr = hash11(id * 1.37 + 3.1);
    let f = fract(uv.x * 96.0);
    let gap = select(0.0, 1.0, f < 0.18 && hash11(id + 11.0) < 0.6);
    let c = select(sa(uv), sb(uv), local > thr);
    return mix(c, vec4f(0.0, 0.0, 0.0, c.a), gap * 0.85);
}
"#;

pub fn register(r: &mut Registry) {
    register_items(r, items());
}
