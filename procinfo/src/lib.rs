use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[cfg(feature = "dynamic")]
use wezterm_dynamic::{FromDynamic, ToDynamic};

mod linux;
mod macos;
mod windows;

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "dynamic", derive(FromDynamic, ToDynamic))]
pub enum LocalProcessStatus {
    Idle,
    Run,
    Sleep,
    Stop,
    Zombie,
    Tracing,
    Dead,
    Wakekill,
    Waking,
    Parked,
    LockBlocked,
    Unknown,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "dynamic", derive(FromDynamic, ToDynamic))]
pub struct LocalProcessInfo {
    /// The process identifier
    pub pid: u32,
    /// The parent process identifier
    pub ppid: u32,
    /// The COMM name of the process. May not bear any relation to
    /// the executable image name. May be changed at runtime by
    /// the process.
    /// Many systems truncate this
    /// field to 15-16 characters.
    pub name: String,
    /// Path to the executable image
    pub executable: PathBuf,
    /// The argument vector.
    /// Some systems allow changing the argv block at runtime
    /// eg: setproctitle().
    pub argv: Vec<String>,
    /// The current working directory for the process, or an empty
    /// path if it was not accessible for some reason.
    pub cwd: PathBuf,
    /// The status of the process. Not all possible values are
    /// portably supported on all systems.
    pub status: LocalProcessStatus,
    /// A clock value in unspecified system dependent units that
    /// indicates the relative age of the process.
    pub start_time: u64,
    /// The console handle associated with the process, if any.
    #[cfg(windows)]
    pub console: u64,
    /// Child processes, keyed by pid
    pub children: HashMap<u32, LocalProcessInfo>,
}

