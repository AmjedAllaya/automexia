// From: https://github.com/alacritty/alacritty/blob/04ea367e3baa7e51933e9a595da793b4c8a4aa8f/alacritty/src/macos/proc.rs

use std::ffi::CStr;
use std::fmt::{self, Display, Formatter};
use std::io;
#[cfg(target_os = "macos")]
use std::mem::MaybeUninit;
#[cfg(target_os = "macos")]
use std::os::raw::c_int;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

/// Error during working directory retrieval.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),

    /// Expected return size didn't match libproc's.
    #[cfg(target_os = "macos")]
    InvalidSize,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(target_os = "macos")]
            Error::InvalidSize => None,
            Error::Io(err) => err.source(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(target_os = "macos")]
            Error::InvalidSize => write!(f, "Invalid proc_pidinfo return size"),
            Error::Io(err) => {
                write!(f, "Error getting current working directory: {err}")
            }
        }
    }
}

impl From<io::Error> for Error {
    fn from(val: io::Error) -> Self {
        Error::Io(val)
    }
}

/// Bindings for libproc.
#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
mod sys {
    use std::os::raw::{c_char, c_int, c_longlong, c_void};

    pub const PROC_PIDVNODEPATHINFO: c_int = 9;

    type gid_t = c_int;
    type off_t = c_longlong;
    type uid_t = c_int;
    type fsid_t = fsid;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct fsid {
        pub val: [i32; 2usize],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct vinfo_stat {
        pub vst_dev: u32,
        pub vst_mode: u16,
        pub vst_nlink: u16,
        pub vst_ino: u64,
        pub vst_uid: uid_t,
        pub vst_gid: gid_t,
        pub vst_atime: i64,
        pub vst_atimensec: i64,
        pub vst_mtime: i64,
        pub vst_mtimensec: i64,
        pub vst_ctime: i64,
        pub vst_ctimensec: i64,
        pub vst_birthtime: i64,
        pub vst_birthtimensec: i64,
        pub vst_size: off_t,
        pub vst_blocks: i64,
        pub vst_blksize: i32,
        pub vst_flags: u32,
        pub vst_gen: u32,
        pub vst_rdev: u32,
        pub vst_qspare: [i64; 2usize],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct vnode_info {
        pub vi_stat: vinfo_stat,
        pub vi_type: c_int,
        pub vi_pad: c_int,
        pub vi_fsid: fsid_t,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct vnode_info_path {
        pub vip_vi: vnode_info,
        pub vip_path: [c_char; 1024usize],
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct proc_vnodepathinfo {
        pub pvi_cdir: vnode_info_path,
        pub pvi_rdir: vnode_info_path,
    }

    extern "C" {
        pub fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;

        pub fn proc_pidinfo(
            pid: c_int,
            flavor: c_int,
            arg: u64,
            buffer: *mut c_void,
            buffersize: c_int,
        ) -> c_int;

        // pub fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    }
}

#[cfg(target_os = "macos")]
pub fn macos_process_name(pid: libc::c_int) -> String {
    get_proc_path(pid)
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn get_proc_path(pid: i32) -> Option<PathBuf> {
    if pid <= 0 {
        return None;
    }
    let mut buffer = [0_u8; 4 * 1024];
    // SAFETY: libproc receives initialized caller-owned storage and its exact
    // capacity. No uninitialized Vec length or foreign reference escapes.
    let length = unsafe {
        sys::proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32)
    };
    if length <= 0 || length as usize > buffer.len() {
        return None;
    }
    decode_native_path(&buffer).ok()
}

#[cfg(target_os = "macos")]
pub fn macos_cwd(pid: libc::c_int) -> Result<PathBuf, Error> {
    let mut info = MaybeUninit::<sys::proc_vnodepathinfo>::uninit();
    let info_ptr = info.as_mut_ptr() as *mut libc::c_void;
    let size = std::mem::size_of::<sys::proc_vnodepathinfo>() as c_int;

    // SAFETY: libproc writes the complete repr(C) record into this storage.
    // Only a full-size response permits initialization; the named record stays
    // alive while its bounded path bytes are copied into an owned PathBuf.
    let count =
        unsafe { sys::proc_pidinfo(pid, sys::PROC_PIDVNODEPATHINFO, 0, info_ptr, size) };
    if count <= 0 {
        return Err(io::Error::last_os_error().into());
    }
    if count != size {
        return Err(Error::InvalidSize);
    }
    let record = unsafe { info.assume_init() };
    let bytes = record.pvi_cdir.vip_path.map(|byte| byte as u8);
    decode_native_path(&bytes)
}

fn decode_native_path(buffer: &[u8]) -> Result<PathBuf, Error> {
    let value = CStr::from_bytes_until_nul(buffer).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "invalid process path")
    })?;
    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(value.to_bytes())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_process_path_preserves_unix_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let input = b"/tmp/native-\xff\0ignored";
        assert_eq!(
            decode_native_path(input).unwrap().as_os_str().as_bytes(),
            b"/tmp/native-\xff"
        );
    }

    #[test]
    fn native_process_path_requires_a_bounded_terminator() {
        assert!(decode_native_path(b"unterminated").is_err());
        assert!(decode_native_path(&[]).is_err());
        assert_eq!(
            decode_native_path(b"/tmp/\xe9\xa1\xb9\xe7\x9b\xae\0").unwrap(),
            PathBuf::from("/tmp/项目")
        );
    }

    #[cfg(target_os = "macos")]
    use std::{env, process};

    #[test]
    #[cfg(target_os = "macos")]
    fn native_process_metadata_owns_results_and_handles_unavailable_processes() {
        let executable = std::env::current_exe().unwrap();
        for _ in 0..64 {
            let path = get_proc_path(process::id() as i32).expect("current process path");
            assert_eq!(
                std::fs::canonicalize(path).unwrap(),
                std::fs::canonicalize(&executable).unwrap()
            );
        }
        assert!(get_proc_path(-1).is_none());
        assert!(macos_process_name(-1).is_empty());
        assert!(macos_cwd(-1).is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn cwd_matches_current_dir() {
        assert_eq!(
            macos_cwd(process::id() as i32).ok(),
            env::current_dir().ok()
        );
    }
}
