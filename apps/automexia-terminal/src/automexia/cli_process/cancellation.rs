//! Signal ownership is scoped to explicit CLI work, never GUI event handling.
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

static ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static CANCELLED: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static SHELL_OWNS_CTRL_C: AtomicBool = AtomicBool::new(false);

pub(crate) struct Cancellation {
    #[cfg(unix)]
    flag: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    #[cfg(unix)]
    signals: Vec<signal_hook::SigId>,
}

impl Cancellation {
    pub fn install() -> io::Result<Self> {
        Self::install_mode(false)
    }

    /// A native shell shares the Windows console with its helper. Its Ctrl+C
    /// event interrupts the current command; it must not retire the whole shell.
    pub(crate) fn install_interactive_shell() -> io::Result<Self> {
        Self::install_mode(true)
    }

    fn install_mode(shell_owns_ctrl_c: bool) -> io::Result<Self> {
        if ACTIVE.swap(true, Ordering::AcqRel) {
            return Err(io::Error::other(
                "a local command already owns cancellation",
            ));
        }
        #[cfg(windows)]
        SHELL_OWNS_CTRL_C.store(shell_owns_ctrl_c, Ordering::Release);
        #[cfg(not(windows))]
        let _ = shell_owns_ctrl_c;
        let result = Self::register();
        if result.is_err() {
            ACTIVE.store(false, Ordering::Release);
        }
        result
    }

    #[cfg(windows)]
    fn register() -> io::Result<Self> {
        CANCELLED.store(false, Ordering::Release);
        if SHELL_OWNS_CTRL_C.load(Ordering::Acquire)
            // A new-process-group parent can pass an inherited ignore bit to
            // this console helper. Restore native delivery before spawning the
            // interactive shell; never change the parent's handler or modes.
            && unsafe {
                windows_sys::Win32::System::Console::SetConsoleCtrlHandler(None, 0)
            } == 0
        {
            return Err(io::Error::other("native shell Ctrl+C delivery unavailable"));
        }
        // The callback only changes an atomic flag. Cleanup stays with the
        // command owner, outside the OS signal thread and within its deadline.
        if unsafe {
            windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(handle), 1)
        } == 0
        {
            return Err(io::Error::other("local command cancellation unavailable"));
        }
        Ok(Self {})
    }

    #[cfg(unix)]
    fn register() -> io::Result<Self> {
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut signals = Vec::new();
        for signal in [
            signal_hook::consts::SIGINT,
            signal_hook::consts::SIGTERM,
            signal_hook::consts::SIGHUP,
            signal_hook::consts::SIGQUIT,
        ] {
            match signal_hook::flag::register_usize(signal, flag.clone(), signal as usize)
            {
                Ok(id) => signals.push(id),
                Err(_) => {
                    for id in signals {
                        signal_hook::low_level::unregister(id);
                    }
                    return Err(io::Error::other(
                        "local command cancellation unavailable",
                    ));
                }
            }
        }
        Ok(Self { flag, signals })
    }

    pub fn cancelled(&self) -> bool {
        #[cfg(windows)]
        {
            CANCELLED.load(Ordering::Acquire)
        }
        #[cfg(unix)]
        {
            self.signal().is_some()
        }
    }

    /// The explicit interactive wrapper restores signal exit semantics only
    /// after retiring its child and closing the local metadata scope.
    #[cfg(unix)]
    pub(crate) fn signal(&self) -> Option<i32> {
        let signal = self.flag.load(Ordering::Acquire);
        if signal == 0 {
            None
        } else {
            i32::try_from(signal).ok()
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn handle(event: u32) -> i32 {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    if event == CTRL_C_EVENT && SHELL_OWNS_CTRL_C.load(Ordering::Acquire) {
        // Each console process receives this event. Acknowledge our copy while
        // the native shell handles its own; do not install an inherited ignore
        // flag or synthesize/rebroadcast an extra signal.
        return 1;
    }
    if matches!(event, CTRL_C_EVENT | CTRL_BREAK_EVENT) {
        CANCELLED.store(true, Ordering::Release);
        1
    } else {
        0
    }
}

impl Drop for Cancellation {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(handle), 0);
        }
        #[cfg(unix)]
        for id in self.signals.drain(..) {
            signal_hook::low_level::unregister(id);
        }
        ACTIVE.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn amx_process_cancellation_has_one_scoped_owner_and_releases_capacity() {
        for _ in 0..8 {
            let guard = Cancellation::install().unwrap();
            assert!(!guard.cancelled());
            assert!(Cancellation::install().is_err());
            #[cfg(windows)]
            {
                assert_eq!(unsafe { handle(99) }, 0);
                assert!(!guard.cancelled());
                assert_eq!(
                    unsafe { handle(windows_sys::Win32::System::Console::CTRL_C_EVENT) },
                    1
                );
                assert!(guard.cancelled());
            }
            #[cfg(unix)]
            {
                for signal in [
                    signal_hook::consts::SIGINT,
                    signal_hook::consts::SIGTERM,
                    signal_hook::consts::SIGHUP,
                    signal_hook::consts::SIGQUIT,
                ] {
                    signal_hook::low_level::raise(signal).unwrap();
                    assert!(guard.cancelled());
                    assert_eq!(guard.signal(), Some(signal));
                }
            }
            drop(guard);
            #[cfg(windows)]
            {
                use windows_sys::Win32::System::Console::{
                    CTRL_BREAK_EVENT, CTRL_C_EVENT,
                };
                let shell = Cancellation::install_interactive_shell().unwrap();
                assert!(Cancellation::install().is_err());
                assert_eq!(unsafe { handle(CTRL_C_EVENT) }, 1);
                assert!(!shell.cancelled(), "Ctrl+C belongs to the native shell");
                assert_eq!(unsafe { handle(CTRL_BREAK_EVENT) }, 1);
                assert!(shell.cancelled(), "explicit Ctrl+Break retires the helper");
            }
        }
    }
}