impl LocalProcessInfo {
    /// Walk this sub-tree of processes and return a unique set
    /// of executable base names. eg: `foo/bar` and `woot/bar`
    /// produce a set containing just `bar`.
    pub fn flatten_to_exe_names(&self) -> HashSet<String> {
        let mut names = HashSet::new();

        fn flatten(item: &LocalProcessInfo, names: &mut HashSet<String>) {
            if let Some(exe) = item.executable.file_name() {
                names.insert(exe.to_string_lossy().into_owned());
            }
            for proc in item.children.values() {
                flatten(proc, names);
            }
        }

        flatten(self, &mut names);
        names
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    pub fn with_root_pid(_pid: u32) -> Option<Self> {
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    pub fn current_working_dir(_pid: u32) -> Option<PathBuf> {
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    pub fn executable_path(_pid: u32) -> Option<PathBuf> {
        None
    }
}

/// How deep a process tree we are willing to materialise.
///
/// The visited set below already bounds the total work to "one node per
/// process on the machine", but it does not bound the *depth*: a machine with
/// a thousand processes strung into one parent chain still means a thousand
/// nested [`LocalProcessInfo`] frames here, and every consumer walks the
/// result recursively too. Windows gives the GUI main thread a 1 MiB stack,
/// which is where this whole family of bugs bit us. Real shell trees are
/// single-digit deep, so anything past this is pathology: stop descending and
/// keep the terminal alive rather than take the process down for a subtree
/// nobody is looking at.
const MAX_TREE_DEPTH: usize = 128;

impl LocalProcessInfo {
    /// Materialise the process tree rooted at `root` out of a flat snapshot of
    /// every process on the machine.
    ///
    /// Every platform backend has the same shape -- take a system-wide
    /// snapshot, then recurse into whichever entries name the current entry as
    /// their parent -- and every one of them had the same bug, so the walk
    /// lives here once instead of three times.
    ///
    /// **The parent/child relation is not a tree.** It is an arbitrary
    /// directed graph that merely usually looks like one. Pids get recycled,
    /// and no OS rewrites a child's recorded parent-pid when the parent dies,
    /// so a dead parent's pid can be reissued to a process that is already a
    /// descendant of the orphan -- and now the graph has a cycle. A plain
    /// recursive expansion follows that cycle forever and overflows the stack
    /// no matter how big the stack is; CI hit exactly that as
    /// `STATUS_STACK_OVERFLOW`.
    ///
    /// `visited` makes the output a strict tree: each pid is expanded at most
    /// once, so an edge pointing at something already in the tree is
    /// **dropped** rather than followed. Dropping beats the alternative
    /// (duplicating the subtree until some depth cutoff) because being a tree
    /// is the whole contract of this type: `children` owns its children, so a
    /// process that were its own descendant is not even representable, and
    /// every consumer relies on each process appearing exactly once --
    /// `flatten_to_exe_names` here, `count_descendants` and
    /// `detect_agent_in_process_tree` over in unterm-engine. A duplicated
    /// subtree would inflate the descendant count and let the "which program
    /// is in the foreground" search settle on a process's own ancestor.
    /// Nothing real is lost, either: an edge is only ever dropped when its
    /// target already sits in the tree under the first parent that claimed it.
    ///
    /// A process whose parent-pid is *itself* needs no special case: each
    /// entry is marked visited before its children are considered, so a
    /// self-edge is just the shortest possible back-edge and dies the same
    /// way.
    ///
    /// `pid_of` / `ppid_of` read the identifiers out of the platform's own
    /// snapshot record; `make_node` builds the childless node for one record.
    /// That last one is the expensive half on Windows -- it opens the process
    /// -- so it is only called for entries we actually keep.
    pub fn build_tree<T, P, Q, M>(root: &T, procs: &[T], pid_of: P, ppid_of: Q, make_node: M) -> Self
    where
        P: Fn(&T) -> u32,
        Q: Fn(&T) -> u32,
        M: Fn(&T) -> Self,
    {
        fn build<T, P, Q, M>(
            entry: &T,
            procs: &[T],
            pid_of: &P,
            ppid_of: &Q,
            make_node: &M,
            visited: &mut HashSet<u32>,
            depth: usize,
        ) -> LocalProcessInfo
        where
            P: Fn(&T) -> u32,
            Q: Fn(&T) -> u32,
            M: Fn(&T) -> LocalProcessInfo,
        {
            let pid = pid_of(entry);
            let mut node = make_node(entry);
            // The backends fill children in themselves; this walk owns that
            // field, so start from a clean slate whatever they handed us.
            node.children.clear();

            if depth >= MAX_TREE_DEPTH {
                log::warn!("process tree deeper than {MAX_TREE_DEPTH} at pid {pid}; stopping here");
                return node;
            }

            for kid in procs {
                if ppid_of(kid) != pid {
                    continue;
                }
                let kid_pid = pid_of(kid);
                // `insert` reports false when this pid is already in the tree:
                // the cycle back-edge, a self-parent, or the same pid listed
                // twice in a torn snapshot. All three mean "do not expand".
                if !visited.insert(kid_pid) {
                    log::debug!(
                        "process tree: pid {kid_pid} names pid {pid} as its parent \
                         but is already in the tree; dropping the edge"
                    );
                    continue;
                }
                let child = build(kid, procs, pid_of, ppid_of, make_node, visited, depth + 1);
                node.children.insert(kid_pid, child);
            }

            node
        }

        let mut visited = HashSet::new();
        visited.insert(pid_of(root));
        build(root, procs, &pid_of, &ppid_of, &make_node, &mut visited, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for the platform snapshot records: all `build_tree` ever
    /// asks of them is a pid and a parent pid.
    struct Entry {
        pid: u32,
        ppid: u32,
    }

    fn entries(pairs: &[(u32, u32)]) -> Vec<Entry> {
        pairs
            .iter()
            .map(|&(pid, ppid)| Entry { pid, ppid })
            .collect()
    }

    fn node(entry: &Entry) -> LocalProcessInfo {
        LocalProcessInfo {
            pid: entry.pid,
            ppid: entry.ppid,
            name: format!("proc{}", entry.pid),
            executable: PathBuf::from(format!("/bin/proc{}", entry.pid)),
            argv: vec![format!("proc{}", entry.pid)],
            cwd: PathBuf::new(),
            status: LocalProcessStatus::Run,
            start_time: entry.pid as u64,
            #[cfg(windows)]
            console: 0,
            children: HashMap::new(),
        }
    }

    fn tree(pairs: &[(u32, u32)], root_pid: u32) -> LocalProcessInfo {
        let procs = entries(pairs);
        let root = procs
            .iter()
            .find(|entry| entry.pid == root_pid)
            .expect("root in snapshot");
        LocalProcessInfo::build_tree(root, &procs, |e| e.pid, |e| e.ppid, node)
    }

    /// Every pid reachable from the root, with the multiplicity it actually
    /// occurs with -- so a duplicated subtree shows up as a longer list.
    fn pids(tree: &LocalProcessInfo) -> Vec<u32> {
        let mut out = vec![tree.pid];
        let mut kids: Vec<_> = tree.children.values().collect();
        kids.sort_by_key(|kid| kid.pid);
        for kid in kids {
            out.extend(pids(kid));
        }
        out
    }

    #[test]
    fn an_ordinary_tree_is_built_in_full() {
        let tree = tree(&[(1, 0), (2, 1), (3, 1), (4, 2)], 1);
        assert_eq!(pids(&tree), vec![1, 2, 4, 3]);
    }

    /// The stack-overflow bug: 3 says its parent is 1, but 1 is already an
    /// ancestor of 3. Following that edge recurses forever.
    #[test]
    fn a_cycle_terminates_and_the_back_edge_is_dropped() {
        let tree = tree(&[(1, 3), (2, 1), (3, 2)], 1);
        assert_eq!(pids(&tree), vec![1, 2, 3]);
        // The root in particular must not turn up beneath itself.
        assert!(tree.children[&2].children[&3].children.is_empty());
    }

    /// A dead parent's pid recycled onto its own child: 2 ends up claiming
    /// itself as its parent.
    #[test]
    fn a_process_that_is_its_own_parent_does_not_recurse() {
        let tree = tree(&[(1, 0), (2, 2)], 2);
        assert_eq!(pids(&tree), vec![2]);
    }

    /// Same, but reached as somebody's child rather than as the root.
    #[test]
    fn a_self_parented_child_is_kept_once() {
        let tree = tree(&[(1, 0), (2, 1), (3, 3)], 1);
        assert_eq!(pids(&tree), vec![1, 2]);
    }

    /// A torn snapshot can list the same pid twice. Expanding it twice would
    /// double every count taken over the tree.
    #[test]
    fn a_duplicated_snapshot_entry_is_expanded_once() {
        let tree = tree(&[(1, 0), (2, 1), (2, 1), (3, 2)], 1);
        assert_eq!(pids(&tree), vec![1, 2, 3]);
    }

    /// A pathologically deep chain must not run the stack out either. The
    /// visited set alone would happily build 5000 nested frames.
    #[test]
    fn an_absurdly_deep_chain_is_cut_off() {
        let pairs: Vec<(u32, u32)> = (1..=5000).map(|pid| (pid, pid - 1)).collect();
        let tree = tree(&pairs, 1);
        assert_eq!(pids(&tree).len(), MAX_TREE_DEPTH + 1);
    }

    #[test]
    fn flatten_to_exe_names_sees_each_process_once() {
        let tree = tree(&[(1, 3), (2, 1), (3, 2)], 1);
        assert_eq!(tree.flatten_to_exe_names().len(), 3);
    }
}
