//! Bounded, non-executing import of Ghostty keybindings.
//!
//! Ghostty configuration is untrusted input. This module reads only `keybind`
//! and `config-file`, rejects symlinks and global bindings, follows includes
//! with canonical cycle detection, and never invokes Ghostty or a shell.

use automexia_keybindings::{
    parse_binding_line, BindingOperation, BindingOrigin, BindingScope, ParsedLine,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024;
const MAX_FILES: usize = 32;
const MAX_INCLUDE_DEPTH: usize = 8;
const MAX_LINES: usize = 4_096;
const MAX_COMMENT_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationClassification {
    Exact,
    Translated,
    Unsupported,
    Unsafe,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationEntry {
    pub file: String,
    pub line: usize,
    pub classification: MigrationClassification,
    pub binding: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationReport {
    pub schema_version: u32,
    pub source_files: usize,
    pub entries: Vec<MigrationEntry>,
    pub exact: usize,
    pub translated: usize,
    pub unsupported: usize,
    pub unsafe_entries: usize,
    pub applied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup: Option<String>,
}

impl MigrationReport {
    fn from_entries(source_files: usize, entries: Vec<MigrationEntry>) -> Self {
        let count = |classification| {
            entries
                .iter()
                .filter(|entry| entry.classification == classification)
                .count()
        };
        Self {
            schema_version: 1,
            source_files,
            exact: count(MigrationClassification::Exact),
            translated: count(MigrationClassification::Translated),
            unsupported: count(MigrationClassification::Unsupported),
            unsafe_entries: count(MigrationClassification::Unsafe),
            entries,
            applied: false,
            backup: None,
        }
    }

    pub fn migratable_bindings(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.classification,
                    MigrationClassification::Exact | MigrationClassification::Translated
                )
            })
            .map(|entry| entry.binding.as_str())
    }
}

#[derive(Debug)]
struct ReadBudget {
    files: usize,
    lines: usize,
    bytes: u64,
    root: PathBuf,
    active: BTreeSet<PathBuf>,
}

