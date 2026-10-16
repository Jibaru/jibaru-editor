# Jibaru Editor

An open-source, cross-platform (macOS and Windows) video editor with a workflow
modeled on Filmora, built to be driven by AI agents as well as by hand.

- **Native performance:** Rust engine, GPU compositing with wgpu (Metal / D3D12 / Vulkan), FFmpeg decoding and hardware encoding.
- **Filmora project compatibility:** opens and saves `.wfp` projects with a lossless round-trip.
- **Agent-first:** a headless core with a serializable, undoable command bus, exposed through a CLI (`jb --json`) and an MCP server.

This is a clean-room implementation. It contains no code, assets or branding
from Wondershare; `.wfp` support is based on analyzing project files for
interoperability.

## Layout

| Path | Purpose |
| --- | --- |
| `crates/jb-core` | Project model, timeline evaluation, command bus and undo |
| `crates/jb-filmora` | `.wfp` reader/writer and effect coverage scanner |
| `crates/jb-media` | Media probing and decoding (FFmpeg) |
| `crates/jb-render` | GPU compositor, effects and transitions |
| `crates/jb-export` | Encoding sequences to video files |
| `crates/jb-cli` | The `jb` command-line tool |
| `crates/jb-mcp` | MCP server for agents |
| `app` | Desktop app (Tauri + React) |

## Building

Requirements: a stable Rust toolchain, FFmpeg 8 or newer as shared libraries
(tested with 9.0) and libclang (used by the FFmpeg bindings). The `ffmpeg`
command-line tool must be on `PATH` to run the tests, which generate their
own small clips.

**macOS**

```sh
brew install ffmpeg pkgconf
cargo build --release
cargo test --workspace
```

**Windows** (PowerShell): download an LGPL *shared* FFmpeg build such as
`ffmpeg-n9.0-latest-win64-lgpl-shared-9.0.zip` from
[BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds/releases), extract
it, then:

```powershell
$env:FFMPEG_DIR = "C:\path\to\ffmpeg-n9.0-latest-win64-lgpl-shared-9.0"
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"   # winget install LLVM.LLVM
$env:PATH = "$env:FFMPEG_DIR\bin;$env:PATH"           # DLLs at runtime
cargo build --release
cargo test --workspace
```

Benchmarks:

```sh
cargo run --release -p jb-media --example bench_decode -- /path/to/clip.mp4
cargo run --release -p jb-export --example bench_export -- /path/to/clip.mp4
```

Set `JB_NO_HWDEC=1` / `JB_NO_HWENC=1` to force software decoding / encoding
and `JB_FFMPEG_LOG=info` for FFmpeg logs.

### FFmpeg licensing

FFmpeg is always linked dynamically and the editor only uses LGPL
components: hardware encoders (VideoToolbox, NVENC, Quick Sync, AMF, Media
Foundation), libopenh264 or FFmpeg's native MPEG-4 encoder as software
fallbacks, and the native AAC encoder. GPL encoders (libx264, libx265) are
never used. The Homebrew FFmpeg is a GPL build and is fine for development;
distributed builds must bundle an LGPL build.

## License

Apache-2.0. FFmpeg is linked dynamically in its LGPL configuration.
