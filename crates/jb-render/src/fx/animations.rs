//! Entry/exit animations (Filmora "Animation" tab). They only modify the
//! layer transform and opacity: "in" animations end exactly at the clip's own
//! transform at progress 1, "out" animations start from it at progress 0.
//! Offsets are normalized canvas units, scale multiplies, rotation is degrees.

use super::{info, AnimFx};
use crate::registry::{AnimState, FxKind::Animation, ParamSpec as P, ParamValues, Registry};

fn ease_out(t: f64) -> f64 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}
fn ease_in(t: f64) -> f64 {
    t.clamp(0.0, 1.0).powi(3)
}
fn smooth(t: f64) -> f64 {
    let x = t.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
fn back_out(t: f64, c1: f64) -> f64 {
    let x = t.clamp(0.0, 1.0) - 1.0;
    1.0 + (c1 + 1.0) * x * x * x + c1 * x * x
}
fn bounce_out(t: f64) -> f64 {
    let (n1, d1) = (7.5625, 2.75);
    let mut t = t.clamp(0.0, 1.0);
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        t -= 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        t -= 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        t -= 2.625 / d1;
        n1 * t * t + 0.984375
    }
}
fn seg(t: f64, a: f64, b: f64) -> f64 {
    ((t - a) / (b - a)).clamp(0.0, 1.0)
}
fn scale(s: &mut AnimState, k: f64) {
    s.scale[0] *= k;
    s.scale[1] *= k;
}

type Apply = fn(&mut AnimState, f32, &ParamValues);

fn add(r: &mut Registry, id: &'static str, name: &'static str, desc: &'static str, params: &[P], apply: Apply) {
    r.register(AnimFx { info: info(id, name, "Animation", Animation, desc, params), apply });
}

/// Slide directions: (dx, dy) of the off-screen side.
const DIRS: [(&str, f64, f64); 4] = [("left", -1.0, 0.0), ("right", 1.0, 0.0), ("up", 0.0, -1.0), ("down", 0.0, 1.0)];

fn slide_in(s: &mut AnimState, p: f32, dir: usize) {
    let (_, dx, dy) = DIRS[dir];
    let k = 1.0 - ease_out(p as f64);
    s.position[0] += dx * k;
    s.position[1] += dy * k;
}
fn slide_out(s: &mut AnimState, p: f32, dir: usize) {
    let (_, dx, dy) = DIRS[dir];
    let k = ease_in(p as f64);
    s.position[0] += dx * k;
    s.position[1] += dy * k;
}

