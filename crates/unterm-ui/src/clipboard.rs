//! 剪贴板模块：支持文本和图片剪贴板操作

use arboard::Clipboard;
use std::path::PathBuf;

/// 剪贴板内容类型
pub enum ClipboardContent {
    /// 纯文本
    Text(String),
    /// 图片（已保存为文件，返回路径）
    ImagePath(String),
    /// 空
    Empty,
}

/// 剪贴板管理器
pub struct ClipboardManager {
    clipboard: Option<Clipboard>,
    /// 截图保存目录
    screenshot_dir: PathBuf,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let clipboard = Clipboard::new().ok();
        let screenshot_dir = Self::default_screenshot_dir();
        // 确保目录存在
        let _ = std::fs::create_dir_all(&screenshot_dir);
        Self {
            clipboard,
            screenshot_dir,
        }
    }

    fn default_screenshot_dir() -> PathBuf {
        let home = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".into())
        } else {
            std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
        };
        PathBuf::from(home).join(".unterm").join("screenshots")
    }

    /// 读取剪贴板内容 - 自动检测文本还是图片
    pub fn read(&mut self) -> ClipboardContent {
        let cb = match &mut self.clipboard {
            Some(c) => c,
            None => return ClipboardContent::Empty,
        };

        // 先尝试读图片，提取所需数据后释放 borrow
        let img_result = cb.get_image().ok().map(|img_data| {
            let width = img_data.width as u32;
            let height = img_data.height as u32;
            let bytes = img_data.bytes.to_vec();
            (width, height, bytes)
        });

        if let Some((width, height, bytes)) = img_result {
            match self.save_rgba_to_file(width, height, &bytes) {
                Ok(path) => return ClipboardContent::ImagePath(path),
                Err(e) => {
                    tracing::warn!("剪贴板图片保存失败: {}", e);
                }
            }
            // 重新获取 clipboard 引用（save_rgba_to_file 不借用 clipboard）
            let cb = self.clipboard.as_mut().unwrap();
            if let Ok(text) = cb.get_text() {
                if !text.is_empty() {
                    return ClipboardContent::Text(text);
                }
            }
        } else {
            // 回退到文本
            if let Ok(text) = cb.get_text() {
                if !text.is_empty() {
                    return ClipboardContent::Text(text);
                }
            }
        }

        ClipboardContent::Empty
    }

    /// 读取纯文本（忽略图片）
    pub fn read_text(&mut self) -> Option<String> {
        self.clipboard.as_mut()?.get_text().ok()
    }

    /// 写入文本到剪贴板
    pub fn write_text(&mut self, text: &str) {
        if let Some(cb) = &mut self.clipboard {
            let _ = cb.set_text(text);
        }
    }

    /// 将 RGBA 图片数据保存为 PNG 文件，返回文件路径
    fn save_rgba_to_file(&self, width: u32, height: u32, bytes: &[u8]) -> anyhow::Result<String> {
        // arboard ImageData 是 RGBA 格式
        let img = image::RgbaImage::from_raw(width, height, bytes.to_vec())
            .ok_or_else(|| anyhow::anyhow!("无法创建图片"))?;

        // 生成文件名
        let now = chrono::Local::now();
        let filename = format!("screenshot_{}.png", now.format("%Y%m%d_%H%M%S"));
        let filepath = self.screenshot_dir.join(&filename);

        // 保存
        img.save(&filepath)?;

        let path_str = filepath.to_string_lossy().to_string();
        tracing::info!("剪贴板图片已保存: {}", path_str);
        Ok(path_str)
    }
}
