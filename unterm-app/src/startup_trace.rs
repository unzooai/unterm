use std::io::Write as _;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static TRACE: OnceLock<Mutex<StartupTrace>> = OnceLock::new();

struct StartupTrace {
    enabled: bool,
    started: Instant,
    previous: Instant,
    path: Option<std::path::PathBuf>,
}

pub fn init() {
    let mut trace = TRACE
        .get_or_init(|| Mutex::new(StartupTrace::new()))
        .lock()
        .unwrap();
    trace.mark("process.start");
}

pub fn mark(what: &str) {
    let Some(trace) = TRACE.get() else {
        return;
    };
    trace.lock().unwrap().mark(what);
}

impl StartupTrace {
    fn new() -> Self {
        let enabled = startup_trace_enabled(std::env::var("UNTERM_STARTUP_TRACE").ok().as_deref());
        let now = Instant::now();
        let path = enabled
            .then(|| unterm_protocol::state_path("startup-trace.log"))
            .flatten();
        let mut trace = Self {
            enabled,
            started: now,
            previous: now,
            path,
        };
        if enabled {
            trace.write_header();
        }
        trace
    }

    fn mark(&mut self, what: &str) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        let total = now.duration_since(self.started).as_millis();
        let delta = now.duration_since(self.previous).as_millis();
        self.previous = now;
        self.write_line(&format!("total_ms={total} delta_ms={delta} {what}\n"));
        log::info!("startup trace total_ms={total} delta_ms={delta} {what}");
    }

    fn write_header(&mut self) {
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        self.write_line(&format!(
            "\n[{stamp}] unterm startup pid={} version={} commit={}\n",
            std::process::id(),
            unterm_protocol::PRODUCT_VERSION,
            unterm_protocol::BUILD_COMMIT
        ));
    }

    fn write_line(&self, line: &str) {
        let Some(path) = &self.path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

fn startup_trace_enabled(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|value| {
            !value.eq_ignore_ascii_case("0")
                && !value.eq_ignore_ascii_case("false")
                && !value.eq_ignore_ascii_case("off")
        })
}

#[cfg(test)]
mod tests {
    use super::startup_trace_enabled;

    #[test]
    fn startup_trace_env_is_opt_in() {
        assert!(!startup_trace_enabled(None));
        assert!(!startup_trace_enabled(Some("")));
        assert!(!startup_trace_enabled(Some("0")));
        assert!(!startup_trace_enabled(Some("false")));
        assert!(!startup_trace_enabled(Some("off")));
        assert!(startup_trace_enabled(Some("1")));
        assert!(startup_trace_enabled(Some("yes")));
    }
}
