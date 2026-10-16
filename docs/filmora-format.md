# Filmora `.wfp` project format

Reverse-engineered reference for the project files written by Wondershare
Filmora 15 (Mac 15.0–15.7, `serializationVersion` 27–28). It was derived by
black-box analysis of project files only (four user projects plus one preset
project shipped as a sample) for interoperability; nothing here comes from
Filmora's code or assets. `crates/jb-filmora` implements it.

Status markers used below:

- **verified**: observed consistently across the corpus and checked by tests;
- **inferred**: consistent with the corpus but only one or two samples exist;
- **unverified**: not present in the corpus; the implementation guesses and
  says so in its warnings.

Times are in **ticks of 100 ns** (10,000,000 per second) unless a field is
explicitly in seconds; this is also `jb_core::Ticks`.

## 1. Container

A `.wfp` is a plain ZIP archive. Entries are a mix of *stored* (media
descriptions, thumbnails) and *deflated* (documents) members, all with
Unix mode `0100666`. No encryption, no comment, no extra fields.

| Entry | Content |
|---|---|
| `ProjectFolder/project_info.json` | Project settings (pretty JSON, 4-space indent). |
| `ProjectFolder/Medias/medias_info.json` | Media bin (compact JSON). |
| `ProjectFolder/Medias/<timeline_mediaId>/timeline.wesproj` | **The timeline** (compact JSON, keys sorted). |
| `ProjectFolder/Medias/<timeline_mediaId>/extra.json` | Fonts, template usage, clip→media map, markers. |
| `ProjectFolder/Medias/<GUID>/media.json` | Probe info of one media bin item. |
| `ProjectFolder/Medias/<GUID>/thumbnail.png` | Media bin thumbnail. |
| `ProjectFolder/Anon/Cover/thumb.fsthumb` | Project cover (a PNG). |
| `ProjectFolder/Anon/AppData/MacOS/functionExtraData.json` | Small opaque map, e.g. `{"9":"2"}`. |

GUIDs everywhere are Filmora's own notation: 16 uppercase hex bytes joined by
dashes, e.g. `18-73-4F-E3-A3-93-4C-57-9C-64-A3-11-F4-4B-D1-1C`. Object
identities inside the timeline (`thisUId`, track `uuid`, bus ids) are
ordinary lowercase UUIDs.

Every JSON object Filmora writes has its keys in **byte-wise alphabetical
order** (verified); the writer inserts new keys in sorted position.

## 2. `project_info.json`

| Field | Meaning |
|---|---|
| `project_file_name` | Project name shown in the UI. |
| `project_timeline_resolution` | `[width, height]`. |
| `project_timeline_framerate` | `[num, den]`, e.g. `[30000, 1001]`. |
| `project_timeline_ratio` | `[16, 9]`. |
| `project_sample_rate` | e.g. `44100`. |
| `project_timeline_duration` | Main timeline duration (ticks). |
| `project_current_position` | Playhead, in frames. |
| `timeline_mediaId` | Media bin id of the timeline item; names the folder of `timeline.wesproj`. |
| `project_guid` | Project GUID (also used for the backup folder). |
| `proj_zip_save_path`, `proj_cover_proj_path` | Absolute paths at save time. |
| `project_editor_*_version`, `vbl_*_version` | Filmora and engine versions. |
| `project_date_create`, `project_date_modify` | Unix seconds. |
| `project_color_space`, `project_audio_bitdepth`, `project_player_*`, `project_archive`, `project_storage`, `project_backup`, `project_cover`, `project_custom_cover`, `project_type`, `project_template_name`, `nSizeRatioType`, `project_used_pack_list`, `project_source`, `bRemoveProjectTypeFor*` | Opaque, preserved. |
| `user_data` | `[{key, data}]` with plain string data (not base64). |

## 3. `medias_info.json`

