//! What the foreground process is doing: CPU, memory, how long it has run.
//!
//! Shown in the top bar next to the git branch, so the answer to "why is this
//! machine hot" is in the window rather than in another tool. Three numbers,
//! sampled per process id.
//!
//! Refreshing is the whole problem. The bar repaints at the display's rate and
//! the value is wanted for whichever pane is in front, so a naive read would
//! fork a process per frame. Instead every reader gets the cached value
//! immediately and a refresh happens on another thread when the cache is
//! stale, with a cap on how many can be in flight at once.

use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// One sighting of a process: how much of a core it is using, how much memory
/// it holds, how long it has been alive, and what it is called.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcStatus {
    /// Percent of one core. Above 100 on a process using more than one.
    pub cpu_pct: f32,
    /// Resident memory, in bytes.
    pub rss_bytes: u64,
    /// Wall-clock seconds since it started.
    pub uptime_secs: u64,
    /// Its name, so the reader knows whose CPU this is.
    pub name: String,
}

/// How long a sample is good for. Long enough that a repaint does not cause a
/// read, short enough that a number that is changing looks like it is.
const CACHE_TTL: Duration = Duration::from_millis(2000);
/// How long a process nobody has asked about stays in the map.
const PRUNE_AFTER: Duration = Duration::from_secs(60);
/// Below this the map is not worth walking.
const PRUNE_MIN_SIZE: usize = 256;
/// How many refreshes may be running at once. Switching through many tabs
/// quickly asks about many processes; this is what stops that being a fork
/// bomb.
const MAX_INFLIGHT: usize = 16;

#[derive(Default)]
struct Cache {
    by_pid: HashMap<u32, (Instant, Option<ProcStatus>)>,
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Cache::default()))
}

fn inflight() -> &'static Mutex<HashSet<u32>> {
    static INFLIGHT: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
    INFLIGHT.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Drop entries nobody has looked at in a while.
///
/// Only once the map is big enough to be worth walking: a terminal with four
/// tabs would otherwise pay for this on every read and never drop anything.
fn prune_stale<K, V>(map: &mut HashMap<K, (Instant, V)>, min_size: usize, after: Duration)
where
    K: Eq + Hash + Clone,
{
    if map.len() < min_size {
        return;
    }
    map.retain(|_, (at, _)| at.elapsed() < after);
}

/// What is known about `pid` right now.
///
/// Returns immediately, with the previous sample if the current one is stale,
/// and `None` the first time a process is asked about. Never blocks the
/// caller: a stats line is not worth a frame.
pub fn status_for(pid: u32) -> Option<ProcStatus> {
    let cached;
    {
        let mut cache = cache().lock();
        prune_stale(&mut cache.by_pid, PRUNE_MIN_SIZE, PRUNE_AFTER);
        match cache.by_pid.get(&pid) {
            Some((at, status)) if at.elapsed() < CACHE_TTL => return status.clone(),
            Some((_, status)) => cached = status.clone(),
            None => cached = None,
        }
    }

    let mut running = inflight().lock();
    if running.len() < MAX_INFLIGHT && running.insert(pid) {
        let spawned = std::thread::Builder::new()
            .name("process-stats".into())
            .spawn(move || {
                let fresh = sample(pid);
                cache().lock().by_pid.insert(pid, (Instant::now(), fresh));
                inflight().lock().remove(&pid);
            });
        if spawned.is_err() {
            running.remove(&pid);
        }
    }
    cached
}

/// Sample synchronously, for tests and for callers already off the UI thread.
pub fn sample_now(pid: u32) -> Option<ProcStatus> {
    sample(pid)
}

/// Last (wall clock, cumulative CPU time) seen for a process, both in the
/// 100 ns units Windows counts in.
///
/// A percent needs two samples: Windows reports how much CPU a process has
/// used since it started, not how much it is using now.
#[cfg(windows)]
struct CpuSample {
    wall_100ns: u64,
    cpu_100ns: u64,
}

#[cfg(windows)]
fn cpu_samples() -> &'static Mutex<HashMap<u32, (Instant, CpuSample)>> {
    static SAMPLES: OnceLock<Mutex<HashMap<u32, (Instant, CpuSample)>>> = OnceLock::new();
    SAMPLES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(windows)]
