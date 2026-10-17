//! More art3b scenes (motion-graphics packs, themed and business graphics).
//! Same conventions as `art3b_items.rs`: progress `t` runs 0..1 over the
//! transition (60 frames at Filmora's default length), `asp` is the frame
//! aspect, and every shape is authored here from the kit's primitives.

use super::kit::*;

/// Band between the parallel lines `n·p = s0` and `n·p = s1`, where the
/// unit normal is `n = (sin th, cos th)` (the lines run at `th` above the
/// horizontal, rising to the right).
pub fn band(th: f32, s0: f32, s1: f32) -> Prim {
    let (n0, n1) = (th.sin(), th.cos());
    let mid = 0.5 * (s0 + s1);
    Prim::rect(n0 * mid, n1 * mid, 0.5 * (s1 - s0).abs(), 3.0).rot(n1.atan2(n0))
}

/// Value of a keyframed track, `None` outside its first and last key.
pub fn track(f: f32, keys: &[(f32, f32)]) -> Option<f32> {
    let (a, b) = (keys.first()?.0, keys.last()?.0);
    (f >= a && f <= b).then(|| kf(f, keys))
}

/// Two wedges of the new clip slide in from the top-left and bottom-right
/// corners along a shallow diagonal, each led by a thin white edge, and
/// meet in the middle; thin mint lines then fan out from the seam.
pub fn corner_wedge_close(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let th = 37.6f32.to_radians();
    let mut s = Scene::new();
    let top = kf(f, &[(2.0, -0.95), (3.0, -0.8), (6.0, -0.725), (8.0, -0.625), (10.0, -0.55), (11.0, -0.45), (14.0, -0.3), (15.0, -0.175), (17.0, -0.075), (18.0, 0.0), (19.0, 0.12)]);
    let bot = kf(f, &[(8.0, 0.95), (10.0, 0.63), (11.0, 0.6), (14.0, 0.475), (15.0, 0.4), (17.0, 0.3), (18.0, 0.225), (19.0, 0.1)]);
    let line = 0xE8FFF0;
    if top < bot {
        s.push(band(th, -2.0, top).tex_b());
        s.push(band(th, top - 0.006, top).col(line));
        if f > 8.0 {
            s.push(band(th, bot, 2.0).tex_b());
            s.push(band(th, bot, bot + 0.006).col(line));
        }
    } else {
        s.mix = 1.0;
    }
    // Lines fanning out from the seam once the wedges have met.
    let tracks: [&[(f32, f32)]; 6] = [
        &[(18.0, -0.05), (21.0, -0.22), (23.0, -0.42), (25.0, -0.56), (26.0, -0.65), (27.0, -0.83), (28.0, -0.95)],
        &[(20.0, 0.04), (21.0, -0.03), (22.0, -0.09), (23.0, -0.24), (25.0, -0.41), (26.0, -0.48), (27.0, -0.65), (29.0, -0.81), (30.0, -0.95)],
        &[(19.0, 0.12), (21.0, 0.26), (22.0, 0.33), (23.0, 0.47), (25.0, 0.6), (26.0, 0.68), (27.0, 0.81), (28.0, 0.95)],
        &[(21.0, 0.1), (22.0, 0.16), (23.0, 0.28), (24.0, 0.36), (26.0, 0.53), (27.0, 0.71), (28.0, 0.8), (29.0, 0.95)],
        &[(26.0, 0.1), (27.0, 0.02), (29.0, -0.06), (31.0, -0.25), (33.0, -0.41), (34.0, -0.48), (35.0, -0.67), (37.0, -0.86), (38.0, -0.95)],
        &[(26.0, 0.12), (27.0, 0.17), (29.0, 0.26), (31.0, 0.46), (33.0, 0.6), (34.0, 0.69), (35.0, 0.8)],
    ];
    for k in tracks {
        if let Some(o) = track(f, k) {
            s.push(band(th, o - 0.002, o + 0.002).col(line).alpha(0.9));
        }
    }
    s
}

