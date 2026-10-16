//! Coverage scanner: an inventory of every effect, transition, title
//! template, animation, sticker and clip type a project uses. It drives
//! which Filmora features the renderer implements.

use crate::import::{all_effects, is_animation_chain, timelines};
use crate::json::*;
use crate::mapping::*;
use crate::WfpProject;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageCategory {
    Transition,
    /// Built-in per-clip effects (transform, crop, volume, fade, ...).
    BuiltIn,
    VideoEffect,
    ColorAdjustment,
    AudioEffect,
    Animation,
    /// Shape/pen drawing effects.
    Graphic,
    TitleTemplate,
    TextTemplate,
    /// Store resources placed as clips (stickers, elements, sound effects).
    Resource,
    Font,
    ClipType,
}

impl CoverageCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Transition => "Transitions",
            Self::BuiltIn => "Built-in clip effects",
            Self::VideoEffect => "Video effects / filters",
            Self::ColorAdjustment => "Color adjustments",
            Self::AudioEffect => "Audio effects",
            Self::Animation => "Animations",
            Self::Graphic => "Graphic (shape/pen) effects",
            Self::TitleTemplate => "Title templates",
            Self::TextTemplate => "Text templates",
            Self::Resource => "Store resources used as clips (stickers, elements, sounds)",
            Self::Font => "Fonts",
            Self::ClipType => "Clip types",
        }
    }
}

