//! The art3b items: one scene function per transition. Progress `t` runs
//! 0..1 over the transition (Filmora's default 2.002 s = 60 frames), `asp`
//! is the frame aspect. Each comment names the motion it recreates.

use super::kit::*;
use super::packs::*;

/// Diagonal capsule streaks in blue and teal shoot along a 50° diagonal
/// from both corners, stretch until they cover the frame and fly on,
/// leaving the new clip behind them.
fn diagonal_streak_sweep(t: f32, asp: f32) -> Scene {
    let mut s = Scene::cut(t, 0.37);
    let a = -50f32.to_radians();
    let (d, n) = ([a.cos(), a.sin()], [-a.sin(), a.cos()]);
    let span = 0.5 * (asp * a.sin().abs() + a.cos().abs());
    let reach = 0.5 * (asp * a.cos().abs() + a.sin().abs()) + 0.1;
    let lanes = 22;
    let lw = 2.0 * span / lanes as f32;
    for i in 0..lanes {
        let r = |k| rnd(i, k);
        let o = -span + lw * (i as f32 + 0.5) + 0.2 * lw * (r(1) - 0.5);
        let w = 0.5 * lw * (1.15 + 0.5 * r(2));
        let dir = if r(7) < 0.5 { 1.0 } else { -1.0 };
        // Head crosses the frame, then the tail follows it out.
        let h0 = 0.03 + 0.2 * r(4);
        let h1 = (h0 + 0.1 + 0.08 * r(5)).min(0.355);
        // Outer lanes clear first, the central ones last: lane i is gone
        // when the area of the lanes farther out reaches the cleared share.
        let mut out = 0.0;
        let mut tot = 0.0;
        for j in 0..lanes {
            let oj = -span + lw * (j as f32 + 0.5);
            let c = chord(asp, d, n, oj);
            tot += c;
            if oj.abs() > o.abs() {
                out += c;
            } else if j == i {
                out += 0.5 * c;
            }
        }
        let t1 = kf(out / tot, &[(0.0, 0.40), (0.15, 0.43), (0.34, 0.48), (0.51, 0.52), (0.64, 0.57), (0.77, 0.60), (0.89, 0.63), (1.0, 0.69)]) + 0.02;
        let t0 = t1 - 0.08;
        let head = lerp(-reach, reach, seg(t, h0, h1));
        let tail = lerp(-reach, reach, seg(t, t0, t1));
        if t < h0 || t > t1 || head - tail < 1e-3 {
            continue;
        }
        let mid = dir * 0.5 * (head + tail);
        let hl = 0.5 * (head - tail);
        let (cx, cy) = (mid * d[0] + o * n[0], mid * d[1] + o * n[1]);
        let (c1, c2) = if r(6) < 0.65 { (0x2AA4F2, 0x4CC4E6) } else { (0x5ED8C8, 0x2AA4F2) };
        s.push(Prim::capsule(cx, cy, hl, w, 1.0).rot(a + if dir < 0.0 { PI } else { 0.0 }).col(c1).gradx(c2, -hl, hl));
    }
    s
}

/// Length of the line through the frame at lateral offset `o` (direction `d`, normal `n`).
fn chord(asp: f32, d: [f32; 2], n: [f32; 2], o: f32) -> f32 {
    let (px, py) = (o * n[0], o * n[1]);
    let (mut lo, mut hi) = (-1e3f32, 1e3f32);
    for (p, dd, h) in [(px, d[0], 0.5 * asp), (py, d[1], 0.5)] {
        if dd.abs() < 1e-6 {
            if p.abs() > h {
                return 0.0;
            }
            continue;
        }
        let (a, b) = ((-h - p) / dd, (h - p) / dd);
        lo = lo.max(a.min(b));
        hi = hi.min(a.max(b));
    }
    (hi - lo).max(0.0)
}

