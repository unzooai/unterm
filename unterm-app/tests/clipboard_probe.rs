//! One-off probe: what does arboard see on the real clipboard right now?
//! Run by hand with --nocapture while the clipboard holds a system
//! screenshot; not part of any suite's assertions.

#[test]
#[ignore]
fn probe_clipboard() {
    let mut board = arboard::Clipboard::new().expect("open clipboard");
    match board.get_text() {
        Ok(text) => println!("text: {:?} ({} bytes)", &text[..text.len().min(60)], text.len()),
        Err(err) => println!("text err: {err}"),
    }
    match board.get_image() {
        Ok(image) => println!("image: {}x{}, {} bytes", image.width, image.height, image.bytes.len()),
        Err(err) => println!("image err: {err}"),
    }
}
