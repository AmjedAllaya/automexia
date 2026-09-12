//! Completion accounting only; process-wrap retains launch and kill ownership.
use super::{cleanup_error, CLEANUP_TIMEOUT, POLL_INTERVAL};
use process_wrap::std::{CommandWrap, CommandWrapper, JobObject};
use std::{
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    process::{Child, Command},
    sync::Arc,
    time::Instant,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
    JobObjectBasicAccountingInformation, JobObjectBasicProcessIdList,
    QueryInformationJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
};
use windows_sys::Win32::{
    Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SYNCHRONIZE,
    },
};

const MAX_MEMBERS: usize = 256;

// The Win32 variable-length process-list layout, with a fixed resource ceiling.
#[repr(C)]
struct MemberList {
    assigned: u32,
    count: u32,
    ids: [usize; MAX_MEMBERS],
}

fn valid_member_counts(assigned: u32, count: u32) -> bool {
    count <= assigned && assigned as usize <= MAX_MEMBERS
}

#[derive(Clone, Debug)]
pub(super) struct CompletionJob(Arc<OwnedHandle>);

impl CompletionJob {
    pub(super) fn pin_members(&self) -> io::Result<Vec<OwnedHandle>> {
        let mut list = MemberList {
            assigned: 0,
            count: 0,
            ids: [0; MAX_MEMBERS],
        };
        if unsafe {
            QueryInformationJobObject(
                self.0.as_raw_handle(),
                JobObjectBasicProcessIdList,
                std::ptr::from_mut(&mut list).cast(),
                std::mem::size_of_val(&list) as u32,
                std::ptr::null_mut(),
            )
        } == 0
            || !valid_member_counts(list.assigned, list.count)
        {
            return Err(cleanup_error());
        }
        let mut members = Vec::with_capacity(list.count as usize);
        for &id in &list.ids[..list.count as usize] {
            let id = u32::try_from(id).map_err(|_| cleanup_error())?;
            let raw = unsafe {
                OpenProcess(
                    PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                    0,
                    id,
                )
            };
            if raw.is_null() {
                // An identity already destroyed between enumeration and opening
                // needs no wait. Other failures cannot certify completion.
                if io::Error::last_os_error().raw_os_error()
                    == Some(ERROR_INVALID_PARAMETER as i32)
                {
                    continue;
                }
                return Err(cleanup_error());
            }
            let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
            let mut belongs = 0;
            if unsafe {
                IsProcessInJob(
                    handle.as_raw_handle(),
                    self.0.as_raw_handle(),
                    &mut belongs,
                )
            } == 0
            {
                return Err(cleanup_error());
            }
            if belongs == 0 {
                // Never treat a potentially reused ID as an owned live member.
                // A signaled handle is harmless; a live mismatch fails closed.
                if !member_stopped(&handle)? {
                    return Err(cleanup_error());
                }
            } else {
                members.push(handle);
            }
        }
        Ok(members)
    }

    pub(super) fn new() -> io::Result<Self> {
        // Unnamed, non-inheritable and without breakaway/UI/termination limits.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(cleanup_error());
        }
        // SAFETY: the successful creation transfers this unique handle to RAII.
        Ok(Self(Arc::new(unsafe {
            OwnedHandle::from_raw_handle(handle)
        })))
    }

    pub(super) fn is_empty(&self) -> io::Result<bool> {
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: the owned job and correctly sized output live across the call.
        if unsafe {
            QueryInformationJobObject(
                self.0.as_raw_handle(),
                JobObjectBasicAccountingInformation,
                std::ptr::from_mut(&mut accounting).cast(),
                std::mem::size_of_val(&accounting) as u32,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(cleanup_error());
        }
        Ok(accounting.ActiveProcesses == 0)
    }
}

pub(super) fn members_stopped(members: &[OwnedHandle]) -> io::Result<bool> {
    for member in members {
        if !member_stopped(member)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn member_stopped(member: &OwnedHandle) -> io::Result<bool> {
    match unsafe { WaitForSingleObject(member.as_raw_handle(), 0) } {
        WAIT_OBJECT_0 => Ok(true),
        WAIT_TIMEOUT => Ok(false),
        _ => Err(cleanup_error()),
    }
}

impl CommandWrapper for CompletionJob {
    fn pre_spawn(&mut self, _: &mut Command, core: &CommandWrap) -> io::Result<()> {
        // JobObject's pre-spawn hook MUST suspend the child. All post-spawn hooks
        // run before its wrap-child hook assigns the inner job and resumes it.
        if core.get_wrap::<JobObject>().is_none() {
            return Err(cleanup_error());
        }
        Ok(())
    }

    fn post_spawn(
        &mut self,
        _: &mut Command,
        child: &mut Child,
        _: &CommandWrap,
    ) -> io::Result<()> {
        // Both handles are owned and live. Assign before any child code executes,
        // so even immediate descendants belong to this accounting hierarchy.
        if unsafe {
            AssignProcessToJobObject(self.0.as_raw_handle(), child.as_raw_handle())
        } != 0
        {
            return Ok(());
        }
        // A failed post-spawn hook otherwise drops a still-suspended raw child.
        // It cannot have descendants yet. Terminate and boundedly reap it here.
        let _ = child.kill();
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        while matches!(child.try_wait(), Ok(None)) && Instant::now() < deadline {
            std::thread::sleep(POLL_INTERVAL);
        }
        Err(cleanup_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amx_process_completion_requires_suspended_job_owner_before_spawn() {
        let job = CompletionJob::new().unwrap();
        assert!(job.is_empty().unwrap());
        assert!(job.pin_members().unwrap().is_empty());
        assert!(members_stopped(&[]).unwrap());
        let mut command = CommandWrap::from(Command::new("missing-fixture"));
        command.wrap(job);
        let error = command.spawn().unwrap_err();
        assert_eq!(
            error.to_string(),
            "local tool cleanup could not be confirmed"
        );
    }

    #[test]
    fn amx_process_completion_list_layout_matches_win32() {
        use windows_sys::Win32::System::JobObjects::JOBOBJECT_BASIC_PROCESS_ID_LIST;
        assert_eq!(
            std::mem::offset_of!(MemberList, ids),
            std::mem::offset_of!(JOBOBJECT_BASIC_PROCESS_ID_LIST, ProcessIdList)
        );
        assert_eq!(
            std::mem::align_of::<MemberList>(),
            std::mem::align_of::<JOBOBJECT_BASIC_PROCESS_ID_LIST>()
        );
        assert_eq!(MAX_MEMBERS, 256);
        for count in [0, 1, 255, 256] {
            assert!(valid_member_counts(count, count));
        }
        for (assigned, count) in [(0, 1), (255, 256), (257, 256), (u32::MAX, 0)] {
            assert!(!valid_member_counts(assigned, count));
        }
    }
}
