//! A terminal that does not use WezTerm.
//!
//! next-core runs the shell and owns the screen; unterm-render draws it; winit
//! provides the window. This binary exists alongside `unterm` rather than
//! replacing it, so the working terminal keeps working while this one grows.

mod terminal;

fn main() -> anyhow::Result<()> {
    println!("unterm-app: terminal front end built on next-core");
    Ok(())
}
