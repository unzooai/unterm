#requires -Version 5.1
<#
Build the Unterm MSI on Windows.

Prerequisites:
  - cargo build --release -p unterm-app -p unterm-cli -p unterm-core
  - WiX 6 .NET tool installed at .\.tools\wix (or pass -WixPath).
    Install with:
      dotnet tool install --tool-path .\.tools wix --version 6.0.1
      .\.tools\wix.exe extension add -g WixToolset.UI.wixext/6.0.1
      .\.tools\wix.exe extension add -g WixToolset.Util.wixext/6.0.1
    See .github/workflows/release-windows.yml for the canonical recipe.

Usage:
  pwsh -File ci\build-msi.ps1
  pwsh -File ci\build-msi.ps1 -Version 0.2.1 -OutDir dist
#>
[CmdletBinding()]
param(
  [string]$Version,
  [string]$OutDir = "dist",
  [string]$TargetDir = "target\release",
  [string]$WixPath  = ".\.tools\wix.exe",
  # CPU arch of the build: "x64" or "arm64". Drives the MSI filename and WiX's
  # -arch (component bitness + ProgramFiles64 resolution). Defaults from the
  # host so a native arm runner produces an arm64 MSI without extra args.
  [ValidateSet("x64", "arm64")]
  [string]$Arch = $(if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" })
)
$ErrorActionPreference = "Stop"

# Resolve version: prefer arg, else read installer/Unterm.wxs
if (-not $Version) {
  $wxs = Get-Content installer\Unterm.wxs -Raw
  # Do not take the XML declaration's `version="1.0"` for the product
  # version. Match the Package element explicitly.
  if ($wxs -match '(?s)<Package\b.*?\bVersion="([0-9.]+)"') { $Version = $Matches[1] } else { throw "package version not found" }
}

$stage = Join-Path $OutDir ("unterm-stage-" + $Version)
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Path $stage | Out-Null

# Native helper DLLs must match the build arch: x64 under assets\windows,
# arm64 under assets\windows\arm64.
$helpers = if ($Arch -eq "arm64") { "assets\windows\arm64" } else { "assets\windows" }
$payload = @(
  "$TargetDir\unterm.exe",
  "$TargetDir\unterm-cli.exe",
  "$TargetDir\unterm-core.exe",
  "$helpers\conhost\conpty.dll",
  "$helpers\conhost\OpenConsole.exe",
  "$helpers\angle\libEGL.dll",
  "$helpers\angle\libGLESv2.dll"
)
foreach ($f in $payload) {
  if (-not (Test-Path $f)) { throw "missing: $f" }
  Copy-Item $f $stage
}
# Mesa software-GL fallback is x64-only (no arm64 prebuilt); the WiX Mesa
# component is gated on $(sys.BUILDARCH), so skip staging it for arm64.
if ($Arch -ne "arm64") {
  $mesa = Join-Path $stage "mesa"
  New-Item -ItemType Directory -Path $mesa | Out-Null
  Copy-Item "assets\windows\mesa\opengl32.dll" $mesa
}

# Product-default config, in the declarative format the terminal reads now.
# The WiX DefaultUntermConf component references defaults\unterm.conf; it
# lives in defaults\ rather than beside the exe so an installed default can
# never outrank the user's own.
$defaults = Join-Path $stage "defaults"
New-Item -ItemType Directory -Path $defaults | Out-Null
Copy-Item "assets\unterm.conf" $defaults

# The new renderer draws product chrome with the bundled symbols face and
# uses the bundled emoji face as its last fallback. They are runtime assets,
# not development-only test data.
$fonts = Join-Path $stage "assets\fonts"
New-Item -ItemType Directory -Path $fonts -Force | Out-Null
Copy-Item "assets\fonts\SymbolsNerdFontMono-Regular.ttf" $fonts
Copy-Item "assets\fonts\NotoColorEmoji.ttf" $fonts
# The default terminal face, opened by file name at startup.
Copy-Item "assets\fonts\JetBrainsMono-Regular.ttf" $fonts

if (-not (Test-Path $WixPath)) {
  throw "WiX not found at $WixPath. Download wix.exe from https://github.com/wixtoolset/wix and place it there."
}

# Make sure the WixUI extension is registered for this wix.exe — needed to
# resolve the <ui:WixUI Id="WixUI_Minimal"/> reference at build time.
# `wix extension add` is idempotent; it noops if already added.
& $WixPath extension add -g WixToolset.UI.wixext/6.0.1 | Out-Null
& $WixPath extension add -g WixToolset.Util.wixext/6.0.1 | Out-Null
if ($LASTEXITCODE -ne 0) {
  throw "WiX UI extension registration failed with exit code $LASTEXITCODE"
}

$msiName = "Unterm-$Version-$Arch.msi"
$msiPath = Join-Path $OutDir $msiName

& $WixPath build installer\Unterm.wxs `
  -ext WixToolset.UI.wixext `
  -ext WixToolset.Util.wixext `
  -d "SourceDir=$stage" `
  -arch $Arch `
  -o $msiPath
if ($LASTEXITCODE -ne 0) {
  throw "WiX MSI build failed with exit code $LASTEXITCODE"
}

Write-Host "MSI: $msiPath"
