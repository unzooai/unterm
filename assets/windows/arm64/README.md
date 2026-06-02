# Windows ARM64 helper binaries

Native aarch64 counterparts of the x64 blobs one level up, bundled into the
ARM64 MSI/zip by `ci/deploy.sh` + `ci/build-msi.ps1` when building for
`aarch64-pc-windows-msvc`.

Provenance (all verified ARM64 via PE COFF machine type 0xAA64):

- `conhost/conpty.dll`, `conhost/OpenConsole.exe`
  Microsoft ConPTY NuGet `Microsoft.Windows.Console.ConPTY` v1.24.260512001
  (MIT, same upstream as the x64 conhost binaries — see ../conhost/README.md).
  `runtimes/win-arm64/native/conpty.dll` + `build/native/runtimes/arm64/OpenConsole.exe`.

- `angle/libEGL.dll`, `angle/libGLESv2.dll`
  Chromium's ANGLE, taken from the official Electron arm64 release
  `electron-v42.3.0-win32-arm64.zip` (BSD-3-Clause).

Mesa's software-GL `opengl32.dll` is intentionally NOT shipped for ARM64:
mesa3d has no maintained Windows-arm64 prebuilt, and ANGLE (D3D-backed) is
the primary GL path on ARM. The MSI omits the Mesa component for arm64 via
the `$(sys.BUILDARCH)` guard in installer/Unterm.wxs.
