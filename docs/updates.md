# Updates and releases

Jibaru Editor updates itself on macOS and Windows with
[tauri-plugin-updater](https://v2.tauri.app/plugin/updater/). Releases are
GitHub Releases of this repository; there is no update server.

## How the app updates

- **Endpoint:** `https://github.com/Jibaru/jibaru-editor/releases/latest/download/latest.json`
  (`plugins.updater` in `app/src-tauri/tauri.conf.json`). GitHub redirects
  `releases/latest` to the newest published, non-prerelease release, so
  publishing a release is what ships an update.
- **When:** 8 s after startup and then every 24 h while the app runs
  (`startUpdateChecks` in `app/src/updates.ts`), unless the user turned off
  **Help > Check for Updates Automatically**. **Help > Check for Updates...**
  checks on demand and also reports "You're up to date" and errors. Both
  items are in the native macOS menu (`app/src-tauri/src/menu.rs`) and the
  in-window menu bar on Windows (`app/src/actions.ts`).
- **Notice:** a card in the bottom-right corner
  (`app/src/components/UpdateNotice.tsx`): "A new version X is available",
  the release notes, and **Update now**, **Remind me later** (hidden until
  the next check) and **Skip this version** (automatic checks ignore that
  version; a manual check still offers it). Automatic notices wait while an
  export runs.
- **Update now:** asks to save unsaved changes (Save / Don't Save / Cancel),
  downloads the package with a progress bar, and the plugin verifies its
  signature with the public key in `tauri.conf.json`; a package whose
  signature or signed version (`requireSignedVersion`) does not match is
  rejected. Installing waits until a running export finishes, also when the
  export started during the download. Then:
  - **macOS:** the `.app` is replaced in place (an administrator prompt
    appears only when the user cannot write to its folder) and the app
    relaunches.
  - **Windows:** the NSIS installer runs in passive mode (progress bar, no
    questions), replaces the files including the FFmpeg DLLs and starts the
    new version.

The automation hook (`JB_AUTOMATION_DIR`) exposes the same functions as
`window.__jb.updates` (`checkForUpdates(true)`, `updateNow({ prompt: false })`,
`updatePhase()`), which the Windows update test uses.

## Signing key

Update packages are signed with a minisign key made by
`pnpm tauri signer generate`. This is not code signing: it only proves to
the updater that a package comes from us. The public key is committed in
`tauri.conf.json`; the private key is stored outside the repository:

| File | Contents |
| --- | --- |
| `~/.tauri/jibaru-editor.key` | Private key (encrypted with the password) |
| `~/.tauri/jibaru-editor.key.password` | Its password |
| `~/.tauri/jibaru-editor.key.pub` | Public key (same as in `tauri.conf.json`) |

and as the repository secrets `TAURI_SIGNING_PRIVATE_KEY` (the key file's
contents) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

**Back up the key and the password** (a password manager holds both). If the
key is lost, installed apps can no longer be updated: a new key means
shipping a release that users must download and install by hand once. Never
commit the private key. To restore the secrets:

```sh
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/jibaru-editor.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD < ~/.tauri/jibaru-editor.key.password
```

## Release artifacts

`.github/workflows/build.yml` makes the updater packages when the signing
secrets exist (pushes to `main`, releases; not PRs from forks):

| Platform | Package | Made by |
| --- | --- | --- |
| `darwin-aarch64` | `Jibaru-Editor-<v>-macos-arm64.app.tar.gz` + `.sig` | `scripts/package/macos.sh`: a tarball of the final `.app`, after the LGPL FFmpeg dylibs are bundled and everything is signed, then `tauri signer sign --app-version <v>` |
| `windows-x86_64` | `Jibaru-Editor-<v>-windows-x64-setup.exe` + `.sig` | `tauri build` with `createUpdaterArtifacts` (set by `scripts/package/windows-prepare.ps1`); the installer carries the FFmpeg DLLs |

The tarball the Tauri bundler would make on macOS predates the FFmpeg
bundling, so it is not used. Intel macOS (`darwin-x86_64`) is picked up by
the manifest script when a `macos-x86_64` package exists, but is not built
yet.

`scripts/release/latest-json.py` checks every signature against the public
key (`scripts/release/verify-sig.py`, pure Python) and writes `latest.json`:

```json
{
  "version": "0.1.1",
  "notes": "<release notes>",
  "pub_date": "2026-10-16T19:00:00Z",
  "platforms": {
    "darwin-aarch64": { "url": ".../releases/download/v0.1.1/Jibaru-Editor-0.1.1-macos-arm64.app.tar.gz", "signature": "..." },
    "windows-x86_64": { "url": ".../releases/download/v0.1.1/Jibaru-Editor-0.1.1-windows-x64-setup.exe", "signature": "..." }
  }
}
```

## Cutting a release

```sh
scripts/release/bump.sh 0.2.0          # Cargo.toml, tauri.conf.json, package.json, Cargo.lock
git commit -am "Release 0.2.0"
git tag v0.2.0
git push origin main v0.2.0
```

The tag runs `.github/workflows/release.yml`:

1. `build`: packages for both platforms, smoke tested, updater signatures
   checked; plus an older (0.0.1) Windows installer for step 3.
2. `publish`: creates the release with generated notes (edit them on GitHub
   before or after; the notes in `latest.json` are taken at publish time),
   writes `latest.json`, uploads every file with `SHA256SUMS.txt` and
   publishes the release as latest. The tag must match the version.
3. `update-test-windows`: downloads the live `latest.json` and verifies the
   Windows package's signature, then installs the previous release (or the
   0.0.1 build when no earlier release has the updater) silently, runs its
   update check through the automation hook and requires the installed
   version to become the new one, with the FFmpeg DLLs in place and the app
   relaunched.

A manual run (`gh workflow run release.yml [-f tag=v0.2.0]`) only refreshes a
draft release, which the updater does not see. To fix release notes after
publishing, edit the release and rerun the workflow, or edit `latest.json`.

## Testing an update locally (macOS)

Build the previous version with the same public key, run it outside
`/Applications` and trigger the check:

```sh
git worktree add /tmp/jb-old <previous tag or commit> && cd /tmp/jb-old/app
pnpm install && pnpm tauri build --bundles app
cp -R "../target/release/bundle/macos/Jibaru Editor.app" ~/tmp-update-test/
JB_AUTOMATION_DIR=/tmp/jb-auto "$HOME/tmp-update-test/Jibaru Editor.app/Contents/MacOS/jibaru-editor" &
echo "await __jb.updates.checkForUpdates(true); await __jb.updates.updateNow({ prompt: false })" > /tmp/jb-auto/u.js
# after the relaunch:
defaults read "$HOME/tmp-update-test/Jibaru Editor.app/Contents/Info.plist" CFBundleShortVersionString
```

or use Help > Check for Updates... in the app.
