# Driving Jibaru Editor from agents

Jibaru Editor has two agent-facing front ends. Both run on the same command bus (`jb_core::Command`) that the UI uses:

- **`jb-mcp`** is an MCP server over stdio. It holds one open project session with undo/redo.
- **`jb`** is a stateless CLI. Each invocation loads a project, does one thing and saves the result. It is meant for scripts and CI.

Build both with `cargo build --release -p jb-cli -p jb-mcp`. The binaries end up in `target/release/` (`jb`, `jb-mcp`; `.exe` on Windows).

## Connecting jb-mcp

### Claude Code

```sh
claude mcp add jibaru -- /absolute/path/to/target/release/jb-mcp
```

Or check it into the project as `.mcp.json`:

```json
{
  "mcpServers": {
    "jibaru": { "command": "/absolute/path/to/target/release/jb-mcp", "args": [] }
  }
}
```

### Claude Desktop

Edit `claude_desktop_config.json`. On macOS it is in `~/Library/Application Support/Claude/`, and on Windows in `%APPDATA%\Claude\`:

```json
{
  "mcpServers": {
    "jibaru": { "command": "/absolute/path/to/jb-mcp" }
  }
}
```

On Windows, use a path like `"C:\\tools\\jb-mcp.exe"`. Restart Claude Desktop after editing.

Relative file paths given to tools resolve against the server's working directory, which the client chooses. Prefer absolute paths.

### Tools

| Tool | Purpose |
|---|---|
| `new_project` | Create a `.jbp` file (`path`, `width`, `height`, `fps`, `name`, `overwrite`) and open it. |
| `open_project` | Open a `.jbp` or a Filmora `.wfp` (its original documents are kept for a lossless save). |
| `save_project` | Save to the session file, or to `path` (save as), as `.jbp` or (for projects opened from a `.wfp`) `.wfp`. Edits stay in memory until you save; `warnings` lists edits Filmora cannot store. |
| `get_project` | Summary of the project. `detail`: `summary` \| `clips` (default) \| `full` (raw model). |
| `list_media` | Media assets with ids, kinds and durations. |
| `import_media` | Probe files and add them to the media bin as one undo step. Returns the new media ids. |
| `execute_command` | Apply one `Command`. Its `inputSchema` embeds the full Command JSON Schema. Supports `dry_run`. |
| `undo` / `redo` | Session history. |
| `evaluate_frame` | Layers and audio voices at a time, so edits can be checked without rendering. |
| `find_clips` | Filter clips by `at`, `from`/`to`, `track_id`/`track_index`, `name`, `media_id` and `sequence_id`. |
| `get_schema` | JSON Schema for `command`, `project` or `frame_plan`. |
| `render` | Encode the session (or a range, `preview: "540p"` for drafts) to a video file. |
| `preview_frame` | Render one frame to a PNG/JPEG, to look at an edit. |
| `list_presets` | Speed ramp, equalizer, blend mode, animation and keyframe-path names accepted by the commands. |
| `normalize_audio` | Measure a clip's loudness and normalize it to `target_lufs` (Auto Normalization). |

Resources: `jb://schema/command`, `jb://schema/project`, `jb://schema/frame_plan`, `jb://session`, `jb://project` and `jb://project/full`.

Every mutating tool returns a `session` object: `path`, `dirty`, `can_undo`, `can_redo` and `duration`. `execute_command`, `import_media`, `undo` and `redo` also return `created` / `removed` id lists for `tracks`, `clips`, `media` and `effects`. That is how you learn the id of a new track or of the right half of a split.

Errors come back as tool results with `isError: true` and `structuredContent.error = {code, message, hint}`. A "not found" error's `hint` lists the valid ids. The server may handle concurrent requests in any order, so wait for each response before sending the next mutating call.

### A typical session

```jsonc
new_project     {"path": "/work/demo.jbp", "width": 1920, "height": 1080, "fps": "30"}
import_media    {"paths": ["/work/a.mp4"]}                       // -> added[0].id = M
execute_command {"command": {"op": "add_track", "kind": "video"}} // -> created.tracks[0] = T
execute_command {"command": {"op": "add_clip", "track_id": "T",
                 "clip": {"source": {"type": "media", "media_id": "M"},
                          "start": 0, "end": 50000000, "source_in": 0}}}
execute_command {"command": {"op": "split_clip", "clip_id": "<clip id>", "at": 20000000}}
evaluate_frame  {"at": "00:00:03"}
save_project    {}
```

