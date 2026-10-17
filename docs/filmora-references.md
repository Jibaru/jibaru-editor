# Filmora reference renders

Reference exports made **by Filmora 15.7.3 (Mac)** of test projects: one
transition per cut, one effect/filter per pair of clips (projects written
with Jibaru's `.wfp` writer), and title templates and Motion presets placed
in Filmora itself, all on our own synthetic footage. They are the ground
truth for fitting clean-room shaders to Filmora's look (`docs/fidelity.md`
describes the comparison harness, `docs/filmora-catalog.md` the item
inventory).

No media is in the repository. The test footage, batch projects and exports
live in the scratch directory of the machine that produced them (`root` and
relative paths in `docs/filmora-references.json`); they are regenerated with
the steps below. Only Filmora's exported output and item names/ids were
used; no Filmora asset file (shader, LUT, overlay, image) was read or copied.

## Machine-readable index

`docs/filmora-references.json`:

| Field | Meaning |
|---|---|
| `export` | Export presets (below); `batches[].export_preset` says which one a batch used. |
| `media` | The four test clips: path and description. |
| `layout` | How items are placed in each kind of batch. |
| `batches[]` | `id`, `kind` (`transition`/`effect`/`title`/`animation`), `project` (.wfp), `export` (.mp4), `timeline_duration_s`, `frames`, `video_bitrate_bps`, `size_bytes`, `export_preset`, `store_records`, `clip_s` (effect clip length), `media`, `items[]`. |
| `items[]` | `index`, `name`, `catalog_key` (key in `docs/filmora-catalog.json`), `in_repo_catalog`, `guid` (**exact case**, as Filmora needs it; `null` for titles and Motion presets, which Filmora stores by store id), `res_id`, `slug`, `kind`, `category`, `premium`, `builtin`, the item window, `max_dev_from_plain`, `status`, and `rerendered_in` for superseded items. |
| `summary` | Item counts per kind and status. |
| `downloads` | Download walk results per library tab (last state seen per row name). |
| `skipped` | Downloaded catalog items not put in a batch, with the reason. |

Item windows are in seconds of the export (frame `n` is at `n × 1001 / 30000`):

- **Transitions**: `start_s` … `end_s` (centred on `cut_s`, `duration_s` long),
  outgoing clip `from`, incoming clip `to`. At least 0.5 s of plain clip
  separates consecutive transitions, so the clean frames just outside the
  window are the A and B references.
- **Effects / filters / overlays**: `grid_s` (the item on gridA) and
  `scene_s` (the same item on sceneA). Both clips play their source from
  1.001 s, so the unprocessed reference frame for export time `t` is source
  frame `round((1.001 + t − start) × 30000 / 1001)` of the clip's media.
  Clips are 2.002 s in fx001–fx003/fxfix1 and 1.001 s from fx004 on (disk
  budget).
- **Titles** (`tt001`): `window_s`, one template after another on a track
  above gridA clips of 5.005 s (source from 1.001 s), each at its default
  duration and default text.
