//! Signal ownership is scoped to explicit CLI work, never GUI event handling.
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

static ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static CANCELLED: AtomicBool = AtomicBool::new(false);

pub(crate) struct Cancellation {
    #[cfg(unix)]
    flag: std::sync::Arc<AtomicBool>,
    #[cfg(unix)]
    signals: Vec<signal_hook::SigId>,
}

impl Cancellation {
    pub fn install() -> io::Result<Self> {
        if ACTIVE.swap(true, Ordering::AcqRel) {
            return Err(io::Error::other(
                "a local command already owns cancellation",
            ));
        }
        let result = Self::register();
        if result.is_err() {
            ACTIVE.store(false, Ordering::Release);
        }
        result
    }

    #[cfg(windows)]
    fn register() -> io::Result<Self> {
        CANCELLED.store(false, Ordering::Release);
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
        let flag = std::sync::Arc::new(AtomicBool::new(false));
        let mut signals = Vec::new();
        for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
            match signal_hook::flag::register(signal, flag.clone()) {
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
            self.flag.load(Ordering::Acquire)
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn handle(event: u32) -> i32 {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
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
                signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
                assert!(guard.cancelled());
            }
            drop(guard);
        }
    }
}
