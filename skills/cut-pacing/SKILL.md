---
name: cut-pacing
description: Choose and trim takes for a personal vlog in this house style — median shot 6–10 s, mostly trims inside the same clip to remove dead time, butted clips with no gaps. Use when selecting footage, trimming clips or tightening a chapter.
---

# Cut pacing

- **Shot length:** median 6–10 s per shot (10th percentile ~2.4 s, 90th ~27 s). A finished vlog has 65–107 clips.
- **Mostly trims, not B-roll:** 80–90% of cuts remove dead time inside the same camera clip rather than intercutting other shots.
- **No gaps or overlaps:** clips are butted together, with at most a handful of gaps over 0.1 s per project.

**Keep:** lines spoken to camera, arrivals, meeting people, conversations with jokes, food, pets, striking scenery (plane window, city views).

**Cut:** long silences, repeated shaky or blurry takes, fingers over the lens, dark shots with nothing in them, the same sentence said twice.

**Avoid:** TikTok-style rapid cutting; splitting a funny conversation in the middle of the joke.

## With Jibaru

- Find candidate clips with `jb find-clips --at/--range/--media`.
- Trim with `trim_clip`, `ripple_trim`, `split_clip` + `remove_clip` (`ripple: true`).
- Keep the camera audio linked to its video (link groups move and trim together).
