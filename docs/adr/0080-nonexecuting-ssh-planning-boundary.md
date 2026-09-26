# ADR 0080: Non-executing SSH planning library

Status: proposed for maintainer review; implementation is non-activating.
This record grants no remote execution permission and does not amend the
structured-action shell-evaluation prohibition or protected activation gates.

## Current ownership

System OpenSSH remains the only SSH transport. The existing connectivity model
owns native connection validation/review and its destination budget. The existing
application broker owns approved execution; VT/PTY and renderer ownership is
unchanged. The inventory extension still only discovers granted public records.

The private `automexia-ssh-integration` crate contains effect-free enhancement
classification and scoped, untrusted metadata contracts. Its application CLI is
read-only. An enhanced candidate cannot be appended to an approved native binding.
Its remote-shell source is experimental data, not an executable structured action.
No local `sh -c`, remote SSH process, automatic profile edit or new launch path is
introduced by this change. The redundant inner shell layer was removed.

## Reuse and placement

A module in `automexia-connectivity` was considered. The narrow private crate
keeps shell-source resource churn and shell compatibility tests independently
compilable from the established native-connection model. This is a first-party
library, not an ecosystem component, inventory extension, daemon or public SDK.
If those independent boundaries cease to be useful, merge it back into the existing
owner rather than multiplying facades. No speculative cross-product consumer is
claimed. The native review path has no new forwarding wrapper.

Encoding uses workspace Base64, property tests use workspace proptest, and
Criterion workloads extend the existing application benchmark target. Process
supervision reuses `tools/ci/qa_process.py`. No new external package is selected.
The supplied Bash core is a different remote, profile-preserving compatibility
adapter, not a copy of the local CLI/listing integration. It has one canonical
resource embedded with `include_str!`, exercised by its actual Rust exporter.

## Invariants and current evidence scope

The crate has no I/O, clocks, threads, global services, credentials or terminal
framing. The existing VT parser owns OSC/Base64 decoding. Remote directory data
has explicit pane/generation identity and is never emitted as an unscoped OSC 7
local path. Negotiation cannot exceed its caller-supplied capability ceiling.
The remote core preserves native hooks/status, uses private umask only in setup
subshells, declines conflicting names, emits on terminal stderr and caches CWD
encoding. Removal/replacement of the resource never authorizes a launch.

Architecture validation uses the canonical Cargo metadata registry plus recursive
source drift checks. The checks are not a security sandbox. Mutations cover
platform/build dependencies, nested modules, gate changes and duplicate owners.
The coupled assurance ledgers name the tests and benchmark owner. Passing model,
Bash or benchmark tests is not live SSH, native GUI or release approval.

## Compatibility and rollback

The CLI remains a non-executing `ssh-integration status|inspect` interface.
The advisory readiness version stays 1; the new optional scoped CWD value has its
own `AMXSSHCWD1` prefix. No live reader or persisted record is migrated. Unsupported
shells and configurations remain non-enhanced planning outcomes. The update
installer journals the pre-update files; rollback returns to that exact snapshot
without discarding unrelated work. Existing installer receipts are retained.
