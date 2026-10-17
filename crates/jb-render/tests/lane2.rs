//! Lane 2 fitted transitions: every item resolves through its Filmora GUID,
//! starts on A, ends on B, and never renders a broken (black) frame at any
//! fitted key (degenerate corner pins must not poison the blend).

mod common;

use jb_render::testing::*;

#[test]
fn lane2_aliases_resolve() {
    let reg = jb_render::Registry::builtin();
    for (fid, to) in jb_render::aliases_lane2::FILMORA_LANE2 {
        assert_eq!(reg.canonical(&format!("filmora:{fid}")), Some(*to), "{fid}");
    }
}

#[test]
fn lane2_items_render_every_key() {
    let Some(mut r) = common::renderer(64, 36) else { return };
    let p = project(64, 36);
    let src = MemorySource::ab(64, 36);
    let mean =
        |d: &[u8]| d.chunks_exact(4).map(|px| px[0] as f64 + px[1] as f64 + px[2] as f64).sum::<f64>() / d.len() as f64;
    let floor = mean(&src.frames["a"].data).min(mean(&src.frames["b"].data)) * 0.02;
    for table in jb_render::fx::lane2_layers::tables() {
        let id = table.id.as_str();
        let mut ps: Vec<f32> = table.layers.iter().flat_map(|l| l.keys.iter().map(|k| k.p)).collect();
        ps.retain(|p| (0.0..1.0).contains(p));
        ps.sort_by(f32::total_cmp);
        ps.dedup();
        for prog in ps {
            let out = r.render(&p, &transition_plan(id, prog as f64, &[]), &src);
            assert!(r.last_report().unknown_kinds.is_empty(), "{id}");
            if std::env::var_os("JB_LANE2_DEBUG").is_some() {
                eprintln!("{id} {prog:.4} mean {:.1}", mean(&out.data));
                if let Some(dir) = std::env::var_os("JB_LANE2_DUMP") {
                    let mut ppm = format!("P6 64 36 255\n").into_bytes();
                    ppm.extend(out.data.chunks_exact(4).flat_map(|px| [px[0], px[1], px[2]]));
                    std::fs::write(std::path::Path::new(&dir).join(format!("{id}_{prog:.4}.ppm")), ppm).unwrap();
                }
            }
            // Fully covered frames must not go black (NaN blends render as 0).
            let covered = table.eval(prog).iter().any(|(l, _)| l.edge == jb_render::fx::lane2_layers::Edge::Mirror);
            if covered {
                assert!(mean(&out.data) > floor, "{id} at {prog}: black frame");
            }
        }
        let start = r.render(&p, &transition_plan(id, 0.0, &[]), &src);
        let end = r.render(&p, &transition_plan(id, 0.999, &[]), &src);
        assert!(mean(&start.data) > floor && mean(&end.data) > floor, "{id}: black ends");
    }
}
