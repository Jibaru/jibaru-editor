<div align="center">

# Jibaru Editor

**A fast, open-source video editor for macOS and Windows — built to be edited by hand and by AI agents.**

[![CI](https://github.com/Jibaru/jibaru-editor/actions/workflows/ci.yml/badge.svg)](https://github.com/Jibaru/jibaru-editor/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Jibaru/jibaru-editor?sort=semver)](https://github.com/Jibaru/jibaru-editor/releases/latest)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Windows-lightgrey)

[Download](https://github.com/Jibaru/jibaru-editor/releases/latest) ·
[Documentation](docs/) ·
[Agents & MCP](docs/agents.md) ·
[Report a bug](https://github.com/Jibaru/jibaru-editor/issues)

</div>

---

Jibaru Editor is a desktop video editor with a familiar, timeline-based
workflow: multitrack editing, transitions, effects, titles, color and audio
tools, real-time GPU preview and hardware-accelerated export. Its editing
engine is headless and fully scriptable, so the same operations available in
the interface can be performed from the command line or by an AI agent over
the [Model Context Protocol](https://modelcontextprotocol.io).

It also opens and saves **Filmora `.wfp` projects**, so existing projects can
be edited, rendered and handed back without conversion.

## Features

### Editing
- Multitrack timeline with split, trim, ripple, roll and slip edits, snapping, markers, groups and linked audio
- Speed control with presets and custom speed ramps, reverse, freeze frame and pitch-preserving retiming
- Keyframes for transform, opacity, volume and every effect parameter
- Unlimited undo and redo, autosave and backups

### Effects and color
- 190+ transitions, effects, filters and in/out animations, rendered on the GPU
- Titles with fonts, stroke, shadow and background
- Color correction: exposure, white balance, HSL, curves, color wheels and `.cube` LUTs
- Masks, chroma key, blend modes, background blur, drop shadow, stabilization and lens correction

### Audio
- Per-clip volume, pan, fades and crossfades
- 10-band equalizer, loudness normalization (LUFS), denoise and voice filters

### Performance
- Rust engine with wgpu compositing on Metal, Direct3D 12 and Vulkan
- Hardware decoding and encoding (VideoToolbox, NVENC, Quick Sync, AMF)
- Smooth 4K editing with automatic proxies (including GoPro `.LRV` files)
- Exports 4K HEVC at roughly 3× real time on an Apple M4 Pro

### Interoperability
- Lossless read/write of Filmora `.wfp` projects: unedited projects round-trip byte for byte
- Renders are measured frame by frame against Filmora's own exports ([fidelity report](docs/fidelity.md))
- Native `.jbp` project format (JSON)

### Built for agents
- Every edit is a serializable, undoable command with a published JSON Schema
- `jb` command-line tool with stable `--json` output for scripts and CI
- `jb-mcp` server so Claude and other MCP clients can inspect, edit and render projects

## Installation

Download the latest version from the
[Releases page](https://github.com/Jibaru/jibaru-editor/releases/latest):

| Platform | File |
| --- | --- |
| macOS (Apple silicon) | `Jibaru-Editor-<version>-macos-arm64.dmg` |
| Windows (x64) | `Jibaru-Editor-<version>-windows-x64-setup.exe` |
| Command-line tools | `jb-<version>-<platform>.zip` |

The app updates itself: it checks for new versions at startup and from
**Help → Check for Updates…**. Every update is verified against the
project's signing key before it is installed.

> **First launch on macOS:** builds are not yet notarized by Apple. Right-click
> the app in Applications and choose **Open** the first time.
>
> **First launch on Windows:** until the installer is code-signed, SmartScreen
> may show a warning. Choose **More info → Run anyway**.

## Using it with AI agents

Add the MCP server to Claude Code:

```sh
claude mcp add jibaru -- /path/to/jb-mcp
```

Then ask, for example, *"open ~/Videos/trip.wfp, cut the silences longer than
two seconds and export a 1080p preview"*. See [docs/agents.md](docs/agents.md)
for Claude Desktop, the full tool list and the CLI contract.

The same engine is available from the shell:

```sh
jb info project.wfp                       # tracks, clips, media, duration
jb exec project.wfp '{"op":"split_clip","clip_id":"…","at":625000000}'   # times in 100 ns ticks
jb render project.wfp -o out.mp4 --preview 540p
jb frame project.wfp --at 62.5 -o frame.png
jb fx coverage project.wfp                # effects used and whether they are supported
```

## Architecture

```
            ┌──────────────┐   ┌──────────┐   ┌──────────┐
            │ Desktop app  │   │  jb CLI  │   │  jb-mcp  │
            │ Tauri + React│   │          │   │  (agents)│
            └──────┬───────┘   └────┬─────┘   └────┬─────┘
                   └────────────────┼──────────────┘
                          Command bus + undo (jb-core)
          ┌──────────────┬──────────┴───┬───────────────┐
     jb-filmora      jb-render       jb-media        jb-export
   .wfp read/write  GPU compositor  FFmpeg decode   encode + mux
                    effects (WGSL)  audio mixing
```

| Crate | Purpose |
| --- | --- |
| [`jb-core`](crates/jb-core) | Project model, timeline evaluation, command bus and undo |
| [`jb-filmora`](crates/jb-filmora) | Filmora `.wfp` reader/writer and effect coverage scanner |
| [`jb-media`](crates/jb-media) | Probing, hardware decoding, proxies, caches and audio processing |
| [`jb-render`](crates/jb-render) | wgpu compositor and the effect, transition and title registry |
| [`jb-export`](crates/jb-export) | Parallel render → encode → mux pipeline |
| [`jb-cli`](crates/jb-cli) | The `jb` command-line tool |
| [`jb-mcp`](crates/jb-mcp) | MCP server exposing the command bus |
| [`app`](app) | Desktop application (Tauri 2, React, TypeScript) |

## Building from source

```sh
brew install ffmpeg pkgconf        # macOS; see docs/building.md for Windows
cargo build --release
cargo test --workspace
cd app && pnpm install && pnpm tauri dev
```

Full instructions, benchmarks and environment variables are in
[docs/building.md](docs/building.md). Packaging, signing and releases are
covered in [docs/packaging.md](docs/packaging.md) and
[docs/updates.md](docs/updates.md).

## Documentation

| Document | Contents |
| --- | --- |
| [docs/agents.md](docs/agents.md) | MCP server, CLI contract and editing command reference |
| [docs/building.md](docs/building.md) | Building and testing on macOS and Windows |
| [docs/packaging.md](docs/packaging.md) | Bundling FFmpeg, signing and installers |
| [docs/updates.md](docs/updates.md) | Auto-update and release process |
| [docs/filmora-format.md](docs/filmora-format.md) | The `.wfp` project format |
| [docs/fidelity.md](docs/fidelity.md) | Render fidelity measurements against Filmora |
| [docs/ui-controls.md](docs/ui-controls.md) | Status of every control in the interface |

## Contributing

Bug reports and pull requests are welcome. Please open an
[issue](https://github.com/Jibaru/jibaru-editor/issues) describing the problem
or the change before starting larger work. Code and comments are written in
English, every edit goes through `jb-core` commands, and `cargo test
--workspace` plus `pnpm typecheck` in `app/` should pass before submitting.

## Privacy

Jibaru Editor has no accounts, telemetry or analytics. The only network
request it makes on its own is the update check against GitHub Releases. See
[PRIVACY.md](PRIVACY.md).

## License and notices

Jibaru Editor is licensed under the [Apache License 2.0](LICENSE).

FFmpeg is linked dynamically and only in its LGPL configuration; GPL encoders
such as libx264 and libx265 are never used. Distributed builds bundle an LGPL
FFmpeg build together with its license.

Jibaru Editor is an independent, clean-room project. It is not affiliated with,
endorsed by or sponsored by Wondershare. Filmora is a trademark of its
respective owner and is mentioned only to describe file-format
compatibility. The project contains no code, assets or branding from
Wondershare; `.wfp` support is based on analyzing project files for
interoperability.