fn sample(pid: u32) -> Option<ProcStatus> {
    // Read by hand rather than by running something. There is no `ps` here,
    // and asking PowerShell costs a hidden process launch per refresh and
    // still cannot report a percent -- a percent is a difference between two
    // readings, and a program that exits has only made one.
    use winapi::shared::minwindef::FILETIME;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{GetProcessTimes, OpenProcess};
    use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;
    use winapi::um::winbase::QueryFullProcessImageNameW;
    use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;

    fn as_100ns(time: &FILETIME) -> u64 {
        ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
    }

    struct Handle(winapi::um::winnt::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let handle = Handle(handle);

    let (created, kernel, user) = unsafe {
        let mut created: FILETIME = std::mem::zeroed();
        let mut exited: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        if GetProcessTimes(handle.0, &mut created, &mut exited, &mut kernel, &mut user) == 0 {
            return None;
        }
        (created, kernel, user)
    };
    let now_100ns = unsafe {
        let mut now: FILETIME = std::mem::zeroed();
        GetSystemTimeAsFileTime(&mut now);
        as_100ns(&now)
    };
    let created_100ns = as_100ns(&created);
    let cpu_100ns = as_100ns(&kernel) + as_100ns(&user);
    let uptime_secs = now_100ns.saturating_sub(created_100ns) / 10_000_000;

    let cpu_pct = {
        let mut samples = cpu_samples().lock();
        prune_stale(&mut samples, PRUNE_MIN_SIZE, PRUNE_AFTER);
        let previous = samples.insert(
            pid,
            (
                Instant::now(),
                CpuSample {
                    wall_100ns: now_100ns,
                    cpu_100ns,
                },
            ),
        );
        // The first sighting has no window to measure over, so it reports the
        // average across the process's whole life -- which is the honest
        // answer to "what has it been doing", just not to "what is it doing".
        let (wall_delta, cpu_delta) = match previous {
            // Both subtractions are guarded: the clock can step backwards when
            // the machine syncs time, and a reused process id would make the
            // CPU total appear to shrink.
            Some((_, previous))
                if now_100ns > previous.wall_100ns && cpu_100ns >= previous.cpu_100ns =>
            {
                (now_100ns - previous.wall_100ns, cpu_100ns - previous.cpu_100ns)
            }
            _ => (now_100ns.saturating_sub(created_100ns), cpu_100ns),
        };
        if wall_delta == 0 {
            0.0
        } else {
            (cpu_delta as f64 / wall_delta as f64 * 100.0) as f32
        }
    };

    let rss_bytes = unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if GetProcessMemoryInfo(handle.0, &mut counters, counters.cb) != 0 {
            counters.WorkingSetSize as u64
        } else {
            0
        }
    };

    // The image name without its extension: `pwsh`, not `pwsh.exe`.
    let name = unsafe {
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        if QueryFullProcessImageNameW(handle.0, 0, buffer.as_mut_ptr(), &mut length) != 0 {
            let full = String::from_utf16_lossy(&buffer[..length as usize]);
            std::path::Path::new(&full)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_else(|| "?".to_string())
        } else {
            "?".to_string()
        }
    };

    Some(ProcStatus {
        cpu_pct,
        rss_bytes,
        uptime_secs,
        name,
    })
}

#[cfg(unix)]
fn sample(pid: u32) -> Option<ProcStatus> {
    // POSIX `ps` with empty `=` headers prints the values and no titles, one
    // space-separated line. The same call works on macOS, Linux and the BSDs.
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pcpu=,rss=,etime=,comm="])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // The fields come left-padded (`  0.0   1712 ...`), so splitting on a
    // single space yields empty fragments and the memory reading is lost.
    // Splitting on whitespace collapses each run into one separator.
    let line = String::from_utf8_lossy(&output.stdout);
    let mut parts = line.split_whitespace();
    let cpu_pct: f32 = parts.next()?.parse().ok()?;
    let rss_kb: u64 = parts.next()?.parse().ok()?;
    let uptime_secs = parse_etime(parts.next()?);
    // The name can have spaces in it -- a login shell starts with a dash --
    // so whatever is left is the name.
    let name = parts.collect::<Vec<_>>().join(" ");
    Some(ProcStatus {
        cpu_pct,
        rss_bytes: rss_kb * 1024,
        uptime_secs,
        name: if name.is_empty() {
            "?".to_string()
        } else {
            name
        },
    })
}

/// Read `ps`'s elapsed time: `MM:SS`, `HH:MM:SS`, or `DD-HH:MM:SS`.
#[cfg_attr(windows, allow(dead_code))]
fn parse_etime(text: &str) -> u64 {
    let (days, rest) = match text.split_once('-') {
        Some((days, rest)) => (days.parse::<u64>().unwrap_or(0), rest),
        None => (0, text),
    };
    let parts: Vec<u64> = rest.split(':').filter_map(|part| part.parse().ok()).collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [hours, minutes, seconds] => (*hours, *minutes, *seconds),
        [minutes, seconds] => (0, *minutes, *seconds),
        _ => (0, 0, 0),
    };
    days * 86_400 + hours * 3_600 + minutes * 60 + seconds
}