/// Four red panels slide in from the sides along the frame's diagonals and
/// lock into a full red screen with an X seam and a glint, then slide back
/// out along the X to uncover the new clip.
fn red_x_panels(t: f32, asp: f32) -> Scene {
    let mut s = Scene::cut(t, 0.5);
    let th = 37f32.to_radians();
    let far = 2.5;
    let (c, sn) = (th.cos(), th.sin());
    // (bisector, corner directions along the X lines, entry start, exit start)
    let panels = [
        ([0.0, -1.0], [-c, -sn], [c, -sn], 0.26, 0.61),
        ([1.0, 0.0], [c, -sn], [c, sn], 0.24, 0.625),
        ([0.0, 1.0], [c, sn], [-c, sn], 0.29, 0.6),
        ([-1.0, 0.0], [-c, sn], [-c, -sn], 0.255, 0.615),
    ];
    for (k, (b, l1, l2, t0, t2)) in panels.iter().enumerate() {
        let enter = 1.0 - ease_out(seg(t, *t0, t0 + 0.17));
        let leave = ease_in2(seg(t, *t2, t2 + 0.2));
        let off = 1.25 * enter + 1.3 * leave + 0.004;
        let o = [b[0] * off, b[1] * off];
        let pt = |d: [f32; 2], r: f32| [o[0] + d[0] * r, o[1] + d[1] * r];
        let outer = [o[0] + b[0] * far, o[1] + b[1] * far];
        let q = Prim::quad([pt([0.0, 0.0], 0.0), pt(*l1, far), outer, pt(*l2, far)]);
        let shade = [0xD8282C, 0xCF2529, 0xD32A2D, 0xCB2428][k];
        s.push(q.col(shade));
        // Chevrons on the side panels.
        if k % 2 == 1 && off < 0.6 {
            let a = if b[0] > 0.0 { 0.0 } else { PI };
            for j in 0..4 {
                let x = b[0] * (0.5 * asp - 0.05 - 0.045 * j as f32) + o[0];
                for y in [-0.38, 0.38] {
                    s.push(Prim::star(x, y, 0.035, 3.0, 0.15).rot(a).col(0xF06A6A).alpha(0.35).stroke(0.006));
                }
            }
        }
    }
    // Glint where the panels meet.
    let g = tri(t, 0.42, 0.5, 0.58);
    if g > 0.0 {
        s.push(Prim::star(0.0, 0.0, 0.5 * g, 4.0, 0.04).rot(PI / 4.0).col(0xFF8080).alpha(0.5 * g).soft(0.01).add());
        s.push(Prim::circle(0.0, 0.0, 0.06).col(0xFF9090).alpha(0.4 * g).soft(0.06).add());
    }
    s
}

/// A tilted square window onto the new clip grows out of the left of the
/// frame inside a thick blue frame, a white frame and a pale blue frame
/// that spin past; the window grows until it fills the screen.
fn square_frames_zoom(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    // Window onto B.
    let hs = kf(f, &[(9.0, 0.0), (10.0, 0.03), (15.0, 0.1), (18.0, 0.2), (21.0, 0.33), (25.0, 0.55), (30.0, 0.9), (36.0, 1.4), (44.0, 2.0)]);
    let rot = kf(f, &[(9.0, 0.8), (17.0, 0.35), (21.0, 0.17), (25.0, 0.08), (30.0, -0.06), (40.0, -0.1)]);
    let cx = kf(f, &[(9.0, -0.26), (17.0, -0.22), (25.0, -0.16), (30.0, -0.1), (40.0, 0.0)]);
    let cy = kf(f, &[(9.0, 0.08), (17.0, 0.1), (25.0, 0.06), (30.0, 0.1), (40.0, 0.0)]);
    if f >= 44.0 {
        s.mix = 1.0;
    } else if hs > 0.0 {
        // Tinted halo behind the window, then the window itself.
        s.push(Prim::rect(cx, cy, hs * 1.12, hs * 1.12).rot(rot * 1.4 + 0.12).col(0x8A2E5A).alpha(0.55));
        s.push(Prim::rect(cx, cy, hs, hs).rot(rot).tex_b());
    }
    // Thick blue frame, nearly upright, growing faster.
    let bh = kf(f, &[(13.0, 0.0), (15.0, 0.1), (18.0, 0.2), (21.0, 0.33), (25.0, 0.5), (30.0, 0.75), (36.0, 1.05), (40.0, 1.3)]);
    let bw = kf(f, &[(13.0, 0.02), (21.0, 0.03), (25.0, 0.05), (30.0, 0.07), (36.0, 0.1)]);
    if bh > 0.0 && f < 40.0 {
        s.push(Prim::rect(cx + 0.03, cy, bh, bh).rot(0.5 * rot).stroke(bw).col(0x2F6BF5));
    }
    // White frame inside.
    let wh = kf(f, &[(16.0, 0.0), (18.0, 0.03), (21.0, 0.07), (25.0, 0.15), (30.0, 0.35), (36.0, 0.58), (42.0, 0.62), (48.0, 0.66)]);
    let ww = kf(f, &[(16.0, 0.006), (25.0, 0.015), (30.0, 0.04), (36.0, 0.05), (42.0, 0.03), (48.0, 0.005)]);
    if wh > 0.0 && f < 48.0 {
        let (wx, wy) = (kf(f, &[(25.0, cx), (36.0, 0.0)]), kf(f, &[(25.0, cy), (36.0, 0.12)]));
        s.push(Prim::rect(wx, wy, wh, wh).rot(-0.3 * rot + 0.05).stroke(ww).col(0xF2F4FA));
    }
    // Pale blue frame spinning in from the top right.
    let ph = kf(f, &[(18.0, 0.0), (19.0, 0.03), (25.0, 0.1), (30.0, 0.22), (36.0, 0.32), (42.0, 0.36), (48.0, 0.38)]);
    if ph > 0.0 && f < 48.0 {
        let px = kf(f, &[(19.0, 0.55), (30.0, 0.5), (36.0, 0.45), (48.0, 0.38)]);
        let py = kf(f, &[(19.0, -0.3), (30.0, -0.28), (36.0, -0.2), (48.0, -0.15)]);
        let pr = kf(f, &[(19.0, 0.6), (30.0, 0.3), (36.0, 0.1), (48.0, 0.0)]);
        let pw = kf(f, &[(19.0, 0.008), (30.0, 0.03), (36.0, 0.04), (42.0, 0.02), (48.0, 0.004)]);
        s.push(Prim::rect(px, py, ph, ph).rot(pr).stroke(pw).col(0x62B0F5));
    }
    // Small diamonds.
    for (k, (x, y, f0)) in [(0.55, -0.42, 18.0), (0.62, 0.38, 26.0), (0.4, 0.3, 30.0)].iter().enumerate() {
        let a = tri(f, *f0, f0 + 6.0, f0 + 18.0);
        if a > 0.0 {
            s.push(Prim::poly(*x, *y, 0.02 + 0.01 * k as f32, 4.0).col(0x3A5FD0).alpha(a));
        }
    }
    s
}

