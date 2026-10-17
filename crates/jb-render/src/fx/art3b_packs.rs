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