/// A glossy pink heart swells out of the centre with a burst of sparkles
/// until it covers the frame; the clips cross under its translucent body
/// and it fades away over the new clip.
pub fn sparkle_heart_swell(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    // The background cuts while the heart covers the frame; inside the heart
    // the old clip lingers and cross-fades to the new one.
    let mut s = Scene::cut(t, 25.0 / 60.0);
    let sz = kf(f, &[(4.0, 0.0), (5.0, 0.034), (6.0, 0.062), (7.0, 0.095), (8.0, 0.16), (10.0, 0.23), (11.0, 0.287), (12.0, 0.337), (13.0, 0.382), (14.0, 0.427), (16.0, 0.47), (17.0, 0.5), (18.0, 0.6), (20.0, 0.8), (22.0, 0.95), (24.0, 1.0), (27.0, 0.9), (30.0, 0.84), (50.0, 0.84)]);
    if sz <= 0.0 {
        return s;
    }
    let cx = kf(f, &[(5.0, 0.06), (12.0, 0.03), (20.0, 0.0)]);
    let vy = kf(f, &[(5.0, 0.02), (14.0, -0.01), (20.0, -0.04)]);
    let cy = vy + 0.34 * sz;
    // Opacity of the pink body, and of the old clip seen through it.
    let op = kf(f, &[(4.0, 0.8), (8.0, 1.0), (14.0, 0.98), (20.0, 0.95), (30.0, 0.77), (40.0, 0.55), (48.0, 0.35), (52.0, 0.25), (54.0, 0.12), (56.0, 0.0)]);
    let old = kf(f, &[(34.0, 1.0), (38.0, 0.5), (42.0, 0.15), (48.0, 0.1), (52.0, 0.05), (55.0, 0.0)]);
    let w = 1.25;
    if f >= 25.0 && old > 0.0 {
        s.push(Prim::heart_wide(cx, cy, sz, w).tex_a().alpha(old).soft(0.08 * sz + 0.004));
    }
    if op > 0.0 {
        // Body with a pink core, glossy highlight.
        s.push(Prim::heart_wide(cx, cy, sz * 0.97, w).col(0xFAE4E6).radial(0xF5A0B0, 1.1 * sz).alpha(op).soft(0.1 * sz + 0.004));
        s.push(Prim::ellipse(cx - 0.55 * sz, cy - 0.85 * sz, 0.22 * sz, 0.12 * sz).rot(-0.5).col(0xFFFFFF).alpha(0.3 * op).soft(0.08 * sz));
    }
    // Sparkles: a burst of streaks out of the centre, then twinkling stars.
    for i in 0..18u32 {
        let r = |k: u32| rnd(i, 40 + k);
        let f0 = 9.0 + 6.0 * r(1);
        let a = tri(f, f0, f0 + 3.0, f0 + 7.0);
        if a > 0.0 {
            let ang = TAU * r(2);
            let d = (0.1 + 0.6 * r(3)) * seg(f, f0 - 2.0, f0 + 8.0) * (sz + 0.2);
            let (x, y) = (cx + d * ang.cos(), vy + 0.6 * d * ang.sin());
            s.push(Prim::capsule(x, y, 0.012 + 0.012 * r(4), 0.0035, 0.3).rot(ang).col(0xFFFFFF).alpha(0.9 * a).add());
        }
    }
    for i in 0..44u32 {
        let r = |k: u32| rnd(i, 70 + k);
        let f0 = 10.0 + 44.0 * r(1);
        let a = tri(f, f0, f0 + 2.0 + 2.0 * r(5), f0 + 5.0 + 3.0 * r(6));
        if a > 0.0 {
            let (x, y) = (0.85 * (2.0 * r(2) - 1.0), 0.48 * (2.0 * r(3) - 1.0));
            let rr = 0.008 + 0.018 * r(4);
            s.push(Prim::star(x, y, rr * (0.6 + 0.4 * a), 4.0, 0.18).col(0xFFFFFF).alpha(a).add());
            s.push(Prim::circle(x, y, 0.4 * rr).col(0xFFFFFF).alpha(0.6 * a).soft(0.5 * rr).add());
        }
    }
    s
}

