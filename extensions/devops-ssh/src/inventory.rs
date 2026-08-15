use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::{self, File, Metadata},
    io::{self, Read},
    path::{Path, PathBuf},
};

use glob::{glob_with, MatchOptions};

use crate::{ConnectionRecord, IdentityHint, InventorySnapshot, SourceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantKind {
    User,
    System,
}

impl GrantKind {
    fn source(self) -> SourceKind {
        match self {
            Self::User => SourceKind::OpenSshUser,
            Self::System => SourceKind::OpenSshSystem,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InventoryGrant {
    label: String,
    root: PathBuf,
    entries: Vec<PathBuf>,
    kind: GrantKind,
}

impl InventoryGrant {
    pub fn new(
        label: impl Into<String>,
        root: impl AsRef<Path>,
        entries: impl IntoIterator<Item = impl AsRef<Path>>,
        kind: GrantKind,
    ) -> Result<Self, InventoryError> {
        let label = label.into();
        if label.trim().is_empty()
            || label.len() > 64
            || label.chars().any(char::is_control)
            || label.contains('/')
            || label.contains(char::from(92))
        {
            return Err(InventoryError::Grant(
                "grant label is empty, sensitive, or too long".into(),
            ));
        }
        let root = canonical_nonsymlink(root.as_ref(), "grant root")?;
        let mut canonical_entries = Vec::new();
        for entry in entries {
            let entry = entry.as_ref();
            reject_symlink(entry, "grant entry")?;
            let canonical = fs::canonicalize(entry)
                .map_err(|error| InventoryError::io("grant entry", error))?;
            require_inside(&root, &canonical)?;
            canonical_entries.push(canonical);
        }
        canonical_entries.sort();
        canonical_entries.dedup();
        if canonical_entries.is_empty() {
            return Err(InventoryError::Grant(
                "grant must contain at least one exact entry file".into(),
            ));
        }
        Ok(Self {
            label,
            root,
            entries: canonical_entries,
            kind,
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InventoryLimits {
    pub max_file_bytes: usize,
    pub max_total_bytes: usize,
    pub max_files: usize,
    pub max_include_depth: usize,
    pub max_aliases: usize,
    pub max_value_bytes: usize,
    pub max_line_bytes: usize,
}

impl Default for InventoryLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 1024 * 1024,
            max_total_bytes: 8 * 1024 * 1024,
            max_files: 128,
            max_include_depth: 8,
            max_aliases: 10_000,
            max_value_bytes: 4 * 1024,
            max_line_bytes: 16 * 1024,
        }
    }
}

impl InventoryLimits {
    fn validate(self) -> Result<Self, InventoryError> {
        if self.max_file_bytes == 0
            || self.max_total_bytes < self.max_file_bytes
            || self.max_files == 0
            || self.max_include_depth == 0
            || self.max_aliases == 0
            || self.max_value_bytes == 0
            || self.max_line_bytes < self.max_value_bytes
        {
            return Err(InventoryError::Limit(
                "inventory limits are zero or internally inconsistent".into(),
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub source: String,
    pub line: Option<usize>,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct ScanOutcome {
    pub snapshot: InventorySnapshot,
    pub watched_files: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub enum InventoryError {
    Grant(String),
    Limit(String),
    Source {
        source: String,
        line: Option<usize>,
        message: String,
    },
    InvalidMetadata(String),
    Persistence(String),
    Watch(String),
}

impl InventoryError {
    fn io(context: &str, error: io::Error) -> Self {
        Self::Source {
            source: context.into(),
            line: None,
            message: format!("filesystem operation failed ({:?})", error.kind()),
        }
    }
}

impl fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grant(message)
            | Self::Limit(message)
            | Self::InvalidMetadata(message)
            | Self::Persistence(message)
            | Self::Watch(message) => formatter.write_str(message),
            Self::Source {
                source,
                line,
                message,
            } => {
                write!(formatter, "{source}")?;
                if let Some(line) = line {
                    write!(formatter, ":{line}")?;
                }
                write!(formatter, ": {message}")
            }
        }
    }
}

impl std::error::Error for InventoryError {}

#[derive(Default)]
struct Draft {
    hostname: Option<String>,
    username: Option<String>,
    port: Option<u16>,
    proxy_jump_configured: bool,
    identity_hint: IdentityHint,
    source: SourceKind,
}

struct ScanState {
    limits: InventoryLimits,
    files: usize,
    bytes: usize,
    visited: BTreeSet<PathBuf>,
    watched: BTreeSet<PathBuf>,
    drafts: BTreeMap<String, Draft>,
    diagnostics: Vec<Diagnostic>,
}

pub fn scan_inventory(
    grants: &[InventoryGrant],
    limits: InventoryLimits,
    generation: u64,
) -> Result<ScanOutcome, InventoryError> {
    let limits = limits.validate()?;
    let mut state = ScanState {
        limits,
        files: 0,
        bytes: 0,
        visited: BTreeSet::new(),
        watched: BTreeSet::new(),
        drafts: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    for grant in grants {
        for entry in &grant.entries {
            state.parse_file(grant, entry, 0, Vec::new())?;
        }
    }
    let records = state
        .drafts
        .into_iter()
        .map(|(alias, draft)| ConnectionRecord {
            id: format!("openssh:{alias}"),
            alias,
            hostname: draft.hostname,
            username: draft.username,
            port: draft.port,
            proxy_jump_configured: draft.proxy_jump_configured,
            identity_hint: draft.identity_hint,
            source: draft.source,
        })
        .collect::<Vec<_>>();
    for record in &records {
        record.validate()?;
    }
    Ok(ScanOutcome {
        snapshot: InventorySnapshot {
            generation,
            records,
            observed_files: state.files,
            observed_bytes: state.bytes,
        },
        watched_files: state.watched.into_iter().collect(),
        diagnostics: state.diagnostics,
    })
}

impl ScanState {
    fn parse_file(
        &mut self,
        grant: &InventoryGrant,
        path: &Path,
        depth: usize,
        mut aliases: Vec<String>,
    ) -> Result<Vec<String>, InventoryError> {
        if depth > self.limits.max_include_depth {
            return Err(source_error(grant, None, "include depth limit exceeded"));
        }
        reject_symlink(path, grant.label())?;
        let canonical =
            fs::canonicalize(path).map_err(|error| source_io(grant, None, error))?;
        require_inside(&grant.root, &canonical)?;
        validate_source_permissions(grant, &canonical)?;
        self.watched.insert(canonical.clone());
        if !self.visited.insert(canonical.clone()) {
            self.diagnostics.push(Diagnostic {
                source: grant.label.clone(),
                line: None,
                message: "include cycle or repeated include skipped".into(),
            });
            return Ok(aliases);
        }
        self.files = self
            .files
            .checked_add(1)
            .ok_or_else(|| source_error(grant, None, "file counter overflow"))?;
        if self.files > self.limits.max_files {
            return Err(source_error(grant, None, "included file limit exceeded"));
        }
        let metadata =
            fs::metadata(&canonical).map_err(|error| source_io(grant, None, error))?;
        if !metadata.is_file() {
            return Err(source_error(grant, None, "source is not a regular file"));
        }
        let bytes = read_bounded(
            &canonical,
            &metadata,
            self.limits.max_file_bytes,
            grant.label(),
        )?;
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| source_error(grant, None, "byte counter overflow"))?;
        if self.bytes > self.limits.max_total_bytes {
            return Err(source_error(grant, None, "aggregate byte limit exceeded"));
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| source_error(grant, None, "source is not valid UTF-8"))?;

        for (index, raw_line) in text.lines().enumerate() {
            let line_number = index + 1;
            if raw_line.len() > self.limits.max_line_bytes {
                return Err(source_error(
                    grant,
                    Some(line_number),
                    "line length limit exceeded",
                ));
            }
            let tokens = tokenize(raw_line)
                .map_err(|message| source_error(grant, Some(line_number), &message))?;
            if tokens.is_empty() {
                continue;
            }
            let (keyword, values) = split_keyword_values(&tokens);
            if values
                .iter()
                .any(|value| value.len() > self.limits.max_value_bytes)
            {
                return Err(source_error(
                    grant,
                    Some(line_number),
                    "display value limit exceeded",
                ));
            }
            match keyword.as_str() {
                "host" => {
                    aliases = values
                        .iter()
                        .filter(|value| concrete_alias(value))
                        .cloned()
                        .collect();
                    for alias in &aliases {
                        if !self.drafts.contains_key(alias) {
                            if self.drafts.len() >= self.limits.max_aliases {
                                return Err(source_error(
                                    grant,
                                    Some(line_number),
                                    "concrete alias limit exceeded",
                                ));
                            }
                            self.drafts.insert(
                                alias.clone(),
                                Draft {
                                    source: grant.kind.source(),
                                    ..Draft::default()
                                },
                            );
                        }
                    }
                }
                "match" => {
                    aliases.clear();
                    self.diagnostics.push(Diagnostic {
                        source: grant.label.clone(),
                        line: Some(line_number),
                        message: "conditional Match block excluded from static inventory"
                            .into(),
                    });
                }
                "include" => {
                    for value in values {
                        for include in expand_include(
                            grant,
                            &canonical,
                            &value,
                            line_number,
                            self.limits.max_files.saturating_sub(self.files),
                        )? {
                            aliases =
                                self.parse_file(grant, &include, depth + 1, aliases)?;
                        }
                    }
                }
                "hostname" => set_first(
                    &mut self.drafts,
                    &aliases,
                    |draft| &mut draft.hostname,
                    values.first(),
                ),
                "user" => set_first(
                    &mut self.drafts,
                    &aliases,
                    |draft| &mut draft.username,
                    values.first(),
                ),
                "port" => {
                    if let Some(value) = values.first() {
                        let port = value.parse::<u16>().map_err(|_| {
                            source_error(grant, Some(line_number), "invalid static port")
                        })?;
                        if port == 0 {
                            return Err(source_error(
                                grant,
                                Some(line_number),
                                "invalid static port",
                            ));
                        }
                        for alias in &aliases {
                            if let Some(draft) = self.drafts.get_mut(alias) {
                                draft.port.get_or_insert(port);
                            }
                        }
                    }
                }
                "proxyjump" => {
                    for alias in &aliases {
                        if let Some(draft) = self.drafts.get_mut(alias) {
                            draft.proxy_jump_configured = true;
                        }
                    }
                }
                "identityfile" => {
                    for alias in &aliases {
                        if let Some(draft) = self.drafts.get_mut(alias) {
                            draft.identity_hint = IdentityHint::FileReferencePresent;
                        }
                    }
                }
                "certificatefile" => {
                    for alias in &aliases {
                        if let Some(draft) = self.drafts.get_mut(alias) {
                            draft.identity_hint =
                                IdentityHint::CertificateReferencePresent;
                        }
                    }
                }
                "pkcs11provider" | "securitykeyprovider" => {
                    for alias in &aliases {
                        if let Some(draft) = self.drafts.get_mut(alias) {
                            draft.identity_hint =
                                IdentityHint::HardwareOrProviderReferencePresent;
                        }
                    }
                }
                "proxycommand" | "localcommand" | "remotecommand" => {
                    self.diagnostics.push(Diagnostic {
                        source: grant.label.clone(),
                        line: Some(line_number),
                        message: format!(
                            "{keyword} excluded; Automexia never evaluates executable SSH directives"
                        ),
                    });
                }
                _ => {}
            }
        }
        Ok(aliases)
    }
}

fn set_first(
    drafts: &mut BTreeMap<String, Draft>,
    aliases: &[String],
    field: impl Fn(&mut Draft) -> &mut Option<String>,
    value: Option<&String>,
) {
    if let Some(value) = value {
        for alias in aliases {
            if let Some(draft) = drafts.get_mut(alias) {
                field(draft).get_or_insert_with(|| value.clone());
            }
        }
    }
}

fn split_keyword_values(tokens: &[String]) -> (String, Vec<String>) {
    let first = &tokens[0];
    if let Some((keyword, value)) = first.split_once('=') {
        let mut values = Vec::with_capacity(tokens.len());
        if !value.is_empty() {
            values.push(value.to_string());
        }
        values.extend(tokens[1..].iter().cloned());
        (keyword.to_ascii_lowercase(), values)
    } else {
        (first.to_ascii_lowercase(), tokens[1..].to_vec())
    }
}

fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == char::from(92) {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character == '#' && !quoted {
            break;
        } else if character.is_whitespace() && !quoted {
            if !current.is_empty() {
                result.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if escaped || quoted {
        return Err("unterminated quote or escape".into());
    }
    if !current.is_empty() {
        result.push(current);
    }
    Ok(result)
}

fn concrete_alias(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('!')
        && !value.starts_with('-')
        && !value.contains(['*', '?', '[', ']'])
        && !value.contains(['$', '%'])
        && !value.as_bytes().contains(&0x60)
        && !value.chars().any(char::is_whitespace)
}

fn expand_include(
    grant: &InventoryGrant,
    source: &Path,
    value: &str,
    line: usize,
    remaining: usize,
) -> Result<Vec<PathBuf>, InventoryError> {
    if value.contains(['$', '%', '~']) || value.as_bytes().contains(&0x60) {
        return Err(source_error(
            grant,
            Some(line),
            "dynamic include expansion is not permitted",
        ));
    }
    let candidate = Path::new(value);
    let candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        source
            .parent()
            .ok_or_else(|| source_error(grant, Some(line), "source has no parent"))?
            .join(candidate)
    };
    let pattern = candidate.to_string_lossy();
    let options = MatchOptions {
        case_sensitive: cfg!(not(windows)),
        require_literal_separator: true,
        require_literal_leading_dot: true,
    };
    let paths = glob_with(&pattern, options)
        .map_err(|_| source_error(grant, Some(line), "invalid include pattern"))?;
    let mut expanded = Vec::new();
    for entry in paths {
        if expanded.len() >= remaining {
            return Err(source_error(
                grant,
                Some(line),
                "included file limit exceeded",
            ));
        }
        let entry = entry
            .map_err(|_| source_error(grant, Some(line), "include traversal failed"))?;
        reject_symlink(&entry, grant.label())?;
        let canonical = fs::canonicalize(&entry)
            .map_err(|error| source_io(grant, Some(line), error))?;
        require_inside(&grant.root, &canonical)?;
        expanded.push(canonical);
    }
    expanded.sort();
    expanded.dedup();
    Ok(expanded)
}

fn read_bounded(
    path: &Path,
    metadata: &Metadata,
    limit: usize,
    source: &str,
) -> Result<Vec<u8>, InventoryError> {
    if metadata.len() > limit as u64 {
        return Err(InventoryError::Source {
            source: source.into(),
            line: None,
            message: "file byte limit exceeded".into(),
        });
    }
    let file = File::open(path).map_err(|error| InventoryError::io(source, error))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| InventoryError::io(source, error))?;
    if bytes.len() > limit {
        return Err(InventoryError::Source {
            source: source.into(),
            line: None,
            message: "file changed while reading and exceeded its byte limit".into(),
        });
    }
    Ok(bytes)
}

fn canonical_nonsymlink(path: &Path, label: &str) -> Result<PathBuf, InventoryError> {
    reject_symlink(path, label)?;
    fs::canonicalize(path).map_err(|error| InventoryError::io(label, error))
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), InventoryError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| InventoryError::io(label, error))?;
    if metadata.file_type().is_symlink() {
        return Err(InventoryError::Source {
            source: label.into(),
            line: None,
            message: "symbolic links are not accepted by the inventory boundary".into(),
        });
    }
    Ok(())
}

fn require_inside(root: &Path, path: &Path) -> Result<(), InventoryError> {
    if !path.starts_with(root) {
        return Err(InventoryError::Grant(
            "source resolves outside its exact filesystem grant".into(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn validate_source_permissions(
    grant: &InventoryGrant,
    path: &Path,
) -> Result<(), InventoryError> {
    use std::os::unix::fs::MetadataExt;

    // SAFETY: geteuid has no preconditions and does not retain resources.
    let expected_uid = match grant.kind {
        GrantKind::User => unsafe { libc::geteuid() },
        GrantKind::System => 0,
    };
    let mut cursor = Some(path);
    while let Some(candidate) = cursor {
        reject_symlink(candidate, grant.label())?;
        let metadata =
            fs::metadata(candidate).map_err(|error| source_io(grant, None, error))?;
        if metadata.uid() != expected_uid || metadata.mode() & 0o022 != 0 {
            return Err(source_error(
                grant,
                None,
                "source ownership or writable permission is unsafe",
            ));
        }
        if candidate == grant.root {
            break;
        }
        cursor = candidate.parent();
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_source_permissions(
    _grant: &InventoryGrant,
    _path: &Path,
) -> Result<(), InventoryError> {
    Ok(())
}

fn source_error(
    grant: &InventoryGrant,
    line: Option<usize>,
    message: &str,
) -> InventoryError {
    InventoryError::Source {
        source: grant.label.clone(),
        line,
        message: message.into(),
    }
}

fn source_io(
    grant: &InventoryGrant,
    line: Option<usize>,
    error: io::Error,
) -> InventoryError {
    InventoryError::Source {
        source: grant.label.clone(),
        line,
        message: format!("filesystem operation failed ({:?})", error.kind()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Write;

    fn write(path: &Path, contents: &str) {
        let mut file = File::create(path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    fn grant(root: &Path, entry: &Path) -> InventoryGrant {
        InventoryGrant::new("user-config", root, [entry], GrantKind::User).unwrap()
    }

    #[test]
    fn indexes_only_concrete_static_aliases_and_first_values() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        write(
            &config,
            r#"
Host prod *.dynamic !blocked
  HostName first.example
  HostName ignored.example
  User deploy
  Port 2222
  IdentityFile ~/.ssh/id_ed25519
  ProxyCommand dangerous %h
Match exec "touch /tmp/never"
  HostName never.example
Host staging
  HostName staging.example
"#,
        );
        let outcome = scan_inventory(
            &[grant(root.path(), &config)],
            InventoryLimits::default(),
            7,
        )
        .unwrap();
        assert_eq!(outcome.snapshot.generation, 7);
        assert_eq!(outcome.snapshot.records.len(), 2);
        let prod = outcome
            .snapshot
            .records
            .iter()
            .find(|record| record.alias == "prod")
            .unwrap();
        assert_eq!(prod.hostname.as_deref(), Some("first.example"));
        assert_eq!(prod.username.as_deref(), Some("deploy"));
        assert_eq!(prod.port, Some(2222));
        assert_eq!(prod.identity_hint, IdentityHint::FileReferencePresent);
        assert!(outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("never evaluates")));
    }

    #[test]
    fn expands_bounded_includes_in_lexical_order() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        let fragments = root.path().join("conf.d");
        fs::create_dir(&fragments).unwrap();
        write(&config, "Include conf.d/*.conf\n");
        write(
            &fragments.join("20-second.conf"),
            "Host beta\n HostName beta\n",
        );
        write(
            &fragments.join("10-first.conf"),
            "Host alpha\n HostName alpha\n",
        );
        let outcome = scan_inventory(
            &[grant(root.path(), &config)],
            InventoryLimits::default(),
            1,
        )
        .unwrap();
        let aliases = outcome
            .snapshot
            .records
            .iter()
            .map(|record| record.alias.as_str())
            .collect::<Vec<_>>();
        assert_eq!(aliases, ["alpha", "beta"]);
        assert_eq!(outcome.snapshot.observed_files, 3);
        assert_eq!(outcome.watched_files.len(), 3);
    }

    #[test]
    fn rejects_oversize_dynamic_include_and_redacts_paths() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        write(&config, "Include $HOME/.ssh/private\n");
        let error = scan_inventory(
            &[grant(root.path(), &config)],
            InventoryLimits::default(),
            1,
        )
        .unwrap_err();
        assert!(!error
            .to_string()
            .contains(root.path().to_string_lossy().as_ref()));
        assert!(!error.to_string().contains("private"));

        write(&config, &"x".repeat(1025));
        let limits = InventoryLimits {
            max_file_bytes: 1024,
            max_total_bytes: 1024,
            ..InventoryLimits::default()
        };
        assert!(scan_inventory(&[grant(root.path(), &config)], limits, 1).is_err());
    }

    #[test]
    fn rejects_out_of_grant_includes_zero_ports_and_option_aliases() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("granted");
        fs::create_dir(&root).unwrap();
        let outside = parent.path().join("outside.conf");
        write(&outside, "Host outside\n");
        let config = root.join("config");
        write(
            &config,
            &format!(
                "Include {}\n",
                outside.to_string_lossy().replace(char::from(92), "/")
            ),
        );
        let error =
            scan_inventory(&[grant(&root, &config)], InventoryLimits::default(), 1)
                .unwrap_err();
        assert!(error.to_string().contains("outside"));
        assert!(!error
            .to_string()
            .contains(parent.path().to_string_lossy().as_ref()));

        write(
            &config,
            "Host -oProxyCommand=bad\nHostName never\nHost safe\nPort 0\n",
        );
        assert!(
            scan_inventory(&[grant(&root, &config)], InventoryLimits::default(), 1)
                .is_err()
        );
    }

    #[test]
    fn alias_and_include_depth_limits_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        write(&config, "Host one two three\n");
        let limits = InventoryLimits {
            max_aliases: 2,
            ..InventoryLimits::default()
        };
        assert!(scan_inventory(&[grant(root.path(), &config)], limits, 1).is_err());

        for depth in 0..4 {
            let next = if depth == 3 {
                "Host final".to_string()
            } else {
                format!("Include {}\n", depth + 1)
            };
            write(&root.path().join(depth.to_string()), &next);
        }
        let limits = InventoryLimits {
            max_include_depth: 2,
            ..InventoryLimits::default()
        };
        assert!(
            scan_inventory(&[grant(root.path(), &root.path().join("0"))], limits, 1)
                .is_err()
        );
    }

    #[test]
    fn include_cycles_are_bounded_and_reported() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first");
        let second = root.path().join("second");
        write(&first, "Include second\nHost first\n");
        write(&second, "Include first\nHost second\n");
        let outcome =
            scan_inventory(&[grant(root.path(), &first)], InventoryLimits::default(), 1)
                .unwrap();
        assert_eq!(outcome.snapshot.records.len(), 2);
        assert!(outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("cycle")));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_group_writable_sources() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        write(&config, "Host prod\n");
        fs::set_permissions(&config, fs::Permissions::from_mode(0o620)).unwrap();
        assert!(scan_inventory(
            &[grant(root.path(), &config)],
            InventoryLimits::default(),
            1
        )
        .is_err());
    }

    proptest::proptest! {
        #[test]
        fn arbitrary_sources_never_panic(input in proptest::collection::vec(any::<u8>(), 0..32768)) {
            let root = tempfile::tempdir().unwrap();
            let config = root.path().join("config");
            fs::write(&config, input).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
            }
            let _ = scan_inventory(
                &[grant(root.path(), &config)],
                InventoryLimits::default(),
                1,
            );
        }
    }
}
