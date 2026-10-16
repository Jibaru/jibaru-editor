# Prepares the Tauri bundle on Windows: copies the FFmpeg DLLs into
# app/src-tauri/ffmpeg-dlls (git-ignored) and writes
# app/src-tauri/tauri.bundle.conf.json, which lists them as resources so the
# NSIS installer puts them next to the executable. Pass that file to
# `pnpm tauri build --config src-tauri/tauri.bundle.conf.json`.
#
# When WINDOWS_CERTIFICATE (base64 .pfx) and WINDOWS_CERTIFICATE_PASSWORD are
# set, the config also signs the executable, installer and uninstaller.
#
#   scripts/package/windows-prepare.ps1 -FfmpegDir <dir>
param([Parameter(Mandatory)][string]$FfmpegDir)
$ErrorActionPreference = "Stop"

$tauri = Join-Path $PSScriptRoot "..\..\app\src-tauri" | Resolve-Path
$dlls = Join-Path $tauri "ffmpeg-dlls"
if (Test-Path $dlls) { Remove-Item -Recurse -Force $dlls }
New-Item -ItemType Directory $dlls | Out-Null
Copy-Item (Join-Path $FfmpegDir "bin\*.dll") $dlls
Copy-Item (Join-Path $FfmpegDir "LICENSE.txt") (Join-Path $dlls "FFMPEG-LICENSE.txt")

$resources = [ordered]@{}
foreach ($f in Get-ChildItem $dlls) { $resources["ffmpeg-dlls/$($f.Name)"] = $f.Name }
$resources["../../LICENSE"] = "LICENSE.txt"

$windows = [ordered]@{}
if ($env:WINDOWS_CERTIFICATE -and $env:WINDOWS_CERTIFICATE_PASSWORD) {
  $pfx = Join-Path $env:RUNNER_TEMP "codesign.pfx"
  [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))
  $windows["signCommand"] = "signtool sign /f `"$pfx`" /p `"$env:WINDOWS_CERTIFICATE_PASSWORD`" /fd sha256 /tr http://timestamp.digicert.com /td sha256 %1"
  Write-Output "code signing enabled"
}

$conf = [ordered]@{ bundle = [ordered]@{ resources = $resources; windows = $windows } }
$conf | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $tauri "tauri.bundle.conf.json")
Get-Content (Join-Path $tauri "tauri.bundle.conf.json") |
  Where-Object { $_ -notmatch "signCommand" }
