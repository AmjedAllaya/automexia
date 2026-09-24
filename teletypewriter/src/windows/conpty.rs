use crate::Winsize;
use std::io::{Error, Result};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::{mem, ptr};
use tracing::*;

use crate::windows::pipes::{EventedAnonRead, EventedAnonWrite};

use windows_sys::core::{HRESULT, PWSTR};
use windows_sys::s;
use windows_sys::Win32::Foundation::{CloseHandle, FreeLibrary, HANDLE, HMODULE, S_OK};
use windows_sys::Win32::Globalization::{
    CompareStringOrdinal, CSTR_EQUAL, CSTR_GREATER_THAN, CSTR_LESS_THAN,
};
use windows_sys::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
    JobObjectExtendedLimitInformation, QueryInformationJobObject,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LOAD_LIBRARY_SEARCH_SYSTEM32,
};

use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
    ResumeThread, TerminateProcess, UpdateProcThreadAttribute, CREATE_SUSPENDED,
    CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    STARTUPINFOW,
};

use std::os::windows::ffi::OsStrExt;

use crate::windows::child::ChildExitWatcher;
use crate::windows::{cmdline, win32_string, Pty};

#[cfg(test)]
#[path = "conpty_loader_tests.rs"]
mod loader_tests;

/// Load the pseudoconsole API from conpty.dll if possible, otherwise use the
/// standard Windows API.
///
/// The conpty.dll from the Windows Terminal project
/// supports loading OpenConsole.exe, which offers many improvements and
/// bugfixes compared to the standard conpty that ships with Windows.
///
/// Only the application-sibling conpty.dll is considered. Its installation
/// directory must be trusted; dependency lookup excludes the current directory
/// and PATH. The DLL itself owns discovery of its optional OpenConsole host.
type CreatePseudoConsoleFn =
    unsafe extern "system" fn(COORD, HANDLE, HANDLE, u32, *mut HPCON) -> HRESULT;
type ResizePseudoConsoleFn = unsafe extern "system" fn(HPCON, COORD) -> HRESULT;
type ClosePseudoConsoleFn = unsafe extern "system" fn(HPCON);

// Own exactly the module reference acquired by one successful LoadLibraryExW.
// Never construct this guard from a borrowed GetModuleHandleW result.
struct ConptyLibrary(HMODULE);

impl Drop for ConptyLibrary {
    fn drop(&mut self) {
        // SAFETY: This non-null handle owns one loader reference. API pointers
        // never escape ConptyApi, and Conpty closes before its API is dropped.
        if unsafe { FreeLibrary(self.0) } == 0 {
            warn!("Bundled ConPTY module release failed");
        }
    }
}

// Build directly from the native path so Windows code units are never decoded
// lossily. Preflight the complete result before allocating its UTF-16 buffer.
fn bundled_conpty_path(executable: &std::path::Path) -> Result<Vec<u16>> {
    let invalid = || {
        Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid bundled ConPTY path",
        )
    };
    if !executable.is_absolute() || executable.file_name().is_none() {
        return Err(invalid());
    }
    for (index, unit) in executable.as_os_str().encode_wide().enumerate() {
        if unit == 0 || index >= MAX_NATIVE_STRING_UNITS - 1 {
            return Err(invalid());
        }
    }
    let parent = executable.parent().ok_or_else(invalid)?;
    let mut units = 0;
    let mut last = None;
    for unit in parent.as_os_str().encode_wide() {
        units += 1;
        if unit == 0 || units >= MAX_NATIVE_STRING_UNITS {
            return Err(invalid());
        }
        last = Some(unit);
    }
    let separator = !matches!(last, Some(0x5c | 0x2f));
    let total = units + usize::from(separator) + "conpty.dll".len() + 1;
    if total > MAX_NATIVE_STRING_UNITS {
        return Err(invalid());
    }
    let mut path = Vec::new();
    path.try_reserve_exact(total).map_err(|_| {
        Error::new(
            std::io::ErrorKind::OutOfMemory,
            "bundled ConPTY path allocation failed",
        )
    })?;
    path.extend(parent.as_os_str().encode_wide().map(|unit| {
        if unit == 0x2f {
            0x5c
        } else {
            unit
        }
    }));
    if separator {
        path.push(0x5c);
    }
    path.extend("conpty.dll".encode_utf16());
    path.push(0);
    Ok(path)
}