/// Store metadata of a resource (`userData` key 12 and `resinfo.json`).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ResourceInfo {
    pub res_id: Option<i64>,
    /// Store slug, e.g. `2_Fast_Zoom`.
    pub slug: Option<String>,
    /// 1 title, 2 transition, 3 filter/effect, 6 audio, ...
    pub res_type: Option<i64>,
    /// English title from the installed resource's `resinfo.json`.
    pub title: Option<String>,
    /// Installed resource folder, relative to the resource root.
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ParamUsage {
    /// Filmora paramType (2 float, 3 double, 5 int, 6 string).
    pub param_type: Option<i64>,
    pub count: usize,
    /// Distinct sample values (at most a handful).
    pub samples: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoverageItem {
    pub category: CoverageCategory,
    /// Filmora id (effect id, transition id, template slug, ...).
    pub id: String,
    /// Effect kind used in the Jibaru model (`filmora:<id>`), when applicable.
    pub kind: Option<String>,
    /// Best human name found.
    pub name: Option<String>,
    /// The `display` field stored in the project.
    pub display: Option<String>,
    pub resource: Option<ResourceInfo>,
    pub count: usize,
    /// `thisUId`s of the clips using it.
    pub clips: Vec<String>,
    /// Effect chains it was found in.
    pub chains: BTreeSet<String>,
    pub params: BTreeMap<String, ParamUsage>,
    /// Projects it was found in (filled by [`Coverage::merge`]).
    pub projects: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Coverage {
    pub items: Vec<CoverageItem>,
}

const MAX_SAMPLES: usize = 6;

impl Coverage {
    pub fn get(&self, category: CoverageCategory, id: &str) -> Option<&CoverageItem> {
        self.items.iter().find(|i| i.category == category && i.id == id)
    }

    /// Adds another coverage (e.g. of another project) into this one.
    pub fn merge(&mut self, other: &Coverage) {
        for it in &other.items {
            match self.items.iter_mut().find(|i| i.category == it.category && i.id == it.id) {
                Some(mine) => {
                    mine.count += it.count;
                    mine.clips.extend(it.clips.iter().cloned());
                    mine.chains.extend(it.chains.iter().cloned());
                    mine.projects.extend(it.projects.iter().cloned());
                    mine.name = mine.name.take().or_else(|| it.name.clone());
                    mine.display = mine.display.take().or_else(|| it.display.clone());
                    if mine.resource.is_none() {
                        mine.resource = it.resource.clone();
                    }
                    for (k, p) in &it.params {
                        let m = mine.params.entry(k.clone()).or_default();
                        m.count += p.count;
                        m.param_type = m.param_type.or(p.param_type);
                        for s in &p.samples {
                            if m.samples.len() < MAX_SAMPLES && !m.samples.contains(s) {
                                m.samples.push(s.clone());
                            }
                        }
                    }
                }
                None => self.items.push(it.clone()),
            }
        }
        self.sort();
    }

    fn sort(&mut self) {
        self.items.sort_by(|a, b| a.category.cmp(&b.category).then(b.count.cmp(&a.count)).then(a.id.cmp(&b.id)));
    }

    /// Fills names from an index of installed store resources.
    pub fn resolve(&mut self, index: &ResourceIndex) {
        for it in &mut self.items {
            let found = it
                .resource
                .as_ref()
                .and_then(|r| {
                    r.res_id
                        .and_then(|id| index.by_id.get(&id))
                        .or_else(|| r.slug.as_ref().and_then(|s| index.by_slug.get(s)))
                })
                .or_else(|| index.by_slug.get(&it.id))
                .cloned();
            if let Some(f) = found {
                let r = it.resource.get_or_insert_with(ResourceInfo::default);
                r.title = r.title.take().or(f.title.clone());
                r.folder = r.folder.take().or(f.folder.clone());
                r.res_id = r.res_id.or(f.res_id);
                r.slug = r.slug.take().or(f.slug.clone());
                r.res_type = r.res_type.or(f.res_type);
                if let Some(t) = &f.title {
                    it.name = Some(t.clone());
                }
            }
        }
    }

    /// A Markdown inventory, one table per category.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let mut cats: Vec<CoverageCategory> = self.items.iter().map(|i| i.category).collect();
        cats.dedup();
        for cat in cats {
            out += &format!("## {}\n\n", cat.label());
            out += "| Id | Name | Count | Projects | Parameters (type: samples) |\n|---|---|---:|---:|---|\n";
            for it in self.items.iter().filter(|i| i.category == cat) {
                let mut name = it.name.clone().unwrap_or_default();
                if let Some(d) = it.display.as_ref().filter(|d| Some(*d) != it.name.as_ref()) {
                    name += &format!(" (display `{d}`)");
                }
                if let Some(r) = &it.resource {
                    let mut bits = Vec::new();
                    if let Some(id) = r.res_id {
                        bits.push(format!("res {id}"));
                    }
                    if let Some(s) = &r.slug {
                        bits.push(format!("slug `{s}`"));
                    }
                    if let Some(t) = r.res_type {
                        bits.push(format!("type {t}"));
                    }
                    if !bits.is_empty() {
                        name += &format!(" [{}]", bits.join(", "));
                    }
                }
                let params: Vec<String> = it
                    .params
                    .iter()
                    .map(|(k, p)| {
                        let ty = p.param_type.map_or(String::new(), |t| format!("{t}"));
                        format!("`{k}` ({ty}: {})", p.samples.join(", "))
                    })
                    .collect();
                out += &format!(
                    "| `{}` | {} | {} | {} | {} |\n",
                    it.id,
                    name.replace('|', "\\|"),
                    it.count,
                    it.projects.len().max(1),
                    params.join("<br>").replace('|', "\\|")
                );
            }
            out += "\n";
        }
        out
    }
}

/// Index of installed store resources, built from the `resinfo.json`
/// metadata files of the resource folders (names and ids only).
#[derive(Debug, Clone, Default)]
pub struct ResourceIndex {
    pub by_id: HashMap<i64, ResourceInfo>,
    pub by_slug: HashMap<String, ResourceInfo>,
}

impl ResourceIndex {
    /// Scans `<root>/<category>/<folder>/resinfo.json`.
    pub fn scan(root: &Path) -> Self {
        let mut idx = Self::default();
        let Ok(cats) = std::fs::read_dir(root) else { return idx };
        for cat in cats.flatten() {
            let Ok(folders) = std::fs::read_dir(cat.path()) else { continue };
            for folder in folders.flatten() {
                let info = folder.path().join("resinfo.json");
                let Ok(text) = std::fs::read(&info) else { continue };
                let Ok(v) = serde_json::from_slice::<Value>(&text) else { continue };
                let rel = format!("{}/{}", cat.file_name().to_string_lossy(), folder.file_name().to_string_lossy());
                let r = ResourceInfo {
                    res_id: i64_of(&v, "id"),
                    slug: str_of(&v, "slug").map(str::to_string),
                    res_type: i64_of(&v, "type"),
                    title: v.get("titles").and_then(|t| str_of(t, "en")).map(str::to_string),
                    folder: Some(rel),
                };
                if let Some(id) = r.res_id {
                    idx.by_id.insert(id, r.clone());
                }
                if let Some(s) = r.slug.clone() {
                    idx.by_slug.insert(s, r);
                }
            }
        }
        idx
    }

    /// Default resource locations of a local Filmora installation.
    pub fn default_roots() -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Some(home) = std::env::var_os("HOME") {
            out.push(
                PathBuf::from(home)
                    .join("Library/Application Support/Wondershare Filmora Mac/ProgramData/Download/Filmora"),
            );
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            out.push(PathBuf::from(appdata).join("Wondershare/Wondershare Filmora/Download/Filmora"));
        }
        out
    }

    pub fn scan_default() -> Self {
        let mut idx = Self::default();
        for root in Self::default_roots() {
            let more = Self::scan(&root);
            idx.by_id.extend(more.by_id);
            idx.by_slug.extend(more.by_slug);
        }
        idx
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty() && self.by_slug.is_empty()
    }
}

