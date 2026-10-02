//! A terminal lease for the existing enhanced CLI child owner, not a process owner.
//!
//! The child lives in its own process group. Terminal access moves only between
//! that pinned group and our original group; native SSH never uses this lease.
use std::{
    fs::{File, OpenOptions},
    io,
    os::fd::{AsRawFd, RawFd},
};

pub(super) struct Foreground {
    terminal: File,
    caller: libc::pid_t,
    child: Option<libc::pid_t>,
    original_modes: libc::termios,
    child_modes: Option<libc::termios>,
    pending_stop: Option<i32>,
    immediate: bool,
    claimed: bool,
    active: bool,
}

impl Foreground {
    pub(super) fn prepare() -> io::Result<Option<Self>> {
        // A forced remote PTY does not make redirected local input interactive.
        // In that case defer terminal access until SSH actually asks /dev/tty
        // for input (for example password authentication).
        // SAFETY: isatty only inspects the inherited standard-input descriptor.
        let immediate = unsafe { libc::isatty(libc::STDIN_FILENO) } == 1;
        let terminal = match OpenOptions::new().read(true).write(true).open("/dev/tty") {
            Ok(terminal) => terminal,
            Err(error)
                if matches!(
                    error.raw_os_error(),
                    Some(libc::ENOENT | libc::ENXIO | libc::ENODEV | libc::ENOTTY)
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        // SAFETY: getpgrp has no arguments or memory requirements.
        let caller = unsafe { libc::getpgrp() };
        if immediate && foreground(terminal.as_raw_fd())? != caller {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "enhanced interactive SSH requires the foreground terminal",
            ));
        }
        let original_modes = modes(terminal.as_raw_fd())?;
        Ok(Some(Self {
            terminal,
            caller,
            child: None,
            original_modes,
            child_modes: None,
            pending_stop: None,
            immediate,
            claimed: false,
            active: true,
        }))
    }

    /// The existing child owner must retain this PID until cleanup finishes.
    /// Handoff occurs after a successful spawn, so failed exec cannot leave an
    /// unknown group owning the terminal. Resume resolves an initial SIGTTIN or
    /// SIGTTOU stop caused by a child that accessed the tty before this handoff.
    pub(super) fn attach(&mut self, child: u32) -> io::Result<()> {
        let child = libc::pid_t::try_from(child)
            .ok()
            .filter(|pid| *pid > 1)
            .ok_or_else(|| io::Error::other("invalid interactive child identity"))?;
        self.child = Some(child);
        if !self.immediate {
            return Ok(());
        }
        if foreground(self.fd())? != self.caller {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "interactive terminal foreground changed before handoff",
            ));
        }
        let stop = stopped(child)?;
        self.original_modes = modes(self.fd())?;
        set_foreground(self.fd(), child)?;
        self.claimed = true;
        if matches!(stop, None | Some(libc::SIGTTIN | libc::SIGTTOU)) {
            signal_group(child, libc::SIGCONT)
        } else {
            // A deliberate suspend before handoff is not a startup I/O race.
            // Preserve the consumed event so the next poll reflects it normally.
            self.pending_stop = stop;
            Ok(())
        }
    }

    /// Observe only stop events; exit status and reaping remain with OwnedChild.
    /// Reflect a stopped SSH job to the outer shell, preserving tty modes for
    /// `fg`. A `bg` continuation never steals another foreground job's terminal.
    pub(super) fn poll_stopped(
        &mut self,
        cancelled: &impl Fn() -> bool,
    ) -> io::Result<()> {
        let Some(child) = self.child else {
            return Ok(());
        };
        let stop = match self.pending_stop.take() {
            Some(stop) => Some(stop),
            None => stopped(child)?,
        };
        let Some(stop) = stop else {
            return Ok(());
        };
        if matches!(stop, libc::SIGTTIN | libc::SIGTTOU)
            && foreground(self.fd())? == self.caller
        {
            // The outer shell may have already processed `fg` while we were
            // observing the previous background read stop. Do not reflect that
            // stale I/O stop as a new suspension requiring another `fg`.
            if !self.claimed {
                self.original_modes = modes(self.fd())?;
            }
            set_foreground(self.fd(), child)?;
            self.claimed = true;
            if let Some(child_modes) = &self.child_modes {
                set_modes(self.fd(), child_modes)?;
            }
            return signal_group(child, libc::SIGCONT);
        }
        // OpenSSH's local suspend escape may stop only its leader. Suspend the
        // same owned group so proxy children cannot keep running during a pause.
        signal_group(child, libc::SIGSTOP)?;
        if foreground(self.fd())? == child {
            self.child_modes = Some(modes(self.fd())?);
            set_foreground(self.fd(), self.caller)?;
            set_modes(self.fd(), &self.original_modes)?;
        }
        if cancelled() {
            return Err(cancelled_error());
        }
        // SAFETY: only this wrapper stops. The outer shell owns its job group;
        // do not stop unrelated pipeline peers or the user's parent shell.
        if unsafe { libc::raise(libc::SIGSTOP) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if cancelled() {
            return Err(cancelled_error());
        }
        let current = foreground(self.fd())?;
        if current == self.caller || current == child {
            if !self.claimed {
                self.original_modes = modes(self.fd())?;
            }
            set_foreground(self.fd(), child)?;
            self.claimed = true;
            if let Some(child_modes) = &self.child_modes {
                set_modes(self.fd(), child_modes)?;
            }
        }
        // Foreground assignment and saved modes must precede continuation.
        // In the background a reader will naturally stop with SIGTTIN again.
        signal_group(child, libc::SIGCONT)
    }

    /// Called after owned group cleanup, before the metadata scope is closed.
    /// A different foreground job may legitimately own the tty after `bg`.
    pub(super) fn restore(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        if !self.claimed {
            self.active = false;
            return Ok(());
        }
        let current = foreground(self.fd())?;
        if current == self.caller || self.child == Some(current) {
            set_foreground(self.fd(), self.caller)?;
            set_modes(self.fd(), &self.original_modes)?;
        }
        self.active = false;
        Ok(())
    }

    fn fd(&self) -> RawFd {
        self.terminal.as_raw_fd()
    }
}