/// Window onto B: the scene's background stays A and B shows through the
/// shape; when `full` the background switches to B (shape covers the frame).
fn window(s: &mut Scene, p: Prim, full: bool) {
    if full {
        s.mix = 1.0;
    } else {
        s.push(p.tex_b());
    }
}

/// A round window onto the new clip opens at the left third inside two
/// red rings and grows until it covers the frame, the rings sweeping off
/// to the right.
fn red_ring_iris(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    let (cx, cy) = (-0.44, -0.03);
    let r = kf(f, &[(8.0, 0.0), (12.0, 0.03), (14.0, 0.05), (18.0, 0.12), (22.0, 0.23), (26.0, 0.37), (30.0, 0.55), (34.0, 0.82), (38.0, 1.02), (42.0, 1.17), (46.0, 1.3), (50.0, 1.38), (53.0, 1.45)]);
    if r <= 0.0 {
        return s;
    }
    if r > 1.44 {
        s.mix = 1.0;
        return s;
    }
    let w1 = (0.28 * r).min(0.13);
    let w2 = (0.16 * r).min(0.09);
    let g0 = (0.04 * r).min(0.012);
    let g1 = kf(r, &[(0.0, 0.0), (0.1, 0.06), (0.3, 0.06), (0.6, 0.0)]);
    // Window, a gap of the old clip, the dark ring, a gap, the light ring.
    s.push(annulus(cx, cy, r + g0 + w1 + g1, r + g0 + w1 + g1 + w2).col(0xDC4646));
    s.push(annulus(cx, cy, r + g0, r + g0 + w1).col(0xD32626));
    s.push(Prim::circle(cx, cy, r).tex_b());
    s
}

/// A round window onto the new clip opens at the centre inside a wide white
/// ring and grows to fill the frame, with orange rings drifting out of the
/// corners and small dots.
fn white_ring_iris(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    let (cx, cy) = (0.0, 0.02);
    let r = kf(f, &[(9.0, 0.0), (14.0, 0.04), (18.0, 0.09), (22.0, 0.16), (24.0, 0.23), (26.0, 0.32), (28.0, 0.44), (30.0, 0.6), (32.0, 0.74), (34.0, 0.84), (36.0, 0.94), (40.0, 1.04)]);
    // Small coloured dots.
    for (k, (x, y, f0, c)) in [(-0.5, -0.2, 14.0, 0xC23A3A), (0.43, 0.15, 16.0, 0x7040A0), (0.3, -0.38, 18.0, 0xD03030)].iter().enumerate() {
        let a = tri(f, *f0, f0 + 6.0, f0 + 16.0);
        if a > 0.0 {
            s.push(Prim::circle(*x, *y, 0.012 + 0.006 * k as f32).col(*c).alpha(a));
            s.push(Prim::circle(*x, *y, 0.024 + 0.008 * k as f32).stroke(0.006).col(*c).alpha(a));
        }
    }
    if r > 0.0 {
        let wr = r * kf(f, &[(14.0, 2.2), (26.0, 1.8), (32.0, 1.35), (40.0, 1.2)]);
        let ww = kf(f, &[(14.0, 0.006), (24.0, 0.012), (32.0, 0.02), (40.0, 0.01)]);
        s.push(Prim::circle(cx, cy, wr).stroke(ww).col(0xF4F4F4).alpha(1.0 - seg(f, 40.0, 46.0)));
        s.push(Prim::circle(cx, cy, r * 1.12 + 0.01).stroke(0.01).col(0x6E3C8C).alpha(0.8));
        window(&mut s, Prim::circle(cx, cy, r), r > 1.03);
    }
    // Orange rings growing out of the corners.
    for (x, y, f0, rr, w) in [(-0.62f32, -0.25, 28.0, 0.12, 0.012), (0.68, 0.25, 30.0, 0.17, 0.03)] {
        let k = seg(f, f0, f0 + 16.0);
        if k > 0.0 && k < 1.0 {
            let a = 1.0 - seg(k, 0.6, 1.0);
            s.push(Prim::circle(x - 0.15 * k * x.signum(), y, rr * (1.0 + 1.6 * k)).stroke(w * (1.0 - 0.6 * k)).col(0xF07A20).alpha(a));
        }
    }
    s
}