```json
{"media_structure": {"visible": "true", "SerializeDataOnlyProjectUsered": "false",
                     "Folder": {..., "media_item": "<GUID>" | ["<GUID>", ...]},
                     "media_item": "<timeline GUID>"},
 "media_items": {"<GUID>": {...}}}
```

`media_structure` is the bin folder tree; `media_item` is a string for one
child and an array for several. Each `media_items` entry:

| Field | Meaning |
|---|---|
| `id` | Its own GUID (same as the key). |
| `download_url` | Absolute path of the file (empty for the timeline item). |
| `name` | Display name (file stem). |
| `media_type` | Bit flag: `8` video, `16` image, `262144` audio, `1048576` timeline. |
| `media_length` | Duration in ticks (images: `50000000`, the default 5 s). |
| `create_time`, `import_time` | Unix seconds. |
| `src_md5` | MD5 of something about the source (not needed). |
| `stream_idx` | `0`, or `-1` for images. |
| `mark_info_list`, `scence_info`, `mediaCreationInfo`, `userData` | Opaque. |
| `timeline_uuid`, `duration`, `enable_modify_mediaId` | Timeline item only; `timeline_uuid` equals the main timeline's userData key `11000`. |

## 4. `media.json`

Probe info of one bin item: `{"version", "file_name", "sourceInfo":
{"basicInfo": {...}, "vidStreamInfos": [...], "audStreamInfos": [...],
"recordInfos": {}}}`; same fields as the timeline `resources` (below), in
camelCase with slightly different names. Not needed for mapping; preserved.

## 5. `extra.json`

| Field | Meaning |
|---|---|
| `mediaClipsMapInfo` | `{"<clip link GUID>": {"mediaId": "<bin GUID>", "subClips": {}}}`: keyed by the clip userData key 3 (see §8.4). |
| `allMarkersInfo.beatDetectInfo` | Per clip link GUID: `{"algorithmType", "level"}`. |
| `fontNameInfo`, `usedBizFont`, `usedTemplateResInfo`, `pendingMarkersInfo`, `highlightInfo`, `TextSentence` | Opaque (mostly empty). |

## 6. `timeline.wesproj`

```json
{"currentTimelineId": 9, "productName": "Filmora", "projectName": "TLB",
 "projectVersion": "6,14,100,43", "serialNumber": 180, "serializationVersion": "28",
 "resources": [...], "timelineInfos": [...]}
```

- `currentTimelineId`: timeline open in the UI (not necessarily the main one).
- `serialNumber`: next free `timelineId` (verified: always greater than every id).
- `resources`: one probe record per source file, matched to clips by `filename`.
- `timelineInfos`: the main timeline (`type: 0`) followed by nested
  timelines (`type: 1`) used by title clips.

### 6.1 `resources[]`

| Field | Meaning |
|---|---|
| `filename` | Clip-style URL (see §8.1). |
| `sourceUuid` | Stable id of the file, repeated on clips as `sourceUuid`. |
| `streamType` | `2` video container, `3` audio file, `5` still image. Voice-over `.m4a` recordings report `2` with a 0×0 video stream. |
| `mediaLength` | Ticks (0 for images). |
| `videoStreamCount`, `audioStreamCount`, `programCount`, `subPicStreamCount` | Stream counts. |
| `vidStreamInfo[]` | `width`, `height`, `frameRate {num, den}` (e.g. `2997/100`), `maxFrameRate`, `rotation`, `flipType`, `bitRate`, color info, camera/GPS metadata. |
| `audStreamInfo[]` | `sampleRate`, `channels`, `bitDepth`, `duration` (seconds), `streamLength` (ticks), `description`. |
| `AIGCMeta` | `{"data", "size"}`, e.g. `"{This content is generated by AI}"`. |
| `fourCC`, `bitRate`, `createDate`, `modifyDate`, `alphaPremultiply` | Opaque. |

### 6.2 `timelineInfos[]`

