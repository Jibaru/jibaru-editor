//! Dissolves, fades and colour washes fitted frame by frame on Filmora's
//! exports (docs/fidelity-lane3.md): each is a mix of the two clips whose
//! weight, grade or wash colour follows keyframe tables over progress.

use super::data as D;
use super::kit::{global_mix, register_items, table, Item};
use crate::registry::Registry;

/// Sweeping dissolve: weight of B (display space) = `g(p)` times a soft
/// left-to-right edge at `x = 0.047 + 1.4 p` (0.475 wide, smoothstep), so the
/// left side dissolves first and the right catches up. `g` rows `[p, g]`.
const SWEEP_DISSOLVE_G: &[[f32; 2]] = &[
    [0.0, 0.0],
    [0.1, 0.023],
    [0.2167, 0.1172],
    [0.3667, 0.296],
    [0.5, 0.486],
    [0.6333, 0.6752],
    [0.75, 0.8214],
    [0.8667, 0.9318],
    [1.0, 1.0],
];

fn items() -> Vec<Item> {
    vec![
        Item {
            id: "jb.sweep_dissolve",
            name: "Sweeping Dissolve",
            category: "Dissolve",
            description:
                "Cross-dissolve that starts on the left edge and sweeps right: a soft wide edge travels across \
                      the frame while the overall mix eases in. Fitted on Filmora's export of Fade Dissolve.",
            params: vec![],
            wgsl: format!(
                r#"{}
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let front = 0.047 + 1.4 * p;
    let w = kf_g(p).x * (1.0 - smoothstep(front - 0.2375, front + 0.2375, uv.x));
    return l3_mix(sa(uv), sb(uv), w);
}}
"#,
                table("g", SWEEP_DISSOLVE_G)
            ),
            shift: true,
        },
        Item::new(
            "jb.wash_white",
            "Wash to White",
            "Basic",
            "The clip washes linearly to pure white at the cut and the next clip emerges from it at the same \
         rate. Fitted on Filmora's export of Fade White.",
            global_mix(D::WASH_WHITE, "", ""),
        ),
        Item::new(
            "jb.wash_black",
            "Wash to Black",
            "Basic",
            "The clip darkens to black, holding black for a frame around the cut, and the next clip rises out \
         of it. Fitted on Filmora's export of Fade Single Track.",
            global_mix(D::WASH_BLACK, "", ""),
        ),
        Item::new(
            "jb.gray_fade",
            "Grayscale Dissolve",
            "Dissolve",
            "Both clips lose their colour towards the middle of a cross-dissolve: the picture passes through a \
         gray version of the mix and regains colour on the new clip. Fitted on Filmora's export of Fade \
         Grayscale.",
            global_mix(D::GRAY_FADE, "", ""),
        ),
    ]
}

pub fn register(r: &mut Registry) {
    register_items(r, items());
}
