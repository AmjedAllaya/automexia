# Architecture decision index

Architecture Decision Records (ADRs) preserve context, the chosen approach,
alternatives or constraints, verification, and consequences. They explain why
the current code is shaped this way; guides and references explain how to use
it. Accepted ADRs are append-only except for corrections and links. A new
decision supersedes an old one instead of silently rewriting history.

| ADR | Decision | Why it was chosen |
|---|---|---|
| [0001](adr/0001-standalone-product-boundary.md) | Standalone product boundary | Preserve Rio history/private engine names while removing patch/bootstrap coupling and centralizing product identity. |
| [0002](adr/0002-non-destructive-config-migration.md) | Non-destructive Rio migration | One bounded atomic import gives continuity without modifying or coupling Automexia to Rio data. |
| [0003](adr/0003-extension-capability-and-threading.md) | Extension capability/threading boundary | Keep extension IO and failures away from renderer/PTY paths and make capabilities explicit and least-privilege. |
| [0004](adr/0004-native-persistent-operational-chrome.md) | Native persistent operational chrome | Renderer-owned tabs/context survive output and resize without polluting terminal cells or PTY history. |
| [0005](adr/0005-storage-bounded-build-workflow.md) | Storage-bounded build workflow | An isolated exhaustive target plus one reusable app target prevents normal verification from filling contributor disks. |
| [0006](adr/0006-prompt-context-and-workspace-actions.md) | Prompt context and workspace actions | Generation-scoped semantic metadata and explicit fresh-vs-clone actions prevent stale/cross-session state and shortcut ambiguity. |
| [0007](adr/0007-pane-local-session-tabs.md) | Pane-local session tabs | Tabs belong to their pane and own independent PTYs, preserving spatial mental models and failure isolation. |
| [0008](adr/0008-pane-local-operational-footer.md) | Pane-local operational footer | A passive reserved footer gives useful state without buttons, overlaying terminal content, or coupling to shell output. |
| [0009](adr/0009-launch-time-shell-provisioning.md) | Launch-time shell provisioning | A successful launch should deterministically include supported shell features; manual setup is retained only for repair/uninstall. |
| [0010](adr/0010-ghostty-keyboard-compatibility.md) | Ghostty-compatible profile design | Platform-specific compatibility must be deliberate, collision-tested, and feature-aware instead of blindly copying a table. |
| [0011](adr/0011-restore-automexia-keyboard-defaults.md) | Restore Automexia defaults | Existing users preferred the established workspace model; Ghostty compatibility remains opt-in rather than silently changing muscle memory. |
| [0012](adr/0012-first-party-ssh-and-session-launch-boundary.md) | SSH/session launch boundary (proposed) | Proposes exact argv and system OpenSSH with provider-neutral descriptors; no capability is enabled until its protected review and evidence gates pass. |
| [0013](adr/0013-renderer-independent-accessibility-model.md) | Renderer-independent accessibility model | Semantic accessibility state must not depend on pixels, GPU backends, OCR, or one window-system adapter. |
| [0014](adr/0014-explicit-bounded-image-quick-look.md) | Bounded local image Quick Look | IO-free targeting plus bounded asynchronous decode gives direct preview without remote fetches, UI-thread IO, or unbounded cache/queue risk. |
| [0015](adr/0015-shell-native-completion-and-typed-quick-actions.md) | Shell-native completion and typed Quick Actions | Preserve native editor semantics while providing one bounded persistent action model, opt-in alias projections, and brokered exact execution. |
| [0016](adr/0016-final-artifact-release-trust.md) | Final-artifact release trust | Sign and scan only protected final artifacts, attest the bytes users install, and handle false positives through vendor review rather than security exclusions. |

## When an ADR is required

Add or supersede an ADR when a change affects dependency direction,
persistence/migration, thread/process ownership, security capabilities, public
behavior, protocol interpretation, compatibility policy, or a long-lived
performance/resource invariant. Small bug fixes and implementation details do
not need an ADR unless they change one of those contracts.

Use the structure `Context`, `Decision`, `Alternatives`, `Verification`, and
`Consequences`. If an older ADR lacks one heading, preserve it and link a more
complete superseding record rather than changing the historical decision.
Update [Architecture](ARCHITECTURE.md), the feature assurance ledger, tests, and
user reference in the same pull request.