struct ConptyApi {
    create: CreatePseudoConsoleFn,
    resize: ResizePseudoConsoleFn,
    close: ClosePseudoConsoleFn,
    // Retained through every call, including Conpty::drop and early failures.
    _library: Option<ConptyLibrary>,
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
                    _library: None,
                }
            }
        }
    }

    /// Try the optional application-sibling DLL without ambient path searches.
    fn load_conpty() -> Option<Self> {
        type LoadedFn = unsafe extern "system" fn() -> isize;
        let executable = std::env::current_exe().ok()?;
        let path = bundled_conpty_path(&executable).ok()?;
        // SAFETY: The absolute, bounded, NUL-terminated path remains live for
        // this call. The reserved handle is null; search flags limit filesystem
        // dependency lookup to this DLL's directory and the system directory.
        let hmodule = unsafe {
            LoadLibraryExW(
                path.as_ptr(),
                ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if hmodule.is_null() {
            return None;
        }
        let library = ConptyLibrary(hmodule);
        // SAFETY: The owned reference keeps the module loaded while resolving
        // its documented ConPTY exports. These exact legacy export names use
        // the corresponding Windows ABI. The pointers remain private to this
        // API owner, which retains the guard; any missing export drops it.
        unsafe {
            let create_fn = GetProcAddress(library.0, s!("CreatePseudoConsole"))?;
            let resize_fn = GetProcAddress(library.0, s!("ResizePseudoConsole"))?;
            let close_fn = GetProcAddress(library.0, s!("ClosePseudoConsole"))?;
            Some(Self {
                create: mem::transmute::<LoadedFn, CreatePseudoConsoleFn>(create_fn),
                resize: mem::transmute::<LoadedFn, ResizePseudoConsoleFn>(resize_fn),
                close: mem::transmute::<LoadedFn, ClosePseudoConsoleFn>(close_fn),
                _library: Some(library),
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
        // SAFETY: This owner closes its live pseudoconsole once. Its API and
        // optional library guard remain live until after this call returns.
        unsafe { (self.api.close)(self.handle) }
    }
}

// SAFETY: The ConPTY handle can be sent between threads. Its API retains its
// own process-wide loader reference; moving it cannot release that reference
// or permit a call after the owning Conpty has been dropped.
unsafe impl Send for Conpty {}

// Independent library callers can bypass the configuration file's 4 MiB limit.
// Bound the native copy, including superseded entries, before allocation.
const MAX_ENVIRONMENT_UNITS: usize = 16 * 1024 * 1024;
const MAX_ENVIRONMENT_ENTRIES: usize = 65_536;
pub(super) const MAX_NATIVE_STRING_UNITS: usize = 32_767;

fn invalid_environment() -> Error {
    Error::new(
        std::io::ErrorKind::InvalidInput,
        "invalid Windows environment entry or limit",
    )
}

fn wide_environment_part(value: &std::ffi::OsStr, units: usize) -> Result<Vec<u16>> {
    let mut result = Vec::new();
    result.try_reserve_exact(units).map_err(|_| {
        Error::new(
            std::io::ErrorKind::OutOfMemory,
            "Windows environment allocation failed",
        )
    })?;
    result.extend(value.encode_wide());
    Ok(result)
}

fn compare_environment_names(left: &[u16], right: &[u16]) -> Result<std::cmp::Ordering> {
    let left_len = i32::try_from(left.len()).map_err(|_| invalid_environment())?;
    let right_len = i32::try_from(right.len()).map_err(|_| invalid_environment())?;
    // SAFETY: Both nonempty, owned UTF-16 buffers remain borrowed for this call;
    // their checked lengths describe initialized storage. Windows compares them
    // without retaining or mutating pointers, using its own ordinal case table.
    match unsafe {
        CompareStringOrdinal(left.as_ptr(), left_len, right.as_ptr(), right_len, 1)
    } {
        CSTR_LESS_THAN => Ok(std::cmp::Ordering::Less),
        CSTR_EQUAL => Ok(std::cmp::Ordering::Equal),
        CSTR_GREATER_THAN => Ok(std::cmp::Ordering::Greater),
        _ => Err(Error::other("Windows environment comparison failed")),
    }
}

// Preflight the entire explicit batch before taking an inherited snapshot or
// allocating any wide entry. Superseded entries still consume the same budget.
fn validate_extra_environment(extra_env: &[(String, String)]) -> Result<()> {
    if extra_env.len() > MAX_ENVIRONMENT_ENTRIES {
        return Err(invalid_environment());
    }
    let mut remaining = MAX_ENVIRONMENT_UNITS - 1; // Final block terminator.
    for (name, value) in extra_env {
        if name.is_empty()
            || name.len() > MAX_ENVIRONMENT_UNITS * 3
            || value.len() > MAX_ENVIRONMENT_UNITS * 3
        {
            return Err(invalid_environment());
        }
        remaining = remaining.checked_sub(2).ok_or_else(invalid_environment)?;
        for unit in name.encode_utf16() {
            remaining = remaining.checked_sub(1).ok_or_else(invalid_environment)?;
            if unit == 0 || unit == '=' as u16 {
                return Err(invalid_environment());
            }
        }
        for unit in value.encode_utf16() {
            remaining = remaining.checked_sub(1).ok_or_else(invalid_environment)?;
            if unit == 0 {
                return Err(invalid_environment());
            }
        }
    }
    Ok(())
}

/// Construct one bounded, sorted Unicode environment with last-override wins.
/// The empty block, like every nonempty block, has two terminating UTF-16 NULs.
fn environment_block(
    extra_env: Vec<(String, String)>,
    inherit_environment: bool,
) -> Result<Vec<u16>> {
    validate_extra_environment(&extra_env)?;
    let inherited = inherit_environment.then(std::env::vars_os);
    let entries = inherited.into_iter().flatten().chain(
        extra_env
            .into_iter()
            .map(|(name, value)| (name.into(), value.into())),
    );
    let mut encoded = Vec::new();
    let mut total = 1usize;
    for (name, value) in entries {
        if encoded.len() == MAX_ENVIRONMENT_ENTRIES {
            return Err(invalid_environment());
        }
        let name_units = name.encode_wide().take(MAX_ENVIRONMENT_UNITS + 1).count();
        let value_units = value.encode_wide().take(MAX_ENVIRONMENT_UNITS + 1).count();
        if name_units == 0 {
            return Err(invalid_environment());
        }
        total = total
            .checked_add(name_units)
            .and_then(|n| n.checked_add(value_units))
            .and_then(|n| n.checked_add(2))
            .filter(|n| *n <= MAX_ENVIRONMENT_UNITS)
            .ok_or_else(invalid_environment)?;
        encoded.try_reserve(1).map_err(|_| {
            Error::new(
                std::io::ErrorKind::OutOfMemory,
                "Windows environment allocation failed",
            )
        })?;
        encoded.push((
            wide_environment_part(&name, name_units)?,
            wide_environment_part(&value, value_units)?,
            encoded.len(),
        ));
    }
    let mut comparison_failed = false;
    encoded.sort_unstable_by(|left, right| {
        match compare_environment_names(&left.0, &right.0) {
            Ok(order) => order.then(left.2.cmp(&right.2)),
            Err(_) => {
                comparison_failed = true;
                std::cmp::Ordering::Equal
            }
        }
    });
    if comparison_failed {
        return Err(Error::other("Windows environment comparison failed"));
    }
    encoded.dedup_by(|later, earlier| {
        match compare_environment_names(&later.0, &earlier.0) {
            Ok(std::cmp::Ordering::Equal) => {
                std::mem::swap(later, earlier);
                true
            }
            Ok(_) => false,
            Err(_) => {
                comparison_failed = true;
                false
            }
        }
    });
    if comparison_failed {
        return Err(Error::other("Windows environment comparison failed"));
    }
    let mut block = Vec::new();
    block.try_reserve_exact(total.max(2)).map_err(|_| {
        Error::new(
            std::io::ErrorKind::OutOfMemory,
            "Windows environment allocation failed",
        )
    })?;
    for (name, value, _) in encoded {
        block.extend(name);
        block.push('=' as u16);
        block.extend(value);
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    Ok(block)
}

fn validate_native_string(value: &str) -> Result<()> {
    if value.len() > MAX_NATIVE_STRING_UNITS * 4
        || value.contains('\0')
        || value.encode_utf16().take(MAX_NATIVE_STRING_UNITS).count()
            == MAX_NATIVE_STRING_UNITS
    {
        return Err(Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid Windows launch string",
        ));
    }
    Ok(())
}

fn checked_coord(size: Winsize) -> Result<COORD> {
    if size.ws_col == 0
        || size.ws_row == 0
        || size.ws_col > i16::MAX as u16
        || size.ws_row > i16::MAX as u16
    {
        return Err(Error::new(
            std::io::ErrorKind::InvalidInput,
            "terminal dimensions must be between 1 and 32767",
        ));
    }
    Ok(size.into())
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
    let size = checked_coord(Winsize {
        ws_row: rows,
        ws_col: columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    })?;
    for value in [application_name, command_line, working_directory.as_deref()]
        .into_iter()
        .flatten()
    {
        validate_native_string(value)?;
    }
    // Validate and encode before loading an optional DLL or creating any PTY
    // resources. Ordinary sessions without overrides inherit unchanged.
    let env_block = match env {
        Some(environment) => Some(environment_block(environment, inherit_environment)?),
        None if !inherit_environment => Some(environment_block(Vec::new(), false)?),
        None => None,
    };
    let api = ConptyApi::new();
    let mut pty_handle: HPCON = 0;

    // Passing 0 as the size parameter allows the "system default" buffer
    // size to be used. There may be small performance and memory advantages
    // to be gained by tuning this in the future, but it's likely a reasonable
    // start point.
    let (conout, conout_pty_handle) = miow::pipe::anonymous(0)?;
    let (conin_pty_handle, conin) = miow::pipe::anonymous(0)?;

    // Create the Pseudo Console, using the pipes.
    let result = unsafe {
        (api.create)(
            size,
            conin_pty_handle.as_raw_handle() as HANDLE,
            conout_pty_handle.as_raw_handle() as HANDLE,
            0,
            &mut pty_handle as *mut _,
        )
    };

    if result != S_OK {
        return Err(Error::other(format!(
            "ConPTY creation failed (HRESULT {result:#010x})"
        )));
    }

    // Keep native output drained throughout attachment, including error cleanup.
    let conin = EventedAnonWrite::new(conin);
    let conout = EventedAnonRead::new(conout);
    let mut conpty = Conpty {
        handle: pty_handle,
        api,
        managed_job: None,
    };
    let child = attach_child(
        &mut conpty,
        application_name,
        command_line,
        working_directory,
        env_block,
        managed_tree,
    );
    // ConPTY borrows these endpoints and owns separate copies. Release ours
    // after attachment (including failure), so pipe closure remains detectable.
    drop((conin_pty_handle, conout_pty_handle));
    match child {
        Ok(child_watcher) => {
            let managed = conpty.managed_job.is_some();
            Ok(Pty::new(conpty, conout, conin, child_watcher, managed))
        }
        Err(error) => {
            // No VT consumer owns this failed startup. Reuse the normal drain
            // mechanism before the backend closes, then join the pipe workers.
            conout.discard_remaining();
            drop(conpty);
            Err(error)
        }
    }
}

struct StartupAttributes(*mut std::ffi::c_void);

impl Drop for StartupAttributes {
    fn drop(&mut self) {
        // The initialized list's backing allocation outlives this guard.
        unsafe { DeleteProcThreadAttributeList(self.0.cast()) }
    }
}

fn attach_child(
    conpty: &mut Conpty,
    application_name: Option<&str>,
    command_line: Option<&str>,
    working_directory: &Option<String>,
    mut env_block: Option<Vec<u16>>,
    managed_tree: bool,
) -> Result<ChildExitWatcher> {
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

    let _attributes = StartupAttributes(startup_info_ex.lpAttributeList.cast());

    // Set thread attribute list's Pseudo Console to the specified ConPTY.
    unsafe {
        success = UpdateProcThreadAttribute(
            startup_info_ex.lpAttributeList,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            conpty.handle as *mut std::ffi::c_void,
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
    conpty.managed_job = managed_tree.then(create_managed_job).transpose()?;

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

    if let Some(job) = conpty.managed_job.as_ref() {
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
    Ok(child_watcher)
}

impl Conpty {
    pub fn on_resize(&mut self, window_size: Winsize) -> Result<()> {
        let size = checked_coord(window_size)?;
        let result = unsafe { (self.api.resize)(self.handle, size) };
        if result < 0 {
            Err(Error::other(format!(
                "ConPTY resize failed (HRESULT {result:#010x})"
            )))
        } else {
            Ok(())
        }
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

// Preserve the legacy representation conversion for source compatibility. Native
// launch and resize must use checked_coord before this conversion.
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

#[cfg(test)]
mod resize_tests {
    use super::*;

    unsafe extern "system" fn create_stub(
        _: COORD,
        _: HANDLE,
        _: HANDLE,
        _: u32,
        _: *mut HPCON,
    ) -> HRESULT {
        S_OK
    }
    unsafe extern "system" fn close_stub(_: HPCON) {}
    unsafe extern "system" fn resize_stub(_: HPCON, size: COORD) -> HRESULT {
        if size.X == 79 {
            0x80070057u32 as i32
        } else {
            S_OK
        }
    }

    #[test]
    fn invalid_resize_geometry_never_reaches_a_successful_native_stub() {
        let mut conpty = Conpty {
            handle: 0,
            api: ConptyApi {
                create: create_stub,
                resize: resize_stub,
                close: close_stub,
                _library: None,
            },
            managed_job: None,
        };
        for (cols, rows) in [(0, 24), (80, 0), (32768, 24), (80, 65535)] {
            let error = conpty
                .on_resize(
                    crate::WinsizeBuilder {
                        cols,
                        rows,
                        width: 0,
                        height: 0,
                    }
                    .build(),
                )
                .unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn native_resize_failure_is_retryable_without_panicking() {
        // Inject only the native ABI result; exercise the production adapter
        // and its normal drop path without creating or closing a real handle.
        let mut conpty = Conpty {
            handle: 0,
            api: ConptyApi {
                create: create_stub,
                resize: resize_stub,
                close: close_stub,
                _library: None,
            },
            managed_job: None,
        };
        let native_failure = crate::WinsizeBuilder {
            cols: 79,
            rows: 24,
            width: 0,
            height: 0,
        }
        .build();
        let error = conpty
            .on_resize(native_failure)
            .expect_err("native HRESULT must propagate");
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(
            error.to_string(),
            "ConPTY resize failed (HRESULT 0x80070057)"
        );
        let valid = crate::WinsizeBuilder {
            cols: 80,
            rows: 24,
            width: 0,
            height: 0,
        }
        .build();
        conpty
            .on_resize(valid)
            .expect("a later valid resize succeeds");
    }
}

#[cfg(test)]
mod native_boundary_tests {
    use super::*;

    #[test]
    fn native_string_limits_count_utf16_units_and_geometry_keeps_signed_edges() {
        assert!(validate_native_string(&"a".repeat(32_766)).is_ok());
        assert!(validate_native_string(&"a".repeat(32_767)).is_err());
        assert!(validate_native_string(&"🙂".repeat(16_383)).is_ok());
        assert!(validate_native_string(&"🙂".repeat(16_384)).is_err());
        let size = checked_coord(Winsize {
            ws_col: 32_767,
            ws_row: 1,
            ws_xpixel: 0,
            ws_ypixel: 0,
        })
        .unwrap();
        assert_eq!((size.X, size.Y), (32_767, 1));
        let size = checked_coord(Winsize {
            ws_col: 1,
            ws_row: 32_767,
            ws_xpixel: 0,
            ws_ypixel: 0,
        })
        .unwrap();
        assert_eq!((size.X, size.Y), (1, 32_767));
    }

    #[test]
    fn native_environment_budget_has_exact_utf16_unit_and_entry_edges() {
        let block = environment_block(
            vec![("A".into(), "x".repeat(MAX_ENVIRONMENT_UNITS - 4))],
            false,
        )
        .unwrap();
        assert_eq!(block.len(), MAX_ENVIRONMENT_UNITS);
        assert_eq!(&block[..2], &[65, 61]);
        assert!(block[2..block.len() - 2].iter().all(|unit| *unit == 120));
        assert_eq!(&block[block.len() - 2..], &[0, 0]);
        drop(block);
        assert_eq!(
            environment_block(
                vec![("A".into(), "x".repeat(MAX_ENVIRONMENT_UNITS - 3))],
                false
            )
            .unwrap_err()
            .kind(),
            std::io::ErrorKind::InvalidInput
        );
        let entries = vec![("A".to_owned(), String::new()); MAX_ENVIRONMENT_ENTRIES];
        assert_eq!(
            environment_block(entries, false).unwrap(),
            vec![65, 61, 0, 0]
        );
        let entries = vec![("A".to_owned(), String::new()); MAX_ENVIRONMENT_ENTRIES + 1];
        assert_eq!(
            environment_block(entries, false).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn empty_explicit_environment_has_two_terminators() {
        assert_eq!(environment_block(Vec::new(), false).unwrap(), vec![0, 0]);
    }

    #[test]
    fn environment_preserves_unicode_equals_and_last_override() {
        let block = environment_block(
            vec![
                ("AMX_B".into(), "old".into()),
                ("AMX_A".into(), "界=🙂".into()),
                ("amx_b".into(), "new=tail".into()),
            ],
            false,
        )
        .unwrap();
        let expected: Vec<u16> =
            "AMX_A=界=🙂\0amx_b=new=tail\0\0".encode_utf16().collect();
        assert_eq!(block, expected);
    }

    #[test]
    fn environment_uses_windows_ordinal_case_rules() {
        let block = environment_block(
            vec![
                ("AMX_ä".into(), "old".into()),
                ("AMX_[".into(), "punctuation".into()),
                ("AMX_a".into(), "letter".into()),
                ("AMX_Ä".into(), "last".into()),
            ],
            false,
        )
        .unwrap();
        let expected: Vec<u16> = "AMX_a=letter\0AMX_[=punctuation\0AMX_Ä=last\0\0"
            .encode_utf16()
            .collect();
        assert_eq!(block, expected);
    }

    #[test]
    fn nul_launch_strings_are_rejected_before_native_allocation() {
        let fixture = tempfile::tempdir().unwrap();
        let absent = fixture.path().join("missing-executable.exe");
        let program = absent.to_str().unwrap();
        let with_nul = format!("{program}\0suffix");
        for (application, command, directory) in [
            (Some(with_nul.as_str()), Some(program), None),
            (Some(program), Some(with_nul.as_str()), None),
            (Some(program), Some(program), Some(with_nul.clone())),
        ] {
            let error = new(application, command, &directory, None, true, true, 80, 24)
                .err()
                .expect("NUL input must fail");
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
            assert!(!error.to_string().contains("suffix"));
        }
    }

    #[test]
    fn invalid_native_dimensions_are_rejected_before_child_lookup() {
        let fixture = tempfile::tempdir().unwrap();
        let absent = fixture.path().join("missing-executable.exe");
        let program = absent.to_str().unwrap();
        for (columns, rows) in [(0, 24), (80, 0), (32768, 24), (80, 65535)] {
            let result = new(
                Some(program),
                Some(program),
                &None,
                None,
                true,
                true,
                columns,
                rows,
            );
            let error = result.err().expect("invalid geometry must fail");
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn invalid_environment_is_rejected_before_child_lookup() {
        let fixture = tempfile::tempdir().unwrap();
        let absent = fixture.path().join("missing-executable.exe");
        let program = absent.to_str().unwrap();
        for (name, value) in [
            ("", "value"),
            ("AMX=BAD", "value"),
            ("AMX_BAD\0NAME", "value"),
            ("AMX_TEST", "value\0other"),
        ] {
            let result = new(
                Some(program),
                Some(program),
                &None,
                Some(vec![(name.into(), value.into())]),
                false,
                true,
                80,
                24,
            );
            let error = result.err().expect("invalid environment must fail");
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
            assert!(!error.to_string().contains("value"));
        }
    }
}
