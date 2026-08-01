//! Native interactive screenshot flow carried over from the 0.57.4 UI.
//!
//! On Windows this deliberately uses the system snipping overlay.  Besides
//! feeling native, that route leaves the saved PNG, image data, file-drop
//! data and plain path on the clipboard together, which is the behaviour the
//! previous front end exposed.

fn output_dir() -> anyhow::Result<std::path::PathBuf> {
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("screenshots");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(windows)]
pub fn capture_selected_region(hide_window: bool) -> anyhow::Result<std::path::PathBuf> {
    use base64::Engine as _;

    let pid = std::process::id();
    let prefix = if hide_window {
        "region_hidden"
    } else {
        "region_visible"
    };
    let output_path = output_dir()?.join(format!(
        "{}_{}.png",
        prefix,
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));
    let path = output_path.display().to_string().replace('\'', "''");
    let hide_script = if hide_window {
        "foreach ($win in $windows) { [UntermStatusCapture]::ShowWindow($win, 0) | Out-Null }\nStart-Sleep -Milliseconds 350"
    } else {
        "[UntermStatusCapture]::SetForegroundWindow($hwnd) | Out-Null\nStart-Sleep -Milliseconds 120"
    };
    let restore_script = if hide_window {
        "foreach ($win in $windows) { [UntermStatusCapture]::ShowWindow($win, 5) | Out-Null }\n  [UntermStatusCapture]::SetForegroundWindow($hwnd) | Out-Null"
    } else {
        "[UntermStatusCapture]::SetForegroundWindow($hwnd) | Out-Null"
    };
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class UntermStatusCapture {{
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
  public static IntPtr[] WindowsForPid(uint pid) {{
    var windows = new List<IntPtr>();
    EnumWindows((hWnd, lParam) => {{
      uint windowPid;
      GetWindowThreadProcessId(hWnd, out windowPid);
      if (windowPid == pid && IsWindowVisible(hWnd)) windows.Add(hWnd);
      return true;
    }}, IntPtr.Zero);
    return windows.ToArray();
  }}
}}
"@
$proc = Get-Process -Id {pid} -ErrorAction Stop
$windows = [UntermStatusCapture]::WindowsForPid([uint32]$proc.Id)
if ($windows.Count -eq 0) {{ throw "No visible window handle" }}
$hwnd = $windows[0]
{hide_script}
try {{
  [System.Windows.Forms.Clipboard]::Clear()
  Start-Process "ms-screenclip:"
  $deadline = [DateTime]::Now.AddSeconds(90)
  $image = $null
  while ([DateTime]::Now -lt $deadline) {{
    Start-Sleep -Milliseconds 250
    if ([System.Windows.Forms.Clipboard]::ContainsImage()) {{
      $image = [System.Windows.Forms.Clipboard]::GetImage()
      break
    }}
  }}
  if ($image -eq $null) {{ throw "Screenshot canceled or timed out" }}
  $image.Save('{path}', [System.Drawing.Imaging.ImageFormat]::Png)
  $clipboardImage = [System.Drawing.Image]::FromFile('{path}')
  $pngBytes = [System.IO.File]::ReadAllBytes('{path}')
  $pngStream = New-Object System.IO.MemoryStream
  $pngStream.Write($pngBytes, 0, $pngBytes.Length)
  $pngStream.Position = 0
  $fileDrop = New-Object System.Collections.Specialized.StringCollection
  [void]$fileDrop.Add('{path}')
  $data = New-Object System.Windows.Forms.DataObject
  $data.SetImage($clipboardImage)
  $data.SetFileDropList($fileDrop)
  $data.SetText('{path}')
  $data.SetData('PNG', $false, $pngStream)
  $data.SetData('image/png', $false, $pngStream)
  try {{
    $set = $false
    for ($i = 0; $i -lt 10 -and -not $set; $i++) {{
      try {{
        [System.Windows.Forms.Clipboard]::SetDataObject($data, $true)
        $set = $true
      }} catch {{ Start-Sleep -Milliseconds 120 }}
    }}
    if (-not $set) {{ throw "Clipboard is busy" }}
  }} finally {{
    $clipboardImage.Dispose()
    $pngStream.Dispose()
  }}
  $image.Dispose()
}} finally {{
  {restore_script}
}}
"#
    );

    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut command = std::process::Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-STA",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &encoded,
    ]);
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000);
    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("PowerShell screenshot returned {status}");
    }
    if !output_path.exists() {
        anyhow::bail!("screenshot file was not created: {}", output_path.display());
    }
    Ok(output_path)
}

#[cfg(not(windows))]
pub fn capture_selected_region(_hide_window: bool) -> anyhow::Result<std::path::PathBuf> {
    anyhow::bail!("native interactive capture is not available on this platform")
}
