# Public architecture decision index

This index lists decisions needed to understand the public free terminal.
Unreleased advanced and commercial ADRs are maintained privately and are not
listed or summarized here.

| ADR | Public decision |
|---|---|
| [0001](adr/0001-standalone-product-boundary.md) | Standalone product identity and boundary |
| [0002](adr/0002-non-destructive-config-migration.md) | Non-destructive configuration migration |
| [0003](adr/0003-extension-capability-and-threading.md) | Extension capability and threading boundary |
| [0004](adr/0004-native-persistent-operational-chrome.md) | Native persistent terminal chrome |
| [0005](adr/0005-storage-bounded-build-workflow.md) | Storage-bounded contributor workflow |
| [0006](adr/0006-prompt-context-and-workspace-actions.md) | Bounded prompt context and session actions |
| [0007](adr/0007-pane-local-session-tabs.md) | Pane-local tabs with independent PTYs |
| [0008](adr/0008-pane-local-operational-footer.md) | Passive pane-local footer |
| [0009](adr/0009-launch-time-shell-provisioning.md) | Launch-time shell integration |
| [0010](adr/0010-ghostty-keyboard-compatibility.md) | Explicit keyboard compatibility |
| [0011](adr/0011-restore-automexia-keyboard-defaults.md) | Automexia keyboard defaults remain primary |
| [0012](adr/0012-first-party-ssh-and-session-launch-boundary.md) | System OpenSSH and terminal session ownership |
| [0013](adr/0013-renderer-independent-accessibility-model.md) | Renderer-independent accessibility semantics |
| [0014](adr/0014-explicit-bounded-image-quick-look.md) | Explicit bounded local image preview |
| [0015](adr/0015-shell-native-completion-and-typed-quick-actions.md) | Shell-native completion and explicit insertion |
| [0016](adr/0016-final-artifact-release-trust.md) | Final-artifact release trust |
| [0017](adr/0017-session-only-shell-integration.md) | Session-only shell integration by default |
| [0018](adr/0018-terminal-first-remote-operations.md) | Terminal-first system remote interoperability |
| [0019](adr/0019-acyclic-owned-crate-dependencies.md) | Acyclic owned-crate dependencies |
| [0020](adr/0020-hybrid-build-wrap-adopt-boundary.md) | Build, wrap, and adopt boundary |
| [0026](adr/0026-versioned-ghostty-keybinding-profiles.md) | Versioned compatibility profiles |
| [0027](adr/0027-redacted-compatibility-inspector.md) | Redacted compatibility inspector |
| [0028](adr/0028-bounded-parked-pty-topology-history.md) | Bounded closed-tab topology history |
| [0031](adr/0031-versioned-hosted-ci-and-repository-protection.md) | Hosted CI and repository protection |
| [0036](adr/0036-application-owned-runtime-user-preferences.md) | Application-owned appearance preferences |
| [0037](adr/0037-public-binary-release-distribution.md) | Public binary release distribution |
| [0039](adr/0039-content-addressed-development-cache.md) | Content-addressed development cache lifecycle |

## When an ADR is required

Add or supersede an ADR when a public change affects dependency direction,
persistence, process/thread ownership, capabilities, protocol interpretation,
compatibility, accessibility, or a long-lived resource invariant.

Private feature decisions are created and reviewed outside the public
documentation tree until an explicit publication decision.

## D0 traceability

D0 is governed by ADR 0012 and preserves the split between application-owned
terminal sessions and system OpenSSH-owned network and credential authority.