/// Memory in as few characters as it can be said in.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{}M", bytes / MB)
    } else if bytes >= KB {
        format!("{}K", bytes / KB)
    } else {
        format!("{bytes}B")
    }
}

/// A duration at one unit of precision: what matters is the order of
/// magnitude, and a bar full of seconds counting up is a distraction.
pub fn format_uptime(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3_600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h{:02}m", seconds / 3_600, (seconds % 3_600) / 60)
    } else {
        format!("{}d{:02}h", seconds / 86_400, (seconds % 86_400) / 3_600)
    }
}

/// `8.0% cpu  1.4G  4m`, or nothing at all.
///
/// Two spaces rather than a middle dot: the dot does not share a baseline with
/// the digits in a monospaced font, and the gap reads cleaner against a run of
/// numbers anyway. `cpu` is lower case so there is no single run of capitals
/// pulling the eye to one segment.
pub fn render_segment(status: &Option<ProcStatus>) -> String {
    let Some(status) = status else {
        return String::new();
    };
    format!(
        "{:.1}% cpu  {}  {}",
        status.cpu_pct,
        format_bytes(status.rss_bytes),
        format_uptime(status.uptime_secs)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one process guaranteed to exist is this one.
    #[test]
    fn this_process_can_be_sampled() {
        let pid = std::process::id();
        let first = sample_now(pid).expect("a process can read itself");
        assert!(first.rss_bytes > 0, "no memory reported");
        assert!(!first.name.is_empty() && first.name != "?", "no name");
    }

    /// A percent is a difference between two readings. The first reading has
    /// nothing to subtract from, so the second is where the answer is; burn a
    /// little CPU between them and it has to show up.
    #[cfg(windows)]
    #[test]
    fn cpu_is_measured_between_two_readings() {
        let pid = std::process::id();
        let _seed = sample_now(pid).expect("a process can read itself");
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(150) {
            std::hint::black_box(start.elapsed());
        }
        let busy = sample_now(pid).expect("a process can read itself");
        assert!(
            busy.cpu_pct > 5.0,
            "a busy process reported {}% cpu",
            busy.cpu_pct
        );
    }

    /// A process id nobody is using reads as nothing rather than as zeroes: a
    /// row of zeroes looks like an idle process, not a missing one.
    #[test]
    fn a_process_that_is_not_there_reports_nothing() {
        // Nothing has this id: it is above the range the kernels allocate from
        // and, on Windows, is not a multiple of four.
        assert_eq!(sample_now(0xFFFF_FFFE), None);
    }

    /// The first call cannot answer, and must not block waiting to.
    #[test]
    fn the_cached_reader_never_blocks() {
        let pid = std::process::id();
        let start = Instant::now();
        let _ = status_for(pid);
        assert!(
            start.elapsed() < Duration::from_millis(50),
            "reading took {:?}",
            start.elapsed()
        );
    }

    /// And once the refresh has landed, it answers.
    #[test]
    fn a_refresh_lands_in_the_cache() {
        let pid = std::process::id();
        let _ = status_for(pid);
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if status_for(pid).is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("no sample arrived within five seconds");
    }

    #[test]
    fn memory_is_written_at_the_size_it_is() {
        assert_eq!(format_bytes(512), "512B");
        assert_eq!(format_bytes(2048), "2K");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5M");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024 / 2), "1.5G");
    }

    #[test]
    fn a_duration_is_written_at_one_unit_of_precision() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(59), "59s");
        assert_eq!(format_uptime(60), "1m");
        assert_eq!(format_uptime(3_599), "59m");
        assert_eq!(format_uptime(3_661), "1h01m");
        assert_eq!(format_uptime(90_061), "1d01h");
    }

    /// `ps` writes elapsed time three different ways depending on how long the
    /// process has been up, and all three have to read the same.
    #[test]
    fn every_shape_of_elapsed_time_is_understood() {
        assert_eq!(parse_etime("05:30"), 330);
        assert_eq!(parse_etime("01:05:30"), 3_930);
        assert_eq!(parse_etime("2-01:05:30"), 2 * 86_400 + 3_930);
        assert_eq!(parse_etime("nonsense"), 0);
    }

    #[test]
    fn the_segment_says_all_three_numbers() {
        let status = Some(ProcStatus {
            cpu_pct: 8.0,
            rss_bytes: 3 * 1024 * 1024 * 1024 / 2,
            uptime_secs: 240,
            name: "pwsh".into(),
        });
        assert_eq!(render_segment(&status), "8.0% cpu  1.5G  4m");
    }

    /// Nothing known is nothing shown -- not `0.0% cpu  0B  0s`, which reads
    /// as a process doing nothing rather than as a process not yet read.
    #[test]
    fn nothing_known_shows_nothing() {
        assert_eq!(render_segment(&None), "");
    }
}