pub fn default_source_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let xdg = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    [
        xdg.join("ghostty").join("config.ghostty"),
        xdg.join("ghostty").join("config"),
        home.join("Library")
            .join("Application Support")
            .join("com.mitchellh.ghostty")
            .join("config.ghostty"),
        home.join("Library")
            .join("Application Support")
            .join("com.mitchellh.ghostty")
            .join("config"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

pub fn preview(source: &Path) -> Result<MigrationReport, String> {
    let root = canonical_regular(source, "Ghostty configuration")?;
    let mut budget = ReadBudget {
        files: 0,
        lines: 0,
        bytes: 0,
        root: root.clone(),
        active: BTreeSet::new(),
    };
    let mut entries = Vec::new();
    read_config(&root, 0, &mut budget, &mut entries)?;
    Ok(MigrationReport::from_entries(budget.files, entries))
}

fn read_config(
    path: &Path,
    depth: usize,
    budget: &mut ReadBudget,
    entries: &mut Vec<MigrationEntry>,
) -> Result<(), String> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(format!(
            "Ghostty include depth exceeds the limit of {MAX_INCLUDE_DEPTH}"
        ));
    }
    let canonical = canonical_regular(path, "Ghostty configuration include")?;
    if !budget.active.insert(canonical.clone()) {
        return Err("Ghostty configuration include cycle detected".into());
    }
    budget.files = budget.files.saturating_add(1);
    if budget.files > MAX_FILES {
        budget.active.remove(&canonical);
        return Err(format!("Ghostty configuration exceeds {MAX_FILES} files"));
    }

    let bytes = read_bounded(&canonical)?;
    budget.bytes = budget.bytes.saturating_add(bytes.len() as u64);
    if budget.bytes > MAX_TOTAL_BYTES {
        budget.active.remove(&canonical);
        return Err(format!(
            "Ghostty configuration exceeds {MAX_TOTAL_BYTES} aggregate bytes"
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| "Ghostty configuration must be valid UTF-8".to_string())?;
    let label = source_label(&budget.root, &canonical);
    let mut comments = Vec::new();
    let mut includes = Vec::new();

    for (index, raw) in text.lines().enumerate() {
        budget.lines = budget.lines.saturating_add(1);
        if budget.lines > MAX_LINES {
            budget.active.remove(&canonical);
            return Err(format!("Ghostty configuration exceeds {MAX_LINES} lines"));
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            comments.clear();
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix('#') {
            let comment = comment.trim();
            if comments.iter().map(String::len).sum::<usize>() + comment.len()
                <= MAX_COMMENT_BYTES
            {
                comments.push(comment.to_string());
            }
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            comments.clear();
            continue;
        };
        match key.trim() {
            "keybind" => {
                let binding = unquote(value.trim());
                entries.push(classify_binding(
                    label.clone(),
                    index + 1,
                    binding,
                    std::mem::take(&mut comments),
                ));
            }
            "config-file" => {
                let value = unquote(value.trim());
                let (optional, value) = match value.strip_prefix('?') {
                    Some(path) => (true, path.to_string()),
                    None => (false, value),
                };
                if value.is_empty() || value.as_bytes().contains(&0) {
                    budget.active.remove(&canonical);
                    return Err("Ghostty include path is invalid".into());
                }
                let include = PathBuf::from(value);
                let include = if include.is_absolute() {
                    include
                } else {
                    canonical
                        .parent()
                        .ok_or_else(|| "Ghostty include has no parent".to_string())?
                        .join(include)
                };
                includes.push((include, optional));
                comments.clear();
            }
            _ => comments.clear(),
        }
    }

    // Ghostty processes config-file directives after the containing file.
    for (include, optional) in includes {
        if optional && !include.exists() {
            continue;
        }
        read_config(&include, depth + 1, budget, entries)?;
    }
    budget.active.remove(&canonical);
    Ok(())
}

fn classify_binding(
    file: String,
    line: usize,
    binding: String,
    comments: Vec<String>,
) -> MigrationEntry {
    if binding.is_empty() {
        return MigrationEntry {
            file,
            line,
            classification: MigrationClassification::Unsafe,
            binding,
            comments,
            diagnostic: Some("default_reset_requires_manual_review"),
        };
    }
    let parsed = parse_binding_line(&binding, BindingOrigin::Imported);
    let Ok(ParsedLine::Binding(spec)) = parsed else {
        return MigrationEntry {
            file,
            line,
            classification: MigrationClassification::Unsupported,
            binding,
            comments,
            diagnostic: Some("unsupported_binding_syntax_or_action"),
        };
    };
    if spec.scope == BindingScope::OperatingSystemGlobal {
        return MigrationEntry {
            file,
            line,
            classification: MigrationClassification::Unsafe,
            binding,
            comments,
            diagnostic: Some("global_binding_requires_platform_permission_review"),
        };
    }
    let translated = binding.contains("cmd+")
        || binding.contains("command+")
        || binding.contains("opt+")
        || matches!(&spec.operation, BindingOperation::Bind(actions) if actions.iter().any(|action| {
            let raw_id = binding
                .rsplit_once('=')
                .map(|(_, value)| value)
                .unwrap_or_default()
                .split(':')
                .next()
                .unwrap_or_default()
                .trim()
                .replace('-', "_");
            action.id.as_str() != raw_id
        }));
    MigrationEntry {
        file,
        line,
        classification: if translated {
            MigrationClassification::Translated
        } else {
            MigrationClassification::Exact
        },
        binding,
        comments,
        diagnostic: translated.then_some("normalized_alias"),
    }
}

fn unquote(value: &str) -> String {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn source_label(root: &Path, path: &Path) -> String {
    if root == path {
        "root".into()
    } else {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("included")
            .chars()
            .filter(|character| !character.is_control())
            .take(128)
            .collect()
    }
}

fn canonical_regular(path: &Path, role: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("{role} is unavailable: {}", error.kind()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{role} must be a regular non-symlink file"));
    }
    path.canonicalize()
        .map_err(|error| format!("{role} could not be resolved: {}", error.kind()))
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!("configuration could not be inspected: {}", error.kind())
    })?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!(
            "Ghostty configuration file exceeds {MAX_FILE_BYTES} bytes"
        ));
    }
    let file = fs::File::open(path).map_err(|error| {
        format!("configuration could not be opened: {}", error.kind())
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("configuration could not be read: {}", error.kind()))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(format!(
            "Ghostty configuration file exceeds {MAX_FILE_BYTES} bytes"
        ));
    }
    Ok(bytes)
}

pub fn apply(
    source: &Path,
    destination: &Path,
    confirmed: bool,
) -> Result<MigrationReport, String> {
    if !confirmed {
        return Err("migration apply requires explicit confirmation".into());
    }
    let mut report = preview(source)?;
    let bindings = report.migratable_bindings().collect::<Vec<_>>();
    if bindings.is_empty() {
        return Err("no safe Ghostty keybindings are available to apply".into());
    }

    let existing = match fs::symlink_metadata(destination) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(
                    "Automexia configuration must be a regular non-symlink file".into()
                );
            }
            let bytes = read_bounded(destination)?;
            String::from_utf8(bytes)
                .map_err(|_| "Automexia configuration must be valid UTF-8".to_string())?
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(format!(
                "Automexia configuration could not be inspected: {}",
                error.kind()
            ));
        }
    };
    if section_has_key(&existing, "bindings", "keybinds") {
        return Err("Automexia configuration already owns typed keybinds; merge the dry-run report manually".into());
    }

    let mut document = existing.clone();
    insert_section_value(
        &mut document,
        "bindings",
        &format!(
            "keybinds = [{}]",
            bindings
                .iter()
                .map(|binding| serde_json::to_string(binding).expect("strings serialize"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    if !section_has_key(&document, "keyboard", "binding-profile") {
        insert_section_value(
            &mut document,
            "keyboard",
            "binding-profile = \"ghostty-1.3\"",
        );
    }
    toml::from_str::<rio_backend::config::Config>(&document)
        .map_err(|_| "generated Automexia configuration failed validation".to_string())?;

    let parent = destination
        .parent()
        .ok_or_else(|| "Automexia configuration has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "configuration directory could not be created: {}",
            error.kind()
        )
    })?;

    let backup = if existing.is_empty() {
        None
    } else {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup = parent.join(format!("config.pre-ghostty-{stamp}.toml.bak"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)
            .map_err(|error| {
                format!(
                    "configuration backup could not be created: {}",
                    error.kind()
                )
            })?;
        file.write_all(existing.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                format!(
                    "configuration backup could not be written: {}",
                    error.kind()
                )
            })?;
        Some(backup)
    };

    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("configuration staging failed: {}", error.kind()))?;
    staged
        .write_all(document.as_bytes())
        .and_then(|_| staged.as_file().sync_all())
        .map_err(|error| format!("configuration staging failed: {}", error.kind()))?;
    staged.persist(destination).map_err(|error| {
        format!("configuration publication failed: {}", error.error.kind())
    })?;

    report.applied = true;
    report.backup = backup.map(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("backup")
            .to_string()
    });
    Ok(report)
}

