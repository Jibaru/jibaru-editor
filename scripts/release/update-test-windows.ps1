# End-to-end update test on Windows: installs an older build silently, starts
# it with the automation hook (JB_AUTOMATION_DIR, see automation_loop in
# app/src-tauri/src/main.rs), asks it to check for updates and install the one
# it finds, then waits until the installed version is the expected one and
# the relaunched app is running.
#
#   scripts/release/update-test-windows.ps1 -Setup <older-setup.exe> -Expect <version>
#
# The update comes from the live endpoint in tauri.conf.json
# (releases/latest/download/latest.json), so run this after publishing.
param(
  [Parameter(Mandatory)][string]$Setup,
  [Parameter(Mandatory)][string]$Expect
)
$ErrorActionPreference = "Stop"
$Expect = $Expect.TrimStart("v")

function Get-Installed {
  $roots = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall", "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall"
  foreach ($k in Get-ChildItem $roots -ErrorAction SilentlyContinue) {
    $p = Get-ItemProperty $k.PSPath
    if ($p.DisplayName -eq "Jibaru Editor") { return $p }
  }
}

function Get-AppExe([string]$dir) {
  Get-ChildItem $dir -Filter *.exe | Where-Object { $_.Name -notmatch "^uninstall" } | Select-Object -First 1
}

Write-Output "installing $Setup"
Start-Process $Setup -ArgumentList "/S" -Wait
$app = Get-Installed
if (-not $app) { throw "the older build did not install" }
$dir = if ($app.InstallLocation) { $app.InstallLocation.Trim('"') } else { Join-Path $env:LOCALAPPDATA "Jibaru Editor" }
$exe = Get-AppExe $dir
$old = $app.DisplayVersion
Write-Output "installed $old at $dir ($($exe.Name))"
if ($old -eq $Expect) { throw "the older build already is $Expect" }

$auto = Join-Path $env:RUNNER_TEMP "jb-automation"
New-Item -ItemType Directory -Force $auto | Out-Null
$log = Join-Path $env:RUNNER_TEMP "update-old-app.log"
$env:JB_AUTOMATION_DIR = $auto
$p = Start-Process $exe.FullName -PassThru -RedirectStandardError $log -RedirectStandardOutput "$log.out"
Start-Sleep 20
if ($p.HasExited) { Get-Content $log -ErrorAction SilentlyContinue; throw ("the older app exited early (0x{0:X8})" -f $p.ExitCode) }

# Retries cover the automatic startup check still running.
$js = @'
let info = null
for (let i = 0; i < 6 && !info; i++) {
  info = await __jb.updates.checkForUpdates(true)
  if (!info) await new Promise(r => setTimeout(r, 5000))
}
__jb.log('update-check ' + JSON.stringify(info) + ' phase ' + JSON.stringify(__jb.updates.updatePhase()))
if (info) {
  await __jb.updates.updateNow({ prompt: false })
  __jb.log('update-result ' + JSON.stringify(__jb.updates.updatePhase()))
}
'@
# Written under another name first: the hook picks up *.js files as they appear.
Set-Content -Path (Join-Path $auto "update.tmp") -Value $js
Rename-Item (Join-Path $auto "update.tmp") "update.js"

$deadline = (Get-Date).AddMinutes(6)
$now = $null
while ((Get-Date) -lt $deadline) {
  Start-Sleep 5
  $now = (Get-Installed).DisplayVersion
  if ($now -eq $Expect) { break }
}
Write-Output "--- older app log"
Get-Content $log -ErrorAction SilentlyContinue
if ($now -ne $Expect) { throw "the installed version is $now after waiting, expected $Expect" }
Write-Output "registry: Jibaru Editor $now"

$exe = Get-AppExe $dir
$fileVersion = (Get-Item $exe.FullName).VersionInfo.ProductVersion
Write-Output "executable: $($exe.Name) $fileVersion"
if (-not $fileVersion.StartsWith($Expect)) { throw "the executable is version $fileVersion, expected $Expect" }
if (-not (Get-ChildItem $dir -Filter "avcodec-*.dll")) { throw "the FFmpeg DLLs are missing after the update" }

# The installer relaunches the app (/R).
$running = $null
for ($i = 0; $i -lt 24 -and -not $running; $i++) {
  Start-Sleep 5
  $running = Get-Process -Name $exe.BaseName -ErrorAction SilentlyContinue
}
if (-not $running) { throw "the updated app was not relaunched" }
Write-Output "relaunched: pid $($running.Id -join ', ')"
$running | Stop-Process -Force
Write-Output "updated $old -> $Expect"