/// A big mint disc with an orange ring, spinning white arcs, dark orbit
/// arcs and dash marks rolls in from the left, idles at the centre and
/// rolls out to the right; the new clip follows the disc's centre line.
pub fn memphis_disc_roll(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    let cx = kf(f, &[(9.0, -1.7), (13.0, -1.01), (15.0, -0.75), (16.0, -0.49), (17.0, -0.31), (18.0, -0.2), (19.0, -0.12), (21.0, -0.06), (23.0, 0.0), (25.0, 0.04), (28.0, 0.06), (33.0, 0.07), (35.0, 0.1), (37.0, 0.15), (39.0, 0.21), (40.0, 0.27), (41.0, 0.36), (42.0, 0.48), (43.0, 0.58), (44.0, 0.68), (45.0, 0.82), (46.0, 1.0), (47.0, 1.25), (48.0, 1.5), (50.0, 2.0)]);
    if f >= 49.0 {
        s.mix = 1.0;
    } else if cx > -0.9 {
        s.push(Prim::rect_xy(-2.0, -1.0, cx, 1.0).tex_b());
    }
    if cx > 1.6 || cx < -1.6 {
        return s;
    }
    let deg = |d: f32| d.to_radians();
    let dark = 0x454547;
    // Dark orbit arcs and dash marks behind the disc.
    let orb = deg(-4.0 * (f - 27.0));
    s.push(Prim::arc(cx, 0.0, 0.6, 0.024, deg(55.0)).rot(deg(180.0) + orb).col(dark));
    s.push(Prim::arc(cx, 0.0, 0.6, 0.024, deg(38.0)).rot(deg(-10.0) + orb).col(dark));
    s.push(Prim::arc(cx, 0.0, 0.64, 0.02, deg(22.0)).rot(deg(120.0) - orb).col(0x7DE2C2));
    let dash = deg(40.0 + 2.0 * (f - 27.0));
    for k in 0..4 {
        let r = 0.5 + 0.035 * k as f32;
        let (x, y) = (cx + r * dash.cos(), r * dash.sin());
        s.push(Prim::capsule(x, y, 0.05 - 0.008 * k as f32, 0.007, 1.0).rot(deg(-12.0)).col(dark));
    }
    // Orange crescent, the disc, the ring.
    s.push(Prim::circle(cx - 0.045, -0.02, 0.5).col(0xFFA552));
    s.push(Prim::circle(cx, 0.0, 0.514).col(0x7FE5C3));
    s.push(annulus_c(cx, 0.0, 0.148, 0.31).col(0xFFA552));
    // Spinning white arcs.
    let ai = deg(47.0 + 7.0 * (f - 27.0));
    let ao = deg(130.0 + 9.0 * (f - 27.0));
    s.push(Prim::arc(cx, 0.0, 0.158, 0.016, deg(47.0)).rot(ai).col(0xF6F7EE));
    s.push(Prim::arc(cx, 0.0, 0.33, 0.02, deg(42.0)).rot(ao).col(0xF6F7EE));
    // Small satellites: a mint dot, an orange ring and a pink dot.
    let sd = deg(48.0 - 2.0 * (f - 18.0));
    s.push(Prim::circle(cx + 0.56 * sd.cos(), 0.56 * sd.sin(), 0.04).col(0x7FE5C3));
    s.push(Prim::circle(cx - 0.42, 0.47, 0.04).stroke(0.012).col(0xFFA552));
    if f < 40.0 {
        s.push(Prim::circle(0.33, -0.4, 0.016).col(0xF27A86));
    }
    s
}

/// Filled ring between radii r0 and r1.
pub fn annulus_c(cx: f32, cy: f32, r0: f32, r1: f32) -> Prim {
    Prim::circle(cx, cy, 0.5 * (r0 + r1)).stroke(r1 - r0)
}

/// Region behind a right-pointing 90° chevron front with a rounded tip at
/// (tip, cy): a huge square turned 45° with rounded corners.
pub fn chevron_region(tip: f32, cy: f32, r: f32) -> Prim {
    let l = 3.0;
    let cx = tip - l * std::f32::consts::SQRT_2 + r * (std::f32::consts::SQRT_2 - 1.0);
    Prim::rect(cx, cy, l, l).rot(PI / 4.0).round(r)
}

/// Rounded chevron bands sweep left to right: a navy band, two dimmed
/// glimpses of the new clip, a white band, then the new clip itself.
pub fn chevron_band_sweep(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    if f >= 19.0 {
        s.mix = 1.0;
        return s;
    }
    let navy = kf(f, &[(1.0, -1.3), (2.0, -1.0), (3.0, -0.75), (4.0, -0.5), (5.0, -0.26), (6.0, -0.02), (7.0, 0.28), (8.0, 0.556), (10.0, 0.83), (11.0, 1.11), (12.0, 1.43)]);
    let back = kf(f, &[(4.0, -1.3), (5.0, -1.0), (6.0, -0.72), (7.0, -0.5), (8.0, -0.26), (10.0, -0.02), (11.0, 0.26), (12.0, 0.63), (13.0, 1.0), (14.0, 1.4)]);
    let white = kf(f, &[(5.0, -1.4), (6.0, -1.1), (7.0, -0.85), (8.0, -0.48), (10.0, -0.13), (11.0, 0.24), (12.0, 0.61), (13.0, 0.97), (14.0, 1.35)]);
    let newc = kf(f, &[(7.0, -1.3), (8.0, -1.0), (10.0, -0.72), (11.0, -0.5), (12.0, -0.26), (13.0, -0.02), (14.0, 0.2), (16.0, 0.43), (17.0, 0.67), (18.0, 1.0), (19.0, 1.3)]);
    let r = 0.14;
    let on = |x: f32| x > -1.25 && f >= 1.0;
    if on(navy) && back < 1.3 {
        s.push(chevron_region(navy + 0.08, 0.0, r).col(0x000000).alpha(0.3).soft(0.06));
        s.push(chevron_region(navy, 0.0, r).col(0x03041C));
    }
    if on(back) && white < 1.3 {
        s.push(chevron_region(back, 0.0, r).tex_b().rgb([0.42, 0.38, 0.5]));
        s.push(chevron_region(0.5 * (back + white), 0.0, r).tex_b().rgb([0.75, 0.72, 0.82]));
    }
    if on(white) {
        s.push(chevron_region(white, 0.0, r).col(0xE9E9EA).grad(0xD9D9DC, -0.5, 0.5));
    }
    if f >= 16.0 {
        s.push(chevron_region(newc + 0.06, 0.0, r).col(0x03041C));
    }
    if on(newc) {
        s.push(chevron_region(newc + 0.03, 0.0, r).col(0x000000).alpha(0.25).soft(0.03));
        s.push(chevron_region(newc, 0.0, r).tex_b());
    }
    s
}

