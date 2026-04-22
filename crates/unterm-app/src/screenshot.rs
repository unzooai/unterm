//! 截图功能模块
//!
//! 提供全屏截图和剪贴板复制功能（Windows 实现）。

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// 截取全屏，返回 base64 编码的 PNG 图片数据
#[cfg(windows)]
pub fn capture_screen() -> Result<String, String> {
    use image::{ImageBuffer, Rgba};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SelectObject, SRCCOPY, GetDC, ReleaseDC,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetDesktopWindow, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    use windows::Win32::UI::HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE};

    unsafe {
        // 确保 DPI 感知，获取真实分辨率
        let _ = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);

        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        tracing::info!("截图分辨率: {}x{}", width, height);

        if width == 0 || height == 0 {
            return Err(format!("获取屏幕尺寸失败: {}x{}", width, height));
        }

        let hwnd = GetDesktopWindow();
        let hdc_screen = GetDC(hwnd);

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
        let old_obj = SelectObject(hdc_mem, hbmp);

        let blt_result = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);
        if blt_result.is_err() {
            SelectObject(hdc_mem, old_obj);
            let _ = DeleteObject(hbmp);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_screen);
            return Err("BitBlt 失败".into());
        }

        // 读取像素数据
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        let buf_size = (width as usize) * (height as usize) * 4;
        let mut pixels: Vec<u8> = vec![0u8; buf_size];
        let scan_lines = GetDIBits(
            hdc_mem,
            hbmp,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // 清理 GDI 资源
        SelectObject(hdc_mem, old_obj);
        let _ = DeleteObject(hbmp);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_screen);

        if scan_lines == 0 {
            return Err("GetDIBits 失败，没有读取到扫描线".into());
        }

        tracing::info!("截图成功，读取了 {} 行", scan_lines);

        // BGRA -> RGBA（就地转换，避免额外分配）
        for i in (0..buf_size).step_by(4) {
            pixels.swap(i, i + 2); // swap B and R
        }

        let img: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(width as u32, height as u32, pixels)
                .ok_or("创建图像缓冲失败")?;

        // 编码为 PNG
        let mut png_buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_buf);
        img.write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("PNG 编码失败: {}", e))?;

        tracing::info!("PNG 编码完成，大小 {} bytes", png_buf.len());

        Ok(BASE64.encode(&png_buf))
    }
}

#[cfg(not(windows))]
pub fn capture_screen() -> Result<String, String> {
    Err("截图功能仅支持 Windows".into())
}

/// 将 base64 编码的 PNG 图片复制到系统剪贴板
///
/// 通过 .NET System.Windows.Forms.Clipboard API 写入，
/// 和 Windows 系统截图工具一样的方式，兼容所有应用（微信、浏览器、Office 等）。
#[cfg(windows)]
pub fn copy_image_to_clipboard(base64_data: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let img_bytes = BASE64
        .decode(base64_data)
        .map_err(|e| format!("base64 解码失败: {}", e))?;

    // 写到临时文件
    let temp_path = std::env::temp_dir().join("unterm_clipboard.png");
    std::fs::write(&temp_path, &img_bytes)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;

    let ps_script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         Add-Type -AssemblyName System.Drawing; \
         $img = [System.Drawing.Image]::FromFile('{}'); \
         [System.Windows.Forms.Clipboard]::SetImage($img); \
         $img.Dispose()",
        temp_path.display()
    );

    let output = std::process::Command::new("powershell.exe")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("启动 PowerShell 失败: {}", e))?;

    // 清理临时文件
    let _ = std::fs::remove_file(&temp_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("剪贴板写入失败: {}", stderr.trim()));
    }

    tracing::info!("图片已复制到剪贴板 (via PowerShell, {} bytes)", img_bytes.len());
    Ok(())
}

#[cfg(not(windows))]
pub fn copy_image_to_clipboard(_base64_data: &str) -> Result<(), String> {
    Err("剪贴板功能仅支持 Windows".into())
}

/// 将 base64 编码的 PNG 保存到文件，返回文件路径
pub fn save_screenshot_to_file(base64_data: &str) -> Result<String, String> {
    let img_bytes = BASE64
        .decode(base64_data)
        .map_err(|e| format!("base64 解码失败: {}", e))?;

    // 截图目录: ~/.unterm/screenshots/
    let dir = dirs::home_dir()
        .ok_or("无法获取用户目录")?
        .join(".unterm")
        .join("screenshots");

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建截图目录失败: {}", e))?;

    // 文件名: screenshot_20260415_123456.png
    let now = chrono::Local::now();
    let filename = format!("screenshot_{}.png", now.format("%Y%m%d_%H%M%S"));
    let path = dir.join(&filename);

    std::fs::write(&path, &img_bytes)
        .map_err(|e| format!("保存截图失败: {}", e))?;

    let path_str = path.to_string_lossy().to_string();
    tracing::info!("截图已保存: {}", path_str);

    // 自动清理：只保留最近 50 张截图
    cleanup_old_screenshots(&dir, 50);

    Ok(path_str)
}

