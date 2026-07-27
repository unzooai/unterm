use crate::CursorSnapshot;

pub(super) fn default_cursor() -> CursorSnapshot {
    CursorSnapshot {
        x: 0,
        y: 0,
        visible: true,
        shape: "Default".to_string(),
    }
}