/// Region behind a right-pointing sharp front through (tip, 0) whose
/// edges fall back by `k` per unit of |y|.
pub fn front_region(tip: f32, k: f32) -> Prim {
    let hy = 8.0;
    let hx = k * hy;
    Prim::rhombus(tip - hx, 0.0, hx, hy)
}

/// Arrow head pointing right with its tip at (tx, ty): two square-ended
/// 45° arms of horizontal thickness `th` whose front edges reach `reach`
/// above and below the tip.
pub fn arrowhead(tx: f32, ty: f32, th: f32, reach: f32) -> [Prim; 2] {
    let r2 = std::f32::consts::FRAC_1_SQRT_2;
    let hl = 0.5 * reach * std::f32::consts::SQRT_2;
    let ht = 0.5 * th * r2;
    [-1.0f32, 1.0].map(|sy| {
        let u = [-r2, sy * r2];
        let n = [-r2, -sy * r2];
        let (cx, cy) = (tx + u[0] * hl + n[0] * ht, ty + u[1] * hl + n[1] * ht);
        Prim::rect(cx, cy, hl, ht).rot(u[1].atan2(u[0]))
    })
}

/// The frame cuts to black and four arrow-head windows onto the new clip
/// dart in from the left one after another, each stopping in its slot;
/// the new clip then fills the gaps from the left.
pub fn arrowhead_slots(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    if f < 11.0 {
        return Scene::new();
    }
    let fill = kf(f, &[(40.0, -1.2), (41.0, -0.72), (42.0, -0.26), (43.0, -0.06), (44.0, 0.09), (46.0, 0.48), (47.0, 0.6), (48.0, 0.85), (49.0, 1.4)]);
    if f >= 49.0 {
        return Scene::cut(t, 0.0);
    }
    let mut s = Scene::new().dip([0.0; 3], 1.0);
    let cards: [&[(f32, f32)]; 4] = [
        &[(11.0, -0.9), (12.0, -0.61), (13.0, -0.315), (14.0, -0.02), (16.0, 0.28), (17.0, 0.57), (18.0, 0.87)],
        &[(16.0, -0.95), (17.0, -0.69), (18.0, -0.45), (19.0, -0.2), (20.0, 0.02), (22.0, 0.26), (23.0, 0.5)],
        &[(21.0, -0.93), (22.0, -0.74), (23.0, -0.56), (24.0, -0.39), (25.0, -0.22), (26.0, -0.04), (28.0, 0.13)],
        &[(25.0, -0.95), (26.0, -0.8), (28.0, -0.69), (29.0, -0.57), (30.0, -0.46), (31.0, -0.35), (32.0, -0.24)],
    ];
    for keys in cards {
        if f < keys[0].0 {
            continue;
        }
        for p in arrowhead(kf(f, keys), 0.0, 0.333, 0.47) {
            s.push(p.tex_b());
        }
    }
    if fill > -1.1 {
        s.push(front_region(fill, 0.5).tex_b());
    }
    s
}

