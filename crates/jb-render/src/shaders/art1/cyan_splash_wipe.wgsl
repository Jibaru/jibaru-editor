// Cyan Splash Wipe: jagged strokes of flat ice-blue paint slash in from the
// lower left until they cover the frame, then tear away towards the lower
// right in diagonal streaks to uncover the new clip. Coverage per frame was
// measured on Filmora's export of Ice Firework Effect Vol 02 Transition 10.
//@p0 cover-in threshold, uncover threshold
//@ 0.0000 -0.935 -9.000
//@ 0.2167 -0.935 -9.000
//@ 0.2333 -0.703 -9.000
//@ 0.2500 -0.701 -9.000
//@ 0.2667 -0.598 -9.000
//@ 0.2833 -0.505 -9.000
//@ 0.3000 -0.320 -9.000
//@ 0.3167 -0.161 -9.000
//@ 0.3333 0.048 -9.000
//@ 0.3500 0.048 -9.000
//@ 0.3667 0.247 -9.000
//@ 0.3833 0.472 -9.000
//@ 0.4000 0.742 -9.000
//@ 0.4167 9.000 -9.000
//@ 0.5666 9.000 -9.000
//@ 0.5667 9.000 -0.501
//@ 0.5833 9.000 -0.412
//@ 0.6000 9.000 -0.316
//@ 0.6167 9.000 -0.207
//@ 0.6333 9.000 -0.068
//@ 0.6500 9.000 -0.068
//@ 0.6667 9.000 0.077
//@ 0.6833 9.000 0.184
//@ 0.7000 9.000 0.184
//@ 0.7167 9.000 0.436
//@ 0.7333 9.000 0.535
//@ 0.7500 9.000 0.535
//@ 0.7667 9.000 0.589
//@ 0.7833 9.000 0.639
//@ 0.8000 9.000 0.708
//@ 0.8167 9.000 0.750
//@ 0.8333 9.000 0.862
//@ 0.9833 9.000 0.862

fn fx(uv: vec2f) -> vec4f {
    let p = to_c(uv);
    let k = u.p[0];
    let sd = vec2f(0.87, -0.5);
    let fin = a1_stroke_field(p, vec2f(0.6, -0.8), sd, 4.0, 5.0, 1.2);
    let fout = a1_stroke_field(p, vec2f(0.5, 0.866), sd, 4.0, 5.0, 1.2);
    let e = 1.5 * u.inv_size.y;
    let cover = smoothstep(k.x + e, k.x - e, fin) * smoothstep(k.y - e, k.y + e, fout);
    let uncovering = k.y > -5.0;
    var col = select(a1_da(uv), a1_db(uv), uncovering);
    col = mix(col, vec3f(0.0, 0.737, 0.996), cover);
    return a1_out(col);
}
