//! 输入处理模块
//! 处理键盘事件，转换为终端输入序列。

use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

/// 输入处理器
pub struct InputHandler;

impl InputHandler {
    pub fn new() -> Self {
        Self
    }

    /// 将 winit 按键事件转换为终端序列
    pub fn key_to_sequence(&self, key: &PhysicalKey, state: ElementState, text: Option<&str>) -> Option<Vec<u8>> {
        if state != ElementState::Pressed {
            return None;
        }

        // 如果有文本输入，直接使用
        if let Some(text) = text {
            if !text.is_empty() {
                return Some(text.as_bytes().to_vec());
            }
        }

        // 特殊按键映射为 ANSI 转义序列
        if let PhysicalKey::Code(code) = key {
            match code {
                KeyCode::Enter => Some(b"\r".to_vec()),
                KeyCode::Backspace => Some(b"\x7f".to_vec()),
                KeyCode::Tab => Some(b"\t".to_vec()),
                KeyCode::Escape => Some(b"\x1b".to_vec()),
                KeyCode::ArrowUp => Some(b"\x1b[A".to_vec()),
                KeyCode::ArrowDown => Some(b"\x1b[B".to_vec()),
                KeyCode::ArrowRight => Some(b"\x1b[C".to_vec()),
                KeyCode::ArrowLeft => Some(b"\x1b[D".to_vec()),
                KeyCode::Home => Some(b"\x1b[H".to_vec()),
                KeyCode::End => Some(b"\x1b[F".to_vec()),
                KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
                KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
                KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
                KeyCode::Insert => Some(b"\x1b[2~".to_vec()),
                _ => None,
            }
        } else {
            None
        }
    }
}
