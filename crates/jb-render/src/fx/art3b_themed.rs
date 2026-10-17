//! Themed art3b items (horror, holiday, sports, business, cartoon): scene
//! functions with our own drawings. Progress `t` runs 0..1 over the
//! transition (60 frames at Filmora's default 2.002 s); `f = 60 t`.

use super::kit::*;

/// Cartoon eye: white ball, coloured iris, pupil and a highlight.
pub fn eye(s: &mut Scene, x: f32, y: f32, r: f32, iris: u32, iris_dark: u32) {
    s.push(Prim::circle(x, y, r).col(0xF6F6F2).radial(0xFFFFFF, r));
    s.push(Prim::circle(x + 0.1 * r, y, 0.62 * r).col(iris_dark).radial(iris, 0.62 * r));
    s.push(Prim::circle(x + 0.1 * r, y, 0.28 * r).col(0x050505));
    s.push(Prim::circle(x - 0.12 * r, y - 0.22 * r, 0.12 * r).col(0xFFFFFF).alpha(0.9));
}

/// Mummy wrap: cloth bandages slide in from both sides until they wrap the
/// frame, a slit opens with a pair of green eyes peering through onto the
/// new clip, the view pushes into the slit and the wrapping falls away.
pub fn bandage_wrap(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    if f >= 57.0 {
        s.mix = 1.0;
        return s;
    }
    // Push into the slit and the wrapping falling away.
    let z = 1.0 + 1.8 * ease_in2(seg(f, 46.0, 54.0));
    let (ox, oy) = (0.0, 0.03);
    let fall = ease_in2(seg(f, 51.0, 56.5));
    let (fx, fy) = (-1.7 * fall, 0.9 * fall);
    let tr = |p: Prim| {
        let p = p.zoom(z, ox, oy);
        Prim { v: { let mut v = p.v; v[1][0] += fx; v[1][1] += fy; v } }
    };
    if fall > 0.0 {
        s.mix = 1.0;
    }
    let n = 24;
    let band = |i: u32, f: f32| -> Option<Prim> {
        let r = |k: u32| rnd(i, 40 + k);
        let y = -0.56 + 1.12 * (i as f32 + 0.5) / n as f32 + 0.02 * (r(1) - 0.5);
        let a = 0.2 * (r(2) - 0.5);
        let hh = 0.032 + 0.018 * r(3);
        let side = if r(4) < 0.5 { -1.0 } else { 1.0 };
        // Bands nearest the slit arrive last.
        let t0 = 9.0 + 7.0 * r(5) + 2.0 * (1.0 - (y.abs() * 2.0).min(1.0));
        let q = ease_out(seg(f, t0, t0 + 5.0));
        if q <= 0.0 {
            return None;
        }
        let x = side * 2.4 * (1.0 - q);
        Some(Prim::rect(x, y, 1.25, hh).rot(a).round(0.4 * hh).col(0xCBC1B0).noise(0xA89D8B, 26.0, r(6) * 9.0).seed(r(7) * 50.0))
    };
    let shadow = |p: Prim| {
        let mut q = p;
        q.v[0][2] += 0.006;
        q.v[1][1] += 0.004;
        q.v[3][0] = 0.0;
        q.rgb(hexc(0x7D7363)).alpha(0.9)
    };
    // Back layer of bands.
    for i in 0..n {
        if let Some(b) = band(i, f) {
            s.push(tr(shadow(b)));
            s.push(tr(b));
        }
    }
    // Slit onto the new clip with the eyes.
    let open = kf(f, &[(31.0, 0.0), (35.0, 0.022), (44.0, 0.03), (47.0, 0.05), (50.0, 0.11), (53.0, 0.2)]);
    let rx = kf(f, &[(31.0, 0.05), (35.0, 0.36), (44.0, 0.42), (50.0, 0.55)]);
    if open > 0.0 && fall < 1.0 {
        s.push(tr(Prim::ellipse(0.0, 0.0, rx, open).tex_b()));
        let er = 0.042;
        for ex in [-0.11f32, 0.11] {
            let mut e = Scene::new();
            eye(&mut e, ex, -0.005 + 0.3 * er, er, 0x46D45A, 0x0E7A22);
            for p in e.prims {
                s.push(tr(p));
            }
        }
        // Lids: a thick ring of cloth around the slit hides the eyes' edges.
        let lid = 0.16;
        s.push(tr(Prim::ellipse(0.0, 0.0, rx + 0.5 * lid, open + 0.5 * lid).stroke(lid).col(0xCBC1B0).noise(0xA89D8B, 26.0, 3.0)));
        // Bands crossing over the lids.
        for (k, (y, a)) in [(-0.05f32, 0.05f32), (0.055, -0.04), (-0.085, -0.03), (0.09, 0.06)].iter().enumerate() {
            let hh = 0.03;
            let b = Prim::rect(0.0, *y + (y.signum() * (open - 0.022)), 1.25, hh).rot(*a).round(0.01).col(0xCBC1B0).noise(0xA89D8B, 26.0, k as f32 * 2.0);
            s.push(tr(shadow(b)));
            s.push(tr(b));
        }
        // Fangs hanging from the cloth under the eyes.
        for fxp in [-0.07f32, 0.08] {
            s.push(tr(Prim::star(fxp, 0.075 + open, 0.014, 3.0, 0.2).rot(PI / 2.0).col(0xF4F4F0)));
        }
    }
    s
}

