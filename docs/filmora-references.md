# Filmora reference renders

Reference exports made **by Filmora 15.7.3 (Mac)** of test projects written
with Jibaru's `.wfp` writer: one transition per cut, or one effect/filter per
pair of clips, on our own synthetic footage. They are the ground truth for
fitting clean-room shaders to Filmora's look (`docs/fidelity.md` describes the
comparison harness, `docs/filmora-catalog.md` the item inventory).

No media is in the repository. The test footage, batch projects and exports
live in the scratch directory of the machine that produced them (`root` and
relative paths in `docs/filmora-references.json`); they are regenerated with the steps
below. Only Filmora's exported output and item names/ids were used; no
Filmora asset file (shader, LUT, overlay, image) was read or copied.

## Machine-readable index

`docs/filmora-references.json`:

| Field | Meaning |
|---|---|
| `export` | Export settings (below). |
| `media` | The four test clips: path and description. |
| `layout` | How items are placed in a batch. |
| `batches[]` | `id`, `kind` (`transition`/`effect`), `project` (.wfp), `export` (.mp4, `null` if not exported), `timeline_duration_s`, `frames`, `video_bitrate_bps`, `size_bytes`, `media`, `items[]`. |
| `items[]` | `index`, `name`, `catalog_key` (key in the catalog builder's output), `in_repo_catalog` (whether that key is in `docs/filmora-catalog.json`), `guid` (**exact case**, as Filmora needs it), `res_id`, `slug`, `kind`, `category`, `premium`, `builtin`, the item window, `max_dev_from_plain`, `status`, and `rerendered_in` for superseded items. |
| `summary` | Item counts per kind and status. |
| `downloads` | Download walk results (rows seen per final icon state). |
| `skipped` | Downloaded catalog items not put in a batch, with the reason. |

Item windows are in seconds of the export (frame `n` is at `n × 1001 / 30000`):

- **Transitions**: `start_s` … `end_s` (centred on `cut_s`, `duration_s` long),
  outgoing clip `from`, incoming clip `to`. At least 0.5 s of plain clip
  separates consecutive transitions, so the clean frames just outside the
  window are the A and B references.
- **Effects / filters**: `grid_s` (the effect on gridA) and `scene_s` (the
  same effect on sceneA), 2.002 s each. Both clips play their source from
  1.001 s, so the unprocessed reference frame for export time `t` is source
  frame `round((1.001 + t − start) × 30000 / 1001)` of the clip's media.

`status`:

| Status | Meaning |
|---|---|
| `rendered` | The window differs from the plain clips (max luma MAD > 2 at 160×90): Filmora applied the item. |
| `not_rendered` | The window looks like the plain clips: Filmora did not apply the item as written (built-in engine nodes, overlay resources and some built-in filters need fields the writer does not set yet), or the item is audio-only. Do not fit against these. |
| `superseded` | Written with an upper-cased GUID and not applied; re-rendered in the batch named by `rerendered_in` (see *GUID case* below). |
| `not_exported` | The batch project is built but was not exported (disk space ran out); export it with the steps below. |

Extracting one item, e.g. a transition at 1080p:

```sh
ffmpeg -ss <start_s - 0.2> -to <end_s + 0.2> -i <export> -c copy item.mp4      # or decode frames
cargo run --release -p jb-cli --example fidelity -- <project> <export> \
    --range <start_s>s-<end_s>s --size 960x540 --out DIR    # ours vs Filmora's on the same project
```

## Export settings

Export → Local, *Match to project settings*: MP4, H.264, 1920×1080,
29.97 fps (30000/1001), SDR Rec.709, quality **Low** (6000 kbit/s target,
≈ 6.1 Mbit/s measured; chosen to fit the disk budget), AAC stereo (silent).
Filmora's *Auto Highlight* side task was cancelled for every export. Projects
are 1920×1080 at 29.97 fps (`template.wfp`, saved by Filmora; every batch is
written from it with `jb exec`).

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
  cut with Filmora's default length 2.002 s (60 frames). 40 transitions,
  102.5 s per batch.
- **Effect batches** (`fxNNN`): per item a gridA clip and a sceneA clip,
  2.002 s each, the item added with default parameters (`Effect` chain,
  `paramList` omitted: Filmora then uses the item's defaults).

Items come from the catalog builder's output after the downloads (local
metadata only), filtered to downloaded transitions, effects and filters with
a GUID, sorted by category and name.

## Batches

| Batch | Kind | Items | Export (MB) | Status | Categories |
|---|---|---:|---:|---|---|
| tr001 | transition | 40 | 78.5 | rendered 38, superseded 2 | uncategorised, Basic |
| tr002 | transition | 40 | 77.8 | rendered 32, not_rendered 4, superseded 4 | Basic → Cubed (built-in engine transitions) |
| tr003 | transition | 40 | 80.9 | rendered 33, superseded 7 | Cyber Monday → Horror |
| tr004 | transition | 40 | 78.4 | rendered 28, superseded 12 | Lifestyle → Plain Shape |
| tr005–tr012 | transition | 320 | ≈ 79 each | all rendered | Replace, Sports, Top Picks, 3D, Brush, Camera |
| trfix1 | transition | 25 | 49.6 | all rendered | the superseded items of tr001–tr004 |
| tr013–tr036 | transition | 929 | — | not exported | Camera → Zoom, Warp, World Cup |
| fx001 | effect/filter | 40 | 124.7 | rendered 18, not_rendered 20, superseded 2 | store effects, built-in filters A–N |
| fx002 | effect/filter | 40 | 122.3 | rendered 16, not_rendered 19, superseded 5 | built-in filters N–W, store filters |
| fx003 | effect | 50 | 154.7 | rendered 46, not_rendered 4 | store Video Effects (overlay resources removed) |
| fxfix1 | effect/filter | 7 | 21.9 | all rendered | the superseded items of fx001–fx002 |

Totals: **476 transitions and 87 effects/filters rendered** in 17 exports
(1.3 GB); 929 transitions are in built but unexported batches.

To export the remaining batches: open `batches/trNNN.wfp` in Filmora
(File → Open Project), Export → Local with the settings above into the
`exports/` folder, then run the verification (`verify.py trNNN` in the
scratch directory: decodes the export at 160×90 and records
`max_dev_from_plain` per item) and regenerate the JSON.

## Findings

- **GUID case.** Filmora resolves store resources by their GUID
  case-sensitively: `0bc06bc0-…` (Fisheye Roll Down) is applied, the same
  GUID upper-cased is silently dropped (the cut renders as a hard cut). 272
  of 1,816 resource GUIDs are not upper case (lower or mixed). The catalog
  stores GUIDs upper-cased for joining; anything that *writes* a Filmora
  project must use the exact case, which `docs/filmora-references.json`
  keeps (`guid`). Reading is unaffected (`aliases.rs` registers both cases).
- **Minimal transition JSON works.** `{"display","id","type":5,"userData":[GUID]}`
  with `tlBegin`/`tlEnd` is enough; Filmora adds `virtualTransitionId` on
  save. Its own userData 80 (1), 8 (3) and 12 (store record) are not needed
  (tested: adding them changes nothing).
- **Effects at defaults.** A store effect or filter written with only
  `id`/`display` (no `paramList`) renders with its default parameters.
  Overlay resources (store type 4: light leaks, film looks) and several
  built-in filters (Black & White 1–5, Grey, …) are not applied from such a
  minimal entry; they need GUI insertion to learn the missing fields.
  Several built-in looks share the engine GUID `BB101941-…` (1977, Amaro,
  Brannan, …); their windows measure differently, which suggests Filmora
  picks the look from the effect's `display` name (the writer sets it to
  the item name). Compare them with each other before fitting.
