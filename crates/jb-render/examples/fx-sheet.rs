//! Renders one contact-sheet PNG per registered effect/transition: five
//! frames at progress 0, 0.25, 0.5, 0.75 and 1 from synthetic test patterns.
//!
//! cargo run -p jb-render --release --example fx-sheet -- [OUT_DIR] [FILTER] [--size WxH]

use jb_core::eval::FramePlan;
use jb_render::testing::*;
use jb_render::{FxKind, Renderer};
use std::io::BufWriter;

fn save(path: &str, w: u32, h: u32, data: &[u8]) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(data)?;
    Ok(())
}

/// Title styles over clip B: plain, stroke, shadow, background, multi-line.
fn write_titles(r: &mut Renderer, project: &jb_core::Project, src: &MemorySource, out_dir: &str, sheet_w: u32) -> anyhow::Result<()> {
    use jb_core::{eval::LayerContent, Shadow, Stroke, TitleData};
    let (w, h) = r.size();
    let base = TitleData {
        text: "Jibaru Title".into(),
        font_family: String::new(),
        font_size: 0.16,
        color: [1.0, 1.0, 1.0, 1.0],
        position: [0.5, 0.5],
        bold: true,
        italic: false,
        stroke: None,
        shadow: None,
        background: None,
    };
    let styles = [
        base.clone(),
        TitleData { stroke: Some(Stroke { color: [0.0, 0.0, 0.0, 1.0], width: 0.012 }), ..base.clone() },
        TitleData { shadow: Some(Shadow { color: [0.0, 0.0, 0.0, 0.8], offset: [0.01, 0.012], blur: 0.01 }), ..base.clone() },
        TitleData { background: Some([0.05, 0.05, 0.1, 0.75]), color: [1.0, 0.8, 0.1, 1.0], italic: true, bold: false, ..base.clone() },
        TitleData { text: "Two lines\nof text".into(), font_size: 0.12, position: [0.5, 0.7], font_family: "Georgia".into(), ..base.clone() },
    ];
    let gap = 4;
    let mut sheet = vec![40u8; (sheet_w * h * 4) as usize];
    for (i, t) in styles.into_iter().enumerate() {
        let mut l = media_layer("t");
        l.content = LayerContent::Title(t);
        let plan = FramePlan { time: 0, layers: vec![media_layer("b"), l] };
        let f = r.render(project, &plan, src);
        let x0 = i as u32 * (w + gap);
        for y in 0..h {
            let srow = (y * w * 4) as usize;
            let drow = ((y * sheet_w + x0) * 4) as usize;
            sheet[drow..drow + (w * 4) as usize].copy_from_slice(&f.data[srow..srow + (w * 4) as usize]);
        }
    }
    save(&format!("{out_dir}/_titles.png"), sheet_w, h, &sheet)
}

fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let (mut w, mut h) = (320u32, 180u32);
    if let Some(i) = args.iter().position(|a| a == "--size") {
        let s = args.remove(i + 1);
        args.remove(i);
        let (a, b) = s.split_once('x').expect("--size WxH");
        (w, h) = (a.parse()?, b.parse()?);
    }
    let out_dir = args.first().cloned().unwrap_or_else(|| "target/fx-sheet".into());
    let filter = args.get(1).cloned().unwrap_or_default();
    std::fs::create_dir_all(&out_dir)?;
    let mut r = Renderer::new(w, h)?;
    eprintln!("adapter: {:?}", r.gpu().adapter_info.name);
    let project = project(w, h);
    let src = MemorySource::ab(w, h);
    let items: Vec<(String, FxKind)> =
        r.registry().iter().filter(|i| i.id.contains(&filter)).map(|i| (i.id.to_string(), i.kind)).collect();
    let steps = [0.0, 0.25, 0.5, 0.75, 1.0];
    let gap = 4u32;
    let sheet_w = w * steps.len() as u32 + gap * (steps.len() as u32 - 1);
    let mut unknown = 0;
    for (id, kind) in &items {
        let mut sheet = vec![40u8; (sheet_w * h * 4) as usize];
        for (i, p) in steps.iter().enumerate() {
            let plan = match kind {
                FxKind::Transition => transition_plan(id, *p, &[]),
                FxKind::Animation => {
                    let mut bg = media_layer("b");
                    bg.effects.clear();
                    let mut fg = media_layer("a");
                    fg.transform.scale = [0.6, 0.6];
                    fg.effects.push(effect(id, *p, &[]));
                    FramePlan { time: 0, layers: vec![bg, fg] }
                }
                _ => effect_plan(id, *p, &[]),
            };
            let f = r.render(&project, &plan, &src);
            if !r.last_report().unknown_kinds.is_empty() || !r.last_report().errors.is_empty() {
                eprintln!("{id}: {:?}", r.last_report());
                unknown += 1;
            }
            let x0 = i as u32 * (w + gap);
            for y in 0..h {
                let srow = (y * w * 4) as usize;
                let drow = ((y * sheet_w + x0) * 4) as usize;
                sheet[drow..drow + (w * 4) as usize].copy_from_slice(&f.data[srow..srow + (w * 4) as usize]);
            }
        }
        let path = format!("{out_dir}/{}.png", id.replace([':', '/'], "_"));
        save(&path, sheet_w, h, &sheet)?;
    }
    if filter.is_empty() || "titles".contains(&filter) {
        write_titles(&mut r, &project, &src, &out_dir, sheet_w)?;
    }
    eprintln!("wrote {} sheets to {out_dir} ({unknown} with problems)", items.len());
    Ok(())
}
