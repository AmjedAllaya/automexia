use crate::Winsize;
use std::io::{Error, Result};
use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle};
use std::{mem, ptr};
use tracing::*;

use crate::windows::pipes::{EventedAnonRead, EventedAnonWrite};

use windows_sys::core::{HRESULT, PWSTR};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, S_OK};
use windows_sys::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
    JobObjectExtendedLimitInformation, QueryInformationJobObject,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::{s, w};

use windows_sys::Win32::System::Threading::{
    CreateProcessW, InitializeProcThreadAttributeList, ResumeThread, TerminateProcess,
    UpdateProcThreadAttribute, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
    EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    STARTUPINFOW,
};

use std::os::windows::ffi::OsStrExt;

use crate::windows::child::ChildExitWatcher;
use crate::windows::{cmdline, win32_string, Pty};

/// Load the pseudoconsole API from conpty.dll if possible, otherwise use the
/// standard Windows API.
///
/// The conpty.dll from the Windows Terminal project
/// supports loading OpenConsole.exe, which offers many improvements and
/// bugfixes compared to the standard conpty that ships with Windows.
///
/// The conpty.dll and OpenConsole.exe files will be searched in PATH and in
/// the directory where Rio's executable is located.
type CreatePseudoConsoleFn =
    unsafe extern "system" fn(COORD, HANDLE, HANDLE, u32, *mut HPCON) -> HRESULT;
type ResizePseudoConsoleFn = unsafe extern "system" fn(HPCON, COORD) -> HRESULT;
type ClosePseudoConsoleFn = unsafe extern "system" fn(HPCON);

struct ConptyApi {
    create: CreatePseudoConsoleFn,
    resize: ResizePseudoConsoleFn,
    close: ClosePseudoConsoleFn,
}

impl ConptyApi {
    fn new() -> Self {
        match Self::load_conpty() {
            Some(conpty) => {
                info!("Using conpty.dll for pseudoconsole");
                conpty
            }
            None => {
                // Cannot load conpty.dll - use the standard Windows API.
                info!("Using Windows API for pseudoconsole");
                Self {
                    create: CreatePseudoConsole,
                    resize: ResizePseudoConsole,
                    close: ClosePseudoConsole,
                }
            }
        }
    }

    /// Try loading ConptyApi from conpty.dll library.
    fn load_conpty() -> Option<Self> {
        type LoadedFn = unsafe extern "system" fn() -> isize;
        unsafe {
            let hmodule = LoadLibraryW(w!("conpty.dll"));
            if hmodule.is_null() {
                return None;
            }
            let create_fn = GetProcAddress(hmodule, s!("CreatePseudoConsole"))?;
            let resize_fn = GetProcAddress(hmodule, s!("ResizePseudoConsole"))?;
            let close_fn = GetProcAddress(hmodule, s!("ClosePseudoConsole"))?;

            Some(Self {
                create: mem::transmute::<LoadedFn, CreatePseudoConsoleFn>(create_fn),
                resize: mem::transmute::<LoadedFn, ResizePseudoConsoleFn>(resize_fn),
                close: mem::transmute::<LoadedFn, ClosePseudoConsoleFn>(close_fn),
            })
        }
    }
}

/// RAII Pseudoconsole.
pub struct Conpty {
    pub handle: HPCON,
    api: ConptyApi,
    managed_job: Option<OwnedHandle>,
}

impl Drop for Conpty {
    fn drop(&mut self) {
        // Kill a managed process tree before closing ConPTY; otherwise
        // ClosePseudoConsole can block while a descendant still owns conout.
        drop(self.managed_job.take());

        // XXX: This will block until the conout pipe is drained. Will cause a deadlock if the
        // conout pipe has already been dropped by this point.
        //
        // See PR #3084 and https://docs.microsoft.com/en-us/windows/console/closepseudoconsole.
        unsafe { (self.api.close)(self.handle) }
    }
}

// The ConPTY handle can be sent between threads.
unsafe impl Send for Conpty {}

