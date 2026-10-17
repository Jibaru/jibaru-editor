// Generated from the lane-4 spot-color fit (see lane4_spot.rs); do not edit by hand.

pub static SPOTS: &[SpotSpec] = &[SpotSpec {
    id: "jb.l4.spot_color_jump",
    name: "Spot Color Jump",
    category: "Stylized",
    description: "Dark monochrome frame with a tinted color window that jumps around on a beat.",
    period: 2.0020,
    lut: include_bytes!("luts/spot_color_mono.lut"),
    rects: &[
            SpotRect { start: 0.0000, rect: [0.0896, 0.0907, 0.3875, 0.3093], m: [[0.9573, 0.1376, -0.1110, 4.2954], [-0.0103, 0.9035, 0.0787, 12.1228], [0.0008, -0.0303, 1.0206, 5.3562]] },
            SpotRect { start: 0.4338, rect: [0.6875, 0.2352, 0.8104, 0.7667], m: [[1.0302, 0.0221, -0.0327, 2.2685], [0.0533, 1.0069, -0.0376, -0.8108], [-0.3029, -0.2087, 1.4455, 41.0863]] },
            SpotRect { start: 0.8675, rect: [0.2000, 0.6407, 0.4979, 0.8593], m: [[0.9949, 0.1737, -0.2081, 16.9508], [-0.0176, 0.8045, 0.1064, 30.7770], [-0.0193, -0.2562, 1.0583, 67.6083]] },
            SpotRect { start: 1.2679, rect: [0.5344, 0.5944, 0.7667, 0.9833], m: [[0.9116, -0.0994, 0.0508, 37.0949], [-0.0538, 0.5904, 0.3241, 18.2765], [-0.1077, -0.2406, 1.2063, -6.8460]] },
            SpotRect { start: 1.7017, rect: [0.3833, 0.1444, 0.6167, 0.5537], m: [[0.8608, -0.2167, 0.2365, 47.1941], [-0.0979, 0.7681, 0.2480, 36.4679], [-0.0442, -0.1917, 1.2237, 15.4675]] },
    ],
}];
