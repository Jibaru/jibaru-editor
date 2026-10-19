---
name: transition-choice
description: Decide which cuts in a personal vlog get a transition and which one, in this house style — about 13% of cuts, one favorite transition per video for place or chapter changes, always 2.0 s centered with a 2 s audio crossfade. Use when adding or reviewing transitions.
---

# Transition choice

**How often:** only about 13% of cuts get a transition; everything else is a hard cut. Never use a transition inside one scene.

**Timing:** always 2.0 s, centered on the cut, with a 2 s audio crossfade on the audio track underneath.

**Which:** one favorite per video, used at place or chapter changes:

| Look | Jibaru id | Filmora GUID |
| --- | --- | --- |
| Fast Zoom | `jb.fast_zoom` | `8D02CB83-6FC6-406B-8506-252F58A1D3B3` |
| Fast Wipe Left | `jb.wipe_left` | `C8965C45-074B-4BF5-948E-D9373D10836C` |
| Slide (occasional) | `jb.pan_slide` | `38B7DD62-E852-4B41-81E1-80901511721D` |
| Fade to black (ending only) | `jb.fade_black` | `3B49DBEE-9A15-4844-B613-E2C2497EA236` |
| Dissolve (intro title only) | `jb.dissolve` | `2981D185-D52E-44f4-ABD5-3CE83890E32E` |

The audio crossfade is Filmora's `audio/blender/transition-fade`. When the project is a `.wfp`, use the `filmora:<GUID>` kind so Filmora still recognizes it after saving.

**Avoid:** mixing many transition types, durations other than 2 s, transitions within a scene.

## With Jibaru

`set_transition` on the outgoing clip with `duration: 20000000` (2 s in 100 ns ticks) and `alignment: "center"`; set the matching audio crossfade on the linked audio clip.
