use super::{input_pipeline, session_handles};
use anyhow::Result;
use std::io::Write;
use std::time::Instant;

pub(super) fn write(pane_id: usize, input: &str) -> Result<()> {
    let handles = session_handles::input_current(pane_id)?;

    let started_at = Instant::now();
    let input = translated_input(input, handles.application_cursor_keys);
    let bytes = input.len();
    let mut writer = handles.writer.lock();
    writer.write_all(input.as_bytes())?;
    writer.flush()?;
    if !input.is_empty() {
        handles
            .activity
            .lock()
            .mark_input(bytes, started_at.elapsed());
    }
    Ok(())
}

/// Report a mouse event to the session, if it asked to hear about it.
///
/// `Ok(false)` means the application has mouse reporting off (or does not want
/// this event kind), so the caller keeps the mouse for terminal-side
/// behaviour like selection and scrollback.
pub(super) fn mouse(pane_id: usize, event: super::mouse_encoding::MouseEvent) -> Result<bool> {
    let handles = session_handles::input_current(pane_id)?;
    let Some(encoded) = super::mouse_encoding::encode_mouse(event, handles.mouse_modes) else {
        return Ok(false);
    };

    let started_at = Instant::now();
    let bytes = encoded.len();
    let mut writer = handles.writer.lock();
    writer.write_all(encoded.as_bytes())?;
    writer.flush()?;
    handles
        .activity
        .lock()
        .mark_input(bytes, started_at.elapsed());
    Ok(true)
}

pub(super) fn paste(pane_id: usize, text: &str) -> Result<()> {
    let handles = session_handles::input_current(pane_id)?;
    let bracketed = handles.bracketed_paste;
    let PasteWire {
        chunks,
        wire_bytes,
        chunk_count,
    } = paste_wire(text, bracketed);
    let started_at = Instant::now();

    {
        let mut writer = handles.writer.lock();
        for chunk in &chunks {
            writer.write_all(chunk.as_bytes())?;
        }
        writer.flush()?;
    }

    if !text.is_empty() || bracketed {
        let mut activity = handles.activity.lock();
        activity.mark_input(wire_bytes, started_at.elapsed());
        activity.mark_paste(
            text.len(),
            wire_bytes,
            chunk_count,
            bracketed,
            started_at.elapsed(),
        );
    }
    Ok(())
}

fn translated_input(input: &str, application_cursor_keys: bool) -> String {
    input_pipeline::application_cursor_input(input, application_cursor_keys)
}

struct PasteWire {
    chunks: Vec<String>,
    wire_bytes: usize,
    chunk_count: usize,
}

fn paste_wire(text: &str, bracketed: bool) -> PasteWire {
    let chunks = input_pipeline::paste_chunks(text, bracketed);
    let wire_bytes = chunks.iter().map(|chunk| chunk.len()).sum::<usize>();
    let chunk_count = chunks.len();
    PasteWire {
        chunks,
        wire_bytes,
        chunk_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translated_input_applies_application_cursor_mode() {
        assert_eq!(translated_input("\x1b[C", true), "\x1bOC");
        assert_eq!(translated_input("\x1b[C", false), "\x1b[C");
    }

    #[test]
    fn paste_wire_reports_chunk_count_and_wire_bytes() {
        let wire = paste_wire("token", true);

        assert_eq!(wire.chunk_count, 3);
        assert_eq!(wire.wire_bytes, "\x1b[200~token\x1b[201~".len());
        assert_eq!(wire.chunks.concat(), "\x1b[200~token\x1b[201~");
    }
}
