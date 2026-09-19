//! One native handler boundary. Callers own destination policy and explicit consent.
use std::io;

pub fn open(target: &str) -> io::Result<()> {
    if target.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid destination",
        ));
    }
    #[cfg(windows)]
    {
        with_com_apartment(|| open_windows(target))
    }
    #[cfg(not(windows))]
    {
        use std::process::{Command, Stdio};
        let program = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        // This one-shot process hands lifetime ownership to the desktop.
        Command::new(program)
            .arg(target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|_| {
                io::Error::other(
                    "could not start the default browser; check the desktop URL handler",
                )
            })
    }
}

/// Directory callers validate filesystem type before this explicit handoff.
/// Windows uses the folder-specific verb, not an executable's default action.
pub(crate) fn open_directory(target: &str) -> io::Result<()> {
    if target.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid directory",
        ));
    }
    #[cfg(windows)]
    {
        with_com_apartment(|| open_windows_verb(target, "explore"))
    }
    #[cfg(not(windows))]
    {
        directory_command(target, cfg!(target_os = "macos"))
            .spawn()
            .map(|_| ())
            .map_err(|_| io::Error::other("could not start the desktop directory handler; check your file manager and desktop session"))
    }
}

#[cfg(any(not(windows), test))]
fn directory_command(target: &str, macos: bool) -> std::process::Command {
    use std::process::{Command, Stdio};
    let mut command = Command::new(if macos { "/usr/bin/open" } else { "xdg-open" });
    if macos {
        // Application bundles are directories too. Reveal in Finder rather than
        // invoking an associated application's default action for a package.
        command.arg("-R");
    }
    command
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[cfg(windows)]
fn with_com_apartment<T>(operation: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    use windows_sys::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
    };
    struct ApartmentGuard;
    impl Drop for ApartmentGuard {
        fn drop(&mut self) {
            // Paired only with a successful initialization on this same thread.
            unsafe { CoUninitialize() };
        }
    }
    // Shell association handlers can require COM even without a GUI event loop.
    let result = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    let _guard = if result >= 0 {
        Some(ApartmentGuard)
    } else if result == windows_sys::Win32::Foundation::RPC_E_CHANGED_MODE {
        // An existing GUI/runtime apartment retains its original ownership.
        None
    } else {
        return Err(io::Error::other("could not initialize the desktop handler"));
    };
    operation()
}

#[cfg(windows)]
fn open_windows(target: &str) -> io::Result<()> {
    open_windows_verb(target, "open")
}

#[cfg(windows)]
fn open_windows_verb(target: &str, verb: &str) -> io::Result<()> {
    let target: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
    let operation: Vec<u16> = verb.encode_utf16().chain(Some(0)).collect();
    // Both buffers are NUL-terminated and live through this synchronous
    // native call. The destination is never interpreted as a shell command.
    let result = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(io::Error::other(
            "default desktop handler failed; check the requested application association",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod directory_tests {
    #[test]
    fn amx_open_directory_commands_preserve_literal_targets_and_reveal_packages() {
        for (macos, program, expected) in [
            (false, "xdg-open", vec!["/fixture/a & café.app"]),
            (true, "/usr/bin/open", vec!["-R", "/fixture/a & café.app"]),
        ] {
            let command = super::directory_command("/fixture/a & café.app", macos);
            assert_eq!(command.get_program(), program);
            assert_eq!(command.get_args().collect::<Vec<_>>(), expected);
        }
        assert!(super::open_directory("invalid\0directory").is_err());
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED,
    };

    #[test]
    fn google_command_native_apartment_balances_success_failure_and_existing_mode() {
        // A fresh native thread provides an independent initialization-count
        // oracle without launching a browser or modifying system associations.
        std::thread::spawn(|| {
            for fail in [false, true] {
                let result = with_com_apartment(|| {
                    assert_eq!(
                        unsafe {
                            CoInitializeEx(
                                std::ptr::null(),
                                COINIT_APARTMENTTHREADED as u32,
                            )
                        },
                        1
                    );
                    unsafe { CoUninitialize() };
                    with_com_apartment(|| Ok(()))?;
                    if fail {
                        Err(io::Error::other("fixture failure"))
                    } else {
                        Ok(())
                    }
                });
                assert_eq!(result.is_err(), fail);
                assert_eq!(
                    unsafe {
                        CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32)
                    },
                    0
                );
                with_com_apartment(|| Ok(())).unwrap();
                assert_eq!(
                    unsafe {
                        CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32)
                    },
                    1
                );
                unsafe {
                    CoUninitialize();
                    CoUninitialize();
                }
            }
            assert!(open("invalid\0destination").is_err());
        })
        .join()
        .unwrap();
    }
}
