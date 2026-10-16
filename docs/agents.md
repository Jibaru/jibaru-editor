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

Relative file paths given to tools resolve against the server's working directory, which the client chooses. Prefer absolute paths. Media probing needs FFmpeg's `ffprobe` on `PATH`; you can also set `JB_FFPROBE` to its location via the server's `env` entry.

### Tools

| Tool | Purpose |
|---|---|
| `new_project` | Create a `.jbp` file (`path`, `width`, `height`, `fps`, `name`, `overwrite`) and open it. |
| `open_project` | Open a `.jbp` (and `.wfp` once the Filmora importer lands). |
| `save_project` | Save to the session file, or to `path` (save as). Edits stay in memory until you save. |
| `get_project` | Summary of the project. `detail`: `summary` \| `clips` (default) \| `full` (raw model). |
| `list_media` | Media assets with ids, kinds and durations. |
| `import_media` | Probe files and add them to the media bin as one undo step. Returns the new media ids. |
| `execute_command` | Apply one `Command`. Its `inputSchema` embeds the full Command JSON Schema. Supports `dry_run`. |
| `undo` / `redo` | Session history. |
| `evaluate_frame` | Layers and audio voices at a time, so edits can be checked without rendering. |
| `find_clips` | Filter clips by `at`, `from`/`to`, `track_id`/`track_index`, `name`, `media_id` and `sequence_id`. |
| `get_schema` | JSON Schema for `command`, `project` or `frame_plan`. |
| `render`, `preview_frame` | Stubs. They return `not_implemented` until `jb-export`/`jb-render` land. |

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
- **Commands** are JSON objects tagged by `op`: `add_media`, `remove_media`, `add_track`, `remove_track`, `set_track_flags`, `add_clip`, `remove_clip`, `move_clip`, `trim_clip`, `split_clip`, `set_speed`, `set_transform`, `set_opacity`, `set_audio`, `add_effect`, `remove_effect`, `set_effect_params`, `set_transition`, `set_title`, `rename_project` and `batch` (several commands as one undo step). `jb schema command` prints the exact schema.
- **Generated ids**: `clip.id`, `clip.name`, `effect.id`, `transition.id` and `asset.id`/`asset.name` may be omitted, and they are generated.
- Commands are **atomic**. A rejected command, such as one causing overlapping clips, a locked track or an unknown id, leaves the project unchanged.

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
jb open <file.wfp>            # pending jb-filmora
jb convert <in.wfp> <out.jbp> # pending jb-filmora
jb fx coverage <file.wfp>     # pending jb-filmora
jb render <project> -o out.mp4 [--preview 540p] [--range A-B]   # pending jb-export
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
