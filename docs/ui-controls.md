# UI controls: behavior audit

Every interactive control of the editor outside the timeline and its toolbar,
what it did before this audit and what it does now. "Before" comes from
reading the code (`app/src` at `b7a8cbd`) and exercising it; "Now" is
covered by the Playwright suite (`cd app && pnpm test:ui`), whose case is
named in the last column.

Status words:

- **works**: does what Filmora's control does, through a jb-core Command
  (undoable, saved to `.jbp` and to `.wfp`), and shows up in the preview.
- **no-op**: clickable, nothing happens.
- **placeholder**: shows a fixed value or local UI state only; nothing reaches
  the project.
- **broken**: does something wrong, or does nothing in the desktop app.
- **unavailable**: Wondershare cloud or AI feature. The control is disabled
  and says so in its tooltip or body. Nothing breaks.
- **gap**: a local feature Jibaru does not have yet. The control is disabled
  and says so.

How edits reach the project: the panels send Commands through `exec`
(`app/src/store.ts`) to the Rust core (Tauri `execute`). Effects are jb-render
registry ids. In a `.wfp` project, edits Filmora can store go to their native
fields (docs/filmora-format.md §13). Jibaru-only state (panel effects, masks,
chroma key, color panel, denoise, channels, voice filters) goes into the
Jibaru sidecar `ProjectFolder/jibaru.json` inside the archive. Filmora
ignores the sidecar, so those edits round-trip in Jibaru only. Saving reports
them as notes ("kept for Jibaru only").

UI tests run against the browser mock, which mirrors jb-core
(`app/src/backend/applyCommand.ts`). `JB_CORE_JOURNAL=<dir> pnpm test:ui`,
then `node tests/replay-core.mjs <dir>`, replays every command the tests sent
through the real core (`jb exec`). The last run (54 cases) replayed 145 commands with no
rejection. Panel effects were also rendered with `jb frame` (background blur,
shadow, corner radius, chroma key, 1:1 ratio) with no unknown effects.

## Player

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| Timeline / Source tabs | works (Source was a silent no-op with no source media) | works; Source is disabled until a media item is opened | view state | player › source monitor |
| Quality dropdown | works | works, remembered per user | native preview size | player › quality menu |
| Split view | no-op | unavailable/gap: disabled ("not available yet") | — | smoke |
| Video scopes | no-op | works: RGB histogram and luma waveform of the shown frame | preview pixels | player › video scopes |
| Previous / next frame | works | works (disabled at the ends) | playhead | player › transport |
| Play / pause, Stop | works | works | native engine | player › transport |
| Aspect ratio (16:9, 9:16, 1:1, 4:3, 3:4, 4:5, 21:9) | placeholder (local state) | works: changes the project resolution (keeps the short side) | `set_project_settings` | player › aspect ratio |
| Mark in `{` / mark out `}` (I / O) | works | works, shown on the progress bar; used by Export "In/Out marks" | view state | player › mark in/out |
| Add marker (M) | missing | works | `add_marker` | player › markers |
| Settings menu | partial (statistics, clear marks) | Show Playback Statistics, Go to Mark In/Out, Clear In/Out | view state | player › settings menu |
| Snapshot | broken in the app (`<a download>` of a blob does nothing in the webview) | works: PNG of the rendered frame saved to `<project>/Snapshots` (or `~/Pictures/Jibaru Snapshots`) and imported into Project Media | Tauri `save_snapshot` | player › snapshot |
| Volume | placeholder (mute toggle only) | works: popover with level slider and mute/unmute | preview volume | player › volume popover |
| Full screen (button, double-click, Esc) | broken in the app (element fullscreen is off in WKWebView) | works: window full screen with the player filling it; Esc leaves | Tauri window | player › full screen |
| Progress bar | works | works, shows the in/out range | playhead | player › transport |
| Eyedropper (from Color) | missing | works: click a point of the player to white-balance | preview pixels | color basic |
| Source monitor: frame step | broken (fixed 1/30 s) | works at the media's frame rate | `<video>` | player › source monitor |
| Source monitor: mark in/out, clear | placeholder (range never used) | works: the range is used by "+", the source "Add to timeline" button and the context menu | `add_clip` `source_in`/length | player › source monitor |
| Source monitor: add to timeline | missing | works | `add_clip` | player › source monitor |

