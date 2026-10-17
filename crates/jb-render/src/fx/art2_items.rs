//! Art 2 items: brush, paint, ink and torn-paper transitions as stroke data
//! for the kit in `art2_paint.rs`. Positions are uv (0..1, y down), widths
//! fractions of the frame height, times progress.

use super::art2_paint::{ArtItem, Ease, Paint, Stroke, Style, ASPECT};

const CAT: &str = "Paint & Paper";

fn item(id: &'static str, name: &'static str, description: &'static str, done: f32, strokes: Vec<Stroke>) -> ArtItem {
    ArtItem { id, name, category: CAT, description, strokes, paints: Vec::new(), done }
}

/// Four tall brush strokes, alternately from the top and the bottom edge:
/// a quick start, then a slow push to the far edge.
fn brush_columns() -> ArtItem {
    let down = |x: f32, w: f32, h1: f32| {
        Stroke::brush([x, -0.06], [x + 0.01, 1.16], w, 0.055, 1.0)
            .via([x - 0.02, 0.5])
            .ease(Ease::OutQuad)
            .knots(0.2, h1, 0.75, 0.6)
    };
    let up = |x: f32, w: f32, h1: f32| {
        Stroke::brush([x, 1.06], [x - 0.01, -0.16], w, 0.055, 1.0)
            .via([x + 0.02, 0.5])
            .ease(Ease::OutQuad)
            .knots(0.2, h1, 0.75, 0.6)
    };
    item(
        "jb.brush_columns",
        "Brush Columns",
        "Four broad paint strokes run down from the top and up from the bottom in alternate columns until the new clip fills the frame.",
        0.985,
        vec![up(0.11, 0.22, 0.27), down(0.36, 0.22, 0.27), up(0.585, 0.24, 0.29), down(0.86, 0.27, 0.26)],
    )
}

/// Dry-brush sweeps across the frame, alternating direction, top to bottom.
fn dry_sweeps() -> ArtItem {
    item(
        "jb.dry_brush_sweeps",
        "Dry Brush Sweeps",
        "Loaded dry brushes sweep across the frame, alternately from the left and the right, leaving streaky bristle marks that fill in to the new clip.",
        0.72,
        vec![
            Stroke::brush([-0.05, 0.31], [1.1, 0.29], 0.2, 0.27, 0.45).via([0.5, 0.32]).dry(0.7).dry_until(0.62).ease(Ease::Linear),
            Stroke::brush([1.05, 0.69], [-0.1, 0.71], 0.19, 0.3, 0.48).via([0.5, 0.64]).dry(0.85).dry_until(0.64).ease(Ease::Linear),
            Stroke::brush([-0.05, 0.92], [1.1, 0.9], 0.12, 0.5, 0.67).dry(0.85).dry_until(0.72).ease(Ease::Linear),
            Stroke::brush([1.05, 0.03], [-0.1, 0.02], 0.08, 0.5, 0.64).dry(0.6).dry_until(0.7).ease(Ease::Linear),
        ],
    )
}

/// One dry stroke across the middle that swells to fill the frame.
fn swelling_stroke() -> ArtItem {
    item(
        "jb.swelling_brush_stroke",
        "Swelling Brush Stroke",
        "A single dry brush stroke races across the middle of the frame, its streaks slowly fill in, then the stroke swells up and down to cover everything.",
        0.735,
        vec![Stroke::brush([1.05, 0.475], [-0.1, 0.465], 0.17, 0.25, 0.34)
            .via([0.5, 0.45])
            .ease(Ease::Out)
            .dry(0.65)
            .dry_until(0.72)
            .grow(0.665, 0.73, 4.2)],
    )
}

/// Three quick strokes: up the right side, across the bottom, across the top.
fn drift_strokes_1() -> ArtItem {
    item(
        "jb.brush_drift_three",
        "Brush Drift Three",
        "Three fast brush strokes drift in: one up the right edge, one across the bottom from the left and a last one across the top.",
        0.45,
        vec![
            Stroke::brush([0.88, 1.12], [0.85, -0.12], 0.3, 0.12, 0.23).ease(Ease::Out).rag(0.15),
            Stroke::brush([-0.08, 0.76], [0.8, 0.72], 0.3, 0.21, 0.29).ease(Ease::Out).rag(0.15),
            Stroke::brush([-0.08, 0.24], [0.85, 0.2], 0.3, 0.36, 0.44).via([0.4, 0.26]).ease(Ease::Out).rag(0.15),
        ],
    )
}

