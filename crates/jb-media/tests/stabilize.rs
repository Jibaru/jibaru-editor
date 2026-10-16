//! Stabilization analysis on a synthetic shaky clip.

mod common;

use jb_core::TICKS_PER_SECOND;

#[test]
fn corrections_follow_camera_shake() {
    // A 480x270 window moving over a still test pattern: the crop offset jitters
    // by ±12 px horizontally and ±6 px vertically from frame to frame.
    let path = common::dir().join("stabilize-shaky2.mp4");
    if !path.exists() {
        let tmp = common::out("stabilize-shaky2.mp4");
        common::ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=25:duration=2",
            "-vf",
            "trim=end_frame=1,loop=loop=49:size=1,setpts=N/25/TB,crop=480:270:x='80+12*sin(n*1.7)':y='45+6*cos(n*1.1)',format=yuv420p",
            "-c:v",
            "mpeg4",
            "-q:v",
            "2",
            tmp.to_str().unwrap(),
        ]);
        std::fs::rename(&tmp, &path).unwrap();
    }
    let times: Vec<i64> = (0..50).map(|i| i * TICKS_PER_SECOND / 25).collect();
    let c = jb_media::stabilize::analyze(&path, &times, 100.0).unwrap();
    assert_eq!(c.len(), 50);
    // The crop window moves right by x(n) so the content moves left: the
    // correction moves it right again, by about x(n) - mean.
    let expect = |n: f64| (12.0 * (n * 1.7).sin() / 480.0, 6.0 * (n * 1.1).cos() / 270.0);
    let (mut ex, mut ey) = (0.0f64, 0.0f64);
    for (n, v) in c.iter().enumerate().skip(5).take(40) {
        let (x, y) = expect(n as f64);
        ex = ex.max((v[0] - x).abs());
        ey = ey.max((v[1] - y).abs());
    }
    assert!(ex < 0.008 && ey < 0.008, "max error {ex:.4} {ey:.4}: {c:?}");
}
