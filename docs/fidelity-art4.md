# Render fidelity: art-4 procedural items

Glitch, warp, 3D-surface and stylized items of Filmora 15.7.3 that lanes 1,
3 and 4 left as stand-ins or unimplemented (`docs/fidelity-lane{1,3,4}.md`),
recreated as our own parametric WGSL (`crates/jb-render/src/fx/art4_*.rs`,
`crates/jb-render/src/shaders/art4/`). Every item is data: a model (pass
structure + shader), a constant vector and a table of keyframe rows over
progress, fitted to Filmora's exported output of the item only (no Filmora
asset file was read). Randomness is seeded integer hashing (PCG) of block
indices and of a frame "beat", so renders are reproducible frame by frame.
Filmora GUIDs map to the items in `crates/jb-render/src/aliases_art4.rs`
(exact case, plus the upper-case form).

## Method

Same harness and windows as the lanes (`crates/jb-cli/examples/fidelity.rs`,
960×540, `--search 0`, windows from `docs/filmora-references.json`). The
keyframe rows were fitted frame by frame with a throwaway pattern-search
fitter at 480×270 against the decoded export, warm-starting each frame from
the previous one. "Before" is the previous mapping (hard cut / effect
dropped, or the lane stand-in); an alias is only switched when "After" is
lower.

## Results

MAD mean / max over the window (0–255 over RGB).

| Item | Batch | id | Before | After | Notes |
|---|---|---|---|---|---|
| Swirl | tr005 | `jb.a4.whirlpool` | 55.63 / 60.6 | 25.54 / 36.7 | elliptical twist + zoom; residual is the fine checker detail of the grid media under a strong warp |
| Ripple | tr005 | `jb.a4.puddle_ripple` | 23.77 / 49.2 | 17.10 / 27.4 | seeded noise bumps; bump positions are statistical, not identical |
| Wave | tr005 | `jb.a4.ring_ripple_reveal` | 27.49 / 48.1 | 21.31 / 34.1 | expanding ring front with concentric ripples |
| Butterfly Wave Scrawler | tr004 | `jb.a4.ribbon_wave_split` | 40.74 / 55.4 | 40.28 / 50.6 | sine-bent lines + RGB split; structure close, phase of the bands differs |

## Gaps

Items of the group not listed above are still rendered by their previous
mapping; see the table history in this file as they land.
