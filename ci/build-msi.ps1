#requires -Version 5.1
<#
Build the Unterm MSI on Windows.

Prerequisites:
  - cargo build --release -p unterm -p unterm-cli -p unterm-mux -p strip-ansi-escapes
  - WiX 4 binary at .\.tools\wix.exe (or pass -WixPath)

Usage:
  pwsh -File ci\build-msi.ps1
  pwsh -File ci\build-msi.ps1 -Version 0.2.1 -OutDir dist
#>
[CmdletBinding()]
param(
  [string]$Version,
  [string]$OutDir = "dist",
  [string]$TargetDir = "target\release",
  [string]$WixPath  = ".\.tools\wix.exe"
)
$ErrorActionPreference = "Stop"

# Resolve version: prefer arg, else read installer/Unterm.wxs
if (-not $Version) {
  $wxs = Get-Content installer\Unterm.wxs -Raw
  if ($wxs -match 'Version="([0-9.]+)"') { $Version = $Matches[1] } else { throw "version not found" }
}

$stage = Join-Path $OutDir ("unterm-stage-" + $Version)
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Path $stage | Out-Null

$payload = @(
  "$TargetDir\unterm.exe",
  "$TargetDir\unterm-cli.exe",
  "$TargetDir\unterm-mux.exe",
  "$TargetDir\strip-ansi-escapes.exe",
  "assets\windows\conhost\conpty.dll",
  "assets\windows\conhost\OpenConsole.exe",
  "assets\windows\angle\libEGL.dll",
  "assets\windows\angle\libGLESv2.dll"
)
foreach ($f in $payload) {
  if (-not (Test-Path $f)) { throw "missing: $f" }
  Copy-Item $f $stage
}
$mesa = Join-Path $stage "mesa"
New-Item -ItemType Directory -Path $mesa | Out-Null
Copy-Item "assets\windows\mesa\opengl32.dll" $mesa

if (-not (Test-Path $WixPath)) {
  throw "WiX not found at $WixPath. Download wix.exe from https://github.com/wixtoolset/wix and place it there."
}

$msiName = "Unterm-$Version-x64.msi"
$msiPath = Join-Path $OutDir $msiName

& $WixPath build installer\Unterm.wxs `
  -d "SourceDir=$stage" `
  -arch x64 `
  -o $msiPath

Write-Host "MSI: $msiPath"