/// A dome of the new clip rises from the bottom inside an orange ring while
/// a small round window with its own orange ring opens at the top right;
/// both grow until the dome fills the frame.
fn orange_ring_rise(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    // Small window, top right.
    let (sx, sy) = (0.53, -0.24);
    let rs = kf(f, &[(10.0, 0.0), (12.0, 0.01), (18.0, 0.04), (22.0, 0.08), (26.0, 0.12), (30.0, 0.16), (36.0, 0.2)]);
    if rs > 0.0 && f < 40.0 {
        let a = 1.0 - seg(f, 34.0, 40.0);
        s.push(Prim::circle(sx, sy, rs * 2.1 + 0.02).stroke(0.012 + 0.08 * rs).col(0xF29A1E).alpha(a));
        s.push(Prim::circle(sx, sy, rs * 1.4 + 0.01).stroke(0.004).col(0x52A846).alpha(a));
        s.push(Prim::circle(sx, sy, rs).tex_b());
    }
    // Big dome from the bottom.
    let cx = kf(f, &[(14.0, -0.34), (24.0, -0.2), (34.0, -0.1)]);
    let cy = kf(f, &[(14.0, 0.55), (24.0, 0.47), (34.0, 0.4)]);
    let r = kf(f, &[(14.0, 0.0), (16.0, 0.1), (19.0, 0.24), (22.0, 0.45), (26.0, 0.66), (30.0, 0.88), (34.0, 1.03), (38.0, 1.15), (42.0, 1.25), (46.0, 1.33), (50.0, 1.4)]);
    if r > 0.0 {
        s.push(Prim::circle(cx, cy, r * 1.35 + 0.03).stroke(0.006).col(0xF6F6F6).alpha(1.0 - seg(f, 30.0, 40.0)));
        let w = (0.12 * r).min(0.1);
        s.push(Prim::circle(cx, cy, r + 0.6 * w).stroke(w).col(0xF5B531));
        window(&mut s, Prim::circle(cx, cy, r), r > 1.39);
    }
    // Large orange arcs sweeping across after the cover.
    for (k, (x, y, f0, rr)) in [(0.25, -0.15, 24.0, 0.25), (-0.1, 0.25, 30.0, 0.3)].iter().enumerate() {
        let q = seg(f, *f0, f0 + 20.0);
        if q > 0.0 && q < 1.0 {
            let a = 1.0 - seg(q, 0.55, 1.0);
            let w = 0.06 * (1.0 - 0.8 * q);
            s.push(Prim::circle(x + 0.1 * q, y + 0.12 * q * k as f32, rr * (1.0 + 1.5 * q)).stroke(w).col(0xF28C14).alpha(a));
        }
    }
    s
}

/// Hexagonal windows onto the new clip open at the top right and the lower
/// left inside green and blue outlines, grow and merge until they cover the
/// frame; a white hexagon outline drifts off as they settle.
fn hex_windows(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    let hex = |x: f32, y: f32, r: f32| Prim::poly(x, y, r, 6.0);
    // First hexagon: top right moving towards the centre.
    let r1 = kf(f, &[(8.0, 0.0), (10.0, 0.02), (14.0, 0.07), (18.0, 0.13), (22.0, 0.23), (26.0, 0.42), (30.0, 0.75), (34.0, 1.1), (38.0, 1.4)]);
    let c1 = [kf(f, &[(8.0, 0.32), (22.0, 0.26), (34.0, 0.1)]), kf(f, &[(8.0, -0.25), (22.0, -0.2), (34.0, -0.05)])];
    let r2 = kf(f, &[(16.0, 0.0), (18.0, 0.03), (22.0, 0.08), (26.0, 0.16), (30.0, 0.3), (34.0, 0.45), (40.0, 0.6)]);
    let c2 = [kf(f, &[(16.0, -0.62), (26.0, -0.45), (40.0, -0.35)]), kf(f, &[(16.0, 0.27), (26.0, 0.18), (40.0, 0.1)])];
    let full = r1 > 1.35;
    for (c, r) in [(c1, r1), (c2, r2)] {
        if r <= 0.0 || full {
            continue;
        }
        s.push(hex(c[0], c[1], r * 1.5 + 0.02).stroke(0.008).col(0x4AA040));
        s.push(hex(c[0], c[1], r * 1.25 + 0.012).stroke(0.008).col(0x4A7BEA));
    }
    for (c, r) in [(c1, r1), (c2, r2)] {
        if r > 0.0 {
            window(&mut s, hex(c[0], c[1], r), full);
        }
    }
    // White hexagon outline growing from the lower left after the cover.
    let q = seg(f, 26.0, 46.0);
    if q > 0.0 && q < 1.0 {
        let a = 1.0 - seg(q, 0.7, 1.0);
        s.push(hex(-0.55 + 0.4 * q, 0.25 - 0.1 * q, 0.06 + 0.3 * q).stroke(0.03 * (1.0 - 0.7 * q)).col(0xF8F8F8).alpha(a));
    }
    s
}