pub fn register(r: &mut Registry) {
    add(r, "jb.anim.fade_in", "Fade In", "Opacity rises from 0 to the clip's opacity.", &[], |s, p, _| {
        s.opacity *= smooth(p as f64);
    });
    add(r, "jb.anim.fade_out", "Fade Out", "Opacity falls from the clip's opacity to 0.", &[], |s, p, _| {
        s.opacity *= 1.0 - smooth(p as f64);
    });
    add(
        r,
        "jb.anim.zoom_in",
        "Zoom In",
        "In: grows from small into place while fading in.",
        &[P::float("from", "Start scale", 0.0, 1.0, 0.3)],
        |s, p, v| {
            let f = v.get("from") as f64;
            scale(s, f + (1.0 - f) * ease_out(p as f64));
            s.opacity *= seg(p as f64, 0.0, 0.4);
        },
    );
    add(
        r,
        "jb.anim.zoom_out",
        "Zoom Out",
        "Out: shrinks away from its place while fading out.",
        &[P::float("to", "End scale", 0.0, 1.0, 0.3)],
        |s, p, v| {
            let t = v.get("to") as f64;
            scale(s, 1.0 + (t - 1.0) * ease_in(p as f64));
            s.opacity *= 1.0 - seg(p as f64, 0.6, 1.0);
        },
    );
    add(
        r,
        "jb.anim.zoom_out_in",
        "Zoom Out (In)",
        "In: starts enlarged and settles down to its size while fading in.",
        &[P::float("from", "Start scale", 1.0, 4.0, 1.6)],
        |s, p, v| {
            let f = v.get("from") as f64;
            scale(s, f + (1.0 - f) * ease_out(p as f64));
            s.opacity *= seg(p as f64, 0.0, 0.4);
        },
    );
    add(
        r,
        "jb.anim.zoom_in_out",
        "Zoom In (Out)",
        "Out: enlarges toward the viewer while fading out.",
        &[P::float("to", "End scale", 1.0, 4.0, 1.6)],
        |s, p, v| {
            let t = v.get("to") as f64;
            scale(s, 1.0 + (t - 1.0) * ease_in(p as f64));
            s.opacity *= 1.0 - seg(p as f64, 0.5, 1.0);
        },
    );
    add(r, "jb.anim.slide_in_left", "Slide In Left", "In: slides in from the left edge.", &[], |s, p, _| {
        slide_in(s, p, 0)
    });
    add(r, "jb.anim.slide_in_right", "Slide In Right", "In: slides in from the right edge.", &[], |s, p, _| {
        slide_in(s, p, 1)
    });
    add(r, "jb.anim.slide_in_up", "Slide In Up", "In: slides in from the top edge.", &[], |s, p, _| slide_in(s, p, 2));
    add(r, "jb.anim.slide_in_down", "Slide In Down", "In: slides in from the bottom edge.", &[], |s, p, _| {
        slide_in(s, p, 3)
    });
    add(r, "jb.anim.slide_out_left", "Slide Out Left", "Out: slides out through the left edge.", &[], |s, p, _| {
        slide_out(s, p, 0)
    });
    add(r, "jb.anim.slide_out_right", "Slide Out Right", "Out: slides out through the right edge.", &[], |s, p, _| {
        slide_out(s, p, 1)
    });
    add(r, "jb.anim.slide_out_up", "Slide Out Up", "Out: slides out through the top edge.", &[], |s, p, _| {
        slide_out(s, p, 2)
    });
    add(r, "jb.anim.slide_out_down", "Slide Out Down", "Out: slides out through the bottom edge.", &[], |s, p, _| {
        slide_out(s, p, 3)
    });
    add(
        r,
        "jb.anim.spin_in",
        "Spin In",
        "In: spins in while growing from nothing.",
        &[P::float("turns", "Turns", -4.0, 4.0, 1.0)],
        |s, p, v| {
            let k = ease_out(p as f64);
            s.rotation -= 360.0 * v.get("turns") as f64 * (1.0 - k);
            scale(s, k);
        },
    );
    add(
        r,
        "jb.anim.spin_out",
        "Spin Out",
        "Out: spins away while shrinking to nothing.",
        &[P::float("turns", "Turns", -4.0, 4.0, 1.0)],
        |s, p, v| {
            let k = ease_in(p as f64);
            s.rotation += 360.0 * v.get("turns") as f64 * k;
            scale(s, 1.0 - k);
        },
    );
    add(r, "jb.anim.bounce_in", "Bounce In", "In: drops from above and bounces into place.", &[], |s, p, _| {
        s.position[1] -= 1.0 - bounce_out(p as f64);
    });
    add(r, "jb.anim.bounce_out", "Bounce Out", "Out: a small hop up, then falls off the bottom.", &[], |s, p, _| {
        let p = p as f64;
        if p < 0.3 {
            s.position[1] -= 0.08 * (std::f64::consts::PI * p / 0.3 * 0.5).sin();
        } else {
            let k = seg(p, 0.3, 1.0);
            s.position[1] += -0.08 + 1.2 * k * k;
        }
    });
    add(
        r,
        "jb.anim.boom",
        "Boom",
        "In: punchy zoom that overshoots and settles, with a short motion blur.",
        &[],
        |s, p, _| {
            let p = p as f64;
            scale(s, back_out(p, 3.0).max(0.0));
            s.opacity *= seg(p, 0.0, 0.2);
            s.blur += 0.008 * (1.0 - seg(p, 0.0, 0.5));
        },
    );
    add(r, "jb.anim.pop_in", "Pop In", "In: pops up from nothing with a slight overshoot.", &[], |s, p, _| {
        scale(s, back_out(p as f64, 1.70158).max(0.0));
    });
    add(
        r,
        "jb.anim.drop_in",
        "Drop In",
        "In: falls in from above, slowing into place while fading in.",
        &[],
        |s, p, _| {
            let p = p as f64;
            s.position[1] -= 0.4 * (1.0 - ease_out(p));
            scale(s, 1.15 - 0.15 * ease_out(p));
            s.opacity *= seg(p, 0.0, 0.5);
        },
    );
    add(
        r,
        "jb.anim.ken_burns",
        "Ken Burns",
        "Slow zoom and pan across the whole range.",
        &[P::float("zoom", "Zoom", 0.0, 100.0, 15.0).unit("%"), P::vec2("pan", "Pan", -0.5, 0.5, [0.03, 0.0])],
        |s, p, v| {
            let k = smooth(p as f64);
            scale(s, 1.0 + v.get("zoom") as f64 / 100.0 * k);
            let pan = v.slot("pan").unwrap_or([0.0; 4]);
            s.position[0] += pan[0] as f64 * k;
            s.position[1] += pan[1] as f64 * k;
        },
    );
}
