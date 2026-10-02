# ADR 0086: Pure SSH contracts and explicit application wrapper

Status: implemented. The original non-executing planning boundary is retained;
the explicit `+ssh` command is a separate application-owned execution path.
The managed connection launch gate is unchanged.

## Ownership

System OpenSSH remains the only transport and authentication/configuration
owner. `automexia-connectivity` keeps native reviewed connection contracts.
`automexia-ssh-integration` owns bounded pure classification, effective-config
interpretation, canonical shell generation and advisory remote facts. The
application command reuses its tool resolver/process lifetime owner and inherited
terminal. It never extends an already reviewed native binding.

`ssh-integration status|inspect` stays read-only. Only the explicit wrapper
observes effective configuration or executes fixed bundled startup source. User
arguments stay exact native SSH arguments, never interpolated into bootstrap
source. Unknown account-shell syntax is never guessed.

## Reuse and placement

The private library isolates shell compatibility resources/tests from the native
connectivity model. It is not a public SDK, extension, daemon or SSH implementation.
Existing Base64 encodes bounded frames. Each supported resource is embedded once
and tested through the actual Rust exporter. The existing QA process owner
supplies bounded test execution and cleanup.

The VT parser remains the sole OSC/Base64 owner. Its per-terminal scope stack
moves local metadata and semantic command state aside during remote execution.
The application generates a random local end preimage with platform facilities
and sends only its SHA-256 digest before the child. The VT reuses workspace
SHA-256 to check the end. No new dependency version, credential store or general
authentication protocol is introduced.

## Trust and lifecycle

Remote generation, shell, user, readiness and directory values are advisory.
Remote CWD never enters the local OSC 7 path or a local provider/clone seed.
Local discovery is suspended while a scope is active or quarantined. A forged
end cannot restore local authority. A valid outer end discards unfinished inner
scopes and restores its own state; resets cannot remove the boundary. A killed
wrapper without a valid end fails closed. Nesting is limited to eight, controls
to 192 bytes, and remote fact parsers retain explicit limits.

Prompt identities map into a monotonic pane namespace after remote entry,
preventing remote aid=1 from colliding with its parent's aid=1. Nested command
timers and metadata chronology keep their existing owner. The renderer projects
scoped facts through existing status/shape models without creating another
terminal or provider runtime. Missing metadata never enables host discovery.

## Compatibility and verification

The CLI is additive. Read-only planning retains its existing fields and reports
wrapper availability separately. `AMXSCOPE1`, `AMXSSH1` and scoped path envelopes
are versioned; no persisted record migration is needed. Native/off mode preserves
OpenSSH behavior. Explicit known-shell startup uses temporary session files and
native profiles; unsupported capabilities degrade truthfully.

Architecture checks enforce effect-free dependencies, exact resource ownership,
scoped CWD and the protected managed gate. Tests cover actual generated shells,
native arguments/exit, malformed/stale/nested metadata, local-provider suppression
and isolated SSH runtime paths. A model or shell pass is not native GUI,
server-platform or release evidence.

## Optional temporary discovery

An explicit helper-upload option supplies automatic remote context without
installing an agent or changing profiles. The helper lives in the application
package and reuses its existing process owner and passive detector. The pure SSH
crate owns only bounded upload/request/revision/result contracts and generated
source. One scanner at a time consumes a complete allowlisted snapshot; stale
results cannot cross a revision or session boundary. An unconfirmed scanner
retirement retains the process owner and disables replacement. Revocation
survives coalesced render frames and requires a fresh result after resumption.
This avoids remote provider CLIs and keeps
discovery off prompt, PTY and rendering paths. The CLI/protocol additions are
versioned and opt-in; existing planning and native passthrough retain their
contracts. Interrupted upload or a failed follow-up connection may leave its
private temporary copy, which must be reported truthfully.