/// Annulus between radii r0 and r1 about (cx, cy).
fn annulus(cx: f32, cy: f32, r0: f32, r1: f32) -> Prim {
    Prim::circle(cx, cy, 0.5 * (r0 + r1)).stroke((r1 - r0).max(0.0))
}

/// An orange chevron sweeps right across the frame, a white chevron follows
/// it uncovering the new clip, and a thin yellow chevron trails behind.
fn chevron_trio_right(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    let k = 0.5;
    let hh = 0.7;
    let white_back = kf(f, &[(18.0, -1.3), (22.0, -0.95), (24.0, -0.78), (26.0, -0.51), (28.0, -0.23), (30.0, 0.08), (32.0, 0.33), (34.0, 0.53), (36.0, 0.68), (38.0, 0.78), (40.0, 0.86), (42.0, 0.95), (46.0, 1.5)]);
    let white_front = white_back + 0.21;
    let orange_front = kf(f, &[(6.0, -0.92), (8.0, -0.84), (12.0, -0.76), (16.0, -0.62), (20.0, -0.37), (22.0, -0.15), (24.0, 0.12), (26.0, 0.43), (28.0, 0.69), (30.0, 0.88), (32.0, 1.2)]);
    let orange_back = (orange_front - 0.39).max(white_front + 0.01);
    let yellow_front = kf(f, &[(28.0, -1.0), (31.0, -0.67), (34.0, -0.31), (37.0, 0.22), (40.0, 0.67), (43.0, 0.9), (46.0, 1.1), (50.0, 1.5)]);
    // New clip left of the white chevron's back edge.
    if white_back > 1.2 {
        s.mix = 1.0;
    } else if white_back > -1.2 {
        s.push(Prim::chevron(white_back, 0.0, 4.0, k, hh).tex_b());
    }
    if orange_front > orange_back && orange_back < 1.2 {
        s.push(Prim::chevron(orange_front, 0.0, orange_front - orange_back, k, hh).col(0xFA8C00));
    }
    if white_back > -1.3 && white_back < 1.3 {
        s.push(Prim::chevron(white_front, 0.0, 0.21, k, hh).col(0xEEF3EC));
    }
    if yellow_front > -0.95 && yellow_front < 1.45 {
        s.push(Prim::chevron(yellow_front, 0.0, 0.075, k, hh).col(0xFFD400));
    }
    s
}

/// Rings: a round window onto the new clip opens at the centre inside a
/// yellow ring while yellow, white and orange rings and arcs close in from
/// the edges; the frame cuts to the new clip and an orange ring thins out.
fn ring_burst_center(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::cut(f / 60.0, 33.0 / 60.0);
    let cut = f >= 33.0;
    // Streaks and a dot announcing the rings.
    for (k, (a0, f0)) in [(-1.0f32, 9.0), (-0.75, 12.0), (-0.9, 15.0)].iter().enumerate() {
        let q = seg(f, *f0, f0 + 5.0);
        if q > 0.0 && q < 1.0 {
            let c = if k == 2 { 0xF4F4F0 } else { 0xFFDD00 };
            s.push(Prim::arc(0.0, 0.0, 0.52 - 0.08 * k as f32, 0.012, 0.18).rot(*a0 + 0.6 * q).col(c));
        }
    }
    // Outer yellow cover closing in and opening again.
    let ry = kf(f, &[(16.0, 1.3), (20.0, 0.95), (24.0, 0.75), (28.0, 0.62), (32.0, 0.62), (36.0, 0.8), (40.0, 1.1)]);
    if ry < 1.1 {
        s.push(annulus(0.0, 0.0, ry, 2.2).col(0xFFE000));
    }
    // White ring.
    let wr = kf(f, &[(17.0, 0.0), (19.0, 0.42), (25.0, 0.55), (31.0, 0.6), (35.0, 0.72), (39.0, 0.8)]);
    let ww = kf(f, &[(17.0, 0.0), (19.0, 0.04), (25.0, 0.12), (31.0, 0.12), (35.0, 0.06), (39.0, 0.0)]);
    if ww > 0.0 {
        if f < 24.0 {
            s.push(Prim::arc(0.0, 0.0, wr, ww, 1.1 + 0.08 * (f - 17.0)).rot(PI).col(0xF2F5EE));
        } else {
            s.push(Prim::circle(0.0, 0.0, wr).stroke(ww).col(0xF2F5EE));
        }
    }
    // Window onto the new clip with its yellow ring.
    let rw = kf(f, &[(14.0, 0.0), (16.0, 0.05), (18.0, 0.15), (20.0, 0.25), (22.0, 0.31), (24.0, 0.36)]);
    if !cut && rw > 0.0 {
        let yw = kf(f, &[(14.0, 0.05), (18.0, 0.07), (22.0, 0.03), (26.0, 0.015)]);
        s.push(Prim::circle(0.0, 0.0, rw + yw).col(0xFFDD00));
        s.push(Prim::circle(0.0, 0.0, rw).tex_b());
    }
    // Orange ring: arcs that close into a ring, thicken after the cut and thin out.
    let orr = kf(f, &[(19.0, 0.3), (24.0, 0.34), (28.0, 0.4), (34.0, 0.4), (40.0, 0.44), (46.0, 0.46)]);
    let ow = kf(f, &[(19.0, 0.015), (24.0, 0.04), (28.0, 0.07), (34.0, 0.1), (37.0, 0.13), (40.0, 0.14), (43.0, 0.03), (47.0, 0.004), (48.0, 0.0)]);
    if f > 19.0 && ow > 0.0 {
        if f < 27.0 {
            let span = 0.9 + 0.28 * (f - 19.0);
            s.push(Prim::arc(0.0, 0.0, orr, ow, span).rot(-0.6 + 0.15 * (f - 19.0)).col(0xF7900A));
        } else {
            s.push(Prim::circle(0.0, 0.0, orr + 0.5 * ow).stroke(ow).col(0xF7900A));
        }
    }
    s
}

