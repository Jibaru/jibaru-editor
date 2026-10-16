# Filmora feature coverage

Union inventory of every effect, transition, title template, store resource
and clip type used by the reference corpus: the four user projects
(2026-08 trips and workshops, Filmora Mac 15.7, 3840×2160 @ 29.97 fps) and
the preset sample project shipped with Filmora (`brush_path_guide`,
1920×1080 @ 25 fps). It is the implementation spec for the renderer: every
row is something a real project needs.

Regenerate with:

```sh
cargo run -p jb-filmora --example wfp_coverage -- ~/Pictures/*/*.wfp \
  "$HOME/Library/Application Support/Wondershare Filmora Mac/ProgramData/Default Effects/PresetProjects/"*.wfp
```

(`--json` prints the full `jb_filmora::Coverage`, including the `thisUId` of
every clip using each item.) Names come from the project itself and, for
store resources, from the `resinfo.json` metadata of the local installation.
In the Jibaru model every effect and transition below appears with kind
`filmora:<Id>` (built-ins in the *Built-in* table are mapped to core clip
fields instead; see `docs/filmora-format.md` §12).

## Summary

| Category | Distinct | Uses | Notes |
|---|---:|---:|---|
| Transitions | 7 | 75 | 35 are the audio cross-fade mirroring a video transition. |
| Built-in clip effects | 8 | 3,843 | Transform and audio volume/fade carry parameters; the rest are structural. |
| Video effects | 1 | 8 | AutoEnhance. |
| Color adjustments | 1 | 35 | AdjustColor, exposure only. |
| Graphic effects | 9 | 27 | Pen/shape pipeline, preset project only. |
| Title templates | 1 | 16 | Basic 1 ("Subtitle 9"), plain Arial white text. |
| Store resources as clips | 1 | 1 | A sound effect. |
| Clip types | 5 | 937 | |

No animations, stickers, filters from the store, masks, keyframed effect
parameters, speed ramps or blend modes occur in the corpus.

## Renderer notes, in priority order

1. **Transform** (`video/effect/transform`): position/scale on every visual
   clip; already mapped to `jb_core::Transform`. Scale is relative to the
   fitted size (`Scale_x` 100 = fitted; 41.6 and 172.5 are observed).
2. **Transitions** (looks below are inferred from the names; each installed store
   transition has a `previewVideo.mp4` next to its `resinfo.json` for reference.
   All are 2.002 s, centered on the cut, or ending at the
   cut after the last clip):
   - `audio/blender/transition-fade` "audio fade": linear audio cross-fade
     on the camera audio track; renders like `jb.dissolve` for audio.
   - `8D02CB83-…` **Fast Zoom** (store `2_Fast_Zoom`): zoom-blur push from
     the outgoing into the incoming clip.
   - `C8965C45-…` **Fast Wipe Left** (store `2_Fast_Wip_Left`): fast
     horizontal wipe/slide to the left with motion blur.
   - `3B49DBEE-…` **Fade** (display `fade_black`): fade through black; used
     at the end of a timeline (fade to black).
   - `2981D185-…` **Dissolve**: cross-dissolve, maps to `jb.dissolve`; used
     as `preTransition` (fade in from nothing) at the start.
   - `38B7DD62-…` **Slide** (store `Slide_771515`): push slide.
   - `521158CA-…` **Film Burn Transition 02**: overlay-based film-burn
     transition (store pack; needs a light-leak overlay of our own).