/// 清理旧截图，只保留最近 max_keep 张
fn cleanup_old_screenshots(dir: &std::path::Path, max_keep: usize) {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "png")
                .unwrap_or(false)
        })
        .collect();

    if files.len() <= max_keep {
        return;
    }

    // 按修改时间排序（旧的在前）
    files.sort_by_key(|f| {
        f.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    let to_delete = files.len() - max_keep;
    for entry in files.into_iter().take(to_delete) {
        let _ = std::fs::remove_file(entry.path());
    }
}

/// 使用 Win32 API 从剪贴板读取 DIB 图片，转为 PNG 字节
#[cfg(windows)]
fn read_clipboard_image_bytes() -> Result<Option<Vec<u8>>, String> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::*;
    use windows::Win32::System::Memory::*;
    use windows::Win32::System::Ole::CF_DIB;

    unsafe {
        // 快速检查：剪贴板是否有图片？
        if IsClipboardFormatAvailable(CF_DIB.0 as u32).is_err() {
            return Ok(None);
        }

        if OpenClipboard(None).is_err() {
            return Ok(None);
        }

        let result = (|| -> Result<Option<Vec<u8>>, String> {
            let handle = GetClipboardData(CF_DIB.0 as u32);
            if handle.is_err() {
                return Ok(None);
            }
            let hmem = HGLOBAL(handle.unwrap().0);

            let ptr = GlobalLock(hmem);
            if ptr.is_null() {
                return Ok(None);
            }

            let size = GlobalSize(hmem);
            if size == 0 {
                let _ = GlobalUnlock(hmem);
                return Ok(None);
            }

            let dib_data = std::slice::from_raw_parts(ptr as *const u8, size);

            // DIB: BITMAPINFOHEADER (40 bytes) + pixels
            if dib_data.len() < 40 {
                let _ = GlobalUnlock(hmem);
                return Ok(None);
            }

            let width = i32::from_le_bytes([dib_data[4], dib_data[5], dib_data[6], dib_data[7]]);
            let height = i32::from_le_bytes([dib_data[8], dib_data[9], dib_data[10], dib_data[11]]);
            let bit_count = u16::from_le_bytes([dib_data[14], dib_data[15]]);
            let header_size = u32::from_le_bytes([dib_data[0], dib_data[1], dib_data[2], dib_data[3]]) as usize;

            if width <= 0 || height == 0 || (bit_count != 24 && bit_count != 32) {
                let _ = GlobalUnlock(hmem);
                return Ok(None);
            }

            let abs_height = height.unsigned_abs() as usize;
            let w = width as usize;
            let bpp = (bit_count / 8) as usize;
            let row_stride = ((w * bpp + 3) / 4) * 4;
            let pixel_data = &dib_data[header_size..];

            let mut rgba = vec![0u8; w * abs_height * 4];
            for y in 0..abs_height {
                let src_y = if height > 0 { abs_height - 1 - y } else { y };
                if src_y * row_stride >= pixel_data.len() { continue; }
                let src_row = &pixel_data[src_y * row_stride..];
                for x in 0..w {
                    let si = x * bpp;
                    let di = (y * w + x) * 4;
                    if si + 2 < src_row.len() {
                        rgba[di] = src_row[si + 2];     // R (DIB = BGR)
                        rgba[di + 1] = src_row[si + 1]; // G
                        rgba[di + 2] = src_row[si];     // B
                        rgba[di + 3] = if bpp == 4 && si + 3 < src_row.len() {
                            src_row[si + 3]
                        } else {
                            255
                        };
                    }
                }
            }

            let _ = GlobalUnlock(hmem);

            let img = image::RgbaImage::from_raw(w as u32, abs_height as u32, rgba)
                .ok_or("创建图片失败")?;
            let mut png_buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut png_buf, image::ImageFormat::Png)
                .map_err(|e| format!("PNG 编码失败: {}", e))?;

            Ok(Some(png_buf.into_inner()))
        })();

        let _ = CloseClipboard();
        result
    }
}

/// 从系统剪贴板读取图片。如果有图片，保存到文件并返回路径；没有图片返回 None。
#[cfg(windows)]
pub fn read_image_from_clipboard() -> Result<Option<String>, String> {
    let png_bytes = read_clipboard_image_bytes()?;
    match png_bytes {
        Some(bytes) => {
            let b64 = BASE64.encode(&bytes);
            let path = save_screenshot_to_file(&b64)?;
            Ok(Some(path))
        }
        None => Ok(None),
    }
}

#[cfg(not(windows))]
pub fn read_image_from_clipboard() -> Result<Option<String>, String> {
    Ok(None)
}

/// 从剪贴板读取图片，返回 base64 PNG 数据（不保存到文件）
#[cfg(windows)]
pub fn read_image_as_base64() -> Result<Option<String>, String> {
    let png_bytes = read_clipboard_image_bytes()?;
    Ok(png_bytes.map(|bytes| BASE64.encode(&bytes)))
}

#[cfg(not(windows))]
pub fn read_image_as_base64() -> Result<Option<String>, String> {
    Ok(None)
}
