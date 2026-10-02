//! Shared current-logon-only descriptor for application-owned local pipes.
//! Endpoint transport, peer verification and process ownership stay with callers.
use std::{ffi::c_void, io, mem::size_of, ptr::null_mut};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_INSUFFICIENT_BUFFER, HANDLE, HLOCAL,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
    SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenLogonSid, TokenUser, PSID, TOKEN_GROUPS,
    TOKEN_INFORMATION_CLASS, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
struct OwnedToken(HANDLE);

impl Drop for OwnedToken {
    fn drop(&mut self) {
        // SAFETY: this type uniquely owns the token handle.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub(crate) struct SecurityDescriptor {
    pub(crate) pointer: *mut c_void,
}

impl SecurityDescriptor {
    pub(crate) fn current_logon() -> io::Result<Self> {
        let user = current_sid_string(TokenUser)?;
        let logon = current_sid_string(TokenLogonSid)?;
        // The current user owns the object, but only the logon SID (plus
        // LocalSystem) receives data access. A same-user process in a different
        // terminal-services logon therefore fails the DACL before peer checks.
        let sddl = format!("O:{user}D:P(A;;GRGW;;;{logon})(A;;GA;;;SY)");
        if sddl.contains(";;;WD") || sddl.contains(";;;AN") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "broad named-pipe trustee is forbidden",
            ));
        }
        let encoded: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
        let mut pointer = null_mut();
        // SAFETY: encoded is terminated and pointer receives one LocalAlloc
        // security descriptor owned by SecurityDescriptor.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                encoded.as_ptr(),
                SDDL_REVISION_1,
                &mut pointer,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { pointer })
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: ConvertStringSecurityDescriptor allocated this pointer with
        // LocalAlloc and this type uniquely owns it.
        unsafe {
            LocalFree(self.pointer as HLOCAL);
        }
    }
}

fn current_sid_string(class: TOKEN_INFORMATION_CLASS) -> io::Result<String> {
    let mut token = INVALID_HANDLE_VALUE;
    // SAFETY: current process is always live and token receives one handle.
    let opened =
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if opened == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = OwnedToken(token);

    let mut required = 0_u32;
    // SAFETY: null query is the documented size-discovery call.
    unsafe {
        GetTokenInformation(token.0, class, null_mut(), 0, &mut required);
    }
    let discovery_error = unsafe { GetLastError() };
    if required == 0 || discovery_error != ERROR_INSUFFICIENT_BUFFER {
        return Err(io::Error::from_raw_os_error(discovery_error as i32));
    }

    let words = (required as usize).div_ceil(size_of::<usize>());
    let mut storage = vec![0_usize; words];
    // SAFETY: usize storage provides sufficient alignment and at least required
    // writable bytes; required is the exact size returned above.
    let loaded = unsafe {
        GetTokenInformation(
            token.0,
            class,
            storage.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    };
    if loaded == 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: the buffer layout matches the requested token information class
    // and remains alive through conversion.
    let sid: PSID = unsafe {
        if class == TokenUser {
            (*(storage.as_ptr().cast::<TOKEN_USER>())).User.Sid
        } else {
            let groups = &*(storage.as_ptr().cast::<TOKEN_GROUPS>());
            if groups.GroupCount == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "current token has no logon SID",
                ));
            }
            groups.Groups[0].Sid
        }
    };
    sid_to_string(sid)
}

fn sid_to_string(sid: PSID) -> io::Result<String> {
    let mut pointer = null_mut();
    // SAFETY: sid points into live token information and pointer receives one
    // LocalAlloc UTF-16 string on success.
    let ok = unsafe { ConvertSidToStringSidW(sid, &mut pointer) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut length = 0_usize;
    // SAFETY: ConvertSidToStringSidW returns a terminated UTF-16 string.
    unsafe {
        while *pointer.add(length) != 0 {
            length += 1;
        }
    }
    // SAFETY: length was measured to the terminator and pointer is live.
    let text = String::from_utf16(unsafe { std::slice::from_raw_parts(pointer, length) })
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "SID was not UTF-16"));
    // SAFETY: the returned SID string is LocalAlloc-owned.
    unsafe {
        LocalFree(pointer as HLOCAL);
    }
    text
}
