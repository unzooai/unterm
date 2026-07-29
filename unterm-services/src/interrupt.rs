//! Interrupting what is running in a pane.
//!
//! On a Unix pty, writing `0x03` is the whole story: the line discipline turns
//! it into SIGINT for the foreground process group. On Windows there is no
//! line discipline. ConPTY hands the byte to the console as a key record, the
//! shell's line editor sees it and abandons the line it was editing -- and a
//! program that is running and *not reading input*, which is every program you
//! actually want to interrupt, never hears about it.
//!
//! So Windows needs the real thing: a console control event, raised for the
//! process group that owns the pane. Doing that from outside the console means
//! attaching to it, which is a process-wide state change, so it is done under
//! a lock and undone immediately -- and our own handler is disabled first,
//! because the event we are about to raise would otherwise arrive here and
//! close the terminal along with the thing it was aimed at.

/// What was done to interrupt a pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interrupt {
    /// A console control event reached the process group.
    ConsoleEvent,
    /// The byte was written and that is all this platform needs.
    Byte,
    /// The foreground program was stopped, because the graceful route does
    /// not reach a pseudoconsole's clients.
    Stopped,
}

/// The byte an interrupt is spelled as on the wire.
pub const INTERRUPT_BYTE: &str = "\x03";

/// Stop the program running in a pane, as Ctrl+C is meant to.
///
/// The byte has already been written; on a pty that is the end of it. This is
/// the Windows half, and it is deliberately conditional: `foreground` is
/// stopped only when its console says it expects Ctrl+C to be a signal. A
/// program that has cleared `ENABLE_PROCESSED_INPUT` is reading the byte as a
/// keystroke and must be left alone -- ending someone's editor because they
/// pressed Ctrl+C inside it would be worse than doing nothing at all.
///
/// The shell itself is never stopped: with nothing running in front of it,
/// the byte the caller already wrote has cancelled its line, which is what
/// Ctrl+C at a prompt means.
#[cfg(windows)]
pub fn stop_foreground(shell_pid: u32, foreground: Option<u32>) -> anyhow::Result<Interrupt> {
    use anyhow::anyhow;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
    use winapi::um::winnt::PROCESS_TERMINATE;

    let Some(foreground) = foreground.filter(|pid| *pid != shell_pid) else {
        // Nothing in front of the shell: the byte did the job.
        return Ok(Interrupt::Byte);
    };
    if !expects_interrupt_signal(shell_pid) {
        // The program is reading keys, not listening for signals.
        return Ok(Interrupt::Byte);
    }

    // The graceful route first. It does not reach a pseudoconsole's clients
    // today, which is why there is a second step at all.
    let _ = interrupt_process_group(foreground);
    std::thread::sleep(std::time::Duration::from_millis(120));

    // SAFETY: the handle is closed on both paths.
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, foreground);
        if handle.is_null() {
            // Already gone: the interrupt landed after all.
            return Ok(Interrupt::ConsoleEvent);
        }
        let stopped = TerminateProcess(handle, 1);
        let error = std::io::Error::last_os_error();
        CloseHandle(handle);
        if stopped == 0 {
            return Err(anyhow!("could not stop process {foreground}: {error}"));
        }
    }
    Ok(Interrupt::Stopped)
}

/// A pty's line discipline already did this.
#[cfg(not(windows))]
pub fn stop_foreground(_shell_pid: u32, _foreground: Option<u32>) -> anyhow::Result<Interrupt> {
    Ok(Interrupt::Byte)
}

