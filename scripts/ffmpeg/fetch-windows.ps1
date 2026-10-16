# Downloads an LGPL shared FFmpeg build for Windows x64 (DLLs, import
# libraries and headers) from BtbN/FFmpeg-Builds and, when running in GitHub
# Actions, exports FFMPEG_DIR, LIBCLANG_PATH and PATH for later steps.
#
#   scripts/ffmpeg/fetch-windows.ps1 [-Dest <dir>] [-Version 9.0]
#
# Prints the FFmpeg directory. Only "lgpl" builds are accepted.
param(
  [string]$Dest = $(if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }),
  [string]$Version = "9.0"
)
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$name = "ffmpeg-n$Version-latest-win64-lgpl-shared-$Version"
$dir = Join-Path $Dest $name
if (-not (Test-Path (Join-Path $dir "bin"))) {
  $url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/$name.zip"
  $zip = Join-Path $Dest "$name.zip"
  Invoke-WebRequest $url -OutFile $zip
  Expand-Archive $zip -DestinationPath $Dest -Force
  Remove-Item $zip
}

# Refuse anything configured with GPL or nonfree components.
$conf = & (Join-Path $dir "bin\ffprobe.exe") -hide_banner -buildconf 2>&1 | Out-String
if ($conf -match "--enable-(gpl|nonfree)") { throw "FFmpeg build in $dir is not LGPL" }

if ($env:GITHUB_ENV) {
  "FFMPEG_DIR=$dir" | Out-File -FilePath $env:GITHUB_ENV -Append
  "LIBCLANG_PATH=C:\Program Files\LLVM\bin" | Out-File -FilePath $env:GITHUB_ENV -Append
  "$dir\bin" | Out-File -FilePath $env:GITHUB_PATH -Append
}
Write-Output $dir
