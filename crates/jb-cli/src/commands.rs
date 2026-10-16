//! Parsing and executing `Command` JSON with agent-friendly conveniences:
//! missing ids are generated, and every execution reports which tracks,
//! clips and media were created or removed so callers learn the new ids.

use crate::error::{code, JbError, JbResult};
use jb_core::{new_id, Command, History, Project};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// Fills in ids (and default names) that agents may omit:
/// `add_media.asset.id/name`, `add_clip.clip.id/name`, `add_effect.effect.id`,
/// `set_transition.transition.id`, recursively through `batch`.
pub fn normalize(v: &mut Value) {
    let Some(obj) = v.as_object_mut() else { return };
    let op = obj.get("op").and_then(Value::as_str).unwrap_or_default().to_string();
    let fill = |o: &mut Value, default_name: Option<String>| {
        if let Some(o) = o.as_object_mut() {
            o.entry("id").or_insert_with(|| Value::String(new_id()));
            if let Some(n) = default_name {
                o.entry("name").or_insert(Value::String(n));
            }
        }
    };
    match op.as_str() {
        "add_media" => {
            if let Some(a) = obj.get_mut("asset") {
                let name = a
                    .get("path")
                    .and_then(Value::as_str)
                    .and_then(|p| std::path::Path::new(p).file_name()?.to_str().map(String::from));
                fill(a, name);
            }
        }
        "add_clip" => {
            if let Some(c) = obj.get_mut("clip") {
                fill(c, Some("Clip".into()));
            }
        }
        "add_effect" => {
            if let Some(e) = obj.get_mut("effect") {
                fill(e, None);
            }
        }
        "set_transition" => {
            if let Some(t) = obj.get_mut("transition").filter(|t| t.is_object()) {
                fill(t, None);
            }
        }
        "batch" => {
            if let Some(Value::Array(cs)) = obj.get_mut("commands") {
                cs.iter_mut().for_each(normalize);
            }
        }
        _ => {}
    }
}

/// Converts one JSON value into a `Command`, after normalization.
pub fn command_from_value(mut v: Value) -> JbResult<Command> {
    normalize(&mut v);
    serde_json::from_value(v).map_err(|e| {
        JbError::new(code::PARSE, format!("invalid command: {e}"))
            .hint("commands are objects tagged by `op` (e.g. {\"op\":\"add_track\",\"kind\":\"video\"}); run `jb schema command` for the exact shape")
    })
}

/// Parses a single command object, a JSON array of commands, or JSON Lines
/// (one command per line, blank lines and `//` comments ignored).
pub fn parse_commands(text: &str) -> JbResult<Vec<Command>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(JbError::invalid("no commands given").hint("pass a Command JSON object, an array, or JSON Lines"));
    }
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return match v {
            Value::Array(items) => {
                items.into_iter().enumerate().map(|(i, c)| command_from_value(c).map_err(|e| e.at_command(i))).collect()
            }
            v => Ok(vec![command_from_value(v)?]),
        };
    }
    trimmed
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.trim_start().starts_with("//"))
        .map(|(i, l)| {
            let v: Value = serde_json::from_str(l).map_err(|e| {
                JbError::new(code::PARSE, format!("line {}: invalid JSON: {e}", i + 1))
                    .hint("JSON Lines input needs exactly one Command object per line")
            })?;
            command_from_value(v).map_err(|mut e| {
                e.message = format!("line {}: {}", i + 1, e.message);
                e
            })
        })
        .collect()
}

#[derive(Default)]
struct Ids {
    tracks: BTreeSet<String>,
    clips: BTreeSet<String>,
    media: BTreeSet<String>,
    effects: BTreeSet<String>,
}

fn ids(p: &Project) -> Ids {
    let mut out = Ids { media: p.media.iter().map(|m| m.id.clone()).collect(), ..Default::default() };
    for s in &p.sequences {
        for t in &s.tracks {
            out.tracks.insert(t.id.clone());
            for c in &t.clips {
                out.clips.insert(c.id.clone());
                out.effects.extend(c.effects.iter().chain(c.animations.iter()).map(|e| e.id.clone()));
            }
        }
    }
    out
}

/// Ids created and removed between two project states.
pub fn diff(before: &Project, after: &Project) -> Value {
    let (a, b) = (ids(before), ids(after));
    let d = |x: &BTreeSet<String>, y: &BTreeSet<String>| x.difference(y).cloned().collect::<Vec<_>>();
    json!({
        "created": { "tracks": d(&b.tracks, &a.tracks), "clips": d(&b.clips, &a.clips), "media": d(&b.media, &a.media), "effects": d(&b.effects, &a.effects) },
        "removed": { "tracks": d(&a.tracks, &b.tracks), "clips": d(&a.clips, &b.clips), "media": d(&a.media, &b.media), "effects": d(&a.effects, &b.effects) },
    })
}

/// Executes commands in order, each as its own undo step. Atomic: if any
/// command fails, `history` is left exactly as it was.
pub fn execute_all(history: &mut History, cmds: &[Command]) -> JbResult<Value> {
    let mut work = history.clone();
    let before = history.project.clone();
    for (i, c) in cmds.iter().enumerate() {
        work.execute(c).map_err(|e| {
            let e = JbError::from(e);
            if cmds.len() > 1 {
                e.at_command(i)
            } else {
                e
            }
        })?;
    }
    let mut report = diff(&before, &work.project);
    report["applied"] = json!(cmds.len());
    *history = work;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_object_array_and_lines() {
        assert_eq!(parse_commands(r#"{"op":"rename_project","name":"a"}"#).unwrap().len(), 1);
        assert_eq!(
            parse_commands(r#"[{"op":"rename_project","name":"a"},{"op":"rename_project","name":"b"}]"#).unwrap().len(),
            2
        );
        let lines = "{\"op\":\"rename_project\",\"name\":\"a\"}\n\n// c\n{\"op\":\"add_track\",\"kind\":\"video\"}\n";
        assert_eq!(parse_commands(lines).unwrap().len(), 2);
        let err = parse_commands("{\"op\":\"nope\"}").unwrap_err();
        assert_eq!(err.code, code::PARSE);
    }

    #[test]
    fn fills_missing_clip_id() {
        let c = parse_commands(
            r#"{"op":"add_clip","track_id":"t","clip":{"source":{"type":"color","color":[1,0,0,1]},"start":0,"end":10}}"#,
        )
        .unwrap();
        match &c[0] {
            Command::AddClip { clip, .. } => assert!(!clip.id.is_empty()),
            _ => panic!(),
        }
    }
}