/// A straight stroke across the whole frame along direction `deg` (degrees,
/// clockwise from +x, y down), offset `o` (height units) from the centre along
/// the normal (90 degrees clockwise of the direction), half width `hw`. The
/// head enters at the far side of `-dir` and leaves past the other edge.
fn band(deg: f32, o: f32, hw: f32, t0: f32, t1: f32) -> Stroke {
    const A: f32 = 16.0 / 9.0;
    let (s, c) = deg.to_radians().sin_cos();
    let (cx, cy) = (0.5 * A - s * o, 0.5 + c * o);
    let l = 1.15;
    let p = |k: f32| [(cx + c * l * k) / A, cy + s * l * k];
    Stroke::brush(p(-1.0), p(1.0), hw, t0, t1)
}

/// Parallel diagonal strokes (down-right), corner first, alternating sides.
fn drift_diagonal_down() -> ArtItem {
    item(
        "jb.diagonal_brush_drift",
        "Diagonal Brush Drift",
        "Diagonal brush strokes sweep down-right across the frame, first the corners, then the middle bands, revealing the next clip.",
        0.56,
        vec![
            band(32.0, -0.85, 0.28, 0.14, 0.24),
            band(215.0, -0.72, 0.3, 0.22, 0.36),
            band(12.0, -0.45, 0.4, 0.32, 0.4).dry(0.4).dry_until(0.45),
            band(192.0, -0.2, 0.28, 0.45, 0.53),
        ],
    )
}

/// Parallel diagonal strokes (up-right), from the top-left corner onwards.
fn drift_diagonal_up() -> ArtItem {
    item(
        "jb.rising_brush_drift",
        "Rising Brush Drift",
        "Brush strokes rise diagonally up-right one after another from the top-left corner to the bottom-right one.",
        0.62,
        vec![
            band(-43.0, -0.95, 0.25, 0.14, 0.25),
            band(-43.0, -0.62, 0.26, 0.26, 0.4),
            band(-43.0, -0.17, 0.25, 0.39, 0.49),
            band(137.0, -0.3, 0.26, 0.49, 0.57),
            band(137.0, -0.75, 0.3, 0.51, 0.6),
        ],
    )
}

/// Three horizontal strokes: top from the right, bottom from the left, middle last.
fn drift_horizontal() -> ArtItem {
    item(
        "jb.crossing_brush_drift",
        "Crossing Brush Drift",
        "Horizontal brush strokes cross the frame in turn: along the top from the right, along the bottom from the left, then through the middle.",
        0.44,
        vec![
            Stroke::brush([1.05, 0.16], [-0.1, 0.17], 0.21, 0.1, 0.2).ease(Ease::Out),
            Stroke::brush([-0.05, 0.8], [1.1, 0.8], 0.22, 0.2, 0.3).ease(Ease::Out),
            Stroke::brush([1.05, 0.48], [-0.1, 0.48], 0.16, 0.27, 0.42).ease(Ease::Out).dry(0.4).dry_until(0.44),
        ],
    )
}

/// Round-headed strokes: middle, bottom, then the top in two passes.
fn drift_rounded() -> ArtItem {
    let s = |a: [f32; 2], b: [f32; 2], w: f32, t0: f32, t1: f32| Stroke::brush(a, b, w, t0, t1).ease(Ease::Out).jag(0.3).rag(0.06);
    item(
        "jb.rounded_brush_drift",
        "Rounded Brush Drift",
        "Round-tipped paint strokes glide across the middle, the bottom and finally the top of the frame.",
        0.45,
        vec![
            s([-0.05, 0.5], [1.1, 0.5], 0.13, 0.08, 0.2),
            s([1.05, 0.82], [-0.1, 0.82], 0.2, 0.12, 0.22),
            s([-0.05, 0.24], [1.1, 0.24], 0.13, 0.28, 0.37),
            s([1.05, 0.06], [-0.1, 0.06], 0.09, 0.34, 0.43),
        ],
    )
}

/// Diagonal strokes: a fast corner sweep, a pause, then the rest in bands.
fn drift_diagonal_pause() -> ArtItem {
    item(
        "jb.paused_diagonal_drift",
        "Paused Diagonal Drift",
        "A quick diagonal stroke takes the top-right of the frame, pauses, then more diagonal strokes finish the job band by band.",
        0.6,
        vec![
            band(33.0, -0.65, 0.42, 0.08, 0.13),
            band(33.0, -0.18, 0.12, 0.27, 0.33),
            band(213.0, -0.6, 0.4, 0.35, 0.42),
            band(33.0, 0.06, 0.15, 0.45, 0.58).dry(0.5).dry_until(0.6),
        ],
    )
}