/// Squares: orange, yellow and white square outlines spin out of the
/// centre around a turning square window onto the new clip that grows to
/// fill the frame.
fn spinning_square_frames(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    let sq = |h: f32, rot: f32| Prim::rect(0.0, 0.0, h, h).rot(rot);
    // Orange frame.
    let ho = kf(f, &[(8.0, 0.0), (10.0, 0.17), (12.0, 0.28), (14.0, 0.36), (16.0, 0.4), (20.0, 0.45), (24.0, 0.5), (28.0, 0.6), (32.0, 0.8), (35.0, 1.1)]);
    let ro = kf(f, &[(8.0, 0.2), (19.0, PI / 4.0), (24.0, 0.95), (35.0, 1.3)]);
    if ho > 0.0 && f < 35.0 {
        s.push(sq(ho, ro).stroke(0.035 + 0.02 * seg(f, 10.0, 24.0)).col(0xF7880A));
    }
    // Yellow frame.
    let hy = kf(f, &[(13.0, 0.0), (14.0, 0.08), (19.0, 0.2), (22.0, 0.32), (25.0, 0.4), (28.0, 0.46), (31.0, 0.53), (34.0, 0.62), (37.0, 0.74), (40.0, 0.88), (42.0, 1.0)]);
    let ry = kf(f, &[(13.0, 0.1), (22.0, 0.3), (25.0, 0.6), (28.0, PI / 4.0), (31.0, 0.9), (34.0, 1.0), (40.0, 1.25)]);
    if hy > 0.0 && f < 42.0 {
        s.push(sq(hy, ry).stroke(0.04).col(0xFFD800));
    }
    // Window onto the new clip and its white border.
    let h = kf(f, &[(16.0, 0.0), (18.0, 0.04), (20.0, 0.08), (22.0, 0.17), (24.0, 0.26), (26.0, 0.32), (28.0, 0.36), (31.0, 0.4), (34.0, 0.5), (37.0, 0.62), (40.0, 0.76), (43.0, 0.95)]);
    let r = kf(f, &[(18.0, 0.0), (22.0, -0.1), (25.0, -0.5), (28.0, -PI / 4.0), (31.0, -PI / 4.0), (34.0, -0.35), (37.0, -0.2), (40.0, -0.08), (43.0, -0.04)]);
    if f >= 43.0 {
        s.mix = 1.0;
        let q = seg(f, 43.0, 47.0);
        if q < 1.0 {
            for x in [-1.0f32, 1.0] {
                s.push(Prim::rect(x * (0.75 + 0.3 * q), 0.0, 0.025, 0.6).rot(0.1).col(0xF2F5EE));
            }
        }
    } else if h > 0.0 {
        s.push(sq(h + 0.03, r + 0.04).col(0xF2F5EE));
        s.push(sq(h, r).tex_b());
    }
    s
}

/// A band of the frame between slanted coordinates x' = x + k·y in
/// [lo, hi], as a rotated rectangle (local y along the band).
fn slant_band(lo: f32, hi: f32, k: f32) -> Prim {
    let n = (1.0 + k * k).sqrt();
    Prim::rect(0.5 * (lo + hi), 0.0, 0.5 * (hi - lo) / n, 0.8).rot(k.atan2(1.0))
}