3. **AdjustColor** (`662E16ED-…`, chain `Color`): only `u_exposure` is set
   (45–100 on Filmora's −100…100 slider); the `bEnable*` flags just enable
   panel sections. Implement as an exposure adjustment.
4. **AutoEnhance** (`45D9AD95-…`): one-knob automatic enhancement,
   `amount` 0–100 (42 observed): auto levels + mild saturation boost.
5. **Titles**: Basic title template, centered or lower-third (`PosY` 0.863),
   Arial, white, no stroke/shadow, glyph height ≈ 4.8% of the frame.
6. **Audio**: per-clip gain (−24…+15 dB) and fade in/out (up to 4.8 s), all
   mapped to `jb_core::AudioSettings`.
7. **Graphic** (`graphic/effect/*`, clip type 26): only in Filmora's preset
   sample; low priority.

## Transitions

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `audio/blender/transition-fade` | audio fade | 35 | 4 |  |
| `8D02CB83-6FC6-406B-8506-252F58A1D3B3` | Fast Zoom [res 1668922, slug `2_Fast_Zoom`, type 2] | 18 | 4 |  |
| `C8965C45-074B-4BF5-948E-D9373D10836C` | Fast Wipe Left [res 1668894, slug `2_Fast_Wip_Left`, type 2] | 13 | 1 |  |
| `3B49DBEE-9A15-4844-B613-E2C2497EA236` | Fade (display `fade_black`) [res 39154, slug `Fade`, type 2] | 6 | 3 |  |
| `2981D185-D52E-44f4-ABD5-3CE83890E32E` | Dissolve [res 39146, slug `Dissolve`, type 2] | 1 | 1 |  |
| `38B7DD62-E852-4B41-81E1-80901511721D` | Slide [res 3386022, slug `Slide_771515`, type 2] | 1 | 1 |  |
| `521158CA-2709-4FB1-8C3C-29781494BE1F` | Film Burn Transition 02 (display `Film_Burn_Transition_02`) [res 63511726, slug `Film_Burn_Transition_02`, type 2] | 1 | 1 |  |

## Built-in clip effects

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `audio/effect/change_channel` | change_channel | 510 | 4 |  |
| `audio/effect/clip_volume` | clip_volume | 510 | 4 |  |
| `audio/effect/ducking` | ducking | 510 | 4 |  |
| `audio/effect/fade` | fade | 510 | 4 | `FadeInTime` (2: 0.5338666439056396, 0.0, 0.6673333048820496, 0.6339666843414307, 1.2345666885375977, 2.702699899673462)<br>`FadeOutTime` (2: 0.600600004196167, 4.8381667137146, 4.304299831390381, 4.404399871826172, 0.4671333432197571, 0.4004000127315521) |
| `audio/effect/volume` | volume | 510 | 4 | `VolumeGain` (2: -14.59459459459459, -6.486486486486485, -24.16216216216216, -16.001756756756755, -9.515270270270271, -7.893648648648648) |
| `audio/effect/equalizer` | equalizer | 439 | 4 |  |
| `video/effect/crop-pan-zoom` | crop-pan-zoom | 427 | 5 |  |
| `video/effect/transform` | transform | 427 | 5 | `EnableTransform` (5: 1)<br>`Position_y` (3: 0.7921953201293945, 0.863336443901062)<br>`Scale_x` (3: 41.560935974121094, 30.235321044921875, 24.126605987548828, 3.915137529373169, 1.7215596437454224, 19.740825653076172)<br>`Scale_y` (3: 41.560935974121094, 6.666666507720947, 172.5244140625)<br>`dwValue` (5: 1186339) |

## Video effects / filters

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `45D9AD95-4F03-4B2F-A8C7-8F7817B73FA4` | AutoEnhance | 8 | 1 | `amount` (3: 42.0) |

## Color adjustments

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `662E16ED-4524-4D13-AAE9-11DBA0C63E17` | AdjustColor | 35 | 1 | `bEnableAutoColor` (5: 1)<br>`bEnableColor` (5: 1)<br>`bEnableHSL` (5: 1)<br>`bEnableLUT` (5: 1)<br>`bEnableLight` (5: 1)<br>`bEnableNewColor` (5: 1)<br>`bEnableNewHSL` (5: 2)<br>`bEnableNewLight` (5: 1)<br>`bEnableVignette` (5: 1)<br>`bEnableWB` (5: 1)<br>`u_exposure` (3: 45.0, 46.0, 100.0, 60.0, 92.0) |

## Graphic (shape/pen) effects

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `graphic/effect/path` | path | 3 | 1 |  |
| `graphic/effect/path_clear` | path_clear | 3 | 1 |  |
| `graphic/effect/path_dropshadow` | path_dropshadow | 3 | 1 |  |
| `graphic/effect/path_fill` | path_fill | 3 | 1 |  |
| `graphic/effect/path_innershadow` | path_innershadow | 3 | 1 | `color` (5: 10000000) |
| `graphic/effect/path_render` | path_render | 3 | 1 |  |
| `graphic/effect/path_stroke` | path_stroke | 3 | 1 | `lineSize` (5: 6) |
| `graphic/effect/path_trim` | path_trim | 3 | 1 |  |
| `graphic/effect/shape_render` | shape_render | 3 | 1 |  |

## Title templates

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `Basic_1` | Basic 1 (display `Subtitle 9 (DefaultPackage_93)`) [res 203568, slug `Basic_1`, type 1] | 16 | 2 |  |

## Text templates

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `Caption` | Caption | 16 | 2 | `Basic.FontSize` (: 24.0)<br>`Basic.TextColor` (: [{"Color":16777215}])<br>`Border.Enable` (: 0)<br>`PosX` (: 0.5)<br>`PosY` (: 0.5, 0.863336443901062)<br>`ScaleX` (: 0.3023532032966614, 0.24126605689525604, 0.039151377975940704, 0.01721559651196003, 0.1974082589149475, 0.12685321271419525)<br>`ScaleY` (: 0.06666667014360428)<br>`Shadow.Enable` (: 0) |

## Store resources used as clips (stickers, elements, sounds)

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `Cartoon_Whoosh_02` | Cartoon Whoosh 02 [res 571754, slug `Cartoon_Whoosh_02`, type 6] | 1 | 1 |  |

## Fonts

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `Arial` | Arial | 16 | 2 |  |

## Clip types

| Id | Name | Count | Projects | Parameters (type: samples) |
|---|---|---:|---:|---|
| `2` | audio | 510 | 4 |  |
| `1` | video / image | 392 | 5 |  |
| `4` | text (scriptBuf) | 16 | 2 |  |
| `7` | nested timeline (title template / compound) | 16 | 2 |  |
| `26` | graphic (pen / shape) | 3 | 1 |  |