/// Builds a `CREATE_UNICODE_ENVIRONMENT` block from the current process
/// environment plus `extra_env` (which overrides inherited variables of the
/// same name): NUL-terminated `KEY=VALUE` UTF-16 entries, with a trailing NUL.
fn environment_block(
    extra_env: Vec<(String, String)>,
    inherit_environment: bool,
) -> Vec<u16> {
    let mut vars: Vec<(std::ffi::OsString, std::ffi::OsString)> = if inherit_environment {
        std::env::vars_os().collect()
    } else {
        Vec::new()
    };
    for (key, value) in extra_env {
        let key = std::ffi::OsString::from(key);
        vars.retain(|(existing, _)| !existing.eq_ignore_ascii_case(&key));
        vars.push((key, value.into()));
    }
    vars.sort_by(|left, right| {
        left.0
            .to_string_lossy()
            .to_ascii_lowercase()
            .cmp(&right.0.to_string_lossy().to_ascii_lowercase())
    });

    let mut block = Vec::new();
    for (key, value) in vars {
        block.extend(key.encode_wide());
        block.push('=' as u16);
        block.extend(value.encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}

#[allow(clippy::too_many_arguments)]
pub fn new(
    application_name: Option<&str>,
    command_line: Option<&str>,
    working_directory: &Option<String>,
    env: Option<Vec<(String, String)>>,
    inherit_environment: bool,
    managed_tree: bool,
    columns: u16,
    rows: u16,
) -> Result<Pty> {
    let api = ConptyApi::new();
    let mut pty_handle: HPCON = 0;

    // Passing 0 as the size parameter allows the "system default" buffer
    // size to be used. There may be small performance and memory advantages
    // to be gained by tuning this in the future, but it's likely a reasonable
    // start point.
    let (conout, conout_pty_handle) = miow::pipe::anonymous(0)?;
    let (conin_pty_handle, conin) = miow::pipe::anonymous(0)?;

    let winsize = Winsize {
        ws_row: rows as libc::c_ushort,
        ws_col: columns as libc::c_ushort,
        ws_xpixel: 0 as libc::c_ushort,
        ws_ypixel: 0 as libc::c_ushort,
    };

    // Create the Pseudo Console, using the pipes.
    let result = unsafe {
        (api.create)(
            winsize.into(),
            conin_pty_handle.into_raw_handle() as HANDLE,
            conout_pty_handle.into_raw_handle() as HANDLE,
            0,
            &mut pty_handle as *mut _,
        )
    };

    assert_eq!(result, S_OK);

    let mut success;

    // Prepare child process startup info.

    let mut size: usize = 0;

    let mut startup_info_ex: STARTUPINFOEXW = unsafe { mem::zeroed() };

    startup_info_ex.StartupInfo.lpTitle = std::ptr::null_mut() as PWSTR;

    startup_info_ex.StartupInfo.cb = mem::size_of::<STARTUPINFOEXW>() as u32;

    // Setting this flag but leaving all the handles as default (null) ensures the
    // PTY process does not inherit any handles from this Rio process.
    startup_info_ex.StartupInfo.dwFlags |= STARTF_USESTDHANDLES;

    // Create the appropriately sized thread attribute list.
    unsafe {
        let failure = InitializeProcThreadAttributeList(
            ptr::null_mut(),
            1,
            0,
            &mut size as *mut usize,
        ) > 0;

        // This call was expected to return false.
        if failure {
            return Err(Error::last_os_error());
        }
    }

    let mut attr_list: Box<[u8]> = vec![0; size].into_boxed_slice();

    // Set startup info's attribute list & initialize it
    //
    // Lint failure is spurious; it's because winapi's definition of PROC_THREAD_ATTRIBUTE_LIST
    // implies it is one pointer in size (32 or 64 bits) but really this is just a dummy value.
    // Casting a *mut u8 (pointer to 8 bit type) might therefore not be aligned correctly in
    // the compiler's eyes.
    #[allow(clippy::cast_ptr_alignment)]
    {
        startup_info_ex.lpAttributeList = attr_list.as_mut_ptr() as _;
    }

    unsafe {
        success = InitializeProcThreadAttributeList(
            startup_info_ex.lpAttributeList,
            1,
            0,
            &mut size as *mut usize,
        ) > 0;

        if !success {
            return Err(Error::last_os_error());
        }
    }

    // Set thread attribute list's Pseudo Console to the specified ConPTY.
    unsafe {
        success = UpdateProcThreadAttribute(
            startup_info_ex.lpAttributeList,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            pty_handle as *mut std::ffi::c_void,
            mem::size_of::<HPCON>(),
            ptr::null_mut(),
            ptr::null_mut(),
        ) > 0;

        if !success {
            return Err(Error::last_os_error());
        }
    }

    let application_name = application_name.map(win32_string);
    let mut command_line = win32_string(&cmdline(command_line));
    let cwd = working_directory.as_ref().map(win32_string);
    // Generic terminals preserve their historic inherited environment. Exact
    // managed launches always receive a complete application-owned block.
    let mut env_block = match env {
        Some(environment) => Some(environment_block(environment, inherit_environment)),
        None if !inherit_environment => Some(environment_block(Vec::new(), false)),
        None => None,
    };
    let managed_job = managed_tree.then(create_managed_job).transpose()?;

    let mut proc_info: PROCESS_INFORMATION = unsafe { mem::zeroed() };
    unsafe {
        success = CreateProcessW(
            application_name
                .as_ref()
                .map_or_else(ptr::null, |value| value.as_ptr()),
            command_line.as_mut_ptr() as PWSTR,
            ptr::null_mut(),
            ptr::null_mut(),
            false as i32,
            match env_block {
                Some(_) => {
                    EXTENDED_STARTUPINFO_PRESENT
                        | CREATE_UNICODE_ENVIRONMENT
                        | if managed_tree { CREATE_SUSPENDED } else { 0 }
                }
                None => {
                    EXTENDED_STARTUPINFO_PRESENT
                        | if managed_tree { CREATE_SUSPENDED } else { 0 }
                }
            },
            match env_block.as_mut() {
                Some(block) => block.as_mut_ptr() as *mut std::ffi::c_void,
                None => ptr::null_mut(),
            },
            cwd.as_ref().map_or_else(ptr::null, |s| s.as_ptr()),
            &mut startup_info_ex.StartupInfo as *mut STARTUPINFOW,
            &mut proc_info as *mut PROCESS_INFORMATION,
        ) > 0;

        if !success {
            return Err(Error::last_os_error());
        }
    }

    if let Some(job) = managed_job.as_ref() {
        let assigned =
            unsafe { AssignProcessToJobObject(job.as_raw_handle(), proc_info.hProcess) };
        if assigned == 0 {
            let error = Error::last_os_error();
            unsafe {
                TerminateProcess(proc_info.hProcess, 1);
                CloseHandle(proc_info.hThread);
                CloseHandle(proc_info.hProcess);
            }
            return Err(error);
        }
        if unsafe { ResumeThread(proc_info.hThread) } == u32::MAX {
            let error = Error::last_os_error();
            unsafe {
                TerminateProcess(proc_info.hProcess, 1);
                CloseHandle(proc_info.hThread);
                CloseHandle(proc_info.hProcess);
            }
            return Err(error);
        }
    }
    unsafe {
        CloseHandle(proc_info.hThread);
    }

    let conin = EventedAnonWrite::new(conin);
    let conout = EventedAnonRead::new(conout);

    let child_watcher = match ChildExitWatcher::new(proc_info.hProcess) {
        Ok(watcher) => watcher,
        Err(error) => {
            unsafe {
                TerminateProcess(proc_info.hProcess, 1);
                CloseHandle(proc_info.hProcess);
            }
            return Err(error);
        }
    };
    let conpty = Conpty {
        handle: pty_handle as HPCON,
        api,
        managed_job,
    };
    let managed = conpty.managed_job.is_some();

    Ok(Pty::new(conpty, conout, conin, child_watcher, managed))
}

impl Conpty {
    pub fn on_resize(&mut self, window_size: Winsize) {
        let result = unsafe { (self.api.resize)(self.handle, window_size.into()) };
        assert_eq!(result, S_OK);
    }

    pub fn terminate_managed_job(&self) -> Result<()> {
        let job = self.managed_job.as_ref().ok_or_else(|| {
            Error::new(
                std::io::ErrorKind::InvalidInput,
                "the PTY has no managed process-tree job",
            )
        })?;
        if unsafe { TerminateJobObject(job.as_raw_handle(), 1) } == 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn managed_job_is_empty(&self) -> Result<bool> {
        let job = self.managed_job.as_ref().ok_or_else(|| {
            Error::new(
                std::io::ErrorKind::InvalidInput,
                "the PTY has no managed process-tree job",
            )
        })?;
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        let queried = unsafe {
            QueryInformationJobObject(
                job.as_raw_handle(),
                JobObjectBasicAccountingInformation,
                ptr::from_mut(&mut accounting).cast(),
                mem::size_of_val(&accounting) as u32,
                ptr::null_mut(),
            )
        };
        if queried == 0 {
            Err(Error::last_os_error())
        } else {
            Ok(accounting.ActiveProcesses == 0)
        }
    }
}

impl From<Winsize> for COORD {
    fn from(window_size: Winsize) -> Self {
        let lines = window_size.ws_row;
        let columns = window_size.ws_col;
        COORD {
            X: columns as i16,
            Y: lines as i16,
        }
    }
}

fn create_managed_job() -> Result<OwnedHandle> {
    let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
    if handle.is_null() {
        return Err(Error::last_os_error());
    }
    // SAFETY: CreateJobObjectW returned a unique owned handle.
    let job = unsafe { OwnedHandle::from_raw_handle(handle) };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let configured = unsafe {
        SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const std::ffi::c_void,
            mem::size_of_val(&limits) as u32,
        )
    };
    if configured == 0 {
        return Err(Error::last_os_error());
    }
    Ok(job)
}