- **Animations** (`an001`): `window_s` is the whole 2.002 s gridA clip the
  Motion preset was applied to; `slot` (`in`/`out`) and `anim_duration_s`
  (as Filmora saved it; `null` = the preset's own default).

`status`:

| Status | Meaning |
|---|---|
| `rendered` | The window differs from the plain clips (max luma MAD > 2 at 160×90): Filmora applied the item. |
| `not_rendered` | The window looks like the plain clips: Filmora did not apply the item as written (built-in engine nodes, utility effects such as BlendMode/Crop/Image Mask/Corner Pin that do nothing at defaults, text-only effects), the item is audio-only (SFX), or a title template is too small or faint to pass the threshold. Do not fit against these. |
| `superseded` | Written with an upper-cased GUID and not applied; re-rendered in the batch named by `rerendered_in` (see *GUID case* below). |

Extracting one item, e.g. a transition at 1080p:

```sh
ffmpeg -ss <start_s - 0.2> -to <end_s + 0.2> -i <export> -c copy item.mp4      # or decode frames
cargo run --release -p jb-cli --example fidelity -- <project> <export> \
    --range <start_s>s-<end_s>s --size 960x540 --out DIR    # ours vs Filmora's on the same project
```

## Export settings

Export → Local, MP4, H.264, 1920×1080, 29.97 fps (30000/1001), SDR Rec.709,
AAC stereo (silent). Filmora's *Auto Highlight* side task was cancelled for
every export. Two quality presets were used:

| Preset | Setting | Measured video bitrate | Batches |
|---|---|---|---|
| Low | *Match to project settings*, Quality Low (6000 kbit/s target) | ≈ 6.1 Mbit/s | tr001–tr012, trfix1, fx001–fx003, fxfix1 |
| JB15M | custom preset saved in Filmora (Advanced: H.264, VBR, 15000 kbit/s) | ≈ 14.8 Mbit/s on transitions and effects, ≈ 8–10 Mbit/s on mostly static content | everything from tr013 on, fx004 on, tt001, an001 |

Projects are 1920×1080 at 29.97 fps (`template.wfp`, saved by Filmora; every
batch is written from it with `jb exec`).

## Test media

| Clip | Content |
|---|---|
| gridA | Still test card, 10 s: red→blue diagonal gradient, 16×9 numbered 120 px tiles (A0…A143) with a 10 px checker in alternate tiles, black grid lines, white circle (r 260 px) and letter A at the centre, white top-left and black bottom-right corner squares; a black bar at y 1062–1080 carries an 8 px white marker at x = 6.4 px × source frame (shows which source frame is on screen). |
| gridB | Same layout: green↔yellow anti-diagonal gradient, tiles B0…B143, letter B. |
| sceneA / sceneB | Jibaru's procedural sample scenes (`jb_render::preview::SampleSource`: A mountain lake panning right, B sunset sea panning left) rendered at 1920×1080, 10 s. sceneB is imported in every project but only sceneA is used by the effect batches. |

All four are libx264 CRF 14, yuv420p, BT.709, 300 frames at 30000/1001. The
card is drawn with CoreGraphics (a 90-line Swift script) and looped with
`ffmpeg -loop 1`, the marker overlaid with `overlay=x='n*6.4'`.

## Layout

- **Transition batches** (`trNNN`): one video track, clips alternating
  gridA/gridB, each playing its source from 3.003 s (handles, so Filmora
  never freezes edge frames), a cut every 2.5 s (longer when a transition
  is), one transition per cut written as `postTransition` centred on the
  cut with length 2.002 s (60 frames). 40 transitions, 102.5 s per batch.
- **Effect batches** (`fxNNN`): per item a gridA clip and a sceneA clip, the
  item added with default parameters (`Effect` chain, `paramList` omitted:
  Filmora then uses the item's defaults). Effects, filters and overlays
  share these batches (`items[].kind`).
- **Title batch** (`tt001`): 40 gridA background clips of 5.005 s; the
  templates were added in Filmora with the `+` button of each row of
  *Titles → Title Templates → Lower Thirds* (list view), which inserts at the
  playhead and moves the playhead to the title's end; then the project was
  saved and the windows read back from the saved project.
- **Animation batch** (`an001`): 40 gridA clips of 2.002 s; clip *k* was
  selected and the *k*-th preset of *Animation → Motion Presets → In*
  (clips 0–22) or *Out* (clips 23–39) clicked; one Out cell was *No Motion*
  (clip 32 has no preset), so 39 presets. Names, slots and durations come
  from the saved project.

Items come from the local library metadata after the downloads (names and
ids only), filtered to downloaded store items with a GUID that have no
rendered reference yet, sorted by kind, category and name.

## Batches

| Batch | Kind | Items | Export (MB) | Status | Categories |
|---|---|---:|---:|---|---|
| tr001–tr004 | transition | 160 | ≈ 79 each | rendered 131, not_rendered 4, superseded 25 | uncategorised → Plain Shape |
| tr005–tr012 | transition | 320 | ≈ 79 each | all rendered | Replace, Sports, Top Picks, 3D, Brush, Camera |
| trfix1 | transition | 25 | 49.6 | all rendered | the superseded items of tr001–tr004 |
| tr013–tr036 | transition | 929 | ≈ 190 each (tr036: 43) | rendered 928, not_rendered 1 (*Color Dissolve 1 SFX*) | Camera → Zoom, Warp, World Cup |
| tr037–tr057 | transition | 814 | ≈ 190 each (tr057: 68) | rendered 813, not_rendered 1 (*Color Dissolve 1 SFX*, a second store id) | downloads of this pass: New Year → Y2K, Wedding & Love |
| fx001–fx003 | effect/filter | 130 | 122–155 | rendered 80, not_rendered 43, superseded 7 | store effects, built-in filters |
| fxfix1 | effect/filter | 7 | 21.9 | all rendered | the superseded items of fx001–fx002 |
| fx004–fx027 | effect/filter/overlay | 957 | ≈ 152 each | rendered 926, not_rendered 31 | Video Effects (Basic → Split Screen), Filters (Basic → Valentine), overlays |
| tt001 | title | 40 | 216.8 | rendered 38, not_rendered 2 (small/faint) | Title Templates / Lower Thirds |
| an001 | animation | 39 | 149.5 | all rendered | Video Animation / In, Out (Motion presets) |

Totals: **2,217 transitions, 1,013 effects/filters/overlays, 38 title
templates and 39 Motion presets rendered** in 88 exports (13.7 GB). Per-item
details, including every `not_rendered` item, are in the JSON.

Not rendered in this pass (fx004 on): utility effects that are invisible at
default settings (*BlendMode*, *Crop*, *Image Mask*, *Corner Pin*, *Basic
3D*, *Allpurpose Skin Tone*, *Beauty Skin*, *Smart Denoise*, *UFT Ripple*),
text-only effects (*Text Glow*, *Text Color Glow*), border effects that draw
on an alpha edge (*Dual Neon Border*, *Glow Border*, *Neon Border 1*,
*Particle Border*, *Solid Border*, *Dynamic Chalk Stroke*), SFX items whose
picture change is too small (*Bounce SFX*, *RGB Glitch SFX*, *TV Closing
SFX*, *Zoom Shake*), and weak grades just under the threshold (*Depth
Temperature*, *Saturation*, *Golden Glade*, *Stadium HDR*, *Soft Matte 01/11*,
*Dark Vibe 07*, *Glitter Overlay 10*, *Noise Texture*, *Sequence Zooming
B&W*).

To export a batch: open `batches/<id>.wfp` in Filmora (File → Open Project),
Export → Local with preset JB15M into the `exports/` folder, then run the
verification (`verify.py <id>`, or `tt_map.py` / `an_map.py` for the GUI
batches: decodes the export at 160×90 and records `max_dev_from_plain` per
item) and regenerate the JSON (`update_refs.py`).

## Findings

- **Store records are required in a fresh Filmora session.** A store
  transition or effect written with only its GUID renders only if Filmora
  already loaded that resource in the running session (the first export
  pass worked because the items had just been downloaded). After a restart,
  Filmora logs `EffectManager GetEffectInfo failed: <GUID>` and drops them
  (tr013 exported with 4 of 40 transitions, the built-in ones; tr008, which
  rendered fully before, re-exported with none). Adding Filmora's store
  record as userData 12 next to the GUID — `{"id": <res_id>, "infoType": 7,
  "is_custom": 0, "sale_mode": 0|1, "source": 5, "res_type": 2, "res_sub_type":
  <sub>, "name": "<slug>", ...}` — makes them render (tr013: 40 of 40). The
  writer now emits it from `extra.filmora.resource` on a transition or effect
  (`crates/jb-filmora/src/export.rs`, `set_store_record`); every batch from
  tr013 and fx004 on carries it (`store_records`).
- **GUID case.** Filmora resolves store resources by their GUID
  case-sensitively: `0bc06bc0-…` (Fisheye Roll Down) is applied, the same
  GUID upper-cased is silently dropped. About 270 store GUIDs are lower or
  mixed case. `docs/filmora-catalog.json` now stores the exact case; the
  `filmora:<GUID>` alias lookup in `jb-render` matches any case, and the
  writer writes GUIDs exactly as imported.
- **Minimal transition JSON works** with the store record above:
  `{"display","id","type":5,"userData":[GUID, store record]}` with
  `tlBegin`/`tlEnd`. Filmora adds `virtualTransitionId` on save.
- **Effects at defaults.** A store effect, filter or overlay written with
  `id`/`display`, no `paramList` and its store record renders with its
  default parameters, overlays (store type 4) included. The earlier
  `not_rendered` overlays of fx001–fx002 were written without store records.
  Several built-in looks share the engine GUID `BB101941-…` (1977, Amaro,
  Brannan, …); their windows measure differently, which suggests Filmora
  picks the look from the effect's `display` name.
- **Transition length.** Dropping *Fisheye Roll Down* from the library onto
  a cut between 2.5 s clips gave it 1.001 s; the batches set 2.002 s
  explicitly for every item.
- **Title templates** inserted with `+` get their default duration (5 s for
  nearly all lower thirds; a few are longer, so the track runs to 203 s) and
  land on the next free track above the clip.

## Downloads

The library was walked in **list view** (one row per item: name, premium
gem, download/add icon) by a script that screenshots the list, reads the
names with macOS Vision OCR, classifies the action icon against two
templates and clicks the download icons, scrolling through every category.
The account is a paid (VIP) plan: premium items downloaded without any
purchase prompt; no trial was started and no AI credit was used (balance
unchanged at 1,000). Items whose name starts with *AI* were skipped on
purpose.

The first walk lost rows whose truncated names repeated
(`Halloween Cinematic Transition …`): after clicking one, the re-located
row was looked up by name only and resolved to an already downloaded twin.
The walker now re-locates the nearest undownloaded row with that name;
re-walking the Transitions tab downloaded **≈ 800 more transitions**
(Trending, Halloween → New Year, Warp → Christmas, Valentine), which are
batches tr037–tr057.

| Tab | Result |
|---|---|
| Transitions | Every category walked again with the fixed walker: 2,198 rows downloaded in total (≈ 2,400 transition items in the library metadata); 34 rows still showed the download icon after the click (download still running or refused), *Anime Attack 03* opens Filmora's "Similar Results" search instead of downloading. |
| Filters | Basic, Annual Highlights (the list continues into Summer … Camera), Mono, Cinematic walked: 360 downloaded, 10 not confirmed, 2 AI filters skipped. The remaining filter categories were not re-walked (they repeat the same items). |
| Effects | Basic, Zoom, Shake (the list continues through most Video Effects categories) and Multi-Frame walked: 606 downloaded, 13 not confirmed. Stopped at the disk budget: Object Outline, 3D, Flicker, Grunge, Slice&Shatter, Split Screen, Blur, Retro, Logo Effects, Slideshow, Exclusive, Sound-Enabled, Pet Effects, Halloween, Valentine, Film Burns, Starburst, Celebration, Particles, Nature, Bokeh, Light and Lens Flare were not walked on their own (many of their items were reached through the continuing lists). |
| Titles | Lower Thirds walked (the list continues into Social Media): 197 downloaded (≈ 13 MB each), then the disk budget was reached. Subtitles, openers and the other ≈ 7,800 template rows were not downloaded. |
| Animations | The Motion presets used in an001 (23 In, 16 Out) were downloaded from the Animation panel when applied. Keyframe presets and text animations were not downloaded. |
| Stickers / Elements | Not walked (disk budget). |
| AI Transitions / AI effects / AI filters | Skipped on purpose (they generate content and consume credits). |

The pass was capped at ≈ 25 GB of new data: ≈ 11.7 GB of downloads (the
Filmora download folder grew from 4.6 to 16.3 GB) and ≈ 12.4 GB of new
exports.
