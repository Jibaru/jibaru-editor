---
name: privacy
description: Handle people who must not be recognizable in a personal vlog — by default everyone stays visible; when the brief asks to hide someone, cut the shot if it isn't essential, otherwise pixelate with tracking. Use when a brief mentions hiding faces, minors, strangers or anyone who didn't consent.
---

# Privacy

**Default:** everyone stays visible. Friends appear on camera normally.

**When the brief asks to hide someone:**
1. If the shot isn't essential, cut it.
2. If it is essential, pixelate (mosaic, not blur) with tracking:
   - Make the box about 30% larger than the face.
   - Hold it for ~8 frames through detection gaps so it doesn't flicker.

**Identity:** face recognition of the user, if available, runs locally against the user's own reference photos. The agent never identifies people by name from images.

## With Jibaru

`add_effect` with `jb.mosaic` (or `jb.pixel_mosaic`) and keyframed position and scale (`add_keyframe`) following the face; verify a few frames with `jb frame --at`. Filmora's own equivalent is AI Face Mosaic (Video › Basic).