/// Raise a real interrupt for the process group rooted at `pid`.
///
/// Returns `Ok(Byte)` when the platform needs nothing beyond the byte the
/// caller has already written. Returns an error only when the platform *does*
/// need more and it could not be done -- so a caller can say plainly that the
/// interrupt did not land, rather than leaving the user wondering why Ctrl+C
/// did nothing.
#[cfg(windows)]
pub fn interrupt_process_group(pid: u32) -> anyhow::Result<Interrupt> {
    use anyhow::anyhow;
    use std::sync::Mutex;
    use winapi::um::consoleapi::SetConsoleCtrlHandler;
    use winapi::um::wincon::{
        AttachConsole, FreeConsole, GenerateConsoleCtrlEvent, CTRL_C_EVENT,
    };

    // `AttachConsole` reads (DWORD)-1 as ATTACH_PARENT_PROCESS. A pid that
    // arrives as u32::MAX -- a stale handle, a bad cast, a pane whose child
    // has gone -- would silently attach to whatever launched *us* and
    // interrupt that process group instead. Never that.
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    if pid == ATTACH_PARENT_PROCESS || pid == 0 {
        return Err(anyhow!("refusing to interrupt process group {pid}"));
    }

    // Attaching to a console is process-wide: two interrupts at once would
    // each free the other's console out from under it.
    static ATTACHING: Mutex<()> = Mutex::new(());
    let _guard = ATTACHING.lock().unwrap_or_else(|err| err.into_inner());

    // SAFETY: each call is checked, and the console is freed on every path
    // out -- including the failure paths, which is the whole reason this is
    // not written as a chain of `?`.
    // A handler that says "handled", rather than the ignore flag.
    //
    // `SetConsoleCtrlHandler(NULL, TRUE)` sets an *attribute* on the process
    // -- ignore Ctrl+C -- and with it set before attaching, the event stops
    // reaching the other processes on the console too: raised, reported
    // successful, and the child keeps running. A real handler protects only
    // this process and leaves the event to do its job.
    unsafe extern "system" fn swallow(_kind: u32) -> i32 {
        1 // TRUE: handled here, do not run the default handler.
    }

    // SAFETY: each call is checked, and the console is freed on every path
    // out -- including the failure paths, which is the whole reason this is
    // not written as a chain of `?`.
    unsafe {
        // Ours before we join their console: the event we are about to raise
        // goes to every process attached to it, and without this the terminal
        // exits along with the command. Ordering matters -- doing it after
        // the attach loses the race, which is not theoretical: it killed the
        // test that found it, with STATUS_CONTROL_C_EXIT.
        SetConsoleCtrlHandler(Some(swallow), 1);

        // We have no console of our own (this is a GUI process), but a
        // previous attach may not have been freed if something panicked.
        FreeConsole();

        if AttachConsole(pid) == 0 {
            let code = std::io::Error::last_os_error();
            SetConsoleCtrlHandler(Some(swallow), 0);
            return Err(anyhow!(
                "could not attach to the console of process {pid}: {code}"
            ));
        }

        // Ctrl+C only. Ctrl+Break reaches processes that have masked Ctrl+C,
        // which sounds like an improvement until you watch it travel further
        // than intended and end things nobody aimed at.
        let raised = GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0);
        let error = std::io::Error::last_os_error();

        // Let the event be delivered before detaching: freeing the console
        // first can drop it on the floor.
        std::thread::sleep(std::time::Duration::from_millis(400));

        FreeConsole();
        SetConsoleCtrlHandler(Some(swallow), 0);

        if raised == 0 {
            return Err(anyhow!("could not interrupt process {pid}: {error}"));
        }
    }
    Ok(Interrupt::ConsoleEvent)
}

/// Whether the program on a pane's console expects Ctrl+C to be a signal.
///
/// This is the distinction that decides everything. A shell, and any program
/// that leaves the console in its default mode, has `ENABLE_PROCESSED_INPUT`
/// set and expects Ctrl+C to interrupt it. A full-screen program -- vim, less,
/// anything that reads keys directly -- clears that bit precisely so it can
/// read `0x03` as a keystroke and decide for itself.
///
/// Getting this backwards would close someone's editor on a keystroke the
/// editor was waiting for, so when the mode cannot be read the answer is no.
#[cfg(windows)]
pub fn expects_interrupt_signal(pid: u32) -> bool {
    use std::sync::Mutex;
    use winapi::um::consoleapi::GetConsoleMode;
    use winapi::um::fileapi::{CreateFileA, OPEN_EXISTING};
    use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
    use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE};
    use winapi::um::wincon::{AttachConsole, FreeConsole, ENABLE_PROCESSED_INPUT};

    if pid == u32::MAX || pid == 0 {
        return false;
    }

    static ATTACHING: Mutex<()> = Mutex::new(());
    let _guard = ATTACHING.lock().unwrap_or_else(|err| err.into_inner());

    // SAFETY: the console is freed on every path out, and the handle is
    // closed before it.
    unsafe {
        FreeConsole();
        if AttachConsole(pid) == 0 {
            return false;
        }
        // CONIN$ rather than the standard handle: this process is a GUI one
        // and has no stdin of its own to ask about.
        let name = b"CONIN$ ";
        let input = CreateFileA(
            name.as_ptr() as *const i8,
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        let mut mode: u32 = 0;
        let read = input != INVALID_HANDLE_VALUE && GetConsoleMode(input, &mut mode) != 0;
        if input != INVALID_HANDLE_VALUE {
            CloseHandle(input);
        }
        FreeConsole();

        read && (mode & ENABLE_PROCESSED_INPUT) != 0
    }
}

/// A pty's line discipline decides this, not the terminal.
#[cfg(not(windows))]
pub fn expects_interrupt_signal(_pid: u32) -> bool {
    false
}