| Field | Meaning |
|---|---|
| `timelineId` | Integer id, referenced by title clips (`timelineId`) and clip userData key 6. |
| `type` | `0` main timeline, `1` nested (title) timeline. |
| `resolutionWidth`, `resolutionHeight`, `frameRate {num, den}`, `aspectRatioX/Y`, `sampleRate`, `channels`, `format` (`"f32"`), `hdrMode`, `hdrMode_force_16bit` | Timeline settings (nested timelines copy the project's). |
| `audioBusInfos[]` | `{busUid, balance (0.5 = center), gain}`; audio tracks reference `busUid`. |
| `trackInfos[]` | Tracks, **bottom-most first** (verified: higher `trackTag` = higher track; title tracks come after the main video track). |
| `userData` | key 3 GUID; 11000 GUID (= bin `timeline_uuid`); 50 name (`"Untitle\0"`); 140 32-char hex; 30306 JSON page list; 30309 GUID; 11003; 36868/36869 nested timelines' design canvas (`"1920.000000"`, `"1080.000000"`). |

### 6.3 `trackInfos[]`

| Field | Meaning |
|---|---|
| `trackType` | `1` video, `2` audio. |
| `uuid` | Track UUID. **Not unique**: every nested title timeline reuses the same track UUID. |
| `trackTag` | Ordering tag (missing on the first audio tracks). |
| `busUuids` | Audio tracks: the bus they feed. |
| `clipList[]` | Clips, sorted by `tlBegin` (verified). |
| `userData` | key 20 on an audio track = index of the video track whose clips' audio it holds (camera audio track); key 21 = `1` on video tracks; key 12000 a one-byte code (`'-'` audio, `'F'`/`' '` video); key 14. Track lock/mute/hide flags were not observed. |

## 7. Clips

### 7.1 Clip types (`type`)

| type | Meaning | Count in corpus |
|---|---|---:|
| 1 | Video or image from a file (`streamId` 0). | 392 |
| 2 | Audio from a file; for camera files this is the audio stream (`streamId` 1) of the same file as a linked type-1 clip. | 510 |
| 4 | Text (inside a nested timeline); content in `scriptBuf`. | 16 |
| 7 | Nested timeline reference (a title template instance). | 16 |
| 26 | Pen/shape graphic (`serviceType: "pen"`, `pathLinkList`). | 3 |

### 7.2 Common fields

| Field | Meaning |
|---|---|
| `thisUId` | Clip UUID, unique in the document. |
| `tlBegin`, `tlEnd` | Timeline range `[tlBegin, tlEnd)` in ticks. |
| `inPoint`, `outPoint` | Source range in the **retimed** source timebase: `inPoint = speed.offset / rate`, `outPoint - inPoint = tlEnd - tlBegin` (verified). Images use a large constant base (`36000000000`). Nested/text clips: `0 .. duration`. |
| `filename` | Source URL (§8.1); empty for graphics; absent for text/nested clips. |
| `sourceUuid` | Links to `resources[]`. |
| `streamId` | `0` video stream / audio file, `1` audio stream of a video file. |
| `speed` | See §7.3 (absent on images, text and nested clips). |
| `effectChainList` | See §9. |
| `pipBuf` + `pipBufSize` | Compositing JSON stored as a string; `pipBufSize = byte length + 1` (C string terminator, verified). See §7.4. |
| `preTransition`, `postTransition` | See §10. |
| `placeholder`, `placeholderStatus` | Always `false` / `0`. |
| `userData` | See §8.4. |

Type-specific fields (preserved as-is unless noted):

- type 1: `backgroundFillBluredness` (20), `image_denoise`, `stabilization`,
  `stabilizeAnalysisData`, `picSequence`, `proxyFilename` (proxy URL),
  `video_cloud_algorithm`.
- type 2: `volumeKeyframe.parameter` and `audioDuckingframe.parameter`
  (keyframe strings, §8.3), `denoiseV3Strength` (50.0), `CacheAlg[]`
  (`AudioDenoiseCache`, param `AudioDenoiseAttenlimdb`), `audio_cloud_algorithm`.
- type 4: `scriptBuf` + `scriptBufSize` (§11).
- type 7: `timelineId`, `isAudio` (0), `keyframeScaleMode` (1).
- type 26: `serviceType`, `pathLinkList`.

### 7.3 Speed

```json
"speed": {"offset": 8.9756333, "offsetEnd": 37.3039333, "reverse": false,
          "speedParam": "{\"Version\": 3, ... \"_totalTime\": 640.9069333}"}
```

- `offset`, `offsetEnd`: true source range in **seconds** (verified).
- Playback rate = `(offsetEnd - offset) / ((tlEnd - tlBegin) / 1e7)`.
- `reverse`: reverse playback (never `true` in the corpus; mapped).
- `speedParam` (keyframe string, §8.3): the rate curve over the whole media,
  `_totalTime` = media length in seconds. A **uniform** speed `r` is stored
  as two keys `[(0, r), (_totalTime, 1.0)]` with `_rightDerivative` /
  `_leftDerivative` = `(1 - r) / _totalTime`, even though playback is a
  constant `r` (verified on 16 retimed clips: the measured ratio equals the
  first key exactly). Normal clips store `[(0, 1), (_totalTime, 1)]`;
  Version 2 strings with a single key also occur. Real speed ramps (more
  keys) were not observed: **unverified**; they are imported as a
  `jb_core::Speed::ramp` by mapping key times (source seconds) to clip-local
  time through the average rate.

### 7.4 `pipBuf`

```json
{"Algorithm":"Bilinear","BlendMode":0,"BlenderName":"video/blender/simple-pip",
 "Enable":true,"Opacity":100.0,"OpacityKeyFrame":"<keyframe string>"}
```

`Opacity` is 0–100. `OpacityKeyFrame` is present on nested/text clips (empty
in the corpus). `BlendMode` is always `0` (normal); other values were not
observed (**unverified**, not written).

## 8. Encodings

### 8.1 File URLs

`filename` is `file://` followed by the absolute path **without its leading
slash** (`file://Users/me/a.mp4`), not percent-encoded (spaces appear raw).
Store resources (downloaded sound effects, stickers) use a path relative to
Filmora's resource download folder instead, e.g.
`6_Cartoon_Whoosh_02/Data/Cartoon Whoosh 02.m4a`, which lives under
`…/ProgramData/Download/Filmora/<category>/`.

### 8.2 Numbers

Durations are integers (ticks). Effect parameter floats of `paramType 2` have
f32 precision (`0.4004000127315521`); `paramType 3` are doubles.

### 8.3 Keyframe strings

Used by `speed.speedParam`, `volumeKeyframe.parameter`,
`audioDuckingframe.parameter` and `pipBuf.OpacityKeyFrame`:

```
{"Version": 3,"ParameterType": 0,"keyframeSets": [{"_time": 0.0,"Interpolation": 6,"_value": 3.0,"_rightDerivative": -0.13290479},{"_time": 15.04836670,"Interpolation": 6,"_value": 1.0,"_leftDerivative": -0.13290479}],"MD5": "be1e3ba2e1fd816cc54e30c8be20bea6","_totalTime": 15.04836670}
```

- Times in seconds printed with 8 decimals (`0.0` for zero).
- `Interpolation`: always `6` in the corpus (curve with optional derivatives).
- **`MD5` = MD5 of the concatenated little-endian f64 triples
  `(_time, Interpolation, _value)` of every key, in order**; derivatives are
  not hashed (verified on all 1,925 keyframe strings of the corpus). An empty
  list hashes to `d41d8cd98f00b204e9800998ecf8427e`; the empty form is
  written `"MD5":"…"` without the space.
- `_totalTime` is present on speed curves only.

### 8.4 `userData`

`[{"data": <base64>, "key": <int>, "size": <decoded byte length>}]`. Blobs
are little-endian `i32`s, single bytes, UTF-8 strings (sometimes with a
trailing NUL) or GUID strings NUL-padded to 64 bytes. `size` always equals
the decoded length (verified).

Clip keys:

| Key | Meaning |
|---|---|
| 1 | Clip kind (i32): 1 video, 2 audio, 5 text, 12 title/nested. |
| 2 | i32 (0, or 1 on nested references). |
| **3** | GUID. **Linked clips (camera video + its audio) share it** (verified on all 385 pairs); otherwise unique. Also the key of `extra.json` `mediaClipsMapInfo`. |
| 6 | i32 id of the owning timeline. |
| 10 | Media bin GUID of the source (verified on every media clip); store slug on title references (`Basic_1`) and resource clips. |
| 12 | Store resource JSON: `{"id", "name" (slug), "res_type", "res_sub_type", "source", "sale_mode", "smart_resource", ...}`. `res_type`: 1 title, 2 transition, 6 audio. |
| 50 | Display name. |
| 55 | Nested timeline template JSON (`Key_AncestralGuid`, `Key_TimelineGuid`, `clip_color_infotag`, `key_timeline_type` = "5", `key_timeline_resource_info` = JSON with `name` (e.g. "Subtitle 9"), `key_resource_packname`, `resourcePackID`, `titles`). |
| 73, 74 | JSON state of the (disabled) AI stylizer effects, including the source path. |
| 102, 103, 13019, 30320, 36866 | Small opaque values. |

Effect keys: 3 GUID, 7 byte `1`, 15005 `"1.0"`, 36878 `"0"`, 36880 `"1"`.
Transition keys: 3 GUID, 8 i32 (3, or 2), 12 resource JSON, 80 i32 `1`,
2002 JSON text.

## 9. Effects

```json
"effectChainList": [
  {"name": "Basic", "effectList": [{"id": "video/effect/crop-pan-zoom", ...},
                                   {"id": "video/effect/transform", "paramList": [...]}]},
  {"name": "Color", "effectList": [{"id": "662E16ED-…", "display": "AdjustColor", ...}]},
  {"name": "Effect", "effectList": []},
  {"name": "_wes_hold_effectchain_", "effectList": []}]
```

Chains seen: `Basic` (video built-ins), `BasicFollow` (audio built-ins on
audio clips; also holds `AutoEnhance` on video clips), `Color`, `Effect`
(user effects and filters), `graphic` (shape rendering), and the empty
`_wes_hold_effectchain_`. Effect object:

| Field | Meaning |
|---|---|
| `id` | Effect id: a path (`video/effect/transform`) for built-ins, a GUID for plug-in effects. |
| `display` | Name. |
| `type` | Always `3` (transitions use `5`). |
| `thisUId` | UUID. |
| `enable` | Optional; `false` disables the effect. |
| `paramList[]` | `{"name", "fxParam": {"paramType", "unValue"}}`; paramType 2 float, 3 double, 5 integer/flag, 6 string. Parameter keyframes were not observed (**unverified**). |
| `effectMask` | Optional (graphics). |
| `placeholder`, `placeholderStatus`, `userData` | See above. |

Built-ins and their parameters:

| Effect | Parameters |
|---|---|
| `video/effect/transform` | `Position_x`, `Position_y`: clip center in normalized frame coordinates, `0.5` = centered, y grows downwards (default `0.5` when absent). `Scale_x`, `Scale_y`: percent of the fitted size (`100` when absent; when only `Scale_x` is present the scale is uniform). `EnableTransform` (1), `dwValue` (bit mask `1186339`). Rotation was not observed (**unverified**: `Rotation`). |
| `video/effect/crop-pan-zoom` | No parameters in the corpus (crop not used). |
| `audio/effect/volume` | `VolumeGain`: dB (−24 … +15 observed). |
| `audio/effect/fade` | `FadeInTime`, `FadeOutTime`: seconds. |
| `audio/effect/clip_volume`, `change_channel`, `ducking`, `equalizer` | Present on every audio clip; no parameters observed. |

Plug-in effects observed: AdjustColor (`662E16ED-4524-4D13-AAE9-11DBA0C63E17`,
chain `Color`, `u_exposure` plus `bEnable*` section flags), AutoEnhance
(`45D9AD95-4F03-4B2F-A8C7-8F7817B73FA4`, `amount` 0–100), and the graphic
pipeline (`graphic/effect/path`, `path_stroke` `lineSize`, `path_fill`,
`path_dropshadow`, `path_innershadow` `color`, `path_trim`, `path_clear`,
`shape_render`, `path_render`). See `docs/filmora-coverage.md`.

## 10. Transitions

`postTransition` blends a clip into the next clip of the same track;
`preTransition` brings the first clip in from nothing.

```json
{"display": "Fast Zoom", "id": "8D02CB83-6FC6-406B-8506-252F58A1D3B3", "type": 5,
 "tlBegin": 1312311000, "tlEnd": 1332331000, "thisUId": "…",
 "placeholder": false, "placeholderStatus": 0, "includeTrimFrames": false, "userData": [...]}
```

- `tlBegin`/`tlEnd` are **absolute** timeline times (verified).
- Between two clips the window is centered on the cut (`cut ± 1.001 s`,
  60 frames at 29.97 fps); after the last clip it ends at the cut
  (fade-to-black `cut − 2.002 s … cut`). Adjacent clips do not overlap.
- Camera audio clips carry their own `audio/blender/transition-fade`
  ("audio fade") mirroring the video transition.
- The human name and store id are in userData key 12 (`name` = store slug,
  e.g. `2_Fast_Zoom`, `id` = store id 1668922); `display` may be a slug too.
- Store resources are installed under
  `…/ProgramData/Download/Filmora/transition/<res_type>_<slug>/` with a
  `resinfo.json` holding localized `titles`.

## 11. Titles

A title instance is a type-7 clip whose `timelineId` names a nested
timeline (`type: 1`) containing one video track with one type-4 text clip.
The template comes from userData key 12 (`Basic_1`, store id 203568) and
key 55 (`Subtitle 9` in `DefaultPackage_93`). The outer clip carries the
instance transform/opacity; the text clip carries the text.

`scriptBuf` (JSON string; `scriptBufSize` = byte length + 1) main fields:

| Field | Meaning |
|---|---|
| `Text` | The text. |
| `PosX`, `PosY` | Text box center, normalized (0.5, 0.5 = centered). Mirrored by the text clip's transform `Position_x/y`. |
| `ScaleX`, `ScaleY` | Text box size as a fraction of the frame; mirrored ×100 by the transform `Scale_x/y`. The Basic title box is `0.0667` of the frame height; its glyphs are ~72% of the box (≈4.8% of the frame height, calibrated against exports). `ScaleX` follows the text width. |
| `ViewSize` | `"3840:2160"`, the canvas the box was laid out on. |
| `Edge` | Box edges `{Left, Right, Top, Bottom}`, normalized. |
| `Layout` | `TextAlign` (`"center"`), `TextAlignMode`, `CharSpace`, `LineSpace`, `Angle`, `Underline`. |
| `TextData[]` | Per-run style. `TextData[0].Basic`: `FontName` ("Arial"), `FontSize` (24, nominal), `FontBold`, `FontItalic`, `TextColor[0].Color` (`0xRRGGBB` integer: 16777215 = white), `TextAlpha` (0–255). `Border`: `Enable`, `Color`, `Alpha`, `Size`. `Shadow`: `Enable`, `Color`, `Alpha`, `Distance`, `Direction`, `BlurRadius`. `CharData` repeats the text. |
| `Background` | Box background: `Enable`, `Color`, `Alpha`, paddings, `Border`, `Gradient`, `Texture`. |
| `Curve`, `Glow`, `Gradient`, `Texture`, `Rotate3D`, `Offset`, `Record`, `AutoLine*`, `TextMode`, … | Advanced styling, preserved. |

Only white text without border/shadow was observed, so the byte order of
colors (`0xRRGGBB` vs `0xBBGGRR`) is **inferred** from the white value and
the importer's history, not proven.

## 12. Mapping to `jb_core`

| Filmora | `jb_core` |
|---|---|
| `project_info` name, resolution, frame rate, sample rate | `Project.name`, `ProjectSettings` |
| `medias_info.media_items` (+ `resources`) | `MediaAsset` (id = bin GUID; kind from `media_type`, size/rate from `resources`) |
| `timelineInfos[i]` | `Sequence` `timeline-<timelineId>`; main = `type 0` |
| `trackInfos[i]` | `Track` (index 0 = bottom) |
| clip type 1/2 | `ClipSource::Media` |
| clip type 7 | `ClipSource::Sequence` (the nested timeline) |
| clip type 4 | `ClipSource::Title(TitleData)`: text, font, `font_size = ScaleY × 0.72`, color, `position = (PosX, PosY)`, bold/italic, stroke, shadow, background |
| other types (26 …) | transparent `ClipSource::Color` placeholder, kept intact on write |
| `tlBegin`/`tlEnd` | `start`/`end` |
| `speed.offset` (or `inPoint` without speed) | `source_in` |
| speed ratio / curve, `reverse` | `Speed { rate, ramp, reverse }` |
| transform `Position_*`, `Scale_*` | `Transform` (`position = Position − 0.5`, `scale = Scale / 100`) |
| `pipBuf.Opacity` (+ keyframes) | `opacity` |
| `VolumeGain` (+ `volumeKeyframe`), `FadeInTime/OutTime` | `AudioSettings.gain_db`, `fade_in`, `fade_out` |
| other effects (`Color`, `Effect`, `graphic`, parameterized built-ins) | `EffectInstance { kind: "filmora:<id>", params }` |
| `postTransition` | `transition_out { kind: "filmora:<id>", duration, alignment }` |
| `preTransition` | entry animation `filmora:<id>` with `range` and role `preTransition` |
| clip userData key 3 shared by ≥ 2 clips | `link_group` |

Every imported entity keeps its identity in `extra.filmora` (`uid`,
`timelineId`, track `timeline`/`index`, media `mediaId`, effect `chain`). The
writer diffs the model against the snapshot taken at read time and patches
only changed fields into the original objects, so everything not listed
here is preserved verbatim; an unedited project is written back
byte-for-byte. Clips created in Jibaru get JSON modeled on the shapes above
(top-level titles are wrapped in a new nested timeline).

Property-panel edits beyond these are written as described in §13. Not
mapped (preserved; edits warned about): track names and heights, clip
groups, beat detection, AI/stabilization/denoise state.

## 13. Property-panel edits

`crates/jb-filmora/src/props.rs` and `markers.rs` implement these. Sources:
the corpus, the timeline of a store template (`resThumbnailTemplate`,
serialization 17) and the data files of Filmora's presets
(`configs/AnimationKeyFrame`, `configs/AnimationNew`, `configs/Speed`),
which contain clip effect objects. Fields are only written when the
corresponding property was edited, so unedited files stay byte-identical.

| Property | Written as | Status |
|---|---|---|
| Sequence markers | `extra.json` `allMarkersInfo["<timeline userData 3 GUID>"] = [{"name", "position" (ticks), "comments", "color" (0xRRGGBB)}]`. `extra.json` is created next to the timeline if missing. | verified (one sample) |
| Clip markers | Same, keyed by the clip's userData 3 GUID (shared by linked clips), `position` clip-local. | inferred |
| Parameter keyframes (transform, effect params, balance) | `paramMapList: [{"key": 3, "keyFrame": {"parameter": "<keyframe string>"}, "name": "<param>"}]` on the effect, values in the effect's own units, times in clip-local seconds. | shape from presets; time base inferred |
| Keyframe interpolation | `Interpolation` 1 = linear with slope derivatives, 7 = hold, 6 = curve: a flat `_rightDerivative` at the segment start eases in, a flat `_leftDerivative` at its end eases out, both = ease in-out. The last key's interpolation is not stored. | codes from presets; ease encoding ours |
| Opacity keyframes | `pipBuf.OpacityKeyFrame` (0–100). | shape verified, keys inferred |
| Volume keyframes | `volumeKeyframe.parameter` (dB). | shape verified, keys inferred |
| Rotation | transform `Rotation`, degrees. | name from presets |
| Flip | transform `FlipX`, `FlipY` (paramType 5). | unverified |
| Fit mode | transform `FitMode` (5): 0 fit, 1 fill, 2 stretch, 3 none. | unverified |
| Crop | `video/effect/crop-pan-zoom` `CropLeft/Top/Right/Bottom` (3, fractions of the source). Pan & Zoom presets use a different keyframed rect, not mapped. | unverified |
| Blend mode | `pipBuf.BlendMode` in the order of Filmora's menu: 0 normal, 1 darken, 2 multiply, 3 color burn, 4 linear burn, 5 darker color, 6 lighten, 7 screen, 8 color dodge, 9 linear dodge (add), 10 lighter color, 11 overlay, 12 soft light, 13 hard light, 14 vivid light, 15 linear light, 16 pin light, 17 hard mix, 18 difference, 19 exclusion, 20 subtract, 21 divide (22 modes, matching the 0–21 range of the store "Allpurpose BlendMode" filter). | inferred |
| Clip disabled | clip `"enable": false`. | unverified |
| Track lock / hide / mute | track `locked`, `visible` (false = hidden video track or muted audio track), `mute`. | `locked`/`visible` seen in serialization 17; `mute` unverified |
| Speed ramp | `speedParam` keys at source seconds (`offset + position × span`) with the rate; `speed.rampPreset` names the preset. Presets (rate over % of the source range): Montage, Hero moment, Bullet time, Jumper, Flash in, Flash out, from `configs/Speed`. | curve values verified; storage inferred |
| Maintain pitch | `speed.keepPitch`; absent = pitch follows speed (how Jibaru rendered imports before). | unverified |
| Freeze frame | clip `"freezeFrame": true`, `offsetEnd = offset`. | unverified |
| Balance (pan) | `audio/effect/volume` `Balance`, −100 … 100. | unverified |
| Pitch | `audio/effect/pitch` `Pitch` (semitones) in `BasicFollow`. | unverified |
| Mute | `audio/effect/clip_volume` `Mute` = 1. | unverified |
| Auto normalization | `audio/effect/clip_volume` `AutoNormalize` = 1, `NormalizeTarget` (LUFS), `MeasuredLoudness`. | unverified |
| Equalizer | `audio/effect/equalizer` `Preset` (string), `Band0`…`Band9` (dB). | unverified |
| Entry/exit animations | Effects in an `Animation` chain: `id` = preset GUID from `configs/AnimationNew` (e.g. Fade1 in `{C0BBEA86-…}`), `AnimationScene` = `animation_scene_in`/`out`/`loop`/`combo` (6), `inPoint`/`outPoint` = clip-local range. Registry animations without a preset are written under their own id, with a warning. | ids verified; placement in the clip unverified |
| Title alignment, spacing | `scriptBuf.Layout.TextAlign` (`left`/`center`/`right`), `CharSpace`/`LineSpace` = spacing × 100. | `TextAlign` verified; spacing units inferred |
