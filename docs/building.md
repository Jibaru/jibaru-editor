# Building from source

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
$env:RUSTFLAGS = "-C link-arg=/STACK:16777216"        # bindgen needs a bigger stack
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

## Packaging and releases

Distributed builds bundle an LGPL shared FFmpeg (built from source on macOS,
BtbN's LGPL build on Windows) inside the app and the CLI packages:

```sh
scripts/ffmpeg/build-macos.sh ~/ffmpeg-lgpl          # LGPL FFmpeg, ~5 min
FFMPEG_DIR=~/ffmpeg-lgpl cargo build --release -p jb-cli -p jb-mcp
(cd app && FFMPEG_DIR=~/ffmpeg-lgpl pnpm tauri build --bundles app)
scripts/package/macos.sh ~/ffmpeg-lgpl dist          # .dmg, .app.zip, jb CLI zip
```

CI builds and smoke tests these packages on every push to `main`; pushing a
`v*` tag (after `scripts/release/bump.sh <version>`) publishes them as a
GitHub Release, which the app's built-in updater picks up. See
[docs/packaging.md](docs/packaging.md) for Windows, signing and details, and
[docs/updates.md](docs/updates.md) for updates and cutting a release.

## Running the desktop app

The preview, audio, export and proxies run in the native
engine (see `app/src-tauri/src/playback.rs`):

```sh
cd app && pnpm install && pnpm build && cd ..
cargo build --release -p jibaru-editor --features tauri/custom-protocol
./target/release/jibaru-editor path/to/project.wfp   # or `cd app && pnpm tauri dev`
```

`JB_PLAYBACK_LOG=1` logs displayed/rendered fps and dropped frames while
playing. `JB_AUTOMATION_DIR=<dir>` (development only) evaluates every `*.js`
file dropped into `<dir>` in the webview, with the editor store exposed as
`window.__jb`, for scripted UI tests.

## FFmpeg licensing

FFmpeg is always linked dynamically and the editor only uses LGPL
components: hardware encoders (VideoToolbox, NVENC, Quick Sync, AMF, Media
Foundation), libopenh264 or FFmpeg's native MPEG-4 encoder as software
fallbacks, and the native AAC encoder. GPL encoders (libx264, libx265) are
never used. The Homebrew FFmpeg is a GPL build and is fine for development;
distributed builds bundle an LGPL build (see
[docs/packaging.md](docs/packaging.md)).