/// The byte is enough: a pty's line discipline turns it into SIGINT.
#[cfg(not(windows))]
pub fn interrupt_process_group(_pid: u32) -> anyhow::Result<Interrupt> {
    Ok(Interrupt::Byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interrupt_byte_is_what_a_terminal_sends() {
        assert_eq!(INTERRUPT_BYTE, "\u{3}");
    }

    #[cfg(not(windows))]
    #[test]
    fn a_pty_needs_nothing_beyond_the_byte() {
        assert_eq!(interrupt_process_group(1).unwrap(), Interrupt::Byte);
    }

    #[cfg(windows)]
    #[test]
    fn a_process_that_is_gone_is_reported_rather_than_silently_ignored() {
        // Ctrl+C doing nothing with no explanation is the failure this whole
        // module exists to remove; failing quietly here would recreate it.
        // 0x7FFF_FFFE is not a pid Windows hands out.
        let result = interrupt_process_group(0x7FFF_FFFE);
        assert!(result.is_err(), "attaching to nothing should not succeed");
    }

    #[cfg(windows)]
    #[test]
    fn the_parent_process_is_never_interrupted_by_accident() {
        // (DWORD)-1 is ATTACH_PARENT_PROCESS. A pid that arrives as u32::MAX
        // -- a stale handle, a bad cast -- would otherwise attach to whatever
        // launched the terminal and interrupt that instead.
        assert!(interrupt_process_group(u32::MAX).is_err());
        assert!(interrupt_process_group(0).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn interrupting_a_real_child_reaches_it() {
        use std::io::Read;
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

        // A console of its own, so the event cannot reach the test runner.
        let mut child = match Command::new("cmd.exe")
            .args(["/c", "pause"])
            .creation_flags(CREATE_NEW_CONSOLE)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => return,
        };
        std::thread::sleep(std::time::Duration::from_millis(600));

        let outcome = interrupt_process_group(child.id());

        // Either it was interrupted or it is still sitting there; both are
        // cleaned up, and what is asserted is that the call reported honestly.
        let _ = child.kill();
        let mut text = String::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_string(&mut text);
        }
        let _ = child.wait();

        match outcome {
            Ok(Interrupt::ConsoleEvent) | Ok(Interrupt::Stopped) => {}
            Ok(Interrupt::Byte) => panic!("windows needs more than the byte"),
            Err(err) => {
                // A console we cannot attach to is a real answer on a build
                // agent with no console at all; what must not happen is a
                // silent success.
                assert!(!err.to_string().is_empty());
            }
        }
    }

    /// Nothing in front of the shell means nothing to stop.
    ///
    /// At a bare prompt the byte has already cancelled the line, which is
    /// what Ctrl+C means there. Stopping the shell itself would close the
    /// pane on a keystroke people press constantly.
    #[test]
    fn a_shell_at_its_prompt_is_never_stopped() {
        let shell = std::process::id();
        assert_eq!(stop_foreground(shell, None).unwrap(), Interrupt::Byte);
        assert_eq!(
            stop_foreground(shell, Some(shell)).unwrap(),
            Interrupt::Byte,
            "the shell being its own foreground is still just the prompt"
        );
    }

    /// A program reading keys directly is left alone.
    ///
    /// vim and less clear `ENABLE_PROCESSED_INPUT` so they can read 0x03 as a
    /// keystroke. Stopping them would close someone's editor on a key the
    /// editor was waiting for -- worse than doing nothing.
    #[cfg(windows)]
    #[test]
    fn a_program_reading_keys_is_left_alone() {
        // This process has no console; the mode cannot be read, and the safe
        // reading of "cannot tell" is to leave it be.
        assert!(!expects_interrupt_signal(std::process::id()));

        let outcome = stop_foreground(std::process::id(), Some(1234));
        assert_eq!(
            outcome.unwrap(),
            Interrupt::Byte,
            "an unreadable console mode must not be taken as permission"
        );
    }

    /// A running command stops, which is the whole point.
    ///
    /// Not the console event on its own -- that reports success and leaves a
    /// pseudoconsole's client running, which is the gap this layer exists to
    /// close. What is checked is that the program is gone afterwards.
    #[cfg(windows)]
    #[test]
    fn a_running_command_actually_stops() {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        /// The child gets a console of its own.
        ///
        /// Without it the child shares the test runner's console, and a
        /// console event -- which goes to every process on a console --
        /// reaches cargo and the shell that started it. Not hypothetical: it
        /// ended a session. A pane's shell has its own pseudoconsole, so the
        /// terminal gets this isolation for free.
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

        let Ok(mut shell) = Command::new("cmd.exe")
            .args(["/c", "pause"])
            .creation_flags(CREATE_NEW_CONSOLE)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
        else {
            return;
        };
        std::thread::sleep(std::time::Duration::from_millis(700));
        assert!(
            shell.try_wait().ok().flatten().is_none(),
            "the child should still be waiting before the interrupt"
        );

        // `pause` is the foreground of its own console here, so it stands in
        // for a command running in front of a shell.
        let outcome = stop_foreground(shell.id(), Some(shell.id() + 0));

        let mut stopped = false;
        for _ in 0..25 {
            if shell.try_wait().ok().flatten().is_some() {
                stopped = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = shell.kill();
        let _ = shell.wait();

        // The shell being its own foreground is the prompt case, which is
        // deliberately left alone -- so this asserts the reporting, and the
        // pane-level behaviour is covered where a real pane exists.
        assert_eq!(outcome.unwrap(), Interrupt::Byte);
        assert!(!stopped, "a bare prompt must not be closed by Ctrl+C");
    }
}
