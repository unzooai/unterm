use serde::{Deserialize, Serialize};

use crate::session::SessionId;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenReadRequest {
    pub session_id: SessionId,
    /// 读取行数，None 表示整个可见区域
    pub lines: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenReadResponse {
    pub content: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenCursorRequest {
    pub session_id: SessionId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenScrollRequest {
    pub session_id: SessionId,
    /// 从滚动缓冲区顶部的偏移量
    pub offset: u32,
    /// 读取的行数
    pub count: u32,
}
