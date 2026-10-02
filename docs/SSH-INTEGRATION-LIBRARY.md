# SSH integration library

Status: private, non-executing development source. Enhanced SSH is disabled.
Ordinary system `ssh` remains the supported way to connect from a terminal pane.
No command on this page enables managed launch or the full remote UI experience.

## Usage

`automexia ssh-integration status` reports current availability.
`automexia ssh-integration inspect -- host-alias` produces redacted JSON from
explicitly supplied assumptions. It does not evaluate SSH configuration, contact
a server, inspect credentials or authorize execution. Login startup and unknown
configurations intentionally do not produce enhanced candidates.

The preview-only options are documented in [CLI reference](CLI-REFERENCE.md#ssh-integration-planning).
Do not run experimental candidate source on a remote host as an activation shortcut.

## Contracts

Native connection review stays in `automexia-connectivity`; the application
retains launch/cleanup ownership and its disabled gate. The library performs
bounded invocation classification, source construction and advisory generation-
bound metadata decoding. The existing VT owner retains OSC parsing and Base64
framing. A remote directory cannot become a local path by conversion in this crate.

The Bash candidate is non-login and session-only. It preserves native prompt
builders and original status for native prompt hooks. Conflicting readonly,
nameref or associative state is not overwritten. Metadata goes to terminal stderr,
never redirected stdout. Private setup permissions do not affect the user's
shell or .bashrc. Optional CWD encoding uses installed Base64 only when CWD changes;
failure revokes that capability. No C/D execution timing or command status is claimed.

## Architecture

See [ADR 0080](adr/0080-nonexecuting-ssh-planning-boundary.md). The crate is
registered in the existing xtask dependency boundary, repository validator and
coupled feature-assurance records. There is no second review facade, transport,
process supervisor, terminal engine or scaffolding framework.

## Verification

Run `python tools/ci/check_ssh_library.py` for focused model/CLI/policy checks.
On a Unix-native checkout, `python tools/ci/check_ssh_library.py --shell-only --require-bash`
builds the actual Rust exporter and runs controlling-PTY regressions against its
output. Python discovery alone tests the resource and explicitly skips bootstrap
cases without that exporter; those skips are not bootstrap evidence.

The existing benchmark owner is `apps/automexia-terminal/benches/automexia_services.rs`.
`python tools/ci/check_ssh_library.py --bench-smoke` checks benchmark assertions.
`python tools/ci/check_ssh_library.py --bench` measures the `ssh_integration` group.
Smoke is not measurement. Absolute timing depends on the host; retain comparable
Criterion results under the existing development-cache policy. No physical-input
latency or live-network benchmark is claimed. Use `cargo ready` for the complete
contributor gate; native Windows and WSL checkouts/build artifacts remain separate.

## Current limits

No enhanced SSH execution, live remote metadata routing, remote-aware pane cloning,
remote context providers, terminfo deployment, or full prompt/status parity is
implemented. Other shell dialects are not active adapters. Native SSH, Windows/WSL,
macOS, authentication and compositor evidence remain separate and unavailable until
executed. The planning and shell tests cannot approve protected activation.