/// Seven columns of stacked capsules (yellow, blue, purple) run through
/// the frame, alternately down from the top and up from the bottom, one
/// after another; the long purple tails pull through last and leave the
/// new clip behind them.
pub fn capsule_column_run(t: f32, asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    let starts = [5.0, 6.0, 8.0, 10.0, 12.0, 12.0, 14.0];
    let n = starts.len();
    let w = asp / n as f32;
    let r = 0.5 * w;
    let yel: &[(f32, f32)] = &[(0.0, 0.0), (2.0, 0.03), (4.0, 0.055), (6.0, 0.14), (8.0, 0.25), (10.0, 0.31), (12.0, 0.47), (14.0, 0.67), (16.0, 0.75), (18.0, 0.94), (20.0, 1.1), (22.0, 1.3), (30.0, 2.2)];
    let blu: &[(f32, f32)] = &[(4.0, 0.0), (6.0, 0.05), (8.0, 0.08), (10.0, 0.14), (12.0, 0.22), (14.0, 0.36), (16.0, 0.47), (18.0, 0.64), (20.0, 0.83), (22.0, 0.94), (24.0, 1.05), (26.0, 1.2), (34.0, 2.2)];
    let pur: &[(f32, f32)] = &[(7.0, 0.0), (8.0, 0.03), (10.0, 0.03), (12.0, 0.11), (14.0, 0.19), (16.0, 0.25), (18.0, 0.39), (20.0, 0.56), (22.0, 0.67), (24.0, 0.86), (26.0, 1.0), (28.0, 1.15), (36.0, 2.2)];
    let tail: &[(f32, f32)] = &[(23.0, -0.05), (24.0, 0.05), (26.0, 0.36), (28.0, 0.5), (30.0, 0.69), (32.0, 0.86), (34.0, 0.92), (36.0, 0.97), (38.0, 1.05), (40.0, 1.3)];
    let mut done = true;
    for (c, st) in starts.iter().enumerate() {
        let tau = f - st;
        let x = -0.5 * asp + w * (c as f32 + 0.5);
        // Descending columns start at the top edge, rising ones at the bottom.
        let dir = if c % 2 == 1 { 1.0 } else { -1.0 };
        let y = |d: f32| dir * (d - 0.5);
        let tl = kf(tau, tail);
        if tl < 1.29 {
            done = false;
        }
        if tl > -0.04 {
            let (y0, y1) = (y(-1.0), y(tl + r));
            s.push(Prim::rect_xy(x - r - 0.002, y0.min(y1), x + r + 0.002, y0.max(y1)).tex_b());
        }
        if tau <= 0.0 || tl > 1.29 {
            continue;
        }
        let back = if tl > -0.04 { tl + r } else { -1.0 };
        for (keys, col) in [(yel, 0xFDD730), (blu, 0x24B0EA), (pur, 0x8A74E2)] {
            let head = kf(tau, keys);
            if head <= 0.0 || tau < keys[0].0 {
                continue;
            }
            let lead = head - r;
            if lead > back {
                s.push(Prim::seg(x, y(back), x, y(lead), r).col(col));
            } else if keys == pur {
                s.push(Prim::circle(x, y(lead), r).col(col));
            }
        }
    }
    if done {
        s.mix = 1.0;
        s.prims.clear();
    }
    s
}

/// Four pinwheel wedges of the new clip open from the centre, each fanning
/// counter-clockwise from a quarter line until they meet.
pub fn pinwheel_wedges(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    // The wedges on the vertical lines open faster than the horizontal ones.
    let wv = kf(f, &[(13.0, 0.0), (14.0, 5.0), (17.0, 15.0), (20.0, 22.5), (23.0, 32.5), (26.0, 40.0), (29.0, 45.0), (32.0, 52.5), (35.0, 57.5), (38.0, 62.5), (41.0, 67.5), (44.0, 70.0), (47.0, 75.0), (50.0, 80.0), (52.0, 81.0), (54.0, 85.0), (56.0, 86.0), (58.0, 88.0), (60.0, 90.0)]);
    let wh = kf(f, &[(13.0, 0.0), (14.0, 5.0), (17.0, 5.0), (20.0, 7.5), (23.0, 10.0), (26.0, 15.0), (29.0, 20.0), (32.0, 22.5), (35.0, 27.5), (38.0, 32.5), (41.0, 37.5), (44.0, 45.0), (47.0, 50.0), (50.0, 57.5), (52.0, 65.0), (54.0, 70.0), (56.0, 77.5), (58.0, 85.0), (60.0, 90.0)]);
    if wv.min(wh) >= 89.9 {
        s.mix = 1.0;
        return s;
    }
    for k in 0..4 {
        let w = if k % 2 == 0 { wv } else { wh };
        if w > 0.0 {
            let a = (90.0 * k as f32 + 90.0 - 0.5 * w).to_radians();
            s.push(Prim::sector(0.0, 0.0, (0.5 * w).min(89.0).to_radians(), a).tex_b());
        }
    }
    s
}

/// The old clip shrinks into a card on a carousel of alternating old / new
/// cards over black; the carousel spins a few cards past and the new card
/// that stops in front grows to fill the frame.
pub fn card_carousel(t: f32, asp: f32) -> Scene {
    let f = t * 60.0;
    if f >= 58.0 {
        return Scene::cut(t, 0.0);
    }
    let mut s = Scene::new().dip([0.0; 3], 1.0);
    let phi = kf(f, &[(0.0, 0.0), (8.0, 10.0), (10.0, 15.5), (12.0, 20.4), (14.0, 29.6), (16.0, 40.0), (18.0, 53.0), (20.0, 72.0), (22.0, 94.7), (24.0, 125.0), (26.0, 161.0), (28.0, 203.0), (30.0, 239.0), (32.0, 278.0), (34.0, 319.0), (36.0, 346.0), (38.0, 374.0), (40.0, 388.0), (42.0, 404.0), (44.0, 418.0), (46.0, 425.0), (48.0, 430.0), (58.0, 450.0)]).to_radians();
    let r = 0.45;
    let mut cards: Vec<(f32, Prim)> = Vec::new();
    for i in 0..6 {
        let a = i as f32 * PI / 2.0 - phi;
        let z = a.cos();
        if z < -0.1 {
            continue;
        }
        let mut x = r * a.sin();
        let mut hh = 0.15 + 0.05 * z;
        let mut depth = z;
        if i == 0 && f < 11.0 {
            // The old clip shrinking out of full screen.
            hh = kf(f, &[(0.0, 0.5), (4.0, 0.415), (8.0, 0.285), (11.0, 0.195)]);
            depth = 2.0;
        }
        if i == 5 && f > 47.5 {
            hh = kf(f, &[(47.5, 0.197), (50.0, 0.24), (52.0, 0.33), (54.0, 0.41), (56.0, 0.46), (58.0, 0.5)]);
            x = kf(f, &[(47.5, 0.16), (58.0, 0.0)]);
            depth = 2.0;
        }
        if i > 0 && f < 4.0 {
            continue;
        }
        let hw = hh * asp;
        cards.push((depth, Prim::card(x, 0.0, hw, hh).quad_tex(i % 2 == 1)));
    }
    cards.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap_or(std::cmp::Ordering::Equal));
    s.extend(cards.into_iter().map(|c| c.1));
    s
}

