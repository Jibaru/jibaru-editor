// Comic burst: a pale paper blob balloons out over the outgoing clip and
// fills the frame, then the incoming clip bursts through it from the
// centre as a spiky ink-splat hole that blows the paper apart into
// shards with shaded edges. Table `cb`: [p, blob radius, hole size, -, -].

const CB_PAPER: vec3f = vec3f(0.820, 0.867, 0.878);
const CB_SHADE: vec3f = vec3f(0.69, 0.75, 0.80);
const CB_N: f32 = 10.0;

// Spiky radial profile of the splat (1 at the spike tips).
fn cb_prof(a: f32) -> f32 {
    let x = (a / TAU + 0.5) * CB_N;
    let i = floor(x);
    let f = x - i;
    let amp = 0.45 + 0.55 * fract(sin((i * 1.37 + 0.4) * 127.1 + 311.7) * 43758.5453);
    let tri = 1.0 - abs(f - 0.5) * 2.0;
    return 0.25 + amp * pow(tri, 1.5);
}

fn fx(uv: vec2f) -> vec4f {
    let k = kf_cb(prog());
    let c = to_c(uv);
    let r = length(c);
    let a = atan2(c.y, c.x);
    let px = ak_px();
    var col = l3_da(uv);
    // Balloon blob (slightly wider than tall, wobbling edge).
    let wob = 1.0 + 0.04 * sin(a * 5.0 + 1.0);
    let db = length(c / vec2f(1.15, 1.0)) - k.x * wob;
    col = mix(col, CB_PAPER, clamp(0.5 - db / px, 0.0, 1.0));
    if (k.y > 0.0) {
        // Splat hole: the spikes are curved slightly so they read as ink.
        let a2 = a + 0.12 * sin(r * 6.0);
        let edge = k.y * cb_prof(a2);
        let dh = r - edge;
        let paper = clamp(0.5 - (-dh) / px, 0.0, 1.0);
        var pc = mix(CB_SHADE, CB_PAPER, smoothstep(0.0, 0.06 + 0.04 * k.y, dh));
        col = mix(l3_db(uv), pc, paper * step(0.0, k.x));
    }
    return l3_out(col);
}