## Properties: Video

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| Transform switch | no-op | works: off resets the transform, on brings the values back | `set_transform` | props-video › transform switch |
| Module keyframe diamond (Transform, Compositing, Adjustment) | no-op | works: keys every parameter of the module at the playhead | `add_keyframe` | props-video › keyframes |
| Scale width / height, proportion lock | works (no keyframes) | works; when keyed, edits write a keyframe at the playhead | `set_param` / `add_keyframe` | props-video › transform |
| Position X / Y | works (no keyframes) | works, keyframable | `set_param` / `add_keyframe` | props-video › transform, keyframes |
| Row diamonds and ◀ ▶ keyframe navigation | no-op (decorative) | works: add/remove at the playhead, jump to previous/next key | `add_keyframe` / `remove_keyframe` | props-video › keyframes |
| Path Curve | placeholder (disabled) | gap: disabled with explanation | — | smoke |
| Anchor Point | placeholder (disabled) | gap: disabled with explanation | — | smoke |
| Rotate field, dial, rotate 90° buttons | works | works, keyframable | `set_param` | props-video › transform |
| Flip H / V | works | works | `set_flip` | props-video › transform |
| Corner radius | broken (`jb.corner_radius` did not exist; rendered as nothing and reported as unknown) | works | `jb.corner_radius` | props-video › corner radius |
| Compositing switch | no-op | works (reset/restore) | `set_blend_mode`, `set_opacity` | props-video › compositing |
| Blend mode | placeholder (disabled, Normal only) | works, 22 modes | `set_blend_mode` | props-video › compositing |
| Opacity | works | works, keyframable | `set_param opacity` | props-video › compositing |
| Background (switch, type Blur/Color, blur level, color) | broken (effect did not exist) | works: fills the canvas around the clip | `jb.background` (canvas pass) | props-video › Background, background color |
| Shadow (distance, angle, blur, opacity, color) | broken (effect did not exist) | works | `jb.shadow` (canvas pass) | props-video › Shadow |
| Auto Enhance (amount) | works (amount slider not keyframable) | works, keyframable, off disables instead of deleting | `jb.auto_enhance` (Filmora AutoEnhance in `.wfp`) | props-video › Auto Enhance |
| Motion Blur | placeholder (switch only) | works: intensity, angle | `jb.motion_blur` | props-video › Motion Blur |
| Stabilization | broken (effect did not exist) | works: local motion estimation, smooth level, extra zoom, re-analyze | `jb.stabilize` + Tauri `analyze_stabilization` (jb-media) | props-video › stabilization |
| Video Denoise | broken (effect did not exist) | works: strength | `jb.denoise` | props-video › Video Denoise |
| Lens Correction | broken (effect did not exist) | works: adjust level | `jb.lens_correction` | props-video › Lens Correction |
| AI Video Enhancer | missing | unavailable (cloud AI) | — | props-video › cloud AI |
| Library effect modules | partial (switch deleted the effect, float params only) | works: switch enables/disables, delete button, float/int sliders (keyframable), bool switches, color pickers | `set_effect_enabled`, `set_effect_params`, `remove_effect` | props-video › library effects |
| Mask: rectangle, circle, single line, double line, heart, star | broken (`jb.mask.*` did not exist) | works: position, rotate, size/spacing, roundness, blur strength, opacity, invert, remove | `jb.mask.*` | props-video › masks |
| Mask: pen, text | no-op (rendered nothing) | gap: disabled | — | props-video › masks |
| Mask: smart brush | no-op | unavailable (AI) | — | props-video › masks |
| Custom (image) mask, Add | no-op | gap: disabled (jb-render has no image decoder) | — | smoke |
| Chroma Key: switch, green/blue/custom color, tolerance, offset, edge thickness, edge feather, spill removal, alpha channel | broken (effect did not exist; tolerance only) | works | `jb.chroma_key` | props-video › chroma key |
| AI Portrait, Smart Cutout | missing | unavailable (cloud AI) | — | props-video › chroma key |
| Reset | partial (transform and opacity) | works: transform, opacity, blend mode and panel effects | batch | props-video › reset |

