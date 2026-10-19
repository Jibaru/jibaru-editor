---
name: color-look
description: Apply color and small finishing touches in this house style — no LUTs or looks, only exposure lifts on dark shots (+45 typical, up to +100), daylight untouched, no reframing of the wide-angle image, speed changes almost never. Use when grading footage or adding stickers or speed effects.
---

# Color look

**Grading:**
- No LUTs, filters or cinematic looks.
- Only the exposure adjustment (Filmora AdjustColor `662E16ED-4524-4D13-AAE9-11DBA0C63E17`) on dark shots: +45 for typical dinner and night scenes, +60 to +100 for the darkest ones.
- Rarely Auto Enhance (`45D9AD95-4F03-4B2F-A8C7-8F7817B73FA4`) at amount ~42.
- Daylight shots stay untouched.

**Framing:** keep the action-camera wide-angle look; no crop, zoom or reposition (transform stays at default).

**Speed:** almost never. When used, it is one short clip sped up (~3×), ramping back to 1×.

**Extras:** an occasional PNG sticker as a joke, picture-in-picture for ~2 s (scale ~40%, lower part of the frame).

**Avoid:** cinematic filters, raising exposure on daylight shots, reframing.

## With Jibaru

- `add_effect` with kind `filmora:662E16ED-4524-4D13-AAE9-11DBA0C63E17` and `u_exposure`; Jibaru renders it as `jb.filmora.adjust_color`, calibrated against Filmora.
- Check dark shots with `jb frame --at` before and after.