impl Drop for Foreground {
    fn drop(&mut self) {
        // Explicit restore reports failures. Drop covers spawn/early errors.
        let _ = self.restore();
    }
}

fn foreground(fd: RawFd) -> io::Result<libc::pid_t> {
    // SAFETY: fd is a live owned terminal descriptor; no borrowed pointers.
    let group = unsafe { libc::tcgetpgrp(fd) };
    if group < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(group)
    }
}

fn modes(fd: RawFd) -> io::Result<libc::termios> {
    // SAFETY: zeroed termios is only an output buffer; tcgetattr initializes it.
    let mut modes = unsafe { std::mem::zeroed() };
    // SAFETY: fd is live and modes points to a correctly sized writable buffer.
    if unsafe { libc::tcgetattr(fd, &mut modes) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(modes)
    }
}

fn set_foreground(fd: RawFd, group: libc::pid_t) -> io::Result<()> {
    with_ttou_blocked(|| {
        // SAFETY: fd is live and group is the caller or its pinned child group.
        if unsafe { libc::tcsetpgrp(fd, group) } < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    })
}

fn set_modes(fd: RawFd, modes: &libc::termios) -> io::Result<()> {
    with_ttou_blocked(|| {
        // SAFETY: the initialized termios reference stays live during the call.
        // TCSANOW avoids an unbounded output drain on cancellation or hangup.
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, modes) } < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    })
}

fn with_ttou_blocked(action: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
    // SAFETY: both signal sets are stack output buffers initialized before use.
    let mut mask = unsafe { std::mem::zeroed() };
    let mut previous = unsafe { std::mem::zeroed() };
    // SAFETY: signal-set pointers are valid. pthread_sigmask changes only this
    // thread, only during tty mutation; no ignored signal disposition leaks to
    // subsequently spawned children or other application threads.
    let status = unsafe {
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGTTOU);
        libc::pthread_sigmask(libc::SIG_BLOCK, &mask, &mut previous)
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status));
    }
    let result = action();
    // SAFETY: previous was initialized by the successful pthread_sigmask call.
    let restored = unsafe {
        libc::pthread_sigmask(libc::SIG_SETMASK, &previous, std::ptr::null_mut())
    };
    if restored != 0 {
        return Err(io::Error::from_raw_os_error(restored));
    }
    result
}

fn stopped(child: libc::pid_t) -> io::Result<Option<i32>> {
    // SAFETY: siginfo is an output buffer; WSTOPPED alone never consumes an exit
    // status. The existing process owner keeps the child PID pinned throughout.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            child as libc::id_t,
            &mut info,
            libc::WSTOPPED | libc::WNOHANG,
        )
    };
    if result < 0 {
        let error = io::Error::last_os_error();
        // Linux can report ECHILD when the pinned child has exited and is no
        // longer eligible for WSTOPPED-only observation. Its exit remains for
        // the existing owner's WEXITED query; an actually lost child is still
        // rejected there, rather than having its status invented here.
        if error.kind() == io::ErrorKind::Interrupted
            || error.raw_os_error() == Some(libc::ECHILD)
        {
            return Ok(None);
        }
        return Err(error);
    }
    // SAFETY: successful waitid initialized the siginfo union for this event.
    if unsafe { info.si_pid() } == child && info.si_code == libc::CLD_STOPPED {
        // SAFETY: CLD_STOPPED selects the initialized child-status union member.
        Ok(Some(unsafe { info.si_status() }))
    } else {
        Ok(None)
    }
}

fn signal_group(group: libc::pid_t, signal: i32) -> io::Result<()> {
    // SAFETY: group is a validated positive PID retained by the child owner;
    // negation targets only that owned process group, never the caller's group.
    if unsafe { libc::kill(-group, signal) } < 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error);
        }
    }
    Ok(())
}

fn cancelled_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::Interrupted,
        "interactive command was cancelled",
    )
}