## Properties: Audio

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| Volume (card slider, Volume row), keyframes | works (no keyframes) | works, keyframable; edits the linked audio of camera clips | `set_param volume` / `add_keyframe` | props-audio-speed › audio basic |
| Audio Channels | placeholder | works: None, Left/Right to Stereo, Mono, Swap | `set_audio_channels` (mixer) | props-audio-speed › audio basic |
| Sound Balance (slider, field, diamond) | works (diamond decorative) | works, keyframable | `set_param pan` | props-audio-speed › audio basic |
| Fade In / Fade Out | works | works | `set_audio_fades` | props-audio-speed › audio basic |
| Pitch | works | works | `set_pitch` | props-audio-speed › audio basic |
| Equalizer preset | placeholder (Default only) | works: 15 Filmora presets | `set_equalizer` | props-audio-speed › equalizer |
| Equalizer Setting | no-op | works: 10 band sliders, Reset | `set_equalizer bands` | props-audio-speed › equalizer |
| Mute | missing | works | `set_mute` | props-audio-speed › audio basic |
| Auto Normalization | placeholder (always off) | works: measures loudness, targets −23/−16/−14/−12 LUFS, shows the gain | Tauri `normalize_clip` → `set_normalization` | props-audio-speed › auto normalization |
| Normal Denoise | broken (added unknown effect `jb.audio.denoise`) | works: strength → attenuation limit | `set_audio` `denoise_db` (jb-media denoiser) | props-audio-speed › denoise |
| Hum Removal (50/60 Hz), Hiss Removal (strength) | broken / missing | works | `set_audio_cleanup` (mixer) | props-audio-speed › voice filters, hum and hiss |
| DeReverb | missing | gap: disabled | — | smoke |
| AI Speech Enhancement, Wind Removal, Voice Changer | broken (Wind added an unknown effect) / missing | unavailable (AI) | — | props-audio-speed › denoise |
| Voice Filters (Telephone, Radio, Megaphone, Echo, Small/Big Room, Robot, Lo-Fi, Underwater) | broken (Echo/Telephone added unknown effects) | works | `set_voice_filter` (mixer) | props-audio-speed › voice filters |
| » more tabs (Audio Ducking, AI Vocal Remover, AI Audio Stretch) | no-op | unavailable, listed disabled | — | smoke |
| Reset | works | works | `set_audio` | — |

## Properties: Speed

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| Speed slider / field | works | works | `set_speed` | props-audio-speed › uniform speed |
| Duration | works | works | `set_speed` | props-audio-speed › uniform speed |
| Reverse Speed | works | works | `set_reverse` | props-audio-speed › uniform speed |
| Ripple Edit | placeholder (forgotten on reselect) | works, remembered per user; later clips follow the new length | `move_clip` batch | props-audio-speed › uniform speed |
| Maintain Pitch | placeholder (local state only) | works | `set_maintain_pitch` | props-audio-speed › uniform speed |
| Frame Interpolation | placeholder | Frame Sampling works; Frame Blending gap; Optical Flow unavailable (AI) | — | props-audio-speed › uniform speed |
| Speed Ramping presets (None, Montage, Hero Moment, Bullet Time, Jumper, Flash In, Flash Out) | placeholder (None always highlighted, clicks did nothing) | works, current preset highlighted | `set_speed_ramp` | props-audio-speed › speed ramping |
| Custom ramp curve editor | missing | works: drag points, double-click to add, right-click to remove | `set_speed_ramp custom points` | props-audio-speed › speed ramping |

