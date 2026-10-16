//! Test clip generation with the ffmpeg CLI. Clips are created in a temp
//! directory at test time; nothing binary is committed.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub const FPS: i64 = 25;
pub const FRAMES: i64 = 50;

pub fn dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let d = std::env::temp_dir().join(format!("jb-media-tests-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    })
}

pub fn ffmpeg(args: &[&str]) {
    let st = Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-nostdin"])
        .args(args)
        .status()
        .expect("the ffmpeg CLI is required to generate test clips");
    assert!(st.success(), "ffmpeg {args:?} failed");
}

/// Whether the ffmpeg CLI has `encoder`.
pub fn has_encoder(encoder: &str) -> bool {
    let out = Command::new("ffmpeg").args(["-hide_banner", "-encoders"]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).lines().any(|l| l.split_whitespace().nth(1) == Some(encoder))
}

/// Video expression whose luma encodes the frame number: Y = 20 + 4 * N.
pub const INDEX_LUMA: &str = "format=yuv420p,geq=lum='20+4*N':cb=128:cr=128";

/// Recovers the frame number from a mean luma value.
pub fn index_from_luma(y: f64) -> i64 {
    ((y - 20.0) / 4.0).round() as i64
}

fn cached(name: &str, make: impl FnOnce(&str)) -> PathBuf {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _g = LOCK.lock().unwrap();
    let p = dir().join(name);
    if !p.exists() {
        let tmp = dir().join(format!("tmp-{name}"));
        make(tmp.to_str().unwrap());
        std::fs::rename(&tmp, &p).unwrap();
    }
    p
}

/// 2 s, 320x240 @ 25 fps MPEG-4 with B-frames and 1 s GOPs, plus a 440 Hz
/// AAC tone at 44.1 kHz.
pub fn index_clip() -> PathBuf {
    cached("index.mp4", |out| {
        let vf = format!("color=c=black:s=320x240:r={FPS}:d=2,{INDEX_LUMA}");
        ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            &vf,
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100:duration=2",
            "-c:v",
            "mpeg4",
            "-q:v",
            "2",
            "-g",
            "25",
            "-bf",
            "2",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-shortest",
            "-f",
            "mp4",
            out,
        ]);
    })
}

/// Same picture at 640x480, no audio.
pub fn index_clip_large() -> PathBuf {
    cached("index-large.mp4", |out| {
        let vf = format!("color=c=black:s=640x480:r={FPS}:d=2,{INDEX_LUMA}");
        ffmpeg(&["-f", "lavfi", "-i", &vf, "-c:v", "mpeg4", "-q:v", "2", "-g", "25", "-bf", "2", "-f", "mp4", out]);
    })
}

/// HEVC version of the index clip using the platform hardware encoder,
/// when the ffmpeg CLI has one.
pub fn index_clip_hevc() -> Option<PathBuf> {
    let enc = ["hevc_videotoolbox", "hevc_nvenc", "hevc_qsv", "hevc_amf"].into_iter().find(|e| has_encoder(e))?;
    let name = "index-hevc.mp4";
    let p = dir().join(name);
    if p.exists() {
        return Some(p);
    }
    let vf = format!("color=c=black:s=640x480:r={FPS}:d=2,{INDEX_LUMA}");
    let tmp = dir().join(format!("tmp-{name}"));
    let st = Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-nostdin", "-f", "lavfi", "-i", &vf, "-c:v", enc, "-b:v", "8M", "-g", "25"])
        .args(["-tag:v", "hvc1", "-f", "mp4"])
        .arg(&tmp)
        .status()
        .ok()?;
    if !st.success() {
        return None;
    }
    std::fs::rename(&tmp, &p).ok()?;
    Some(p)
}

/// Silence for 1 s, then a 0.5-amplitude 1 kHz tone for 1 s (48 kHz mono AAC).
pub fn late_tone() -> PathBuf {
    cached("late-tone.m4a", |out| {
        ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "aevalsrc='if(gte(t,1),0.5*sin(2*PI*1000*t),0)':s=48000:d=2",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-f",
            "mp4",
            out,
        ]);
    })
}

/// Lossless 10-bit clip (FFV1) to exercise P010 output.
pub fn ten_bit_clip() -> PathBuf {
    cached("tenbit.mkv", |out| {
        ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "color=c=gray:s=128x96:r=25:d=0.4",
            "-c:v",
            "ffv1",
            "-pix_fmt",
            "yuv420p10le",
            "-f",
            "matroska",
            out,
        ]);
    })
}

/// PNG-in-MOV clip with alpha to exercise RGBA output.
pub fn alpha_clip() -> PathBuf {
    cached("alpha.mov", |out| {
        ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "color=c=red@0.5:s=64x48:r=25:d=0.4,format=rgba",
            "-c:v",
            "png",
            "-f",
            "mov",
            out,
        ]);
    })
}

/// The index clip tagged with a 90 degree counter-clockwise display rotation.
pub fn rotated_clip() -> PathBuf {
    let src = index_clip();
    cached("rotated.mp4", |out| {
        ffmpeg(&["-display_rotation", "90", "-i", src.to_str().unwrap(), "-c", "copy", "-f", "mp4", out]);
    })
}

/// Mean of the luma plane of an NV12 / P010 frame, in 8-bit units.
pub fn mean_luma(f: &jb_core::VideoFrame) -> f64 {
    let w = f.width as usize;
    let h = f.height as usize;
    let mut sum = 0f64;
    for y in 0..h {
        for x in 0..w {
            sum += match f.format {
                jb_core::PixelFormat::Nv12 => f.data[y * f.stride + x] as f64,
                jb_core::PixelFormat::P010 => {
                    let i = y * f.stride + x * 2;
                    (u16::from_le_bytes([f.data[i], f.data[i + 1]]) >> 8) as f64
                }
                jb_core::PixelFormat::Rgba8 => f.data[y * f.stride + x * 4] as f64,
            };
        }
    }
    sum / (w * h) as f64
}
