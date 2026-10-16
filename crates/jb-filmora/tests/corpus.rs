//! Local-only round trip over real Filmora projects. Not run by default:
//!
//! ```sh
//! cargo test -p jb-filmora --test corpus -- --ignored --nocapture
//! ```
//!
//! Files come from `JB_WFP_CORPUS` (a `:`-separated list of files or
//! directories) or, by default, `~/Pictures/*/*.wfp` and Filmora's preset
//! projects. Each file is copied to a temporary directory first; originals
//! are never touched.

use std::path::{Path, PathBuf};

fn corpus() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = match std::env::var("JB_WFP_CORPUS") {
        Ok(v) => v.split(':').map(PathBuf::from).collect(),
        Err(_) => {
            let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
            let mut r = Vec::new();
            if let Ok(dirs) = std::fs::read_dir(home.join("Pictures")) {
                r.extend(dirs.flatten().map(|d| d.path()).filter(|p| p.is_dir()));
            }
            r.push(home.join(
                "Library/Application Support/Wondershare Filmora Mac/ProgramData/Default Effects/PresetProjects",
            ));
            r
        }
    };
    let mut files = Vec::new();
    for root in roots.drain(..) {
        if root.is_file() {
            files.push(root);
        } else if let Ok(entries) = std::fs::read_dir(&root) {
            files.extend(entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "wfp")));
        }
    }
    files.sort();
    files
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jb-filmora-corpus-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn json_docs(entries: &[jb_filmora::Entry]) -> Vec<(String, Option<serde_json::Value>, usize)> {
    entries
        .iter()
        .map(|e| {
            let v = if e.name.ends_with(".json") || e.name.ends_with(".wesproj") {
                serde_json::from_slice(&e.data).ok()
            } else {
                None
            };
            (e.name.clone(), v, e.data.len())
        })
        .collect()
}

fn check(path: &Path) {
    let copy = scratch(&path.file_name().unwrap().to_string_lossy());
    std::fs::copy(path, &copy).unwrap();
    let wfp = jb_filmora::read(&copy).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let out = scratch("out.wfp");
    let report = jb_filmora::write(&wfp, &out).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let again = jb_filmora::read(&out).unwrap();
    // Every entry survives; JSON documents are semantically identical and
    // everything else is byte-identical.
    assert_eq!(json_docs(&wfp.raw.entries), json_docs(&again.raw.entries));
    for (a, b) in wfp.raw.entries.iter().zip(&again.raw.entries) {
        assert_eq!(a.data, b.data, "{} differs", a.name);
        assert_eq!(a.deflated, b.deflated, "{} compression differs", a.name);
    }
    assert_eq!(wfp.project, again.project);
    let cov = jb_filmora::coverage(&wfp);
    let clips: usize = wfp.project.sequences.iter().flat_map(|s| &s.tracks).map(|t| t.clips.len()).sum();
    println!(
        "{}: {} sequences, {} clips, {} media, {} coverage items, {} warnings",
        path.file_name().unwrap().to_string_lossy(),
        wfp.project.sequences.len(),
        clips,
        wfp.project.media.len(),
        cov.items.len(),
        wfp.warnings.len()
    );
    std::fs::remove_file(&copy).ok();
}

fn clip_count(p: &jb_core::Project) -> usize {
    p.sequences.iter().flat_map(|s| &s.tracks).map(|t| t.clips.len()).sum()
}

/// Applies a few edits to a real project and checks they survive a write.
fn check_edits(path: &Path) {
    use jb_core::*;
    let mut wfp = jb_filmora::read(path).unwrap();
    let before = clip_count(&wfp.project);
    let mut h = History::new(wfp.project.clone());
    let main = h.project.main().clone();
    let mut expected = before;
    // Split the longest media clip of the main sequence (and its linked partner).
    if let Some(c) = main
        .tracks
        .iter()
        .flat_map(|t| &t.clips)
        .filter(|c| matches!(c.source, ClipSource::Media { .. }))
        .max_by_key(|c| c.duration())
    {
        let linked = main
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .filter(|x| x.link_group.is_some() && x.link_group == c.link_group)
            .count()
            .max(1);
        h.execute(&Command::SplitClip { clip_id: c.id.clone(), at: c.start + c.duration() / 2 }).unwrap();
        expected += linked;
    }
    // Retitle every title.
    let titles: Vec<(Id, TitleData)> = h
        .project
        .sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .flat_map(|t| &t.clips)
        .filter_map(|c| match &c.source {
            ClipSource::Title(t) => Some((c.id.clone(), t.clone())),
            _ => None,
        })
        .collect();
    for (id, mut t) in titles {
        t.text = format!("{} (edited)", t.text);
        h.execute(&Command::SetTitle { clip_id: id, title: t }).unwrap();
    }
    h.execute(&Command::RenameProject { name: "edited".into() }).unwrap();
    wfp.project = h.project.clone();
    let out = scratch("edited.wfp");
    let report = jb_filmora::write(&wfp, &out).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let again = jb_filmora::read(&out).unwrap();
    assert_eq!(clip_count(&again.project), expected, "{}", path.display());
    assert_eq!(again.project.name, "edited");
    for s in &again.project.sequences {
        for t in &s.tracks {
            for c in &t.clips {
                if let ClipSource::Title(t) = &c.source {
                    assert!(t.text.ends_with("(edited)"));
                }
            }
        }
    }
    // Writing the re-read project again is a no-op.
    let out2 = scratch("edited2.wfp");
    jb_filmora::write(&again, &out2).unwrap();
    assert_eq!(jb_filmora::read(&out2).unwrap().raw.timeline, again.raw.timeline);
}

#[test]
#[ignore = "needs local Filmora projects"]
fn corpus_survives_edits() {
    for f in corpus() {
        let copy = scratch(&format!("edit-{}", f.file_name().unwrap().to_string_lossy()));
        std::fs::copy(&f, &copy).unwrap();
        check_edits(&copy);
        std::fs::remove_file(&copy).ok();
    }
}

#[test]
#[ignore = "needs local Filmora projects"]
fn corpus_round_trips() {
    let files = corpus();
    assert!(!files.is_empty(), "no .wfp files found; set JB_WFP_CORPUS");
    for f in files {
        check(&f);
    }
}