## Properties: Animation

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| In / Out presets, No Motion | broken (stored as ranged effects instead of animation slots, so not written to `.wfp`) | works | `add_animation` / `remove_animation` | props-anim-color-title › in/out presets |
| Duration | missing | works | `replace_animation` | props-anim-color-title › in/out presets |
| Filter icon | no-op | works: All, Fade, Zoom, Slide, Spin, Bounce, Other | view | props-anim-color-title › in/out presets |
| Download badges | placeholder (decorative, no download exists) | removed (everything is local) | — | — |
| Keyframe Presets tab | placeholder (same grid) | works: Zoom In/Out, Pan Left/Right/Up/Down, Rotate, Fade In, None | `add_keyframe` / `clear_keyframes` | props-anim-color-title › keyframe presets |

## Properties: Color

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| Presets (Default) | works (4 of them) | works; expand bar shows all 10 | `jb.color.adjust` | props-anim-color-title › color basic |
| Presets (Custom), Save as Custom | no-op | works: saved per user, applied as one undo step, right-click deletes | `add_effect` batch | props-anim-color-title › color basic |
| LUT switch, load/change, strength | placeholder (always off) | works with `.cube` files | `jb.color.lut` | props-anim-color-title › LUT |
| Color switch | no-op | works (enables/disables the adjustment) | `set_effect_enabled` | props-anim-color-title › color basic |
| Auto White Balance | no-op | works (gray world on the source frame) | `jb.color.adjust temperature/tint` | props-anim-color-title › color basic |
| Eyedropper | no-op | works (pick a neutral point in the player) | same | — (needs a decoded frame) |
| Temperature, Tint, Vibrance, Saturation, Light sliders, Light reset | works (not keyframable) | works, keyframable | `set_effect_params` | props-anim-color-title › color basic |
| Vignette | missing | works | `jb.color.adjust vignette` | smoke |
| HSL | placeholder ("coming soon") | works: 8 colors × hue/saturation/luminance | `jb.color.hsl` | props-anim-color-title › HSL, curves, wheels |
| Curves | placeholder | works: RGB/Red/Green/Blue curve editor, strength, reset | `jb.color.curves` | props-anim-color-title › HSL, curves, wheels |
| Color Wheels | placeholder | works: shadows/midtones/highlights/global wheels + luminance | `jb.color.wheels` | props-anim-color-title › HSL, curves, wheels |
| Compare | no-op | works: turns the color adjustments off/on | `set_effect_enabled` | props-anim-color-title › color basic |
| » more (Color Match, AI Color Palette) | no-op | listed disabled (gap / AI) | — | smoke |
| Reset | partial (basic only) | works: all color effects | `remove_effect` batch | props-anim-color-title › color basic |

## Properties: Titles and Project Info

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| Text | works | works | `set_title_style` | props-anim-color-title › titles |
| Font | missing | works | `set_title_style font_family` | props-anim-color-title › titles |
| Size, Bold, Italic, Color, Position | works | works (via `set_title_style`, also on Filmora title templates) | `set_title_style` | props-anim-color-title › titles |
| Alignment, Opacity, Letter / Line spacing | missing | works | `set_title_style` | props-anim-color-title › titles |
| Stroke, Shadow, Background (switch, color, width/distance/blur/opacity) | missing | works | `set_title_style` | props-anim-color-title › titles |
| Reset | missing | works | `set_title_style` | props-anim-color-title › titles |
| Project name | works | works | `rename_project` | menus › project info |
| Project location | placeholder text | works: click to reveal in Finder/Explorer | Tauri `reveal_path` | — |
| Thumbnail Edit | no-op | gap: disabled (the first video frame is used) | — | smoke |

