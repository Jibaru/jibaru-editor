//! Fingerprints the renderer sources (Rust and WGSL) so caches of rendered
//! output (library previews) are invalidated whenever the renderer changes.

use std::path::{Path, PathBuf};

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, files);
        } else {
            files.push(p);
        }
    }
}

fn main() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    println!("cargo:rerun-if-changed={}", src.display());
    let mut files = Vec::new();
    walk(&src, &mut files);
    files.sort();
    // FNV-1a over relative paths and contents.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for f in &files {
        eat(f.strip_prefix(&src).unwrap_or(f).to_string_lossy().replace('\\', "/").as_bytes());
        eat(&std::fs::read(f).unwrap_or_default());
    }
    println!("cargo:rustc-env=JB_RENDER_FINGERPRINT={h:016x}");
}