## Conventions shared by both front ends

- **Times inside commands** are integer ticks of 100 ns: `10_000_000` per second.
- **Time arguments** (`--at`, `at`, `from`, `to`, `--range`) accept seconds (`5`, `1.5s`, or a JSON number in MCP), `HH:MM:SS.mmm`, `MM:SS`, `HH:MM:SS:FF`, frames (`120f`) and ticks (`50000000t`).
- **Commands** are JSON objects tagged by `op` (`batch` applies several as one undo step). `jb schema command` / `get_schema` print the exact schema; `jb presets` / `list_presets` list the accepted preset names. See [Editing reference](#editing-reference).
- **Generated ids**: `clip.id`, `clip.name`, `effect.id`, `transition.id`, `marker.id` and `asset.id`/`asset.name` may be omitted, and they are generated.
- Commands are **atomic**. A rejected command, such as one causing overlapping clips, a locked track or an unknown id, leaves the project unchanged.
- **Filmora projects**: `.wfp` files open with their original documents kept. Saving back to `.wfp` patches only what was edited, so everything Jibaru does not model survives; edits Filmora has no place for (track names and heights, clip groups) come back as `warnings`. See `docs/filmora-format.md` §13 for how each property is stored.

## Editing reference

Every Filmora property-panel feature is a command. Times are ticks (10_000_000 per second); keyframe and marker times on clips are clip-local.

| Area | Commands |
|---|---|
| Project & media | `rename_project`, `set_project_settings` (`width`, `height`, `frame_rate`, `background`; sizes even), `rename_media`, `add_media`, `remove_media` |
| Tracks | `add_track`, `remove_track`, `set_track_flags` (lock/mute/hide), `move_track` (reorder, 0 = bottom), `rename_track`, `set_track_height` |
| Placing clips | `add_clip`, `insert_clip` (ripple insert), `overwrite_clip`, `move_clip` (moves link group and group), `remove_clip` (`ripple`), `duplicate_clip`, `replace_media`, `set_clip_enabled` |
| Trimming | `trim_clip`, `ripple_trim` (`edge`: `start`/`end`, `to`), `roll_edit` (cut between a clip and the next adjacent one), `slip_clip` (`delta` source ticks), `split_clip` |
| Structure | `link_clips`, `unlink_clips`, `group_clips`, `ungroup_clips`, `detach_audio`, `create_compound_clip` (nested sequence), `paste_attributes` (`transform`, `crop`, `opacity`, `blend_mode`, `effects`, `animations`, `audio`, `speed`) |
| Transform | `set_transform`, `set_crop` (`[left, top, right, bottom]` insets), `set_flip`, `set_rotation`, `set_fit_mode` (`fit`/`fill`/`stretch`/`none`), `set_blend_mode` (22 Filmora modes), `set_opacity` |
| Keyframes | `add_keyframe`, `remove_keyframe`, `move_keyframe`, `clear_keyframes`, `set_param`. `param`: `transform.position_x`/`position_y`/`scale_x`/`scale_y`/`scale`/`rotation`, `opacity`, `volume`, `pan`, `speed`, `effect.<effect_id>.<param>`. `interpolation`: `linear`, `hold`, `ease_in`, `ease_out`, `ease_in_out`. |
| Speed | `set_speed` (constant, `reverse`), `set_speed_ramp` (`montage`, `hero_moment`, `bullet_time`, `jumper`, `flash_in`, `flash_out`, `custom` with `points: [{position 0..1 of the source, speed}]`, `none`), `set_reverse`, `set_maintain_pitch`, `freeze_frame` (`at`, `duration`) |
| Audio | `set_audio`, `set_volume`, `set_pan`, `set_pitch` (semitones), `set_mute`, `set_audio_fades`, `set_normalization` (or the `normalize_audio` tool / `jb normalize`, which measure loudness), `set_equalizer` (preset or 10 `bands` in dB), `set_audio_channels` (`stereo`, `left`, `right`, `mono`, `swap`), `set_voice_filter` (`telephone`, `radio`, `megaphone`, `echo`, `small_room`, `big_room`, `robot`, `lofi`, `underwater`, or `null`), `set_audio_cleanup` (`hum_removal`: 50/60 Hz mains notch, `hiss_removal`: 0..100; `null` turns one off). On a camera video clip they change its linked audio clip. |
| Effects & animations | `add_effect`, `remove_effect`, `set_effect_params`, `set_effect_enabled`, `add_animation` (`placement`: `in`/`out`/`loop`/`combo`, `duration`), `remove_animation`, `replace_animation`, `set_transition` |
| Titles | `set_title`, `set_title_style` (text, font, size, color, bold, italic, alignment, position, stroke/shadow/background (`null` removes), spacing, `animation`); works on title clips and on Filmora title template instances |
| Markers | `add_marker` (sequence, or `clip_id`), `update_marker`, `remove_marker` |

Split, trims, roll edits and speed changes keep keyframes, speed ramps, ranged effects, animations and markers attached to the content: the right half of a split continues every curve exactly where the left half stops.

```jsonc
// Ken Burns push-in with an ease, a Flash in ramp, and a -14 LUFS music bed.
{"op": "add_keyframe", "clip_id": "C", "param": "transform.scale", "time": 0, "value": {"type": "float", "value": 1.0}}
{"op": "add_keyframe", "clip_id": "C", "param": "transform.scale", "time": 50000000, "value": {"type": "float", "value": 1.2}, "interpolation": "ease_in_out"}
{"op": "set_speed_ramp", "clip_id": "C", "preset": "flash_in"}
{"op": "add_animation", "clip_id": "C", "kind": "jb.anim.fade_in", "placement": "in", "duration": 5000000}
{"op": "set_equalizer", "clip_id": "M", "preset": "low_bass"}
```

## CLI contract

```
jb new <file.jbp> [--width 1920] [--height 1080] [--fps 30] [--name N] [--force] [--dry-run]
jb info <project> [--detail summary|clips|full]
jb exec <project> '<Command JSON | JSON array>' [--output other.jbp] [--dry-run]
jb exec <project> --file cmds.jsonl            # JSON, array or JSON Lines; `-` = stdin
jb schema [command|project|frame-plan|all]
jb eval <project> --at <time> [--sequence ID]
jb find-clips <project> [--at T] [--range A-B] [--track ID|INDEX] [--name S] [--media ID]
jb import-media <project> <files...> [--dry-run]
jb open <file.wfp> [--detail summary|clips|full]
jb convert <in.wfp> <out.jbp> [--force]
jb save <in> <out> [--force]  # .jbp -> .jbp, .wfp -> .wfp or .jbp (edits patched into the Filmora docs)
jb normalize <project> --clip ID [--target -14] [-o out]
jb presets                    # names accepted by the editing commands
jb fx coverage <file.wfp>
jb fx previews [-o DIR] [--filter S] [--force]  # library card stills + hover sprites
jb frame <project> --at <time> -o frame.png [--preview 540p]
jb render <project> -o out.mp4 [--preview 540p] [--range A-B] [--sequence ID]
```

The global `--json` flag makes every command print one JSON document on stdout:

- On success: `{"ok": true, "data": {...}}`.
- On failure: `{"ok": false, "error": {"code": "...", "message": "...", "hint": "..."}}`.

Without `--json`, output is human-readable text and errors go to stderr. `jb exec` runs every command in a file or array, or none of them: if command #N fails, nothing is saved, and the error message names the failing index or line.

Exit codes:

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | I/O or internal error (`io_error`, `probe_failed`) |
| 2 | usage error (`usage_error`: unknown flag or subcommand) |
| 3 | file not found (`file_not_found`) |
| 4 | invalid input or rejected command (`parse_error`, `invalid_argument`, `not_found`, `command_rejected`, `track_locked`, `unsupported_format`) |
| 5 | feature not available yet (`not_implemented`) |

Error codes and JSON field names are stable. New fields may be added, but existing ones are not renamed or removed.