## Library

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| Import button (chevron) | works (files only) | menu: Import Media Files, Import Media Folder; Import from a Camera or Phone is a disabled gap | Tauri `import_media` (folders expand) | library › import menu |
| Import Media card, OS file drop | works | works | `import_media` | library › import menu |
| Record screen / voiceover / webcam | no-op | gap: disabled | — | library › import menu |
| Sidebar: Project Media / Videos / Audio / Images | placeholder (highlight only; "Folder" did nothing) | works: filters by kind | view | library › sidebar |
| Sidebar: Global Media, Cloud Media, Editing Presets, Photos Library | placeholder | empty states (unavailable/gap) | — | library › sidebar |
| Search | works | works | view | library › search |
| Filter (sort / unused only) | no-op | works: sort by date/name/duration/type; unused only | view | library › search, sort |
| More (⋯) | no-op | works: Select All, Delete Unused Media | `remove_media` batch | library › search, sort |
| Thumbnail size slider | placeholder (decorative) | works, remembered per user | view | library › search, sort |
| Media card: select, ⌘/Shift multi-select, double-click (source), "+", drag to timeline | works | works ("+" uses the source in/out range) | `add_clip` | library › media card, drag |
| Card menu: Add to Timeline, Open in Source Monitor | works | works | — | library › media card |
| Card menu: Rename | placeholder (disabled) | works (inline) | `rename_media` | library › media card |
| Card menu: Reveal in Finder, Copy File Path, Delete | works / missing / works | works (Delete disabled with a reason when the media is used) | `reveal_path`, `remove_media` | library › media card |
| Stock Media (Videos, Photos) | placeholder ("Nothing here yet") | unavailable: empty state | — | library › empty states |
| Audio: Project Audio / Music / Sound Effects | partial | Project Audio works (import here too); Music and Sound Effects unavailable empty states | — | library › empty states |
| Titles, Transitions, Effects, Filters: categories, drag to timeline | works | works | `add_clip`, `set_transition`, `add_effect` | library › titles, effects |
| Catalog "+" button, double-click, card menu Apply | missing | works: title at the playhead, effect/filter on the selected clips, transition on the nearest cut | same | library › titles, effects |
| Favorites (star, sidebar Favorites, card menu) | missing | works, per user | localStorage | library › titles, effects |
| Card previews: still, looping preview on hover, overlay buttons (Preview, menu, favorite, add) | generic icon | works: rendered by jb-render on our procedural sample footage, cached per renderer version, loaded as cards scroll into view; also on Animation presets | `jb://fxpreview`, `jb fx previews` | previews › cards |
| Click a card: plays it in the player (Source tab), once, then rests on frame 0; Timeline goes back | missing | works: transitions/animations/titles on the sample footage (2 s), effects/filters on the timeline at the playhead (5 s), like Filmora | `preview_start` / `preview_stop` | previews › clicking a card |
| Effect sliders update the paused frame while dragging | on release only | works (live, no undo step until release) | `preview_command` | — |
| Catalog filter / more menu | no-op | Sort by name; Clear Favorites | view | library › titles, effects |
| Stickers, Templates | placeholder | unavailable: empty state | — | library › empty states |

## Menus (window menu bar on Windows and in the browser; native menu on macOS) and shortcuts

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| File: New, Open, Save, Save As, Import Media Files, Export | works | works | project I/O | menus › File |
| File: Import Media Folder | missing | works | `import_media` | library › import menu |
| File: Quit (window menu) | missing | works (asks to save) | Tauri `quit_app` | — |
| Edit: Undo/Redo, Cut/Copy/Paste, Duplicate, Delete, Ripple Delete | broken in the window menu: disabled states were computed once at startup, so Copy/Paste/Delete stayed disabled after selecting clips | works (state read when the menu opens) | history, clip commands | menus › Edit |
| Edit: Select All, Split | works | works | `split_clip` | menus › Edit |
| Edit: Add Marker (M) | missing | works | `add_marker` | menus › Edit, player › markers |
| Tools: Add Video/Audio Track, Snapping | works | works | `add_track` | menus › Tools |
| Tools: Freeze Frame, Detach Audio, Mute, Stabilization, Chroma Key | missing | works | `freeze_frame`, `detach_audio`, `set_mute`, `jb.stabilize`, `jb.chroma_key` | menus › Tools |
| View: Zoom In/Out/Fit, Play/Pause, Stop, Previous/Next Frame | works | works | — | menus › View |
| View: Mark In/Out, Clear In/Out, Full Screen Preview, Show Playback Statistics | missing | works | — | menus › View |
| Help: Keyboard Shortcuts | works (toast) | works | — | menus › Help |
| Help: About (window menu) | broken in the app (`window.open` does nothing in the webview) | replaced by "Jibaru Editor on GitHub" and "Report an Issue", opened in the browser | Tauri `open_url` | menus › Help |
| Shortcuts: Space, J/K/L, ←/→ (Shift ×5), Home/End, I/O, M, N, ⌘B/C/X/V/D/A/Z, ⌘=/−, ⇧Z, ⌫/⇧⌫, Esc | works (no M; Esc only cleared the selection) | works; Esc also leaves full screen and the eyedropper | — | menus › View, player |