/// Track of positions given in 96ths of the frame width (as measured on
/// the reference), converted to centred x.
fn cols(f: f32, keys: &[(f32, f32)], asp: f32) -> f32 {
    kf(f, keys) / 96.0 * asp - 0.5 * asp
}

/// Motion-blurred red paint streaks shoot across the frame from the left
/// in horizontal lanes; the new clip follows their tails.
pub fn red_streak_lanes(t: f32, asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    // (top row, bottom row of 27, head track, new-clip frontier track)
    type Keys = &'static [(f32, f32)];
    let lanes: [(f32, f32, Keys, Keys); 7] = [
        (0.0, 3.0, &[(33.0, -4.0), (34.0, 9.0), (35.0, 20.0), (36.0, 30.0), (37.0, 40.0), (38.0, 50.0), (40.0, 60.0), (41.0, 68.0), (42.0, 75.0), (43.0, 80.0), (44.0, 87.0), (46.0, 93.0), (47.0, 99.0)], &[(40.0, -6.0), (41.0, 8.0), (42.0, 20.0), (43.0, 36.0), (44.0, 51.0), (46.0, 66.0), (47.0, 81.0), (48.0, 93.0), (49.0, 100.0)]),
        (3.0, 8.0, &[(28.0, -10.0), (29.0, 0.0), (30.0, 12.0), (31.0, 23.0), (32.0, 35.0), (34.0, 45.0), (35.0, 55.0), (36.0, 68.0), (37.0, 78.0), (38.0, 82.0), (40.0, 87.0), (41.0, 92.0), (42.0, 99.0)], &[(34.0, -6.0), (35.0, 8.0), (36.0, 20.0), (37.0, 30.0), (40.0, 48.0), (41.0, 55.0), (42.0, 60.0), (44.0, 63.0), (46.0, 70.0), (47.0, 82.0), (48.0, 94.0), (49.0, 100.0)]),
        (8.0, 12.0, &[(25.0, -10.0), (26.0, 0.0), (28.0, 11.0), (29.0, 21.0), (30.0, 32.0), (31.0, 43.0), (32.0, 55.0), (34.0, 63.0), (35.0, 72.0), (36.0, 78.0), (37.0, 84.0), (38.0, 87.0), (40.0, 93.0), (41.0, 99.0)], &[(35.0, -6.0), (36.0, 7.0), (37.0, 20.0), (38.0, 35.0), (40.0, 50.0), (41.0, 62.0), (42.0, 66.0), (44.0, 69.0), (46.0, 79.0), (47.0, 87.0), (48.0, 96.0), (49.0, 100.0)]),
        (12.0, 15.0, &[(29.0, -8.0), (30.0, 4.0), (31.0, 16.0), (32.0, 28.0), (34.0, 40.0), (35.0, 49.0), (36.0, 61.0), (37.0, 70.0), (38.0, 74.0), (40.0, 75.0), (41.0, 80.0), (42.0, 85.0), (43.0, 87.0), (44.0, 92.0), (46.0, 99.0)], &[(39.0, -6.0), (40.0, 5.0), (41.0, 21.0), (42.0, 35.0), (43.0, 50.0), (44.0, 66.0), (46.0, 81.0), (47.0, 90.0), (48.0, 100.0)]),
        (15.0, 19.0, &[(31.0, -6.0), (32.0, 6.0), (33.0, 12.0), (34.0, 23.0), (35.0, 34.0), (36.0, 43.0), (38.0, 58.0), (40.0, 72.0), (41.0, 80.0), (42.0, 86.0), (43.0, 93.0), (44.0, 99.0)], &[(37.0, -6.0), (38.0, 3.0), (40.0, 20.0), (41.0, 35.0), (42.0, 50.0), (43.0, 65.0), (44.0, 80.0), (46.0, 88.0), (47.0, 94.0), (48.0, 100.0)]),
        (19.0, 21.0, &[(33.0, -6.0), (35.0, 9.0), (36.0, 19.0), (37.0, 29.0), (38.0, 34.0), (40.0, 45.0), (41.0, 55.0), (42.0, 66.0), (43.0, 75.0), (44.0, 78.0), (46.0, 80.0), (47.0, 87.0), (48.0, 92.0), (49.0, 99.0)], &[(43.0, -6.0), (44.0, 5.0), (46.0, 20.0), (47.0, 35.0), (48.0, 50.0), (49.0, 66.0), (50.0, 81.0), (51.0, 100.0)]),
        (21.0, 27.0, &[(30.0, -8.0), (31.0, 3.0), (32.0, 15.0), (34.0, 25.0), (35.0, 36.0), (36.0, 47.0), (37.0, 62.0), (38.0, 78.0), (40.0, 84.0), (41.0, 88.0), (42.0, 93.0), (43.0, 99.0)], &[(37.0, -6.0), (38.0, 9.0), (40.0, 33.0), (41.0, 49.0), (42.0, 61.0), (43.0, 72.0), (44.0, 84.0), (46.0, 88.0), (47.0, 92.0), (48.0, 100.0)]),
    ];
    let mut done = true;
    for (r0, r1, head, front) in lanes {
        let (y0, y1) = (r0 / 27.0 - 0.5, r1 / 27.0 - 0.5);
        let (cy, hh) = (0.5 * (y0 + y1), 0.5 * (y1 - y0));
        let hx = cols(f, head, asp);
        let bx = cols(f, front, asp);
        if bx < 0.5 * asp + 0.02 {
            done = false;
        }
        if bx > -0.5 * asp - 0.05 {
            s.push(Prim::rect_xy(-2.0, cy - hh - 0.003, bx, cy + hh + 0.003).tex_b().soft(0.012));
        }
        if f >= head[0].0 && hx > bx && bx < 0.5 * asp + 0.05 {
            let x0 = (bx - 0.06).max(-0.5 * asp - 0.3);
            let x1 = hx.min(0.5 * asp + 0.3);
            // Glow ahead of the head, then the streak body as a few
            // sub-streaks of slightly different lengths.
            s.push(Prim::rect_xy(x1 - 0.02, cy - hh * 0.8, x1 + 0.09, cy + hh * 0.8).col(0xF0484A).gradx(0xC9787A, -0.05, 0.055).alpha(0.45).soft(0.04));
            let n = ((r1 - r0) * 1.5).round().max(2.0) as u32;
            for k in 0..n {
                let (sy0, sy1) = (y0 + (y1 - y0) * k as f32 / n as f32, y0 + (y1 - y0) * (k + 1) as f32 / n as f32);
                let jit = 0.07 * (rnd(k + 7 * r0 as u32, 5) - 0.5);
                let xe = (x1 + jit).min(0.5 * asp + 0.3);
                if xe > x0 {
                    s.push(Prim::rect_xy(x0, sy0 - 0.004, xe, sy1 + 0.004).col(0xE33A40).gradx(0xF34248, -0.5 * (xe - x0), 0.5 * (xe - x0)).soft(0.012));
                }
            }
        }
    }
    if done {
        s.prims.clear();
        s.mix = 1.0;
    }
    s
}