- **Transition length.** Dropping *Fisheye Roll Down* from the library onto
  a cut between 2.5 s clips gave it 1.001 s, not 2.002 s; the batches set
  2.002 s explicitly for every item.
- **Not rendered transitions** that remain (4, tr002) are built-in engine
  transitions (`wfx_effect/Transition`, catalog keys `builtin:`).

## Downloads

The library was walked in **list view** (one row per item: name, premium
gem, download/add icon) by a script that screenshots the list, reads the
names with macOS Vision OCR, classifies the action icon against two
templates and clicks the download icons, scrolling through every category
(the list auto-advances into the next category). The account is a paid
(VIP) plan: premium items downloaded without any purchase prompt; no trial
was started and no AI credit was used (balance unchanged at 1,000).

| Tab | Result |
|---|---|
| Transitions | Every category walked (Trending … New Year): **1,343 transitions + 104 SFX transitions downloaded** (≈ 4.5 GB), 1,308 of the rows seen premium. One item opened Filmora's "Similar Results" search instead of downloading (*Anime Attack 03*, recorded as blocked). About 100 rows stayed undownloaded because truncated names (`Halloween Cinematic Transition …`) collided in the walker's de-duplication; a later fix (always clicking visible download icons) was cut short by disk space. |
| Effects, Filters, Titles, Animations, Stickers, Overlays | Not walked: the disk (≈ 18 GB free at the start, shared with other jobs) dropped to ≈ 1 GB, so downloads stopped after transitions. Effects/filters already installed (≈ 150 with a GUID) were batched instead. |
| AI Transitions / AI effects | Skipped on purpose (they generate content and consume credits). |

Titles and title animations have no batches yet: Jibaru's writer only
creates Basic title clips, so title templates must be placed in Filmora by
hand.