/// Ragged vertical edge at `x` from y0 to y1: overlapping round splats so a
/// rectangle of paint reads as torn or splashed.
fn ragged_edge(s: &mut Scene, x: f32, y0: f32, y1: f32, r: f32, salt: u32, col: u32, alpha: f32) {
    let n = ((y1 - y0) / (0.9 * r)).ceil() as u32 + 1;
    for i in 0..n {
        let y = y0 + (y1 - y0) * i as f32 / (n - 1).max(1) as f32;
        let rr = r * (0.6 + 0.8 * rnd(i, salt));
        let dx = r * (rnd(i, salt + 1) - 0.5);
        s.push(Prim::blob(x + dx, y, rr, 0.18, 3.0, rnd(i, salt + 2) * 20.0).col(col).alpha(alpha));
    }
}

/// Blood splash: a spiky splat bursts out of the left of the frame until it
/// covers it, then drains away from the left onto the new clip.
pub fn blood_splash(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::cut(t, 37.0 / 60.0);
    let red = 0x850503;
    let r = kf(f, &[(12.0, 0.0), (14.0, 0.3), (16.0, 0.62), (18.0, 0.95), (20.0, 1.3), (22.0, 1.7)]);
    let edge = kf(f, &[(37.0, -1.0), (40.0, -0.62), (42.0, -0.45), (44.0, 0.05), (46.0, 0.45), (48.0, 1.1)]);
    let fade = 1.0 - 0.5 * seg(f, 42.0, 48.0);
    if f < 37.0 {
        if r > 0.0 {
            let (cx, cy) = (-0.32, 0.0);
            s.push(Prim::burst(cx, cy, r, 44.0, 0.5, 0.55, 3.0).rot(0.2 * f / 60.0).col(red).radial(0x6E0201, r));
            s.push(Prim::blob(cx, cy, 0.62 * r, 0.12, 4.0, 1.0).col(red));
            // Droplets thrown ahead of the splat.
            for i in 0..14u32 {
                let a = TAU * rnd(i, 3);
                let d = r * (1.05 + 0.35 * rnd(i, 4));
                let dr = 0.012 + 0.03 * rnd(i, 5);
                s.push(Prim::circle(cx + a.cos() * d, cy + a.sin() * d, dr * (0.5 + r)).col(red));
            }
        }
    } else if edge < 1.05 {
        s.push(Prim::rect_xy(edge, -0.6, 1.2, 0.6).col(red).alpha(fade));
        ragged_edge(&mut s, edge, -0.55, 0.55, 0.07, 7, red, fade);
        // Left-behind spatters on the new clip.
        for i in 0..10u32 {
            let x = -0.85 + 1.7 * rnd(i, 8);
            if x < edge - 0.05 {
                let a = (1.0 - seg(edge - x, 0.0, 0.5)) * fade;
                s.push(Prim::blob(x, -0.45 + 0.9 * rnd(i, 9), 0.02 + 0.04 * rnd(i, 10), 0.3, 3.0, i as f32).col(red).alpha(a));
            }
        }
    }
    s
}

