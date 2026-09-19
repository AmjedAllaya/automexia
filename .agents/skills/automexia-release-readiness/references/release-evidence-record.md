# Release evidence record

## Candidate

- Commit:
- Branch/tag intent:
- Release channel and version:
- Source and working-tree state:
- Toolchain and dependency identity:
- Previous release/baseline:
- Authorized actions:

## Scope

- Included changes:
- Excluded changes:
- Compatibility and migration impact:
- Known limitations:
- Disable, rollback, and uninstall path:

## Gate ledger

For each gate record status, command or procedure, environment, fresh artifact, result, failure investigation, and remaining action.

| Gate | Status | Environment | Evidence/artifact | Remaining action |
|---|---|---|---|---|
| Source and dependency trust | | | | |
| Tests and assurance | | | | |
| Security and privacy | | | | |
| Performance and resources | | | | |
| Windows native/package | | | | |
| macOS native/package | | | | |
| Linux/BSD native/package | | | | |
| Accessibility | | | | |
| Signing/notarization | | | | |
| SBOM/provenance/checksums | | | | |
| Public documentation | | | | |
| Rehearsal/rollback | | | | |
| Hosted protection/governance | | | | |
| Publication verification | | | | |

Use implemented, passed, failed, partial, external, or not applicable with a reason. Never use passed for unavailable evidence.

## Artifact inventory

For every final package and companion artifact record:

- filename and package type;
- target OS and architecture;
- byte size and cryptographic digest;
- source revision and build identity;
- signature/notarization identity and verification result;
- SBOM and provenance reference;
- install/upgrade/uninstall result;
- privacy scan result;
- publication destination after authorization.

## Failure and flake ledger

Retain every failure that affected the candidate until its cause and disposition are documented. Include timeouts, retries, resource exhaustion, nondeterminism, hosted failures, signing failures, package differences, and native-only defects.

## Decision

Choose exactly one:

- locally prepared;
- package-validated;
- native-evidence partial;
- ready for signing;
- ready for authorized publication;
- published and verified;
- blocked.

State the evidence supporting that status, every external gate, the exact next action, required authorization, and rollback owner.
