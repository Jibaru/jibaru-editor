# Packages the Windows CLI (jb.exe, jb-mcp.exe, ffprobe.exe and the FFmpeg DLLs) and
# collects the NSIS installer built by `pnpm tauri build --bundles nsis`
# (see windows-prepare.ps1, which bundles the DLLs into the installer).
#
#   scripts/package/windows.ps1 -FfmpegDir <dir> -OutDir <dir>
#
# Outputs in <OutDir>:
#   jb-<version>-windows-x64.zip
#   Jibaru-Editor-<version>-windows-x64-setup.exe   (when the installer exists)
param(
  [Parameter(Mandatory)][string]$FfmpegDir,
  [Parameter(Mandatory)][string]$OutDir
)
$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root "target" }
$release = Join-Path $target "release"
$version = (Select-String -Path (Join-Path $root "Cargo.toml") -Pattern '^version = "(.*)"' |
  Select-Object -First 1).Matches[0].Groups[1].Value
New-Item -ItemType Directory -Force $OutDir | Out-Null
$OutDir = Resolve-Path $OutDir

function Sign-Files([string[]]$files) {
  if (-not ($env:WINDOWS_CERTIFICATE -and $env:WINDOWS_CERTIFICATE_PASSWORD)) { return }
  $pfx = Join-Path $env:RUNNER_TEMP "codesign.pfx"
  if (-not (Test-Path $pfx)) {
    [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))
  }
  & signtool sign /f $pfx /p $env:WINDOWS_CERTIFICATE_PASSWORD /fd sha256 /tr http://timestamp.digicert.com /td sha256 $files
  if ($LASTEXITCODE -ne 0) { throw "signtool failed" }
}

# --- CLI ------------------------------------------------------------------
$name = "jb-$version-windows-x64"
$stage = Join-Path ([IO.Path]::GetTempPath()) $name
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory $stage | Out-Null
Copy-Item (Join-Path $release "jb.exe"), (Join-Path $release "jb-mcp.exe") $stage
Sign-Files @((Join-Path $stage "jb.exe"), (Join-Path $stage "jb-mcp.exe"))
Copy-Item (Join-Path $FfmpegDir "bin\*.dll") $stage
# ffprobe (same LGPL build) is used by `jb import-media`.
Copy-Item (Join-Path $FfmpegDir "bin\ffprobe.exe") $stage
Copy-Item (Join-Path $root "LICENSE") (Join-Path $stage "LICENSE.txt")
Copy-Item (Join-Path $FfmpegDir "LICENSE.txt") (Join-Path $stage "FFMPEG-LICENSE.txt")
@"
This software uses libraries from the FFmpeg project (https://ffmpeg.org)
under the LGPL version 2.1 or later (FFMPEG-LICENSE.txt). They are the LGPL
shared build from https://github.com/BtbN/FFmpeg-Builds, dynamically linked
and replaceable.
"@ | Set-Content (Join-Path $stage "FFMPEG-NOTICE.txt")
$zip = Join-Path $OutDir "$name.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip
Write-Output "wrote $zip"

# --- Installer --------------------------------------------------------------
$nsis = Get-ChildItem (Join-Path $release "bundle\nsis\*-setup.exe") -ErrorAction SilentlyContinue |
  Select-Object -First 1
if ($nsis) {
  $dest = Join-Path $OutDir "Jibaru-Editor-$version-windows-x64-setup.exe"
  Copy-Item $nsis.FullName $dest
  Write-Output "wrote $dest"
} else {
  Write-Output "note: no NSIS installer under $release\bundle\nsis; skipped"
}