fn section_has_key(document: &str, section: &str, key: &str) -> bool {
    let header = format!("[{section}]");
    let mut active = false;
    for line in document.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            active = trimmed == header;
            continue;
        }
        if active
            && trimmed
                .split_once('=')
                .is_some_and(|(candidate, _)| candidate.trim() == key)
        {
            return true;
        }
    }
    false
}

fn insert_section_value(document: &mut String, section: &str, value: &str) {
    let header = format!("[{section}]");
    let section_offset = {
        document.line_indices().find_map(|(offset, line)| {
            (line.trim() == header).then_some(offset + line.len())
        })
    };
    if let Some(offset) = section_offset {
        document.insert_str(offset, &format!("\n{value}"));
        return;
    }
    if !document.is_empty() && !document.ends_with('\n') {
        document.push('\n');
    }
    if !document.is_empty() {
        document.push('\n');
    }
    document.push_str(&header);
    document.push('\n');
    document.push_str(value);
    document.push('\n');
}

trait LineIndices {
    fn line_indices(&self) -> Box<dyn Iterator<Item = (usize, &str)> + '_>;
}

impl LineIndices for str {
    fn line_indices(&self) -> Box<dyn Iterator<Item = (usize, &str)> + '_> {
        let mut offset = 0usize;
        Box::new(self.split_inclusive('\n').map(move |line| {
            let start = offset;
            offset += line.len();
            (start, line.trim_end_matches(['\r', '\n']))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_follows_relative_includes_preserves_comments_and_rejects_globals() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("parts")).unwrap();
        fs::write(
            root.path().join("config.ghostty"),
            "# Copy selection\nkeybind = performable:ctrl+c=copy\nconfig-file = parts/extra\n",
        )
        .unwrap();
        fs::write(
            root.path().join("parts/extra"),
            "keybind = global:super+grave=toggle_quick_terminal\nkeybind = ctrl+x=quit\n",
        )
        .unwrap();

        let report = preview(&root.path().join("config.ghostty")).unwrap();
        assert_eq!(report.source_files, 2);
        assert_eq!(report.translated, 1);
        assert_eq!(report.exact, 1);
        assert_eq!(report.unsafe_entries, 1);
        assert_eq!(report.entries[0].comments, ["Copy selection"]);
    }

    #[test]
    fn include_cycles_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first");
        let second = root.path().join("second");
        fs::write(&first, "config-file = second\n").unwrap();
        fs::write(&second, "config-file = first\n").unwrap();
        assert!(preview(&first).unwrap_err().contains("cycle"));
    }

    #[cfg(unix)]
    #[test]
    fn source_and_include_symlinks_fail_closed() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let real_source = root.path().join("real-source");
        let linked_source = root.path().join("linked-source");
        fs::write(&real_source, "keybind = ctrl+x=quit\n").unwrap();
        symlink(&real_source, &linked_source).unwrap();
        assert!(preview(&linked_source)
            .unwrap_err()
            .contains("symbolic link"));

        let real_include = root.path().join("real-include");
        let linked_include = root.path().join("linked-include");
        let source = root.path().join("source");
        fs::write(&real_include, "keybind = ctrl+y=quit\n").unwrap();
        symlink(&real_include, &linked_include).unwrap();
        fs::write(&source, "config-file = linked-include\n").unwrap();
        assert!(preview(&source).unwrap_err().contains("symbolic link"));
    }

    #[test]
    fn apply_requires_confirmation_preserves_existing_text_and_creates_backup() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("ghostty");
        let destination = root.path().join("automexia/config.toml");
        fs::write(&source, "# Keep\nkeybind = ctrl+x=quit\n").unwrap();
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, "# user comment\ntheme = \"automexia-dark\"\n").unwrap();

        assert!(apply(&source, &destination, false).is_err());
        let report = apply(&source, &destination, true).unwrap();
        assert!(report.applied);
        assert!(report.backup.is_some());
        let output = fs::read_to_string(&destination).unwrap();
        assert!(output.contains("# user comment"));
        assert!(output.contains("binding-profile = \"ghostty-1.3\""));
        assert!(output.contains("ctrl+x=quit"));
    }
}