/// Flat paint edges close in from every side onto a point.
fn drift_closing_edges() -> ArtItem {
    let e = |a: [f32; 2], b: [f32; 2], t0: f32| Stroke::sweep(a, b, t0, 0.58).style(Style::Clean).rag(0.0).ease(Ease::Linear);
    item(
        "jb.closing_paint_edges",
        "Closing Paint Edges",
        "Flat paint swatches with straight edges push in from the corners and sides until the last patch of the old clip closes up.",
        0.59,
        vec![
            e([-0.05, -0.15], [0.47, 0.62], 0.15),
            e([-0.05, 1.15], [0.45, 0.66], 0.15),
            e([1.1, 1.15], [0.47, 0.7], 0.25),
            e([0.6, -0.15], [0.45, 0.6], 0.3),
            e([1.15, 0.4], [0.48, 0.66], 0.38),
        ],
    )
}

/// Translucent watercolour strips laid from the right edge to the left.
fn wash_strips() -> ArtItem {
    let xs = [0.93, 0.8, 0.67, 0.54, 0.41, 0.28, 0.15, 0.04];
    let ts = [0.08, 0.15, 0.2, 0.23, 0.27, 0.3, 0.33, 0.38];
    let strokes = xs
        .iter()
        .zip(ts)
        .enumerate()
        .map(|(k, (&x, t))| {
            Stroke::brush([x, -0.1], [x + 0.01, 1.1], 0.125, t, t + 0.05)
                .style(Style::Water)
                .rag(0.08)
                .jag(0.4)
                .mottle(0.3)
                .fade_in(0.5, (t + 0.4).min(0.8))
                .seed(k as f32 * 7.3)
        })
        .collect();
    item(
        "jb.watercolor_strips",
        "Watercolor Strips",
        "Translucent watercolour strips are washed in one after another from the right edge to the left and slowly deepen to the new clip.",
        0.85,
        strokes,
    )
}

/// Horizontal watercolour washes, bottom and top first, then the middle.
fn wash_bands() -> ArtItem {
    let w = |y: f32, hw: f32, t0: f32, t1: f32, rtl: bool| {
        let (a, b) = if rtl { ([1.05, y], [-0.1, y]) } else { ([-0.05, y], [1.1, y]) };
        Stroke::brush(a, b, hw, t0, t1).style(Style::Water).rag(0.12).jag(0.6).mottle(0.35).fade_in(0.55, 0.5)
    };
    item(
        "jb.watercolor_bands",
        "Watercolor Bands",
        "Loose horizontal watercolour washes flood the bottom and the top of the frame, then the middle, and settle into the new clip.",
        0.56,
        vec![
            w(0.87, 0.14, 0.26, 0.3, true),
            w(0.1, 0.11, 0.29, 0.33, false),
            w(0.68, 0.12, 0.31, 0.35, true),
            w(0.3, 0.12, 0.33, 0.37, false),
            w(0.5, 0.12, 0.35, 0.39, true),
        ],
    )
}

/// Ink blots that bloom and merge.
fn ink_blooms() -> ArtItem {
    let b = |x: f32, y: f32, r: f32, t0: f32, t1: f32, s: f32| {
        Stroke::blot([x, y], 0.02, r, t0, t1).mottle(0.2).soft(0.035).fade_in(0.375, 0.48).seed(s)
    };
    item(
        "jb.ink_blooms",
        "Ink Blooms",
        "Watercolour blots bloom on the frame, spread and run into each other until the new clip shows through everywhere.",
        0.62,
        vec![
            b(0.308, 0.363, 0.56, 0.145, 0.4, 1.0),
            b(0.846, 0.368, 0.6, 0.18, 0.495, 2.0),
            b(0.758, 1.037, 0.29, 0.303, 0.433, 3.0),
            b(0.288, 0.975, 0.26, 0.356, 0.442, 4.0),
            b(0.612, 0.512, 0.5, 0.338, 0.395, 5.0),
        ],
    )
}

/// A torn hole that opens from a point and widens until it fills the
/// frame, its ragged edge trimmed with a white paper rim.
fn torn_hole() -> ArtItem {
    let blot = || Stroke::blot([0.4575, 0.7075], 0.0, 0.95, 0.1575, 0.595).stretch(2.025, 1.0).style(Style::Torn).rag(0.275).ease(Ease::In).seed(3.0);
    let mut it = item(
        "jb.torn_paper_hole",
        "Torn Paper Hole",
        "A small tear appears in the frame and rips wider and wider, its white paper edge rimming the new clip until it fills the frame.",
        0.7,
        vec![blot().paint(0).rim(0.014), blot()],
    );
    it.paints = vec![Paint::PAPER];
    it
}

