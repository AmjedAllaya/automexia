//! Advisory editor selection, never execution-policy enforcement.
//!
//! Read bounded Windows PowerShell configuration once during resource setup.
//! Windows PowerShell still authorizes every module/script; no policy is written.
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS,
};
use windows_sys::Win32::System::Registry::{
    RegGetValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Advice {
    SystemEditor,
    PrivateEditor,
    Undefined,
    Unavailable,
}

fn parse(value: &str) -> Advice {
    if value.len() > 32 {
        return Advice::SystemEditor;
    }
    match value.to_ascii_lowercase().as_str() {
        "" => Advice::Undefined,
        "remotesigned" | "unrestricted" | "bypass" => Advice::PrivateEditor,
        // AllSigned may require a separate publisher trust decision. Preserve
        // the system editor instead of introducing a startup approval prompt.
        // PowerShell treats unknown/nonempty policy names as Restricted.
        _ => Advice::SystemEditor,
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Sources {
    machine: Advice,
    user: Advice,
    current_user: Advice,
    local_machine: Advice,
}

impl Sources {
    pub(crate) fn permits(self, process: Option<&str>) -> bool {
        [
            self.machine,
            self.user,
            process.map(parse).unwrap_or(Advice::Undefined),
            self.current_user,
            self.local_machine,
        ]
        .into_iter()
        .find(|policy| *policy != Advice::Undefined)
        .unwrap_or(Advice::SystemEditor)
            == Advice::PrivateEditor
    }
}

static SOURCES: OnceLock<Sources> = OnceLock::new();

pub(crate) fn initialize() {
    SOURCES.get_or_init(read_sources);
}

pub(crate) fn permits(process: Option<&str>) -> bool {
    SOURCES
        .get()
        .is_some_and(|sources| sources.permits(process))
}

fn read_sources() -> Sources {
    const PREFERENCE: &str =
        r"SOFTWARE\Microsoft\PowerShell\1\ShellIds\Microsoft.PowerShell";
    Sources {
        machine: group_policy(HKEY_LOCAL_MACHINE),
        user: group_policy(HKEY_CURRENT_USER),
        current_user: string_value(HKEY_CURRENT_USER, PREFERENCE, "ExecutionPolicy"),
        local_machine: string_value(HKEY_LOCAL_MACHINE, PREFERENCE, "ExecutionPolicy"),
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn string_value(hive: HKEY, key: &str, name: &str) -> Advice {
    let (key, name) = (wide(key), wide(name));
    let mut value = [0u16; 64];
    let mut bytes = std::mem::size_of_val(&value) as u32;
    // SAFETY: predefined read-only hive, terminated constant names, and a live
    // aligned buffer whose byte capacity is passed to RegGetValueW.
    let result = unsafe {
        RegGetValueW(
            hive,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if matches!(result, ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) {
        return Advice::Undefined;
    }
    if result != ERROR_SUCCESS
        || bytes < 2
        || !bytes.is_multiple_of(2)
        || bytes as usize > std::mem::size_of_val(&value)
    {
        return Advice::Unavailable;
    }
    let length = bytes as usize / 2 - 1;
    if value[length] != 0 {
        return Advice::Unavailable;
    }
    String::from_utf16(&value[..length])
        .map_or(Advice::Unavailable, |value| parse(&value))
}

fn group_policy(hive: HKEY) -> Advice {
    const GROUP: &str = r"SOFTWARE\Policies\Microsoft\Windows\PowerShell";
    let (key, name) = (wide(GROUP), wide("EnableScripts"));
    let mut enabled = 0u32;
    let mut bytes = 4u32;
    // SAFETY: terminated constant names and a four-byte DWORD destination.
    let result = unsafe {
        RegGetValueW(
            hive,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut enabled as *mut u32).cast(),
            &mut bytes,
        )
    };
    match result {
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Advice::Undefined,
        ERROR_SUCCESS if bytes == 4 && enabled == 0 => Advice::SystemEditor,
        ERROR_SUCCESS if bytes == 4 && enabled == 1 => {
            string_value(hive, GROUP, "ExecutionPolicy")
        }
        _ => Advice::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_advice_preserves_group_policy_precedence_and_unknown_failures() {
        let mut source = Sources {
            machine: Advice::Undefined,
            user: Advice::Undefined,
            current_user: Advice::Undefined,
            local_machine: Advice::Undefined,
        };
        assert!(!source.permits(None));
        assert!(source.permits(Some("RemoteSigned")));
        assert!(!source.permits(Some("invalid")));
        source.local_machine = Advice::PrivateEditor;
        assert!(source.permits(None));
        source.current_user = Advice::SystemEditor;
        assert!(!source.permits(None));
        assert!(!source.permits(Some("AllSigned")));
        source.user = Advice::SystemEditor;
        assert!(!source.permits(Some("Bypass")));
        source.machine = Advice::PrivateEditor;
        assert!(source.permits(Some("Restricted")));
        source.machine = Advice::Unavailable;
        assert!(!source.permits(Some("Bypass")));
    }

    #[test]
    fn native_editor_advice_agrees_with_windows_powershell_policy() {
        use std::process::Command;
        let source = read_sources();
        let modules = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/Modules");
        for policy in [
            None,
            Some("Restricted"),
            Some("AllSigned"),
            Some("RemoteSigned"),
            Some("Unrestricted"),
            Some("Bypass"),
        ] {
            let mut command = Command::new("powershell.exe");
            command.args(["-NoLogo", "-NoProfile", "-NonInteractive"]);
            if let Some(policy) = policy {
                command.args(["-ExecutionPolicy", policy]);
            }
            command
                .args(["-Command", "(Get-ExecutionPolicy).ToString()"])
                .env("PSModulePath", &modules);
            let output = super::super::cli_process::capture(
                command,
                super::super::cli_process::Limits {
                    timeout: std::time::Duration::from_secs(30),
                    stdout: 1024,
                    stderr: 4096,
                },
                || false,
            )
            .expect("bounded native policy control");
            assert!(output.status.success());
            let observed = std::str::from_utf8(&output.stdout).unwrap().trim();
            let inherited = std::env::var("PSExecutionPolicyPreference").ok();
            assert_eq!(
                source.permits(policy.or(inherited.as_deref())),
                parse(observed) == Advice::PrivateEditor
            );
        }
    }
}