/// Rotting hand: palm, four fingers and a thumb on a forearm reaching up
/// from below; `(x, y)` is the palm centre, `k` the scale, `a` the tilt.
fn zombie_hand(s: &mut Scene, x: f32, y: f32, k: f32, a: f32, spread: f32) {
    let skin = 0x2A4A2C;
    let dark = 0x152A18;
    let (ca, sa) = (a.cos(), a.sin());
    let at = |lx: f32, ly: f32| (x + k * (lx * ca - ly * sa), y + k * (lx * sa + ly * ca));
    // Forearm down to the bottom of the frame.
    let (wx, wy) = at(0.0, 0.1);
    let (fx, fy) = at(0.03, 0.9);
    s.push(Prim::seg(wx, wy, fx, fy, 0.075 * k).col(skin).noise(dark, 30.0 / k, 1.0));
    // Fingers.
    let fingers = [(-0.075, 0.16, -0.25), (-0.025, 0.2, -0.08), (0.025, 0.19, 0.06), (0.07, 0.15, 0.22), (0.1, 0.1, 0.9)];
    for (i, (bx, len, ang)) in fingers.iter().enumerate() {
        let ang = ang * (1.0 + spread);
        let (x0, y0) = at(*bx, if i == 4 { 0.02 } else { -0.06 });
        let dir = a + ang - PI / 2.0;
        let (x1, y1) = (x0 + k * len * dir.cos(), y0 + k * len * dir.sin());
        s.push(Prim::seg(x0, y0, x1, y1, 0.02 * k).col(skin).noise(dark, 40.0 / k, i as f32));
        // Dark nail.
        s.push(Prim::circle(x1, y1, 0.012 * k).col(0x0B140C));
    }
    // Palm.
    let (px, py) = at(0.0, 0.0);
    s.push(Prim::rect(px, py, 0.085 * k, 0.09 * k).round(0.04 * k).rot(a).col(skin).noise(dark, 30.0 / k, 2.0));
    // Wound.
    let (gx, gy) = at(0.01, -0.01);
    s.push(Prim::blob(gx, gy, 0.02 * k, 0.3, 3.0, 4.0).col(0x5A0806));
}

/// Zombie hands: a rotting hand lunges up from the bottom left, slaps
/// bloody prints on the screen and drags a smear across it; the new clip
/// opens behind the smear.
pub fn zombie_hands(t: f32, _asp: f32) -> Scene {
    let f = t * 60.0;
    let mut s = Scene::new();
    if f >= 38.0 {
        s.mix = 1.0;
        return s;
    }
    let red = 0x8E0B08;
    // Wipe onto B and the smear ahead of it.
    let edge = kf(f, &[(24.0, -1.0), (26.0, -0.59), (30.0, -0.14), (34.0, 0.28), (38.0, 0.95)]);
    let front = kf(f, &[(13.0, -0.95), (16.0, -0.7), (20.0, -0.45), (24.0, 0.0), (28.0, 0.5), (32.0, 0.95), (36.0, 1.2)]);
    if front > edge {
        let a = 0.85;
        s.push(Prim::rect_xy(edge.max(-1.0), -0.42, front, 0.42).col(red).noise(0x5E0404, 9.0, 0.0).alpha(a));
        ragged_edge(&mut s, front, -0.4, 0.4, 0.08, 21, red, a);
        for yy in [-0.44f32, 0.44] {
            let hw = 0.5 * (front - edge.max(-1.0));
            if hw > 0.0 {
                s.push(Prim::rect(edge.max(-1.0) + hw, yy, hw, 0.05).col(red).alpha(0.6));
            }
        }
    }
    if edge > -1.0 {
        s.push(Prim::rect_xy(-1.0, -0.6, edge, 0.6).tex_b());
        ragged_edge(&mut s, edge, -0.55, 0.55, 0.05, 31, red, 0.5);
    }
    // Hand prints slapped on the screen.
    for (i, (px, py, f0)) in [(-0.62f32, 0.02f32, 15.0f32), (-0.4, -0.12, 19.0)].iter().enumerate() {
        if f > *f0 && px > &edge {
            let mut e = Scene::new();
            zombie_hand(&mut e, *px, *py, 0.9, -0.1 + 0.2 * i as f32, 0.5);
            for p in e.prims.into_iter().skip(1) {
                s.push(p.solid().rgb(hexc(red)).alpha(0.9));
            }
        }
    }
    // The hand: lunges in large, hovers, then sweeps right.
    let k = kf(f, &[(0.0, 2.6), (3.0, 1.4), (5.0, 1.15), (20.0, 1.1)]);
    let hx = kf(f, &[(0.0, -0.75), (3.0, -0.62), (14.0, -0.58), (18.0, -0.5), (22.0, 0.1), (26.0, 0.4), (30.0, 0.75), (34.0, 1.3)]);
    let hy = kf(f, &[(0.0, 0.0), (3.0, 0.08), (8.0, 0.06), (14.0, 0.1), (22.0, 0.12), (34.0, 0.14)]);
    let tilt = 0.15 * (f * 0.5).sin() - 0.1;
    if hx < 1.25 && f > 0.5 {
        zombie_hand(&mut s, hx, hy, k * 0.95, tilt, 0.3);
    }
    s
}
