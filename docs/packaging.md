# Packaging and releases

Distributed builds bundle an **LGPL, shared-library FFmpeg** so they run on
machines without Homebrew or any FFmpeg install. GPL FFmpeg builds (such as
Homebrew's, which enables libx264/libx265) are only for development.

| Package | Contents |
| --- | --- |
| `Jibaru-Editor-<v>-macos-arm64.dmg` / `.app.zip` | The app; FFmpeg dylibs in `Contents/Frameworks`, licenses in `Contents/Resources/licenses` |
| `Jibaru-Editor-<v>-windows-x64-setup.exe` | NSIS installer; FFmpeg DLLs installed next to the executable |
| `jb-<v>-macos-arm64.zip` | `bin/jb`, `bin/jb-mcp`, `bin/ffprobe`, `lib/*.dylib`, licenses |
| `jb-<v>-windows-x64.zip` | `jb.exe`, `jb-mcp.exe`, `ffprobe.exe`, FFmpeg DLLs, licenses |
| `SHA256SUMS.txt` | Checksums (release only) |

`jb import-media` currently probes files with the `ffprobe` tool; the CLI
looks for it next to its own executable first (then `JB_FFPROBE`, then
`PATH`), so the CLI packages ship the LGPL `ffprobe` from the same build.

macOS packages are arm64 only for now; `scripts/ffmpeg/build-macos.sh`
accepts `x86_64` for an Intel build.

## FFmpeg

**macOS** — `scripts/ffmpeg/build-macos.sh <prefix> [arm64|x86_64]` compiles
FFmpeg (`FFMPEG_VERSION`, default 9.0.2) from the ffmpeg.org tarball with
`--enable-shared --disable-autodetect --enable-videotoolbox
--enable-audiotoolbox`, never `--enable-gpl`/`--enable-nonfree`/
`--enable-version3`, and fails if the resulting configuration contains them.
Autodetection is off, so nothing from Homebrew can leak in: the libraries
depend only on system frameworks, zlib and bzip2. Install names are
`@rpath/libavcodec.63.dylib` etc. It takes about 5 minutes; CI caches the
prefix keyed on the script. The license text and exact configure line land in
`<prefix>/share/ffmpeg-license/`.

**Windows** — `scripts/ffmpeg/fetch-windows.ps1` downloads
`ffmpeg-n9.0-latest-win64-lgpl-shared-9.0.zip` from
[BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) and rejects it if
its configuration enables GPL or nonfree parts. In GitHub Actions it also sets
`FFMPEG_DIR`, `LIBCLANG_PATH` and `PATH`.

## Building packages locally

macOS:

```sh
scripts/ffmpeg/build-macos.sh ~/ffmpeg-lgpl
export FFMPEG_DIR=~/ffmpeg-lgpl MACOSX_DEPLOYMENT_TARGET=11.0
cargo build --release -p jb-cli -p jb-mcp
(cd app && pnpm install && pnpm tauri build --bundles app)
scripts/package/macos.sh ~/ffmpeg-lgpl dist
ditto -x -k dist/jb-*-macos-arm64.zip /tmp/jb-pkg
python3 scripts/smoke/smoke.py /tmp/jb-pkg/jb-*/bin --ffmpeg ~/ffmpeg-lgpl/bin/ffmpeg
```

Building with `FFMPEG_DIR` pointing at the LGPL prefix rebuilds
`ffmpeg-sys-next`; use a separate `CARGO_TARGET_DIR` (the packaging script
honors it) to keep the Homebrew development build intact.

`scripts/package/macos.sh` copies the transitive closure of FFmpeg dylibs
into `Contents/Frameworks` (app) or `lib/` (CLI), rewrites every non-system
reference to `@rpath/<name>` with `install_name_tool`, adds
`@executable_path/../Frameworks` (or `../lib`) as `LC_RPATH`, fails if any
binary still references a path outside `/usr/lib`, `/System` or the bundle,
then signs the dylibs and the app (ad-hoc, or with `APPLE_SIGNING_IDENTITY`
and the hardened runtime) and builds the DMG and zips.

Windows (PowerShell, with LLVM installed for libclang):

```powershell
$ff = scripts/ffmpeg/fetch-windows.ps1
$env:FFMPEG_DIR = $ff; $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
$env:RUSTFLAGS = "-C link-arg=/STACK:16777216 -C target-feature=+crt-static"
cargo build --release -p jb-cli -p jb-mcp
cd app; pnpm install
../scripts/package/windows-prepare.ps1 -FfmpegDir $ff
pnpm tauri build --bundles nsis --config src-tauri/tauri.bundle.conf.json
cd ..; scripts/package/windows.ps1 -FfmpegDir $ff -OutDir dist
```

`windows-prepare.ps1` copies the DLLs into `app/src-tauri/ffmpeg-dlls/` and
writes `app/src-tauri/tauri.bundle.conf.json` (both git-ignored), which lists
them as bundle resources so the installer places them next to the
executable. `+crt-static` links the C runtime statically, so neither the CLI
nor the app needs the Visual C++ redistributable; the BtbN DLLs are built
with MinGW and have no such dependency either.

## Smoke test

`scripts/smoke/smoke.py <bin-dir> --ffmpeg <ffmpeg>` runs the packaged
binaries with FFmpeg removed from `PATH`:

1. generates a 2 s 320x180 clip with audio (`testsrc2` + `sine`, MPEG-4/AAC)
   with the given ffmpeg tool;
2. `jb new`, `jb import-media`, `jb exec` (video and audio tracks with the
   clip), `jb render` to MP4 and `jb frame` to PNG, then checks the output
   with ffprobe (video and audio streams, 640x360, ~2 s);
3. an MCP session with `jb-mcp` over stdio: `initialize`, `tools/list`,
   `new_project`, `execute_command`, `get_project`.

On Windows runners this proves decode, render (on the WARP software adapter)
and encode (software fallback) work with only the bundled DLLs.

## CI

`.github/workflows/ci.yml` runs on every push to `main`, on PRs and by hand:

- `rust` (macOS, Windows): build, `hwinfo`, `cargo test --workspace`, with
  Homebrew FFmpeg on macOS and the BtbN LGPL build on Windows.
- `app-frontend`: typecheck and Vite build.
- `packages` (`build.yml`): the packages above, built in release mode
  (`pnpm tauri build` on both platforms), then the smoke test on the CLI
  package, plus launching the app (macOS: from the `.app.zip`; Windows:
  silent NSIS install, FFmpeg DLLs must sit next to the exe, the app must
  stay up for 15 s with FFmpeg off `PATH`). Packages are uploaded as workflow
  artifacts.

### GPU tests on CI runners

Runners have no real GPU: Windows uses WARP and macOS a paravirtualized Metal
device. Golden images in `crates/jb-render/tests/golden` come from real GPUs,
so on software or virtual adapters the noise-driven items (grain, glitch,
VHS, film, ...) are compared and listed in the job summary but not asserted;
all other items keep the normal tolerance. On a real GPU every item is
checked strictly; `JB_GOLDEN_STRICT=1` forces strict checks anywhere. When a
golden fails, the `golden-actual-*` artifact holds the rendered images.

## Releases

`.github/workflows/release.yml` runs on `v*` tags and by hand
(`gh workflow run release.yml [-f tag=v0.1.0]`). It runs `build.yml` and
attaches every package plus `SHA256SUMS.txt` to a **draft** release for the
tag (created if missing, files replaced if it exists). Publish the draft from
the GitHub UI after checking it.

To cut a release: bump `version` in `Cargo.toml`,
`app/src-tauri/tauri.conf.json` and `app/package.json`, commit, then
`git tag v<version> && git push origin v<version>`.

### Signing

Signing happens only when the corresponding repository secrets exist:

| Secret | Use |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64 `.p12` with a Developer ID Application certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Its password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | Notarization (app-specific password); the DMG is notarized and stapled |
| `WINDOWS_CERTIFICATE` | Base64 `.pfx` code-signing certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | Its password |

Without them, the macOS app is ad-hoc signed (users must right-click > Open
the first time, or run `xattr -dr com.apple.quarantine "/Applications/Jibaru
Editor.app"`) and Windows binaries are unsigned (SmartScreen warns).

## LGPL compliance

FFmpeg is dynamically linked and the libraries are shipped as separate,
replaceable files. Each package carries the LGPL text, FFmpeg's `LICENSE.md`
/ `LICENSE.txt`, a notice, and (macOS) `BUILD.txt` with the source URL and
exact configuration. The build scripts above reproduce the libraries.