/// Glossy black slabs slide in diagonally from the left until they cover
/// the frame, then slide on to the right uncovering the new clip, with thin
/// coloured light lines along their edges.
fn glossy_slab_sweep(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    let k: f32 = 0.55;
    let span = 1.18;
    let cover = kf(f, &[(3.0, 0.0), (6.0, 0.02), (8.0, 0.04), (10.0, 0.1), (12.0, 0.16), (14.0, 0.29), (16.0, 0.39), (18.0, 0.57), (20.0, 0.68), (22.0, 0.82), (24.0, 0.9), (26.0, 0.96), (28.0, 1.0)]);
    let reveal = kf(f, &[(30.0, 0.0), (32.0, 0.03), (34.0, 0.07), (36.0, 0.15), (38.0, 0.22), (40.0, 0.34), (42.0, 0.42), (44.0, 0.56), (46.0, 0.65), (48.0, 0.78), (50.0, 0.85), (52.0, 0.93), (54.0, 0.96), (56.0, 0.99), (58.0, 1.0)]);
    if reveal >= 1.0 {
        s.mix = 1.0;
        return s;
    }
    let to_x = |c: f32| -span + 2.0 * span * c;
    // Slots of varying width across the slanted axis.
    let widths = [0.16, 0.24, 0.12, 0.3, 0.18, 0.26, 0.14, 0.22, 0.32, 0.2, 0.17, 0.25];
    let total: f32 = widths.iter().sum();
    let mut lo = -span - 0.05;
    for (i, w) in widths.iter().enumerate() {
        let hi = lo + w * (2.0 * span + 0.1) / total;
        let j = 0.12 * (rnd(i as u32, 11) - 0.5);
        let c = if cover >= 1.0 { 9.0 } else { to_x(cover) + j };
        let r = if reveal <= 0.0 { -9.0 } else { to_x(reveal) + 0.8 * j };
        let (d0, d1) = (lo.max(r), hi.min(c));
        if d1 > d0 {
            let shade = [0x4A4D5C, 0x2E3038, 0x5A5C68, 0x3A3C46][i % 4];
            let hw = 0.5 * (d1 - d0) / (1.0 + k * k).sqrt();
            s.push(slant_band(d0, d1, k).col(0x060609).gradx(shade, -hw, hw * 0.8));
            // Light line on the band's leading edge.
            if i % 3 != 1 {
                let lc = [0xC23A4A, 0x8A4AD0, 0xE0C030][i % 3];
                s.push(slant_band(d1 - 0.012, d1, k).col(lc).alpha(0.8));
            }
        }
        let b1 = hi.min(r);
        if b1 > lo {
            s.push(slant_band(lo - 0.002, b1, k).tex_b());
        }
        lo = hi;
    }
    s
}

macro_rules! item {
    ($id:literal, $name:literal, $cat:literal, $desc:literal, $f:ident) => {
        ArtItem { id: $id, name: $name, category: $cat, description: $desc, scene: $f }
    };
}

