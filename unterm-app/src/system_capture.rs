//! Native interactive screenshot flow carried over from the 0.57.4 UI.
//!
//! On Windows this deliberately uses the system snipping overlay.  Besides
//! feeling native, that route leaves the saved PNG, image data, file-drop
//! data and plain path on the clipboard together, which is the behaviour the
//! previous front end exposed.  macOS goes through `screencapture -i` and
//! Linux probes whichever region-capture tool is installed; both also copy
//! the PNG to the system clipboard for parity with the Windows path.

fn output_dir() -> anyhow::Result<std::path::PathBuf> {
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("screenshots");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(not(windows))]
fn capture_file_name(hide_window: bool) -> String {
    let prefix = if hide_window {
        "region_hidden"
    } else {
        "region_visible"
    };
    format!(
        "{}_{}.png",
        prefix,
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    )
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

/// macOS region screenshot via `screencapture -i`.
///
/// `hide_window=true` hides our app first using `osascript` to ask System
/// Events to hide the frontmost process, runs the interactive picker, then
/// reactivates Unterm. ESC cancels the picker.
#[cfg(target_os = "macos")]
pub fn capture_selected_region(hide_window: bool) -> anyhow::Result<std::path::PathBuf> {
    let output_path = output_dir()?.join(capture_file_name(hide_window));

    if hide_window {
        let _ = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to set visible of process \"unterm\" to false",
            ])
            .status();
        // Brief delay so the window finishes hiding before the picker UI shows.
        std::thread::sleep(std::time::Duration::from_millis(250));
    }

    // -i = interactive selection, -t png = explicit format
    // We do NOT pass -x so the picker chrome and shutter sound stay (matches Win UX).
    let status = std::process::Command::new("/usr/sbin/screencapture")
        .args(["-i", "-t", "png"])
        .arg(&output_path)
        .status();

    if hide_window {
        // Always try to bring our window back, even on cancel/error.
        let _ = std::process::Command::new("osascript")
            .args(["-e", "tell application \"unterm\" to activate"])
            .status();
    }

    let status = status?;
    if !status.success() {
        anyhow::bail!("screencapture exited with {status}");
    }

    if !output_path.exists() {
        anyhow::bail!(
            "Screenshot canceled or file not created: {}",
            output_path.display()
        );
    }

    // Copy to clipboard so the user can paste it elsewhere (parity with Win path).
    let _ = std::process::Command::new("osascript")
        .args([
            "-e",
            &format!(
                "set the clipboard to (read (POSIX file \"{}\") as «class PNGf»)",
                output_path.display()
            ),
        ])
        .status();

    Ok(output_path)
}

/// Linux region screenshot. Probes available tools in order and uses the first
/// one that exists.
///
/// `hide_window=true` is best-effort — most Linux screenshot tools take a
/// short delay flag, but minimizing the window cleanly across X11/Wayland
/// without window-server-specific code is fragile, so we currently skip it
/// and just rely on the tool's own region picker UI.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn capture_selected_region(hide_window: bool) -> anyhow::Result<std::path::PathBuf> {
    let output_path = output_dir()?.join(capture_file_name(hide_window));

    // Try grim+slurp (Wayland), then gnome-screenshot, spectacle, scrot, maim.
    let path_str = output_path.display().to_string();
    let attempts: &[(&str, &[&str])] = &[
        ("grim", &[]), // grim handled specially below because slurp is piped
        ("gnome-screenshot", &["-a", "-f"]),
        ("spectacle", &["-bn", "-r", "-o"]),
        ("scrot", &["-s"]),
        ("maim", &["-s"]),
    ];

    let mut last_err: Option<String> = None;
    for (tool, args) in attempts {
        if !command_exists(tool) {
            continue;
        }

        let status = if *tool == "grim" {
            // grim -g "$(slurp)" <output>
            if !command_exists("slurp") {
                last_err = Some("grim found but slurp is required for region selection".into());
                continue;
            }
            // Run `slurp` to pick a region, capture stdout, pass to grim.
            let slurp = std::process::Command::new("slurp").output();
            let slurp = match slurp {
                Ok(o) if o.status.success() => o,
                Ok(o) => {
                    last_err = Some(format!(
                        "slurp exited with {} (selection cancelled?)",
                        o.status
                    ));
                    continue;
                }
                Err(e) => {
                    last_err = Some(format!("slurp failed: {e}"));
                    continue;
                }
            };
            let geom = String::from_utf8_lossy(&slurp.stdout).trim().to_string();
            std::process::Command::new("grim")
                .args(["-g", &geom])
                .arg(&output_path)
                .status()
        } else {
            let mut cmd = std::process::Command::new(tool);
            cmd.args(*args);
            cmd.arg(&path_str);
            cmd.status()
        };

        match status {
            Ok(s) if s.success() => {
                if output_path.exists() {
                    // Try to copy to clipboard via xclip / wl-copy — best effort.
                    let _ = copy_image_to_clipboard_unix(&output_path);
                    return Ok(output_path);
                } else {
                    last_err = Some(format!("{tool} reported success but no file was created"));
                }
            }
            Ok(s) => {
                last_err = Some(format!("{tool} exited with {s}"));
            }
            Err(e) => {
                last_err = Some(format!("failed to run {tool}: {e}"));
            }
        }
    }

    let _ = hide_window; // currently unused on Linux
    let msg = last_err.unwrap_or_else(|| {
        "No screenshot tool found. Install one of: grim+slurp, gnome-screenshot, spectacle, scrot, or maim".into()
    });
    anyhow::bail!("{}", msg)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn command_exists(name: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return true;
        }
    }
    false
}

#[cfg(all(unix, not(target_os = "macos")))]
fn copy_image_to_clipboard_unix(path: &std::path::Path) -> anyhow::Result<()> {
    use std::io::Write;
    if command_exists("wl-copy") {
        let mut child = std::process::Command::new("wl-copy")
            .args(["--type", "image/png"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        let bytes = std::fs::read(path)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&bytes)?;
        }
        child.wait()?;
        return Ok(());
    }
    if command_exists("xclip") {
        let mut child = std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-t", "image/png", "-i"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        let bytes = std::fs::read(path)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&bytes)?;
        }
        child.wait()?;
        return Ok(());
    }
    Ok(())
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn output_dir_is_under_home_unterm_screenshots() {
        let dir = output_dir().expect("output dir should be creatable");
        assert!(dir.ends_with(".unterm/screenshots"), "got {}", dir.display());
        assert!(dir.is_dir());
    }

    #[test]
    fn capture_file_name_prefix_tracks_hide_window() {
        assert!(capture_file_name(true).starts_with("region_hidden_"));
        assert!(capture_file_name(false).starts_with("region_visible_"));
    }

    #[test]
    fn capture_file_name_is_a_timestamped_png() {
        let name = capture_file_name(false);
        assert!(name.ends_with(".png"));
        // region_visible_YYYYMMDD_HHMMSS_mmm.png
        let stamp = name
            .strip_prefix("region_visible_")
            .and_then(|s| s.strip_suffix(".png"))
            .expect("prefix and suffix present");
        assert_eq!(stamp.len(), "YYYYMMDD_HHMMSS_mmm".len());
        assert!(stamp
            .chars()
            .all(|c| c.is_ascii_digit() || c == '_'));
    }
}
