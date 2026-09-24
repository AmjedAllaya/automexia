use std::io;
use std::ptr;

const INITIAL_BUFFER_BYTES: usize = 1024;
const MAX_BUFFER_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub(super) struct Passwd {
    pub(super) name: String,
    pub(super) dir: String,
    pub(super) shell: String,
}

pub(super) fn get_pw_entry() -> io::Result<Passwd> {
    // SAFETY: getuid takes no pointers and has no ownership preconditions.
    let uid = unsafe { libc::getuid() };
    lookup_passwd(uid, |uid, entry, buffer, result| {
        // SAFETY: the record, result pointer and buffer are writable and live
        // for the call. libc stores strings in this buffer. Nothing borrowed
        // from them escapes: lookup_passwd validates and copies each field.
        unsafe {
            libc::getpwuid_r(uid, entry, buffer.as_mut_ptr().cast(), buffer.len(), result)
        }
    })
}

pub(super) fn lookup_passwd(
    uid: libc::uid_t,
    mut lookup: impl FnMut(
        libc::uid_t,
        &mut libc::passwd,
        &mut [u8],
        &mut *mut libc::passwd,
    ) -> libc::c_int,
) -> io::Result<Passwd> {
    let mut buffer = vec![0; INITIAL_BUFFER_BYTES];
    loop {
        // SAFETY: passwd contains only integer and raw-pointer fields; zero
        // is valid for all fields on supported Unix targets. This also keeps
        // malformed partial native results from exposing uninitialized bytes.
        let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
        let mut result = ptr::null_mut();
        let status = lookup(uid, &mut entry, &mut buffer, &mut result);
        if status == libc::ERANGE && buffer.len() < MAX_BUFFER_BYTES {
            buffer.resize((buffer.len() * 2).min(MAX_BUFFER_BYTES), 0);
            continue;
        }
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status));
        }
        if result.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "account record not found",
            ));
        }
        if !ptr::eq(result, &entry) || entry.pw_uid != uid {
            return Err(invalid_record());
        }
        return Ok(Passwd {
            name: field(&buffer, entry.pw_name)?,
            dir: field(&buffer, entry.pw_dir)?,
            shell: field(&buffer, entry.pw_shell)?,
        });
    }
}

fn invalid_record() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid account record")
}

fn field(buffer: &[u8], value: *const libc::c_char) -> io::Result<String> {
    // Validate the address against the caller-owned buffer, then use only
    // bounded safe slices. Never dereference a native field pointer directly.
    let tail = (value as usize)
        .checked_sub(buffer.as_ptr() as usize)
        .and_then(|offset| buffer.get(offset..))
        .ok_or_else(invalid_record)?;
    let end = tail
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(invalid_record)?;
    let value = std::str::from_utf8(&tail[..end]).map_err(|_| invalid_record())?;
    Ok(value.to_owned())
}
