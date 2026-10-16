// Title pass: draws background, shadow, stroke and fill from a signed
// distance field (A, distance in pixels, negative inside the glyphs).
//   e0 = (field origin x, y) in target pixels
//   e1 = (stroke width px, shadow dx, shadow dy, shadow blur px)
//   e2 = background box (x0, y0, x1, y1) in target pixels
//   e3 = (has stroke, has shadow, has background, background padding px)
//   p[0] fill, p[1] stroke, p[2] shadow, p[3] background (linear RGBA)

fn field(px: vec2f) -> f32 {
    let uv = (px - u.e0.xy) / u.a_size;
    if (!in01(uv)) { return 1.0e4; }
    return sa(uv).r;
}

@fragment
fn fs_title(in: VsOut) -> @location(0) vec4f {
    let p = in.uv * u.size;
    let d = field(p);
    var col = vec4f(0.0);
    if (u.e3.z > 0.5) {
        let pad = u.e3.w;
        let c = 0.5 * (u.e2.xy + u.e2.zw);
        let hs = 0.5 * (u.e2.zw - u.e2.xy) + vec2f(pad);
        let r = pad * 0.6;
        let q = abs(p - c) - hs + vec2f(r);
        let sd = length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
        col = over(lin_color(u.p[3]) * clamp(0.5 - sd, 0.0, 1.0), col);
    }
    let stroke = select(0.0, u.e1.x, u.e3.x > 0.5);
    if (u.e3.y > 0.5) {
        let ds = field(p - u.e1.yz) - stroke;
        let b = max(u.e1.w, 0.5);
        let a = 1.0 - smoothstep(-b, b, ds);
        col = over(lin_color(u.p[2]) * a, col);
    }
    if (u.e3.x > 0.5) {
        col = over(lin_color(u.p[1]) * clamp(0.5 - (d - stroke), 0.0, 1.0), col);
    }
    col = over(lin_color(u.p[0]) * clamp(0.5 - d, 0.0, 1.0), col);
    return col;
}
