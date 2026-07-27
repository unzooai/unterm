use super::{MouseTrackingMode, NextCoreScreen};
use parking_lot::Mutex;
use std::io::Write;
use std::sync::Arc;

const HEADLESS_CELL_WIDTH_PX: usize = 8;
const HEADLESS_CELL_HEIGHT_PX: usize = 16;
pub(super) const MAX_PENDING_TERMINAL_QUERY_BYTES: usize = 128;

pub(super) fn answer_with_pending(
    chunk: &str,
    screen: &NextCoreScreen,
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    pending: &mut String,
) {
    let mut response = Vec::new();
    let input = if pending.is_empty() {
        chunk.to_string()
    } else {
        let mut input = std::mem::take(pending);
        input.push_str(chunk);
        input
    };
    let bytes = input.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] != 0x1b {
            idx += 1;
            continue;
        }

        if bytes.get(idx + 1) != Some(&b'[') {
            if idx + 1 >= bytes.len() {
                set_pending(pending, &input[idx..]);
                break;
            }
            idx += 1;
            continue;
        }

        let mut final_end = None;
        for (offset, c) in input[idx + 2..].char_indices() {
            if ('@'..='~').contains(&c) {
                final_end = Some(idx + 2 + offset + c.len_utf8());
                break;
            }
        }

        let Some(end) = final_end else {
            set_pending(pending, &input[idx..]);
            break;
        };

        if let Some(answer) = response_for_csi(&input[idx + 2..end], screen) {
            response.extend_from_slice(answer.as_slice());
            idx = end;
        } else {
            idx += 1;
        }
    }
    if !response.is_empty() {
        let mut writer = writer.lock();
        writer.write_all(&response).ok();
        writer.flush().ok();
    }
}

fn set_pending(pending: &mut String, value: &str) {
    pending.clear();
    if value.len() <= MAX_PENDING_TERMINAL_QUERY_BYTES {
        pending.push_str(value);
    }
}

fn response_for_csi(csi: &str, screen: &NextCoreScreen) -> Option<Vec<u8>> {
    if let Some(mode) = csi
        .strip_prefix('?')
        .and_then(|params| params.strip_suffix("$p"))
        .and_then(|params| params.parse::<usize>().ok())
    {
        let enabled = match mode {
            1 => screen.application_cursor_keys,
            3 => screen.column_132_mode,
            5 => screen.reverse_video,
            6 => screen.origin_mode,
            7 => screen.auto_wrap,
            12 => screen.cursor_blinking,
            25 => screen.cursor_visible,
            47 => screen.alternate_screen_modes.contains(&47),
            66 => screen.application_keypad,
            69 => screen.left_right_margin_mode,
            1000 => screen.mouse_tracking == MouseTrackingMode::X10,
            1002 => screen.mouse_tracking == MouseTrackingMode::ButtonEvent,
            1003 => screen.mouse_tracking == MouseTrackingMode::AnyEvent,
            1004 => screen.focus_event_reporting,
            1005 => screen.utf8_mouse,
            1006 => screen.sgr_mouse,
            1007 => screen.alternate_scroll,
            1015 => screen.urxvt_mouse,
            1016 => screen.sgr_pixel_mouse,
            1034 => screen.meta_sends_escape,
            1047 => screen.alternate_screen_modes.contains(&1047),
            1049 => screen.alternate_screen_modes.contains(&1049),
            2004 => screen.bracketed_paste,
            2026 => screen.synchronized_output,
            _ => return None,
        };
        return Some(format!("\x1b[?{mode};{}$y", mode_report_state(enabled)).into_bytes());
    }

    if let Some(mode) = csi
        .strip_suffix("$p")
        .and_then(|params| params.parse::<usize>().ok())
    {
        if mode == 4 {
            return Some(
                format!("\x1b[4;{}$y", mode_report_state(screen.insert_mode)).into_bytes(),
            );
        }
    }

    match csi {
        "?6n" => {
            Some(format!("\x1b[?{};{}R", screen.cursor_y + 1, screen.cursor_x + 1).into_bytes())
        }
        "14t" => Some(
            format!(
                "\x1b[4;{};{}t",
                screen.rows * HEADLESS_CELL_HEIGHT_PX,
                screen.cols * HEADLESS_CELL_WIDTH_PX
            )
            .into_bytes(),
        ),
        "18t" => Some(format!("\x1b[8;{};{}t", screen.rows, screen.cols).into_bytes()),
        "5n" => Some(b"\x1b[0n".to_vec()),
        "6n" => Some(format!("\x1b[{};{}R", screen.cursor_y + 1, screen.cursor_x + 1).into_bytes()),
        ">c" | ">0c" => Some(b"\x1b[>0;0;0c".to_vec()),
        "c" | "0c" => Some(b"\x1b[?64;1;2;6;9;15;18;21;22c".to_vec()),
        _ => None,
    }
}

fn mode_report_state(enabled: bool) -> usize {
    if enabled {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responds_to_basic_status_and_device_queries() {
        let mut screen = NextCoreScreen::new(80, 10);
        screen.set_cursor(2, 4);

        assert_eq!(response_for_csi("6n", &screen).unwrap(), b"\x1b[3;5R");
        assert_eq!(response_for_csi("?6n", &screen).unwrap(), b"\x1b[?3;5R");
        assert_eq!(response_for_csi("5n", &screen).unwrap(), b"\x1b[0n");
        assert_eq!(
            response_for_csi("c", &screen).unwrap(),
            b"\x1b[?64;1;2;6;9;15;18;21;22c"
        );
        assert_eq!(response_for_csi(">0c", &screen).unwrap(), b"\x1b[>0;0;0c");
    }

    #[test]
    fn responds_to_window_size_queries() {
        let screen = NextCoreScreen::new(132, 43);

        assert_eq!(
            response_for_csi("14t", &screen).unwrap(),
            b"\x1b[4;688;1056t"
        );
        assert_eq!(response_for_csi("18t", &screen).unwrap(), b"\x1b[8;43;132t");
    }

    #[test]
    fn responds_to_mode_reports() {
        let mut screen = NextCoreScreen::new(80, 10);
        screen.application_cursor_keys = true;
        screen.insert_mode = true;

        assert_eq!(response_for_csi("?1$p", &screen).unwrap(), b"\x1b[?1;1$y");
        assert_eq!(response_for_csi("4$p", &screen).unwrap(), b"\x1b[4;1$y");
        assert!(response_for_csi("?9999$p", &screen).is_none());
    }
}