/// A straight line through the frame centre (x at `cx`) at `deg` degrees.
fn rift_line(cx: f32, deg: f32) -> ([f32; 2], [f32; 2], [f32; 2]) {
    let (s, c) = deg.to_radians().sin_cos();
    let (x, l) = (cx * ASPECT, 1.35);
    ([(x - c * l) / ASPECT, 0.5 - s * l], [cx, 0.5], [(x + c * l) / ASPECT, 0.5 + s * l])
}

/// A thin tear runs down the frame, then the rift swings round to the
/// horizontal while it widens into a torn band that swallows the frame.
fn torn_rift() -> ArtItem {
    // (progress, angle, half width, centre x) keyframes of the rift.
    let k: [(f32, f32, f32, f32); 5] = [(0.32, -83.0, 0.004, 0.36), (0.38, -58.75, 0.012, 0.45), (0.4, -38.75, 0.1, 0.5), (0.507, -3.0, 0.263, 0.5), (0.666, 0.0, 0.425, 0.5)];
    let mut v = Vec::new();
    for i in 0..4 {
        let ((ta, da, wa, xa), (tb, db, wb, xb)) = (k[i], k[i + 1]);
        let (a0, c0, b0) = rift_line(xa, da);
        let (a1, c1, b1) = rift_line(xb, db);
        let s = || {
            let st = Stroke::brush(a0, b0, wa, ta, ta + 0.003)
                .via(c0)
                .ease(Ease::Linear)
                .style(Style::Torn)
                .rag(0.25)
                .jag(0.0)
                .morph(a1, c1, b1, ta, tb)
                .grow(ta, tb, wb / wa)
                .seed(2.0);
            if i < 3 { st.fade_out(tb, tb + 0.001) } else { st }
        };
        v.push(s().paint(0).rim(0.012));
        v.push(s());
    }
    let mut it = item(
        "jb.torn_paper_rift",
        "Torn Paper Rift",
        "A hairline tear splits the frame top to bottom, swings round to the horizontal and rips open into a wide band with white paper edges.",
        0.71,
        v,
    );
    it.paints = vec![Paint::PAPER];
    it
}

/// The old clip peels away diagonally from the top-left corner, rolling
/// up into a glossy paper curl along a torn edge.
fn paper_peel() -> ArtItem {
    let roll = Paint { base: [0.62, 0.62, 0.64], light: [0.97, 0.97, 0.98], streak: 0.1, sheen: 0.6, wet: 0.0, grain: 0.02, shade: 0.55, alpha: 1.0 };
    let mut it = item(
        "jb.paper_roll_peel",
        "Paper Roll Peel",
        "The frame peels off diagonally from the top-left corner like a sheet of paper, rolling into a shiny curl that uncovers the new clip.",
        0.6,
        vec![
            Stroke::sweep_dir(38.25, -1.055, 0.92, 0.31, 0.59).ease(Ease::Smooth).style(Style::Torn).rag(0.05).widths(0.1, 0.1).seed(4.0),
            Stroke::sweep_dir(38.25, -1.025, 0.95, 0.31, 0.59).ease(Ease::Smooth).style(Style::Clean).rag(0.0).paint(0).rim(0.05).invert(),
        ],
    );
    it.paints = vec![roll];
    it
}

/// Library listing, in display order (every item of [`items`]).
pub const LIB_IDS: &[&str] = &["jb.brush_columns", "jb.dry_brush_sweeps", "jb.swelling_brush_stroke", "jb.brush_drift_three", "jb.diagonal_brush_drift", "jb.rising_brush_drift",
    "jb.crossing_brush_drift",
    "jb.rounded_brush_drift",
    "jb.paused_diagonal_drift",
    "jb.closing_paint_edges",
    "jb.watercolor_strips",
    "jb.watercolor_bands",
    "jb.ink_blooms",
    "jb.torn_paper_hole",
    "jb.torn_paper_rift",
    "jb.paper_roll_peel",
];

pub fn items() -> Vec<ArtItem> {
    vec![brush_columns(), dry_sweeps(), swelling_stroke(), drift_strokes_1(), drift_diagonal_down(), drift_diagonal_up(),
        drift_horizontal(),
        drift_rounded(),
        drift_diagonal_pause(),
        drift_closing_edges(),
        wash_strips(),
        wash_bands(),
        ink_blooms(),
        torn_hole(),
        torn_rift(),
        paper_peel(),
    ]
}
