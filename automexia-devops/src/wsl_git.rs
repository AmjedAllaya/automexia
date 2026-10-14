//! Passive Git projection inside the application's killable WSL helper only.
use crate::kubernetes;

const MAX_GIT_BYTES: usize = 4096;
#[cfg(windows)]
const MAX_ANCESTORS: usize = 64;

// Resolve Git's POSIX worktree pointer without interpreting a Windows path or
// permitting traversal above the selected distribution's filesystem root.
fn git_directory(base: &str, pointer: &str) -> Option<String> {
    let value = pointer
        .trim_end_matches(['\r', '\n'])
        .strip_prefix("gitdir: ")?;
    if value.is_empty()
        || value.len() > MAX_GIT_BYTES
        || value.starts_with("//")
        || value.contains(['\\', ':'])
        || value.chars().any(char::is_control)
    {
        return None;
    }
    let absolute = if value.starts_with('/') {
        value.to_owned()
    } else {
        format!("{base}/{value}")
    };
    let mut components = Vec::new();
    for part in absolute.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            _ => components.push(part),
        }
        if components.len() > 128 {
            return None;
        }
    }
    Some(format!("/{}", components.join("/")))
}

#[cfg(windows)]
fn read_git_file(path: &std::path::Path) -> Option<String> {
    use std::io::Read as _;
    use std::os::windows::fs::OpenOptionsExt as _;
    if !matches!(kubernetes::wsl_guest_entry(path), kubernetes::LocalEntry::Present(metadata) if metadata.is_file())
    {
        return None;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x0020_0000)
        .open(path)
        .ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file()
        || kubernetes::reparse_point(&metadata)
        || metadata.len() > MAX_GIT_BYTES as u64
    {
        return None;
    }
    let mut content = String::new();
    file.take((MAX_GIT_BYTES + 1) as u64)
        .read_to_string(&mut content)
        .ok()?;
    (content.len() <= MAX_GIT_BYTES).then_some(content)
}

/// Never call on a renderer or shared discovery thread: WSL filesystem access
/// belongs to the existing deadline-bound application helper.
#[cfg(windows)]
pub fn from_wsl_session(
    session: &automexia_extension_api::SessionFacts,
) -> Option<String> {
    if !kubernetes::is_wsl_session(session) {
        return None;
    }
    let distro = session.distro.as_deref()?;
    let mut cursor = session.cwd.as_ref()?.to_str()?.to_owned();
    kubernetes::wsl_path(distro, &cursor)?;
    for _ in 0..MAX_ANCESTORS {
        let dot_git = kubernetes::wsl_path(
            distro,
            &format!("{}/.git", cursor.trim_end_matches('/')),
        )?;
        let directory = match kubernetes::wsl_guest_entry(&dot_git) {
            kubernetes::LocalEntry::Present(metadata) if metadata.is_dir() => {
                format!("{}/.git", cursor.trim_end_matches('/'))
            }
            kubernetes::LocalEntry::Present(metadata) if metadata.is_file() => {
                git_directory(&cursor, &read_git_file(&dot_git)?)?
            }
            kubernetes::LocalEntry::Missing => {
                let parent = cursor.trim_end_matches('/').rsplit_once('/')?.0;
                if cursor == "/" {
                    return None;
                }
                cursor = if parent.is_empty() {
                    "/".into()
                } else {
                    parent.into()
                };
                continue;
            }
            _ => return None,
        };
        let head = kubernetes::wsl_path(distro, &format!("{directory}/HEAD"))?;
        return crate::context::git_head_label(&read_git_file(&head)?)
            .map(|label| crate::sanitize_label(&label))
            .filter(|label| !label.is_empty());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_pointers_stay_inside_the_selected_linux_root() {
        assert_eq!(
            git_directory("/project/work", "gitdir: ../repo/.git/worktrees/work\n")
                .as_deref(),
            Some("/project/repo/.git/worktrees/work")
        );
        assert_eq!(
            git_directory("/project", "gitdir: /tmp/repo/.git/worktrees/space \n")
                .as_deref(),
            Some("/tmp/repo/.git/worktrees/space ")
        );
        for pointer in [
            "gitdir: //remote/share",
            "gitdir: C:/repo",
            "gitdir: \\remote",
            "gitdir: ../../../outside",
            "gitdir: /tmp/\nfile",
            "gitdir: ",
        ] {
            assert!(git_directory("/project", pointer).is_none());
        }
    }

    #[test]
    fn guest_path_translation_never_accepts_host_or_other_distro_paths() {
        assert_eq!(
            kubernetes::wsl_path("Fixture-Distro", "/tmp/repo ")
                .unwrap()
                .to_string_lossy(),
            r"\\wsl.localhost\Fixture-Distro\tmp\repo "
        );
        for value in [
            "C:/repo",
            "//server/share",
            "/../other",
            "/tmp/a:b",
            "/tmp/\nrepo",
            "/tmp/\\repo",
        ] {
            assert!(kubernetes::wsl_path("Fixture-Distro", value).is_none());
        }
        assert!(kubernetes::wsl_path("../Other", "/tmp").is_none());
        assert!(
            kubernetes::wsl_path("Fixture", &format!("/{}", "x".repeat(4096))).is_none()
        );
    }
}
