<#
.SYNOPSIS
  Build the Unzoo One installer: one bundle that installs Unterm and Unzoo
  Browser.

.DESCRIPTION
  Chains two installers that both remain independently shippable:

    - Unterm.msi          built by ci\build-msi.ps1 (this script can run it)
    - UnzooSetup-*.exe    Unzoo Browser's own installer, from its releases

  Nothing about either product changes. The MSI keeps its own UpgradeCode and
  its own update path; the bundle has a separate one. Installing through the
  bundle and installing the two pieces by hand end up at the same place.

.PARAMETER UnzooSetup
  Path to UnzooSetup-<version>.exe. Download it from the unzoo releases page
  (gh release download -R unzooai/unzoo -p "UnzooSetup-*.exe").

.PARAMETER UntermMsi
  An already-built MSI. Omit to build one now via ci\build-msi.ps1.

.EXAMPLE
  pwsh -File ci\build-bundle.ps1 -UnzooSetup .\dist\UnzooSetup-2.5.32.exe
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$UnzooSetup,
  [string]$UntermMsi,
  [string]$Version,
  [string]$OutDir = "dist",
  [string]$TargetDir = "target\release",
  [string]$WixPath = ".\.tools\wix.exe",
  [string]$ConsoleDir = "..\unzoo-one\dist\client",
  [ValidateSet("x64", "arm64")]
  [string]$Arch = $(if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" })
)
$ErrorActionPreference = "Stop"

if (-not $Version) {
  $cargo = Get-Content "Cargo.toml" -Raw
  if ($cargo -match '(?m)^version\s*=\s*"([^"]+)"') { $Version = $Matches[1] }
  else { throw "Could not read a version out of Cargo.toml; pass -Version." }
}

if (-not (Test-Path $UnzooSetup)) {
  throw "No Unzoo installer at $UnzooSetup. Fetch one with: gh release download -R unzooai/unzoo -p 'UnzooSetup-*.exe'"
}
$UnzooSetup = (Resolve-Path $UnzooSetup).Path

# The bundle skips the browser when the machine already has this version or
# newer, so the version has to come off the file we are about to carry.
if ((Split-Path $UnzooSetup -Leaf) -match 'UnzooSetup-([0-9]+\.[0-9]+\.[0-9]+)') {
  $unzooVersion = $Matches[1]
} else {
  throw "Could not read a version out of $(Split-Path $UnzooSetup -Leaf); expected UnzooSetup-<x.y.z>.exe"
}

if (-not $UntermMsi) {
  Write-Host "No -UntermMsi given; building one."
  & pwsh -File "ci\build-msi.ps1" -Version $Version -OutDir $OutDir -TargetDir $TargetDir -WixPath $WixPath -ConsoleDir $ConsoleDir -Arch $Arch
  if ($LASTEXITCODE -ne 0) { throw "MSI build failed with exit code $LASTEXITCODE" }
  $UntermMsi = Join-Path $OutDir "Unterm-$Version-$Arch.msi"
}
if (-not (Test-Path $UntermMsi)) { throw "No MSI at $UntermMsi" }
$UntermMsi = (Resolve-Path $UntermMsi).Path

if (-not (Test-Path $WixPath)) {
  throw "WiX not found at $WixPath. See ci\build-msi.ps1 for the install steps."
}
# hyperlinkLicense theme + RegistrySearch: the standard bootstrapper and util
# extensions. Both adds are idempotent.
& $WixPath extension add -g WixToolset.BootstrapperApplications.wixext/6.0.1 | Out-Null
& $WixPath extension add -g WixToolset.Util.wixext/6.0.1 | Out-Null

if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }
$bundleName = "UnzooOneSetup-$Version-$Arch.exe"
$bundlePath = Join-Path $OutDir $bundleName

& $WixPath build installer\UnzooOne.wxs `
  -ext WixToolset.BootstrapperApplications.wixext `
  -ext WixToolset.Util.wixext `
  -d "SourceDir=$TargetDir" `
  -d "UntermMsi=$UntermMsi" `
  -d "UnzooSetup=$UnzooSetup" `
  -d "UnzooVersion=$unzooVersion" `
  -d "BundleVersion=$Version" `
  -arch $Arch `
  -o $bundlePath
if ($LASTEXITCODE -ne 0) { throw "Bundle build failed with exit code $LASTEXITCODE" }

$mb = [int]((Get-Item $bundlePath).Length / 1MB)
Write-Host "Bundle: $bundlePath ($mb MB)"
Write-Host "  Unterm $Version + Unzoo Browser $unzooVersion"