pub static ITEMS: &[ArtItem] = &[
    item!("jb.diagonal_streak_sweep", "Diagonal Streak Sweep", "Shapes", "Blue and teal capsule streaks shoot diagonally across the frame from both corners, cover it and fly on, leaving the new clip behind them.", diagonal_streak_sweep),
    item!("jb.red_x_panels", "Red X Panels", "Shapes", "Four red panels slide in along the diagonals and lock into a red screen with an X seam, then slide back out along the X to uncover the new clip.", red_x_panels),
    item!("jb.square_frames_zoom", "Square Frames Zoom", "Shapes", "A tilted square window onto the new clip grows out of the frame inside blue and white square outlines that spin past until it fills the screen.", square_frames_zoom),
    item!("jb.red_ring_iris", "Red Ring Iris", "Shapes", "A round window onto the new clip opens at the left inside two red rings and grows until it covers the frame.", red_ring_iris),
    item!("jb.white_ring_iris", "White Ring Iris", "Shapes", "A round window onto the new clip opens at the centre inside a wide white ring and grows to fill the frame while orange rings drift out of the corners.", white_ring_iris),
    item!("jb.orange_ring_rise", "Orange Ring Rise", "Shapes", "A dome of the new clip rises from the bottom inside an orange ring while a small ringed window opens at the top right, until the dome fills the frame.", orange_ring_rise),
    item!("jb.hex_windows", "Hexagon Windows", "Shapes", "Hexagonal windows onto the new clip open inside green and blue outlines, grow and merge until they cover the frame.", hex_windows),
    item!("jb.chevron_trio_right", "Chevron Trio Right", "Shapes", "Orange, white and yellow chevrons sweep right across the frame; the new clip follows the white one.", chevron_trio_right),
    item!("jb.ring_burst_center", "Ring Burst Center", "Shapes", "A round window onto the new clip opens at the centre while yellow, white and orange rings close in from the edges; the frame cuts to the new clip and an orange ring thins out.", ring_burst_center),
    item!("jb.spinning_square_frames", "Spinning Square Frames", "Shapes", "Orange, yellow and white square outlines spin out of the centre around a turning square window onto the new clip that grows to fill the frame.", spinning_square_frames),
    item!("jb.glossy_slab_sweep", "Glossy Slab Sweep", "Shapes", "Glossy black slabs slide in diagonally until they cover the frame, then slide on to the right uncovering the new clip.", glossy_slab_sweep),
    item!("jb.corner_wedge_close", "Corner Wedge Close", "Shapes", "Two wedges of the new clip slide in from opposite corners along a diagonal, led by thin white edges, and meet in the middle while thin lines fan out from the seam.", corner_wedge_close),
    item!("jb.sparkle_heart_swell", "Sparkle Heart Swell", "Shapes", "A glossy pink heart swells out of the centre with a burst of sparkles until it covers the frame; the clips cross under it and it fades away over the new clip.", sparkle_heart_swell),
    item!("jb.memphis_disc_roll", "Memphis Disc Roll", "Shapes", "A big mint disc with an orange ring, spinning white arcs and dark orbit lines rolls in from the left, idles and rolls out to the right, the new clip following its centre line.", memphis_disc_roll),
    item!("jb.chevron_band_sweep", "Chevron Band Sweep", "Shapes", "Rounded chevron bands sweep from left to right: navy, two dimmed glimpses of the new clip and white, followed by the new clip.", chevron_band_sweep),
    item!("jb.arrowhead_slots", "Arrowhead Slots", "Shapes", "The frame cuts to black and four arrow-head windows onto the new clip dart in from the left one after another into their slots, then the new clip fills the gaps.", arrowhead_slots),
    item!("jb.capsule_column_run", "Capsule Column Run", "Shapes", "Seven columns of yellow, blue and purple capsules run through the frame, alternately down and up, and their long purple tails pull through to leave the new clip behind.", capsule_column_run),
    item!("jb.pinwheel_wedges", "Pinwheel Wedges", "Shapes", "Four pinwheel wedges of the new clip fan open from the centre until they meet.", pinwheel_wedges),
    item!("jb.card_carousel", "Card Carousel", "Shapes", "The old clip shrinks into a card on a carousel of alternating old and new cards over black; the carousel spins and the new card that stops in front grows to fill the frame.", card_carousel),
    item!("jb.red_streak_lanes", "Red Streak Lanes", "Sports", "Motion-blurred red paint streaks shoot across the frame from the left in horizontal lanes, and the new clip follows their tails.", red_streak_lanes),
    item!("jb.photo_desk_pan", "Photo Desk Pan", "Themed", "The camera pans and turns across a paper desk from a print of the old clip to a print of the new clip, past a red string tied into a heart, until the new print fills the frame.", photo_desk_pan),
    item!("jb.cloud_puff_cover", "Cloud Puff Cover", "Cartoon", "Cartoon cloud puffs pop in from the edges until they fill the frame, hold, then pop away from a gap near the top to uncover the new clip.", cloud_puff_cover),
    item!("jb.fireball_swirl", "Fireball Swirl", "Cartoon", "A cartoon fireball swells out of the centre until it fills the frame, then swirls open into spinning orange and yellow rings that break up over the new clip.", fireball_swirl),
    item!("jb.honeycomb_pop", "Honeycomb Pop", "Shapes", "Hexagonal windows onto the new clip pop open at random over a honeycomb and grow until only thin seams of the old clip remain, then close up.", honeycomb_pop),
    item!("jb.vertical_cutouts", "Vertical Cutouts", "Shapes", "Five vertical cut-outs of the new clip pop up one after another from the centre outwards, stretch and merge into one card that grows to fill the frame.", vertical_cutouts),
    item!("jb.rising_thumb_row", "Rising Thumb Row", "Shapes", "The frame cuts to black and three thumbnail cards rise from the bottom one after another into a row; the new card in the middle grows to fill the frame.", rising_thumb_row),
    item!("jb.card_ring_turn", "Card Ring Turn", "Shapes", "The old clip shrinks into one card of a ring of eight square cards over black; the ring turns an eighth and a new card slides out and grows to fill the frame.", card_ring_turn),
];

/// Library listing (registry ids, display order).
pub const LIB_IDS: &[&str] = &["jb.diagonal_streak_sweep", "jb.red_x_panels", "jb.square_frames_zoom", "jb.red_ring_iris", "jb.white_ring_iris", "jb.orange_ring_rise", "jb.hex_windows", "jb.chevron_trio_right", "jb.ring_burst_center", "jb.spinning_square_frames", "jb.glossy_slab_sweep", "jb.corner_wedge_close", "jb.sparkle_heart_swell", "jb.memphis_disc_roll", "jb.chevron_band_sweep", "jb.arrowhead_slots", "jb.capsule_column_run", "jb.pinwheel_wedges", "jb.card_carousel", "jb.red_streak_lanes", "jb.photo_desk_pan", "jb.cloud_puff_cover", "jb.fireball_swirl", "jb.honeycomb_pop", "jb.vertical_cutouts", "jb.rising_thumb_row", "jb.card_ring_turn"];