## Title bar

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| What's new | no-op | opens the releases page | `open_url` | menus › Help, title bar |
| Share | no-op | unavailable (cloud): disabled | — | menus › title bar |
| Feedback | no-op | opens a new issue | `open_url` | menus › title bar |
| Layout | no-op | Default / Larger Player / Larger Timeline | view | menus › title bar |
| Save | works | works | `save_project` | menus › title bar |
| Cloud backup | no-op | unavailable (cloud): disabled | — | menus › title bar |
| Support | no-op | opens the documentation | `open_url` | menus › title bar |
| More (apps) | no-op | About, Keyboard Shortcuts, GitHub | — | menus › title bar |
| Export | works | works | — | menus › export dialog |
| Background task chip (export, proxies) | works | works | — | — |

## Export dialog

| Control | Before | Now | Backend | Test |
|---|---|---|---|---|
| Local / Device / Social | Local works; Device and Social disabled | same, with reasons (Device: gap; Social: cloud) | — | menus › export dialog |
| Name, Save to, folder picker | works | works | — | menus › export dialog |
| Format (MP4, MOV), Encoder (H.264, HEVC), Resolution, Frame Rate, Quality | works | works | Tauri `export_start` | menus › export dialog |
| Range: Entire project, In/Out marks, Custom from/to | works | works | `range` | menus › export dialog |
| Audio | works | works | `audio` | menus › export dialog |
| Hardware Encoding | missing (always on) | works, remembered | `hardware` | menus › export dialog |
| Export, Cancel, Open Folder, New export, Close | works | works | export job | — (needs the native engine) |

## Remaining gaps

Local features Jibaru does not have yet. Each control is disabled and says so:

- Masks: pen, text and image (custom) masks. Path Curve and Anchor Point in Transform.
- Frame Blending interpolation, DeReverb, Color Match, split view, the project thumbnail editor.
- Screen, voiceover and webcam recording. Import from a camera or phone, Global Media, the Photos library, Editing Presets.

Wondershare cloud and AI features are unavailable by design: AI Video
Enhancer, AI Portrait, Smart Cutout, Smart Brush mask, AI Speech
Enhancement, Wind Removal, Voice Changer, AI Vocal Remover, AI Audio
Stretch, Optical Flow, AI Color Palette, Stock Media, music and sound
effect libraries, Stickers, Templates, Share and Cloud backup.

`.wfp` persistence: Jibaru-only edits (masks, chroma key, panel effects,
color panel, denoise, channels, voice filters, hum and hiss removal) are kept
in the archive's Jibaru sidecar. Filmora does not show them. Auto Enhance
is written natively. The color panel is not written as Filmora's AdjustColor
until its parameter mapping is verified in Filmora.

The desktop app was not driven through its GUI for this audit, because the
screen was in use. The Tauri commands are covered by the app crate's tests.
The renderer and mixer paths are covered by jb-render and jb-media tests and
by `jb frame` renders of the UI's commands.
