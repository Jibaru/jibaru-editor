// Generated from the lane-4 spot-color fit (see lane4_spot.rs); do not edit by hand.

pub static SPOTS: &[SpotSpec] = &[SpotSpec {
    id: "jb.l4.spot_color_jump",
    name: "Spot Color Jump",
    category: "Stylized",
    description: "Dark monochrome frame with a tinted color window that jumps around on a beat.",
    period: 2.0020,
    lut: include_bytes!("luts/spot_color_mono.lut"),
    rects: &[
            SpotRect { start: 0.0000, rect: [0.0896, 0.0907, 0.3875, 0.3093], m: [[0.9841, 0.0747, -0.0600, -1.9973], [-0.0224, 0.9277, 0.0573, 13.0167], [-0.0216, 0.0174, 0.9796, 6.1890]] },
            SpotRect { start: 0.4338, rect: [0.6875, 0.2352, 0.8115, 0.7667], m: [[1.0303, 0.0191, -0.0299, -0.8483], [0.0503, 1.0096, -0.0394, -1.7590], [-0.3091, -0.2134, 1.4539, 38.1769]] },
            SpotRect { start: 0.8675, rect: [0.2010, 0.6407, 0.4979, 0.8593], m: [[0.9941, 0.1653, -0.2062, 15.0956], [-0.0182, 0.8083, 0.1026, 29.8435], [-0.0243, -0.2730, 1.0750, 65.0944]] },
            SpotRect { start: 1.2679, rect: [0.5344, 0.5944, 0.7667, 1.0000], m: [[0.9137, -0.1087, 0.0518, 34.9291], [-0.0434, 0.5752, 0.3232, 17.3859], [-0.0757, -0.2634, 1.1980, -9.2333]] },
            SpotRect { start: 1.7017, rect: [0.3833, 0.1444, 0.6167, 0.5537], m: [[0.8554, -0.2133, 0.2356, 44.7999], [-0.1069, 0.7883, 0.2299, 36.7401], [-0.0418, -0.2148, 1.2480, 11.0512]] },
    ],
}];
