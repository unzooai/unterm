# Unterm one-shot installer for Windows.
#
# Usage:
#   irm https://unterm.app/install.ps1 | iex
#
# What it does:
#   - Resolves the latest release tag via the GitHub API.
#   - Downloads Unterm-<version>-x64.msi to %TEMP%.
#   - Runs `msiexec /i` with a small UI so the user can see progress
#     and click Finish — unlike the silent default we shipped in 0.5.1.
#   - Verifies install by looking for unterm.exe under Program Files.
#
# Re-running upgrades in place. Set $env:UNTERM_VERSION = "v0.5.2"
# before piping to pin a specific tag.

$ErrorActionPreference = 'Stop'

$repo    = 'unzooai/unterm'
$version = $env:UNTERM_VERSION

function Say  ($m) { Write-Host "» $m" -ForegroundColor Blue }
function Ok   ($m) { Write-Host "✓ $m" -ForegroundColor Green }
function Warn ($m) { Write-Host "! $m" -ForegroundColor Yellow }
function Die  ($m) { Write-Host "✗ $m" -ForegroundColor Red; exit 1 }

# --- Resolve target tag ----------------------------------------------------
if (-not $version) {
  Say 'looking up latest Unterm release...'
  $api = "https://api.github.com/repos/$repo/releases/latest"
  try {
    $version = (Invoke-RestMethod -Uri $api -Headers @{ 'User-Agent'='unterm-installer' }).tag_name
  } catch {
    Die "couldn't resolve latest tag from $api ($_)"
  }
}
if (-not $version) { Die 'tag_name was empty in API response' }
Ok "Unterm $version"

# --- Architecture check ----------------------------------------------------
# Windows ships x64 only for now. ARM64 Windows users can run x64 via
# Microsoft's emulation layer; native ARM64 builds are a roadmap item.
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne 'AMD64') {
  Warn "Architecture is $arch; the x64 installer will run via emulation."
}

# --- Download MSI ----------------------------------------------------------
# Strip leading 'v' for the Debian-style filename (Unterm-0.5.2-x64.msi),
# but keep it on the GitHub-style download URL component.
$verNoV = $version -replace '^v',''
$asset  = "Unterm-$verNoV-x64.msi"
$url    = "https://github.com/$repo/releases/download/$version/$asset"
$dest   = Join-Path $env:TEMP $asset

Say "downloading $asset"
try {
  # `Invoke-WebRequest -OutFile` is fastest on PS 5.1; PS 7 has parallel
  # but we want broad compat.
  $oldProgress = $ProgressPreference
  $ProgressPreference = 'Continue'
  Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
  $ProgressPreference = $oldProgress
} catch {
  Die "download failed: $_"
}
$size = (Get-Item $dest).Length
Ok ("downloaded {0:N1} MB → {1}" -f ($size/1MB), $dest)

# --- Install ---------------------------------------------------------------
# /qb! — basic UI with no Cancel button (so a user can't abort halfway and
# leave a half-installed state). /norestart — never auto-reboot.
# /l*v ...log — verbose log so when something fails we can read it.
# msiexec returns 0 on success, 1602 if user cancelled, 1603 on generic
# failure. Translate to friendly text.
$log = Join-Path $env:TEMP 'unterm-install.log'
Say 'launching installer (UAC may prompt for elevation)...'
$proc = Start-Process -FilePath 'msiexec.exe' `
  -ArgumentList @('/i', "`"$dest`"", '/qb!', '/norestart', '/l*v', "`"$log`"") `
  -Wait -PassThru
switch ($proc.ExitCode) {
  0     { Ok 'installer reported success.' }
  1602  { Die 'installer was cancelled by the user (UAC denied or you closed it).' }
  1603  { Die "installer failed with code 1603 (fatal error during installation). Log: $log" }
  3010  { Warn 'install succeeded but a reboot is recommended.' }
  default { Die ("installer exited with code {0}. Log: {1}" -f $proc.ExitCode, $log) }
}

# --- Verify ----------------------------------------------------------------
$expected = "$env:ProgramFiles\Unterm\unterm.exe"
if (Test-Path $expected) {
  Ok "installed to $expected"
  Say 'launch with: Start Menu → Unterm'
} else {
  Warn "couldn't find $expected — install may have gone elsewhere. Check log: $log"
}
