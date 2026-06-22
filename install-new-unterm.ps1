# Installs the freshly-built unterm.exe over the Program Files copy.
# Self-elevates (you'll get ONE UAC prompt — click Yes).
$ErrorActionPreference = 'Stop'
$src = 'D:\code\unterm\target\release\unterm.exe'
$dst = 'C:\Program Files\Unterm\unterm.exe'
$bak = 'C:\Program Files\Unterm\unterm.exe.bak-0.50.2'

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltinRole]::Administrator)
if (-not $isAdmin) {
  Write-Host "Requesting administrator rights (accept the UAC prompt)..."
  Start-Process -Verb RunAs -FilePath powershell -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
  return
}

if (-not (Test-Path $src)) { Write-Host "ERROR: build not found at $src"; Start-Sleep 3; return }

# Rename the (possibly running) old binary out of the way — Windows allows
# renaming an in-use exe — then copy the new one into place. Running instances
# keep using the renamed file until restarted, so this never needs to kill
# anything (safe for the window this session may be running in).
if (Test-Path $bak) { Remove-Item $bak -Force -ErrorAction SilentlyContinue }
if (Test-Path $dst) { Rename-Item $dst $bak -Force }
Copy-Item $src $dst -Force

$new = Get-Item $dst
Write-Host ("Installed -> {0}  ({1} bytes, {2})" -f $dst, $new.Length, $new.LastWriteTime)
Write-Host "Done. Close any open Unterm window and reopen it from the Start menu."
Start-Sleep 4