struct Scan {
    items: Vec<CoverageItem>,
    pos: HashMap<(CoverageCategory, String), usize>,
    project: String,
}

impl Scan {
    fn item(&mut self, cat: CoverageCategory, id: &str) -> &mut CoverageItem {
        let key = (cat, id.to_string());
        let i = match self.pos.get(&key) {
            Some(i) => *i,
            None => {
                self.items.push(CoverageItem {
                    category: cat,
                    id: id.to_string(),
                    kind: None,
                    name: None,
                    display: None,
                    resource: None,
                    count: 0,
                    clips: Vec::new(),
                    chains: BTreeSet::new(),
                    params: BTreeMap::new(),
                    projects: BTreeSet::from([self.project.clone()]),
                });
                self.pos.insert(key, self.items.len() - 1);
                self.items.len() - 1
            }
        };
        &mut self.items[i]
    }

    fn hit(&mut self, cat: CoverageCategory, id: &str, clip: Option<&str>) -> &mut CoverageItem {
        let it = self.item(cat, id);
        it.count += 1;
        if let Some(c) = clip {
            if !it.clips.iter().any(|x| x == c) {
                it.clips.push(c.to_string());
            }
        }
        it
    }
}

fn resource_of(v: &Value) -> Option<ResourceInfo> {
    let j: Value = serde_json::from_str(&user_data_str(v, ud::RESOURCE)?).ok()?;
    Some(ResourceInfo {
        res_id: i64_of(&j, "id"),
        slug: str_of(&j, "name").map(str::to_string),
        res_type: i64_of(&j, "res_type"),
        title: None,
        folder: None,
    })
}

/// Title template info from userData key 55 (`key_timeline_resource_info`).
fn timeline_template(v: &Value) -> Option<(String, Option<String>)> {
    let j: Value = serde_json::from_str(&user_data_str(v, ud::TIMELINE_INFO)?).ok()?;
    let info: Value = serde_json::from_str(str_of(&j, "key_timeline_resource_info")?).ok()?;
    let name = str_of(&info, "name")?.to_string();
    let pack = str_of(&info, "key_resource_packname").map(str::to_string);
    Some((name, pack))
}

fn effect_category(chain: &str, id: &str) -> CoverageCategory {
    if is_animation_chain(chain) {
        CoverageCategory::Animation
    } else if chain == "Color" {
        CoverageCategory::ColorAdjustment
    } else if chain == "graphic" || id.starts_with("graphic/") {
        CoverageCategory::Graphic
    } else if matches!(id, FX_TRANSFORM | FX_CROP | FX_VOLUME | FX_FADE) || STRUCTURAL_EFFECTS.contains(&id) {
        CoverageCategory::BuiltIn
    } else if id.starts_with("audio/") {
        CoverageCategory::AudioEffect
    } else {
        CoverageCategory::VideoEffect
    }
}

fn record_params(it: &mut CoverageItem, e: &Value) {
    for p in e.get("paramList").and_then(Value::as_array).into_iter().flatten() {
        let Some(name) = str_of(p, "name") else { continue };
        let fx = p.get("fxParam");
        let u = it.params.entry(name.to_string()).or_default();
        u.count += 1;
        u.param_type = u.param_type.or_else(|| fx.and_then(|f| i64_of(f, "paramType")));
        if let Some(v) = fx.and_then(|f| f.get("unValue")) {
            let mut s = v.to_string();
            if s.len() > 40 {
                s = format!("{}…", s.chars().take(40).collect::<String>());
            }
            if u.samples.len() < MAX_SAMPLES && !u.samples.contains(&s) {
                u.samples.push(s);
            }
        }
    }
}

/// Inventory of everything a project uses.
pub fn coverage(wfp: &WfpProject) -> Coverage {
    let mut scan = Scan { items: Vec::new(), pos: HashMap::new(), project: wfp.project.name.clone() };
    for tl in timelines(&wfp.raw.timeline) {
        for t in tl.get("trackInfos").and_then(Value::as_array).into_iter().flatten() {
            for c in t.get("clipList").and_then(Value::as_array).into_iter().flatten() {
                scan_clip(&mut scan, c);
            }
        }
    }
    let mut cov = Coverage { items: scan.items };
    cov.sort();
    cov
}

fn clip_type_name(ty: i64) -> &'static str {
    match ty {
        CLIP_VIDEO => "video / image",
        CLIP_AUDIO => "audio",
        CLIP_TEXT => "text (scriptBuf)",
        CLIP_TIMELINE => "nested timeline (title template / compound)",
        CLIP_GRAPHIC => "graphic (pen / shape)",
        _ => "unknown",
    }
}

