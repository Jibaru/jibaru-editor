---
name: vlog-structure
description: Lay out a personal vlog in this house style — intro title on black, chronological chapters, 2 s fade-to-black ending, target length and deliverables (main 4K cut plus a 60 s highlight). Use when starting a new edit from raw footage or when restructuring an existing cut.
---

# Vlog structure

**Intro**, one of:
- A title on black for 2.0–2.4 s before the first clip. It may fade in with a 0.6 s Dissolve.
- Straight into a selfie shot to camera with voice-over, no title.

**Body:** chronological, split into chapters by place or moment ("arrival", "lunch", "the game"…). A chapter may open with a short title (see [titles-style](../titles-style/SKILL.md)).

**Ending:** always a 2 s fade to black (`jb.fade_black`). Optionally end with a few still photos, 5 s each, pillarboxed on black, with a fade to black between them.

**Length:** the final cut is 24–53% of the raw footage; for ~50 min of footage expect ~12–28 min.

**Deliverables:**
1. Main cut, 3840×2160, 29.97 fps (30000/1001), 16:9.
2. A highlight of exactly 60 s: one moment per chapter, cut to the beat of one song, no voice-over.
3. Sometimes a 1280×720 copy for messaging apps.

**Avoid:** reordering events out of chronological order, ending without a fade, putting the intro title over video instead of black.

## With Jibaru

- Build the timeline with `jb exec` commands (`add_track`, `add_clip`, `insert_clip`); verify with `jb info` and spot frames with `jb frame --at`.
- Preview fast with `jb render <project> -o preview.mp4 --preview 540p`, then render the 4K main cut.
- Render the highlight as a separate sequence or a copy of the project.
