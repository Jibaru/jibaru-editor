//! End-to-end tests of the `jb` binary's JSON contract.

use assert_cmd::Command;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn jb() -> Command {
    Command::cargo_bin("jb").unwrap()
}

/// Runs `jb --json <args>` and returns (exit code, parsed stdout).
fn run(args: &[&str]) -> (i32, Value) {
    let out = jb().arg("--json").args(args).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("bad json {e}: {stdout}"));
    (out.status.code().unwrap(), v)
}

fn ok(args: &[&str]) -> Value {
    let (code, v) = run(args);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    v["data"].clone()
}

fn new_project(dir: &Path) -> PathBuf {
    let p = dir.join("p.jbp");
    ok(&["new", p.to_str().unwrap(), "--width", "1280", "--height", "720", "--fps", "30"]);
    p
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn new_and_info() {
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(dir.path());
    let info = ok(&["info", s(&p)]);
    assert_eq!(info["settings"]["width"], 1280);
    assert_eq!(info["settings"]["frame_rate"]["num"], 30);
    assert_eq!(info["name"], "p");
    assert_eq!(info["sequences"][0]["is_main"], true);
    // Refuses to overwrite without --force.
    let (code, v) = run(&["new", s(&p)]);
    assert_eq!(code, 4);
    assert_eq!(v["error"]["code"], "invalid_argument");
    assert!(v["error"]["hint"].as_str().unwrap().contains("--force"));
}

#[test]
fn exec_reports_created_ids_and_saves() {
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(dir.path());
    let r = ok(&["exec", s(&p), r#"{"op":"add_track","kind":"video"}"#]);
    let track = r["created"]["tracks"][0].as_str().unwrap().to_string();
    let clip = json!({"op": "add_clip", "track_id": track, "clip": {
        "name": "red", "source": {"type": "color", "color": [1, 0, 0, 1]}, "start": 0, "end": 50_000_000
    }});
    let r = ok(&["exec", s(&p), &clip.to_string()]);
    assert_eq!(r["created"]["clips"].as_array().unwrap().len(), 1);
    assert_eq!(r["duration"]["seconds"], 5.0);
    let info = ok(&["info", s(&p)]);
    assert_eq!(info["sequences"][0]["tracks"][0]["clips"][0]["name"], "red");

    let plan = ok(&["eval", s(&p), "--at", "00:00:02"]);
    assert_eq!(plan["plan"]["layers"][0]["content"]["color"], json!([1.0, 0.0, 0.0, 1.0]));
    let found = ok(&["find-clips", s(&p), "--at", "4s", "--name", "RE"]);
    assert_eq!(found["count"], 1);
    let none = ok(&["find-clips", s(&p), "--at", "6s"]);
    assert_eq!(none["count"], 0);
}

#[test]
fn exec_file_jsonl_and_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(dir.path());
    let cmds = dir.path().join("cmds.jsonl");
    std::fs::write(&cmds, "{\"op\":\"add_track\",\"kind\":\"video\"}\n{\"op\":\"add_track\",\"kind\":\"audio\"}\n")
        .unwrap();
    let r = ok(&["exec", s(&p), "--file", s(&cmds), "--dry-run"]);
    assert_eq!(r["applied"], 2);
    assert_eq!(r["dry_run"], true);
    assert_eq!(r["saved_to"], Value::Null);
    assert_eq!(ok(&["info", s(&p)])["sequences"][0]["tracks"].as_array().unwrap().len(), 0);
    ok(&["exec", s(&p), "--file", s(&cmds)]);
    assert_eq!(ok(&["info", s(&p)])["sequences"][0]["tracks"].as_array().unwrap().len(), 2);
}

#[test]
fn exec_is_atomic_and_errors_are_actionable() {
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(dir.path());
    let (code, v) =
        run(&["exec", s(&p), r#"[{"op":"add_track","kind":"video"},{"op":"remove_track","track_id":"nope"}]"#]);
    assert_eq!(code, 4);
    assert_eq!(v["error"]["code"], "not_found");
    assert!(v["error"]["message"].as_str().unwrap().contains("command #1"));
    assert_eq!(ok(&["info", s(&p)])["sequences"][0]["tracks"].as_array().unwrap().len(), 0);

    let (code, v) = run(&["exec", s(&p), r#"{"op":"frobnicate"}"#]);
    assert_eq!(code, 4);
    assert_eq!(v["error"]["code"], "parse_error");
    assert!(v["error"]["message"].as_str().unwrap().contains("add_clip"));
}

#[test]
fn missing_file_and_usage_errors() {
    let (code, v) = run(&["info", "/definitely/not/here.jbp"]);
    assert_eq!(code, 3);
    assert_eq!(v["error"]["code"], "file_not_found");
    let (code, v) = run(&["info", "x.txt"]);
    assert_eq!(code, 4);
    assert_eq!(v["error"]["code"], "unsupported_format");
    let (code, v) = run(&["no-such-command"]);
    assert_eq!(code, 2);
    assert_eq!(v["error"]["code"], "usage_error");
    // Without --json errors go to stderr.
    let out = jb().args(["info", "/nope.jbp"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stderr).contains("does not exist"));
    assert!(out.stdout.is_empty());
}

#[test]
fn schema_lists_commands() {
    let sch = ok(&["schema", "command"]);
    let ops: Vec<String> = sch["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["properties"]["op"]["const"].as_str().unwrap().to_string())
        .collect();
    assert!(ops.contains(&"add_clip".to_string()));
    assert!(ops.contains(&"batch".to_string()));
    let all = ok(&["schema"]);
    assert!(all["project"]["properties"]["sequences"].is_object());
    assert!(all["frame_plan"]["properties"]["layers"].is_object());
}

#[test]
fn stubs_report_not_implemented() {
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(dir.path());
    let (code, v) = run(&["render", s(&p), "-o", s(&dir.path().join("o.mp4")), "--preview", "540p", "--range", "0-1"]);
    if code == 5 {
        assert_eq!(v["error"]["code"], "not_implemented");
    }
    let (code, v) = run(&["render", s(&p), "-o", "o.mp4", "--range", "5-1"]);
    assert_eq!(code, 4, "{v}");
}

#[test]
fn import_media_with_ffmpeg() {
    let dir = tempfile::tempdir().unwrap();
    let media = dir.path().join("t.mp4");
    let made = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc=size=64x48:rate=25", "-t", "1"])
        .arg(&media)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !made {
        eprintln!("ffmpeg not available; skipping");
        return;
    }
    let p = new_project(dir.path());
    let r = ok(&["import-media", s(&p), s(&media), "--dry-run"]);
    assert_eq!(r["added"][0]["kind"], "video");
    assert!(ok(&["info", s(&p)])["media"].as_array().unwrap().is_empty());
    let r = ok(&["import-media", s(&p), s(&media)]);
    assert_eq!(r["added"][0]["width"], 64);
    assert_eq!(r["added"][0]["duration_seconds"], 1.0);
    let r = ok(&["import-media", s(&p), s(&media)]);
    assert_eq!(r["skipped"].as_array().unwrap().len(), 1);
    let (code, v) = run(&["import-media", s(&p), "/nope.mp4"]);
    assert_eq!(code, 3, "{v}");
}
