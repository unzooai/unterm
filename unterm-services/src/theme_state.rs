//! Process-local handoff from Web Settings/MCP-adjacent surfaces to the
//! native window. The HTTP server and GUI run in the same process, so a small
//! generation-stamped mailbox is enough to make a saved theme apply live.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeRequest {
    pub generation: u64,
    pub id: String,
}

fn state() -> &'static parking_lot::Mutex<ThemeRequest> {
    static STATE: std::sync::OnceLock<parking_lot::Mutex<ThemeRequest>> =
        std::sync::OnceLock::new();
    STATE.get_or_init(|| {
        parking_lot::Mutex::new(ThemeRequest {
            generation: 0,
            id: String::new(),
        })
    })
}

pub fn request(id: impl Into<String>) -> u64 {
    let mut state = state().lock();
    state.generation = state.generation.wrapping_add(1).max(1);
    state.id = id.into();
    state.generation
}

pub fn after(generation: u64) -> Option<ThemeRequest> {
    let state = state().lock();
    (state.generation > generation).then(|| state.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_is_delivered_once_per_observer_generation() {
        let before = state().lock().generation;
        let generation = request("midnight");
        assert!(generation > before);
        assert_eq!(
            after(before).map(|request| request.id),
            Some("midnight".into())
        );
        assert!(after(generation).is_none());
    }
}
