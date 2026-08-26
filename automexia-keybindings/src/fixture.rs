use crate::{
    parse_binding_lines, BindingOrigin, BindingSpec, ParseError, MAX_FIXTURE_BYTES,
};
use serde::{Deserialize, Serialize};

pub const GHOSTTY_1_3_LINUX_KEYBINDS: &str =
    include_str!("../fixtures/ghostty/1.3.1/linux/keybinds.txt");
pub const GHOSTTY_1_3_ACTIONS: &str =
    include_str!("../fixtures/ghostty/1.3.1/linux/actions.txt");
pub const GHOSTTY_1_3_LINUX_PROVENANCE: &str =
    include_str!("../fixtures/ghostty/1.3.1/linux/provenance.json");

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FixtureUpstream {
    pub project: String,
    pub tag: String,
    pub commit: String,
    pub release_date: String,
    pub source_url: String,
    pub source_sha256: String,
    pub source_signature_verified: bool,
    pub minisign_public_key: String,
    pub license: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FixtureGenerator {
    pub generated_at: String,
    pub host: String,
    pub zig_version: String,
    pub zig_archive_sha256: String,
    pub build_mode: String,
    pub app_runtime: String,
    pub version_string: String,
    pub binary_sha256: String,
    pub build_flags: Vec<String>,
    pub commands: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FixtureOutput {
    pub file: String,
    pub lines: usize,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FixtureOutputs {
    pub keybinds: FixtureOutput,
    pub actions: FixtureOutput,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FixtureProvenance {
    pub schema_version: u32,
    pub upstream: FixtureUpstream,
    pub generator: FixtureGenerator,
    pub outputs: FixtureOutputs,
}

pub fn ghostty_1_3_linux_provenance() -> Result<FixtureProvenance, serde_json::Error> {
    serde_json::from_str(GHOSTTY_1_3_LINUX_PROVENANCE)
}

pub fn parse_ghostty_keybind_fixture(
    fixture: &str,
    origin: BindingOrigin,
) -> Result<Vec<BindingSpec>, ParseError> {
    if fixture.is_empty() || fixture.len() > MAX_FIXTURE_BYTES {
        return Err(ParseError::LineTooLong);
    }
    let mut lines = Vec::new();
    for raw in fixture.lines() {
        let line = raw
            .strip_prefix("keybind = ")
            .ok_or(ParseError::InvalidFormat)?;
        lines.push(line);
    }
    parse_binding_lines(lines, origin)
}

pub fn ghostty_1_3_linux_bindings() -> Result<Vec<BindingSpec>, ParseError> {
    parse_ghostty_keybind_fixture(GHOSTTY_1_3_LINUX_KEYBINDS, BindingOrigin::Profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, GHOSTTY_1_3_COMMIT, GHOSTTY_1_3_TAG};
    use sha2::{Digest, Sha256};

    fn sha256(input: &[u8]) -> String {
        use std::fmt::Write as _;

        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(input) {
            write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
        }
        output
    }

    #[test]
    fn checked_in_fixture_has_verified_provenance_and_exact_digests() {
        let provenance = ghostty_1_3_linux_provenance().unwrap();
        assert_eq!(provenance.schema_version, 1);
        assert_eq!(provenance.upstream.tag, GHOSTTY_1_3_TAG);
        assert_eq!(provenance.upstream.commit, GHOSTTY_1_3_COMMIT);
        assert!(provenance.upstream.source_signature_verified);
        assert_eq!(provenance.outputs.keybinds.lines, 72);
        assert_eq!(provenance.outputs.actions.lines, 85);
        assert_eq!(
            sha256(GHOSTTY_1_3_LINUX_KEYBINDS.as_bytes()),
            provenance.outputs.keybinds.sha256
        );
        assert_eq!(
            sha256(GHOSTTY_1_3_ACTIONS.as_bytes()),
            provenance.outputs.actions.sha256
        );
    }

    #[test]
    fn exact_linux_defaults_parse_and_compile_without_collisions() {
        let bindings = ghostty_1_3_linux_bindings().unwrap();
        assert_eq!(bindings.len(), 72);
        let report = compile(&bindings);
        assert!(
            report.is_valid(),
            "compile diagnostics: {:?}",
            report.diagnostics
        );
        assert_eq!(report.registry.unwrap().stats().bindings, 72);
    }

    #[test]
    fn fixture_prefix_and_capacity_are_fail_closed() {
        assert_eq!(
            parse_ghostty_keybind_fixture("ctrl+a=quit\n", BindingOrigin::Profile),
            Err(ParseError::InvalidFormat)
        );
        assert_eq!(
            parse_ghostty_keybind_fixture(
                &"x".repeat(MAX_FIXTURE_BYTES + 1),
                BindingOrigin::Profile
            ),
            Err(ParseError::LineTooLong)
        );
    }
}
