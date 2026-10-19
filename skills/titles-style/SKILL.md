---
name: titles-style
description: Write and style vlog titles in this house style — plain white Arial centered, about 4.8% of frame height, no animation; 2–2.6 s for intro and chapter titles, under 1.3 s for at most three one-word reaction titles. Use when adding intro, chapter or reaction text.
---

# Titles style

**Look:**
- Basic text: Arial, white `#FFFFFF`, centered (x 0.5, y 0.5).
- No animation, background or outline.
- Glyph height is about 4.8% of the frame height (`font_size` ≈ 0.048 in Jibaru).
- Titles go on the track above the main video.

**Durations:**
- Intro and chapter titles: 2.0–2.6 s.
- Reaction titles: 0.5–1.3 s, sometimes placed lower (y ≈ 0.86). They are one word or a short exclamation ("XD", "F").

**Wording:** short, plain, in the video's language. Name the place or the moment: "At the manga café", "Lunch is here", "Party time", "Stop at the dunes".

**Avoid:** animated titles, other fonts, long sentences, more than three reaction titles per video.

## With Jibaru

`add_clip` with a `title` source, then `set_title_style` (`font: "Arial"`, `size`, `color`, `position`). Filmora's Basic/Subtitle template round-trips through `.wfp`.
