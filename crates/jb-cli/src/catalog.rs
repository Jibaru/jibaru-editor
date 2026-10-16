//! `jb fx catalog-status`: implemented vs missing Filmora items, read from
//! the metadata inventory `docs/filmora-catalog.json` and checked against the
//! live effect registry (an item counts as implemented when its recorded `jb`
//! id is registered, or when one of its Filmora GUIDs resolves through the
//! `filmora:<GUID>` aliases).

use crate::error::{code, JbError, JbResult};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How items are bucketed in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum GroupBy {
    /// Proposed implementation group (similar visual mechanics).
    #[default]
    Group,
    /// Filmora store category.
    Category,
    /// Item kind only (transition, effect, filter, ...).
    Kind,
}

#[derive(Debug, Clone, Default)]
pub struct StatusOptions {
    /// Catalog path; defaults to `docs/filmora-catalog.json` in the current
    /// directory or the source checkout.
    pub catalog: Option<PathBuf>,
    pub by: GroupBy,
    /// Only items of this kind.
    pub kind: Option<String>,
    /// Only premium (store `sale_mode` 1) items.
    pub premium_only: bool,
    /// Missing items listed per bucket, most popular first.
    pub top: usize,
}

/// Locates the catalog: explicit path, `./docs/`, or the source checkout.
pub fn default_catalog_path() -> PathBuf {
    let local = PathBuf::from("docs/filmora-catalog.json");
    if local.exists() {
        return local;
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/filmora-catalog.json");
    if src.exists() {
        return src;
    }
    local
}

/// Live implementation of one catalog item, if any.
fn implemented(reg: &jb_render::Registry, item: &Value) -> Option<String> {
    if let Some(id) = item["jb"].as_str() {
        if let Some(c) = reg.canonical(id) {
            return Some(c.to_string());
        }
    }
    item["filmora"]["guids"].as_array().into_iter().flatten().filter_map(Value::as_str).find_map(|g| {
        reg.canonical(&format!("filmora:{g}"))
            .or_else(|| reg.canonical(&format!("filmora:{}", g.to_ascii_uppercase())))
            .map(str::to_string)
    })
}

#[derive(Default)]
struct Bucket {
    total: usize,
    implemented: usize,
    premium: usize,
    premium_missing: usize,
    downloaded_missing: usize,
    /// (score, item) of missing items.
    missing: Vec<(f64, Value)>,
}

pub fn catalog_status(opts: &StatusOptions) -> JbResult<Value> {
    let path = opts.catalog.clone().unwrap_or_else(default_catalog_path);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        JbError::new(code::FILE_NOT_FOUND, format!("cannot read {}: {e}", path.display()))
            .hint("run from the repository root or pass --catalog docs/filmora-catalog.json")
    })?;
    let doc: Value = serde_json::from_str(&text)
        .map_err(|e| JbError::new(code::PARSE, format!("{} is not valid JSON: {e}", path.display())))?;
    let items = doc["items"]
        .as_array()
        .ok_or_else(|| JbError::new(code::PARSE, format!("{} has no `items` array", path.display())))?;
    let reg = jb_render::Registry::builtin();

    let mut buckets: BTreeMap<(String, String), Bucket> = BTreeMap::new();
    let mut kinds: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut stale = Vec::new();
    for it in items {
        let kind = it["kind"].as_str().unwrap_or("?").to_string();
        if opts.kind.as_deref().is_some_and(|k| k != kind) {
            continue;
        }
        let premium = it["premium"].as_bool() == Some(true);
        if opts.premium_only && !premium {
            continue;
        }
        let label = match opts.by {
            GroupBy::Group => it["group"].as_str().unwrap_or("other").to_string(),
            GroupBy::Category => it["category"].as_str().unwrap_or("(none)").to_string(),
            GroupBy::Kind => String::new(),
        };
        let live = implemented(&reg, it);
        if let (Some(rec), None) = (it["jb"].as_str(), &live) {
            stale.push(json!({ "key": it["key"], "jb": rec }));
        }
        let b = buckets.entry((kind.clone(), label)).or_default();
        let k = kinds.entry(kind).or_default();
        b.total += 1;
        k.0 += 1;
        if premium {
            b.premium += 1;
        }
        if live.is_some() {
            b.implemented += 1;
            k.1 += 1;
        } else {
            if premium {
                b.premium_missing += 1;
            }
            if it["downloaded"].as_bool() == Some(true) {
                b.downloaded_missing += 1;
            }
            let score = it["popularity"]["score"].as_f64().unwrap_or(0.0);
            b.missing.push((score, it.clone()));
        }
    }

    let total: usize = kinds.values().map(|k| k.0).sum();
    let done: usize = kinds.values().map(|k| k.1).sum();
    let mut out_buckets = Vec::new();
    for ((kind, label), mut b) in buckets {
        b.missing.sort_by(|a, c| c.0.total_cmp(&a.0));
        let top: Vec<Value> = b
            .missing
            .iter()
            .take(opts.top)
            .map(|(s, it)| {
                json!({
                    "key": it["key"], "name": it["name"], "score": s,
                    "premium": it["premium"], "downloaded": it["downloaded"],
                })
            })
            .collect();
        out_buckets.push(json!({
            "kind": kind, "bucket": label, "total": b.total, "implemented": b.implemented,
            "missing": b.total - b.implemented, "premium": b.premium,
            "premium_missing": b.premium_missing, "downloaded_missing": b.downloaded_missing,
            "top_missing": top,
        }));
    }
    Ok(json!({
        "catalog": path,
        "generated": doc["generated"],
        "total": total,
        "implemented": done,
        "missing": total - done,
        "kinds": kinds.iter().map(|(k, (t, i))| json!({ "kind": k, "total": t, "implemented": i })).collect::<Vec<_>>(),
        "buckets": out_buckets,
        "stale_mappings": stale,
    }))
}