fn scan_clip(scan: &mut Scan, c: &Value) {
    let uid = str_of(c, "thisUId").map(str::to_string);
    let uid = uid.as_deref();
    let ty = i64_of(c, "type").unwrap_or(0);
    let it = scan.hit(CoverageCategory::ClipType, &ty.to_string(), uid);
    it.name = Some(clip_type_name(ty).to_string());

    for (chain, e) in all_effects(c) {
        let id = str_of(e, "id").unwrap_or("");
        let cat = effect_category(chain, id);
        let it = scan.hit(cat, id, uid);
        it.kind = Some(format!("filmora:{id}"));
        it.chains.insert(chain.to_string());
        if it.display.is_none() {
            it.display = str_of(e, "display").map(str::to_string);
            it.name = it.display.clone();
        }
        if it.resource.is_none() {
            it.resource = resource_of(e);
        }
        record_params(it, e);
    }
    for key in ["preTransition", "postTransition"] {
        let Some(t) = c.get(key).filter(|t| t.is_object()) else { continue };
        let id = str_of(t, "id").unwrap_or("");
        let it = scan.hit(CoverageCategory::Transition, id, uid);
        it.kind = Some(format!("filmora:{id}"));
        it.chains.insert(key.to_string());
        if it.display.is_none() {
            it.display = str_of(t, "display").map(str::to_string);
        }
        if it.resource.is_none() {
            it.resource = resource_of(t);
        }
        if it.name.is_none() {
            it.name = it.resource.as_ref().and_then(|r| r.slug.clone()).or_else(|| it.display.clone());
        }
    }
    match ty {
        CLIP_TIMELINE => {
            let res = resource_of(c);
            let tmpl = timeline_template(c);
            let id = res
                .as_ref()
                .and_then(|r| r.slug.clone())
                .or_else(|| tmpl.as_ref().map(|t| t.0.clone()))
                .unwrap_or_else(|| "nested timeline".into());
            let it = scan.hit(CoverageCategory::TitleTemplate, &id, uid);
            if it.resource.is_none() {
                it.resource = res;
            }
            if let Some((name, pack)) = tmpl {
                it.display = Some(match pack {
                    Some(p) => format!("{name} ({p})"),
                    None => name,
                });
            }
            it.name = it.name.take().or_else(|| user_data_str(c, ud::NAME));
        }
        CLIP_TEXT => {
            let name = timeline_template(c).map_or_else(|| "text".to_string(), |t| t.0);
            scan.hit(CoverageCategory::TextTemplate, &name, uid).name = Some(name.clone());
            if let Some(script) = nested(c, "scriptBuf") {
                for td in script.get("TextData").and_then(Value::as_array).into_iter().flatten() {
                    if let Some(font) = td.get("Basic").and_then(|b| str_of(b, "FontName")) {
                        scan.hit(CoverageCategory::Font, font, uid).name = Some(font.to_string());
                    }
                }
                let it = scan.item(CoverageCategory::TextTemplate, &name);
                for (k, path) in [
                    ("Border.Enable", &["Border", "Enable"][..]),
                    ("Shadow.Enable", &["Shadow", "Enable"][..]),
                    ("Basic.FontSize", &["Basic", "FontSize"][..]),
                    ("Basic.TextColor", &["Basic", "TextColor"][..]),
                ] {
                    let mut v = script.get("TextData").and_then(|t| t.get(0));
                    for p in path {
                        v = v.and_then(|x| x.get(*p));
                    }
                    if let Some(v) = v {
                        let u = it.params.entry(k.to_string()).or_default();
                        u.count += 1;
                        let s = v.to_string();
                        if u.samples.len() < MAX_SAMPLES && !u.samples.contains(&s) {
                            u.samples.push(s);
                        }
                    }
                }
                for k in ["PosX", "PosY", "ScaleX", "ScaleY"] {
                    if let Some(v) = script.get(k) {
                        let u = it.params.entry(k.to_string()).or_default();
                        u.count += 1;
                        let s = v.to_string();
                        if u.samples.len() < MAX_SAMPLES && !u.samples.contains(&s) {
                            u.samples.push(s);
                        }
                    }
                }
            }
        }
        _ => {
            if let Some(r) = resource_of(c) {
                let id = r.slug.clone().unwrap_or_else(|| r.res_id.map_or("resource".into(), |i| i.to_string()));
                let it = scan.hit(CoverageCategory::Resource, &id, uid);
                it.name = it.name.take().or_else(|| r.slug.clone());
                it.resource = Some(r);
            }
        }
    }
}
