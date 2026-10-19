---
name: music-and-voice-mix
description: Mix music, camera audio and voice-over for a personal vlog in this house style — one song per chapter, manual gain per segment instead of auto-ducking (music around −15 dB, lower under talk), short fades, 0 dB voice-over pieces. Use when adding music or voice-over or balancing audio.
---

# Music and voice mix

**Music:**
- Present for most or all of the video in travel vlogs. Talk-only hangout videos may have none.
- One song per chapter, so 2–3 music tracks per video. Prefer royalty-free or original instrumentals (chill electronic); warn the user about Content ID if they bring commercial songs.

**No auto-ducking.** Split clips and give each segment a fixed gain, with per-clip fades:

| Situation | Music | Camera audio |
| --- | --- | --- |
| Normal, people talking | −15 to −16 dB (−20 to −25 dB under loud conversation) | 0 dB (up to +10/+15 dB if quiet) |
| Scenery, no dialogue | about −8 dB | −10 to −22 dB |
| Music-only stretches | −8 to 0 dB | muted |

- **Music fades:** in 0.5–1.2 s, out 0.4–4.8 s.
- **Voice-over:** pieces of 2–8 s at 0 dB, about 13–17 per video. Lower music and camera audio under each piece.

**Avoid:** automatic ducking, music over a conversation without lowering it, one song for the whole video.

## With Jibaru

- `split_clip` the music at segment boundaries, then `set_volume` (dB) and `set_audio_fades` per segment; `set_mute` for silenced camera audio.
- `jb normalize` measures loudness when a target LUFS is needed (e.g. the highlight for social media at −14 LUFS).