/// The camera pans and turns across a paper desk from the old clip's
/// photo print to the new clip's print (each print a window onto its clip), passing a red string tied into a
/// heart; the new print ends up filling the frame.
pub fn photo_desk_pan(t: f32, asp: f32) -> Scene {
    let f = t * 60.0;
    if f >= 59.5 {
        return Scene::cut(t, 0.0);
    }
    let mut s = Scene::new().dip(hexc(0xF2ECE7), 1.0);
    // Camera over the desk (world units: the old print is centred at the
    // origin, one unit tall): position, zoom and roll.
    let camx = kf(f, &[(0.0, 0.0), (4.0, 0.17), (8.0, 0.31), (12.0, 0.51), (16.0, 0.65), (20.0, 0.76), (24.0, 0.98), (28.0, 1.13), (32.0, 1.26), (36.0, 1.45), (40.0, 1.6), (44.0, 1.71), (48.0, 1.86), (52.0, 1.96), (56.0, 2.02), (60.0, 2.06)]);
    let camy = kf(f, &[(0.0, 0.0), (4.0, 0.02), (8.0, 0.034), (12.0, 0.024), (16.0, 0.012), (20.0, 0.0), (24.0, -0.027), (28.0, -0.057), (32.0, -0.1), (36.0, -0.167), (40.0, -0.227), (44.0, -0.28), (48.0, -0.344), (52.0, -0.406), (56.0, -0.45), (60.0, -0.5)]);
    let z = kf(f, &[(0.0, 1.0), (4.0, 0.93), (8.0, 0.89), (12.0, 0.83), (16.0, 0.8), (20.0, 0.77), (24.0, 0.73), (28.0, 0.7), (32.0, 0.7), (36.0, 0.72), (40.0, 0.75), (44.0, 0.82), (48.0, 0.9), (52.0, 0.96), (56.0, 1.0)]);
    let rho = kf(f, &[(0.0, 0.0), (20.0, 1.5), (40.0, 2.8), (60.0, 3.0)]).to_radians();
    let (c, sn) = (rho.cos(), rho.sin());
    let to_s = |w: [f32; 2]| {
        let (x, y) = (w[0] - camx, w[1] - camy);
        [z * (c * x - sn * y), z * (sn * x + c * y)]
    };
    // Faint paper grain.
    s.push(Prim::rect(0.0, 0.0, 2.0, 1.0).col(0xF3EDE8).noise(0xE8E0DA, 26.0, 0.0).alpha(0.6));
    let hw = 0.5 * asp;
    // (world centre, own rotation, is the new clip)
    for (wc, rot, new) in [([0.0f32, 0.0f32], 0.0f32, false), ([2.06, -0.5], -3.0f32.to_radians(), true)] {
        let ctr = to_s(wc);
        let a = rot + rho;
        if ctr[0].abs() > 2.6 || ctr[1].abs() > 1.8 {
            continue;
        }
        let b = 0.019 * z;
        let (w2, h2) = (hw * z, 0.5 * z);
        s.push(Prim::rect(ctr[0] + 0.012, ctr[1] + 0.016, w2 + b, h2 + b).rot(a).col(0x5A4A40).alpha(0.28).soft(0.025));
        s.push(Prim::rect(ctr[0], ctr[1], w2 + b, h2 + b).rot(a).col(0xFBFAF8));
        // The print is a window onto its clip as it is on screen.
        let win = Prim::rect(ctr[0], ctr[1], w2, h2).rot(a);
        s.push(if new { win.tex_b() } else { win.tex_a() });
    }
    // Red string tied into a heart, lying on the desk between the prints.
    let red = 0xB3283E;
    let hp = to_s([1.2, 0.32]);
    s.push(Prim::heart_wide(hp[0], hp[1], 0.075 * z, 1.35).rot(rho - 0.15).stroke(0.0065 * z).col(red));
    for (w0, w1, w2) in [([1.2, 0.37], [1.32, 0.5], [1.5, 0.47]), ([1.19, 0.37], [1.1, 0.5], [1.02, 0.62]), ([0.75, 0.6], [0.95, 0.56], [1.1, 0.48])] {
        let (p0, p1, p2) = (to_s(w0), to_s(w1), to_s(w2));
        s.push(Prim::seg(p0[0], p0[1], p1[0], p1[1], 0.0032 * z).col(red));
        s.push(Prim::seg(p1[0], p1[1], p2[0], p2[1], 0.0032 * z).col(red));
    }
    s
}

