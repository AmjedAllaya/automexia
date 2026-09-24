use std::ffi::CStr;
use std::io;

pub(super) fn terminal_name(fd: libc::c_int) -> io::Result<String> {
    #[cfg(target_os = "macos")]
    let buffer = {
        // Darwin's TIOCPTYGNAME ABI is _IOC(IOC_OUT, 't', 83, 128), as
        // specified in XNU bsd/sys/ttycom.h. libc exposes no ptsname_r here.
        const TIOCPTYGNAME: libc::c_ulong = 0x4080_7453;
        let mut buffer = [0_u8; 128];
        // SAFETY: the ioctl writes exactly its ABI's 128-byte output buffer.
        // The kernel validates fd, and the buffer is private to this call.
        if unsafe { libc::ioctl(fd, TIOCPTYGNAME, buffer.as_mut_ptr()) } == -1 {
            return Err(io::Error::last_os_error());
        }
        buffer
    };
    #[cfg(not(target_os = "macos"))]
    let buffer = {
        let mut buffer = [0_u8; 1024];
        // SAFETY: ptsname_r accepts an integer master fd and a writable
        // buffer of the supplied size; no shared static storage is exposed.
        let status =
            unsafe { libc::ptsname_r(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if status != 0 {
            // FreeBSD versions differ: older ones use -1/errno, newer ones
            // return the positive error number as Linux does.
            return Err(if status == -1 {
                io::Error::last_os_error()
            } else {
                io::Error::from_raw_os_error(status)
            });
        }
        buffer
    };
    decode_name(&buffer)
}

pub(super) fn decode_name(buffer: &[u8]) -> io::Result<String> {
    let invalid_name = || io::Error::new(io::ErrorKind::InvalidData, "invalid PTY name");
    CStr::from_bytes_until_nul(buffer)
        .map_err(|_| invalid_name())?
        .to_str()
        .map(str::to_owned)
        .map_err(|_| invalid_name())
}