/// Human rendering of [`catalog_status`].
pub fn status_text(v: &Value) -> String {
    let n = |x: &Value| x.as_u64().unwrap_or(0);
    let mut s = format!(
        "{}: {} of {} items implemented ({} missing)\n",
        v["catalog"].as_str().unwrap_or_default(),
        n(&v["implemented"]),
        n(&v["total"]),
        n(&v["missing"])
    );
    let mut kind = String::new();
    for b in v["buckets"].as_array().into_iter().flatten() {
        let k = b["kind"].as_str().unwrap_or_default();
        if k != kind {
            kind = k.to_string();
            let kt = v["kinds"].as_array().into_iter().flatten().find(|x| x["kind"] == k);
            let (t, i) = kt.map_or((0, 0), |x| (n(&x["total"]), n(&x["implemented"])));
            s += &format!("\n{k} ({i}/{t})\n");
            s += &format!("  {:<34} {:>5} {:>5} {:>7} {:>8}\n", "bucket", "done", "total", "missing", "premium-");
        }
        let label = b["bucket"].as_str().unwrap_or_default();
        let label = if label.is_empty() { "(all)" } else { label };
        let label: String = label.chars().take(34).collect();
        s += &format!(
            "  {:<34} {:>5} {:>5} {:>7} {:>8}\n",
            label,
            n(&b["implemented"]),
            n(&b["total"]),
            n(&b["missing"]),
            n(&b["premium_missing"])
        );
        for m in b["top_missing"].as_array().into_iter().flatten() {
            let tag = if m["premium"].as_bool() == Some(true) { " [premium]" } else { "" };
            s += &format!(
                "      - {} ({}){}\n",
                m["name"].as_str().unwrap_or("?"),
                m["key"].as_str().unwrap_or_default(),
                tag
            );
        }
    }
    let stale = v["stale_mappings"].as_array().map_or(0, Vec::len);
    if stale > 0 {
        s += &format!("\n{stale} catalog `jb` mappings no longer resolve in the registry (see --json)\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_live_aliases_and_buckets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cat.json");
        let doc = json!({
            "generated": "test",
            "items": [
                // Implemented through the filmora:<GUID> alias (Fast Zoom).
                { "key": "a", "kind": "transition", "group": "zoom", "name": "Fast Zoom",
                  "filmora": { "guids": ["8D02CB83-6FC6-406B-8506-252F58A1D3B3"] },
                  "premium": true, "downloaded": true, "jb": null, "popularity": { "score": 5 } },
                // Missing, premium.
                { "key": "b", "kind": "transition", "group": "zoom", "name": "Warp Zoom 9",
                  "filmora": {}, "premium": true, "downloaded": false, "jb": null,
                  "popularity": { "score": 50 } },
                // Recorded mapping that does not exist any more.
                { "key": "c", "kind": "effect", "group": "blur", "name": "Gone",
                  "filmora": {}, "premium": false, "downloaded": false, "jb": "jb.does_not_exist",
                  "popularity": { "score": 1 } },
            ]
        });
        std::fs::write(&path, doc.to_string()).unwrap();
        let v = catalog_status(&StatusOptions { catalog: Some(path), top: 3, ..Default::default() }).unwrap();
        assert_eq!(v["total"], 3);
        assert_eq!(v["implemented"], 1);
        let zoom = v["buckets"].as_array().unwrap().iter().find(|b| b["bucket"] == "zoom").unwrap();
        assert_eq!(zoom["implemented"], 1);
        assert_eq!(zoom["premium_missing"], 1);
        assert_eq!(zoom["top_missing"][0]["key"], "b");
        assert_eq!(v["stale_mappings"][0]["key"], "c");
        assert!(status_text(&v).contains("1 of 3 items implemented"));
    }

    #[test]
    fn repository_catalog_parses() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/filmora-catalog.json");
        if !path.exists() {
            return;
        }
        let v = catalog_status(&StatusOptions { catalog: Some(path), ..Default::default() }).unwrap();
        assert!(v["total"].as_u64().unwrap() > 0);
        assert!(v["implemented"].as_u64().unwrap() > 0);
        assert_eq!(v["stale_mappings"].as_array().unwrap().len(), 0);
    }
}