/// Cartoon cloud puffs pop in from the edges until they fill the frame,
/// hold, then pop away from a gap near the top to uncover the new clip.
pub fn cloud_puff_cover(t: f32, asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::cut(t, 0.5);
    let (nx, ny) = (10, 6);
    let (sx, sy) = (asp / (nx - 1) as f32, 1.0 / (ny - 1) as f32);
    let mut puffs: Vec<(f32, f32, f32, f32)> = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            let id = (j * nx + i) as u32;
            let x = -0.5 * asp + sx * (i as f32 + 0.5 * (j % 2) as f32) + 0.06 * (rnd(id, 1) - 0.5);
            let y = -0.5 + sy * j as f32 + 0.05 * (rnd(id, 2) - 0.5);
            // Distance from the nearest edge (0 at the border, ~0.5 at the centre).
            let edge = (0.5 * asp - x.abs()).min(0.5 - y.abs()).max(0.0) / 0.5;
            let t_in = 1.0 + 10.0 * edge.powf(0.7) + 2.0 * rnd(id, 3);
            // Leaving: from a gap at the upper middle outwards, the bottom last.
            let d = ((x - 0.05).powi(2) + (1.6 * (y + 0.3)).powi(2)).sqrt();
            let t_out = 41.0 + 11.0 * (d / 1.3).min(1.0) + 2.0 * rnd(id, 4);
            let r = 0.15 + 0.08 * rnd(id, 5);
            let k = ease_out(seg(f, t_in, t_in + 4.0)).min(1.0 - ease_in2(seg(f, t_out, t_out + 4.0)));
            if k > 0.0 {
                puffs.push((x, y, r * (0.4 + 0.6 * k), t_in));
            }
        }
    }
    puffs.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
    for (x, y, r, _) in puffs {
        s.push(Prim::circle(x + 0.06 * r, y + 0.08 * r, r).col(0xB4D4E4));
        s.push(Prim::circle(x, y, 0.96 * r).col(0xFCFCFC));
    }
    s
}
