# Feature catalog

This catalog is the human-readable inventory of shipped Automexia v0.4
surfaces. The machine-enforced source is
[`tests/assurance/feature-matrix.json`](../tests/assurance/feature-matrix.json),
which also owns quality and native-platform evidence.

`PR` means deterministic checks run on pull requests. `Controlled` means the
feature additionally needs named hardware, privileges, credentials, or a real
display session before release. A platform marked `External` is supported by
portable code but still needs that controlled native evidence; it is not an
inferred pass.

## User-facing terminal

| Capability | What is shipped | Platform evidence | Canonical docs | Why this design |
|---|---|---|---|---|
| Terminal core | VT/CSI/OSC/DCS parsing, Unicode grid, scrollback, search, reflow, mouse and keyboard selection, Sixel/Kitty/iTerm2 protocol state | PR: Windows/Linux/macOS; nightly fuzz | [UX](LIQUID-HACKER-UX.md), [testing](TESTING.md#terminal-conformance) | [Architecture](ARCHITECTURE.md#vt-control-string-trust-boundary) |
| PTY lifecycle | Unix PTY and Windows ConPTY launch, resize coalescing, ordered input, child exit, teardown, and high-throughput handling | PR: all; controlled Windows lifecycle | [Platform support](PLATFORMS.md), [testing](TESTING.md#native-platform-ownership) | [Architecture](ARCHITECTURE.md#interactive-performance-invariants) |
| Rendering and responsive UI | GPU text/images, experimental CPU fallback, fonts, grapheme width, panes, modal composition, pane-local tabs, active outlines, and passive footers | PR layout/shader; controlled native frames | [UX](LIQUID-HACKER-UX.md), [accessibility](ACCESSIBILITY.md) | [ADR 0004](adr/0004-native-persistent-operational-chrome.md), [ADR 0007](adr/0007-pane-local-session-tabs.md), [ADR 0008](adr/0008-pane-local-operational-footer.md) |
| Windows, tabs, panes, and cloning | Independent OS windows, window tabs, pane-local tabs, fresh splits, exact session clones, clipboard, mouse routing, search, and command palette | PR: all; controlled Windows/WSL | [Keyboard](KEYBOARD.md), [UX](LIQUID-HACKER-UX.md) | [ADR 0006](adr/0006-prompt-context-and-workspace-actions.md), [ADR 0007](adr/0007-pane-local-session-tabs.md) |
| DevOps prompt context | Per-command OS, user, Git, Docker, Kubernetes, cloud, Terraform, environment, complete path, exit state, and duration metadata | PR: Windows/Linux/macOS | [UX](LIQUID-HACKER-UX.md#per-pane-operational-context), [shell integration](SHELL-INTEGRATION.md) | [ADR 0006](adr/0006-prompt-context-and-workspace-actions.md) |
| Shell integration and listings | Automatic PowerShell, CMD, WSL, Bash, Zsh, and Fish provisioning; semantic OSC metadata; icon/category-aware listings without changing piped objects | PR: native hosts | [Shell integration](SHELL-INTEGRATION.md) | [ADR 0009](adr/0009-launch-time-shell-provisioning.md) |
| Local and protocol images | Sixel, Kitty, iTerm2 inline rendering plus bounded hover/click/selection Quick Look for local raster files | PR decoder/state; controlled native visual/lifetime | [Image previews](IMAGE-PREVIEWS.md) | [ADR 0014](adr/0014-explicit-bounded-image-quick-look.md) |
| Configuration and migration | TOML config, themes, platform overrides, live transactional reload, bounded reads, one-time non-destructive Rio import, side-by-side identity | PR: Windows/Linux/macOS | [Configuration](CONFIGURATION.md), [migration](MIGRATION.md) | [ADR 0001](adr/0001-standalone-product-boundary.md), [ADR 0002](adr/0002-non-destructive-config-migration.md) |

## Internal and contributor surfaces

| Capability | What is shipped | Evidence | Canonical docs | Why this design |
|---|---|---|---|---|
| Extension contracts | Private versioned API, bounded worker/cache runtime, cancellation, session isolation, renderer-neutral UI model | PR models; nightly Miri/sanitizers | [Architecture](ARCHITECTURE.md#core-and-extension-ownership) | [ADR 0003](adr/0003-extension-capability-and-threading.md) |
| OpenSSH inventory foundation | Disabled-by-default static concrete-alias indexing, exact grants, bounded includes, user-only atomic metadata, exact-file watchers, and last-known-good refresh; no launch or network authority | PR: Windows/Linux/macOS; nightly fuzz/benchmark | [OpenSSH inventory](SSH-INVENTORY.md) | [ADR 0012](adr/0012-first-party-ssh-and-session-launch-boundary.md) |
| Native completion and Quick Actions | **CP1-CP3.2 implemented locally:** shell-owned completion; typed private Quick Actions with bounded search/review/dry-run administration/import/export and insert/copy; pure deterministic five-shell compilation; explicit opt-in persistent aliases using private immutable generations; and 11 reviewed static DevOps packs with 33 disabled-by-default typed actions, pure health/update handling, and effect-aware alias denial. CP3.3 imports, workspace actions, secrets, exact launch, and CP5 suggestions remain disabled | PR: model/hostile/quoting/Unicode/transfer/CAS/recovery/worker/palette tests; CP3.0 serializer/native-capture/tamper; CP3.1 transaction/contention/loader/uninstall; CP3.2 inventory/health/update/alias/CLI contracts and mutations; nightly: projection and pack fuzz; controlled: native platform and 30-day benchmarks | [Command Productivity](COMMAND-PRODUCTIVITY.md), [DevOps aliases](DEVOPS-ALIASES.md), [CP3.2 testing](TESTING.md#cp32-reviewed-devops-action-packs), [compatibility](COMMAND-PRODUCTIVITY-COMPATIBILITY.md), [shell integration](SHELL-INTEGRATION.md), [threat model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md) | [ADR 0015](adr/0015-shell-native-completion-and-typed-quick-actions.md) plus a required future CP5 bridge ADR |
| Native and Wasm embedding | Inherited private C and WebAssembly embedding crates retained for compatibility | PR compilation/tests; nightly static analysis | [Architecture](ARCHITECTURE.md#layers), crate READMEs | [ADR 0001](adr/0001-standalone-product-boundary.md) |
| Contributor automation | One-command doctor/check/CI/QA/build/run/package/release workflows, isolated cleanup, identity/provenance/architecture contracts | PR on every host | [CLI reference](CLI-REFERENCE.md), [testing](TESTING.md) | [ADR 0005](adr/0005-storage-bounded-build-workflow.md) |
| Packaging and release | Windows MSI/ZIP, macOS universal app/DMG, Linux DEB/RPM/tar, exact publisher signatures, hardened-runtime notarization, controlled Defender scan, final-package checksums, SBOMs, and attestations | Nightly packages; controlled stable release | [Releasing](../RELEASING.md), [release trust](RELEASE-TRUST.md), [packaging](../packaging/README.md) | [ADR 0016](adr/0016-final-artifact-release-trust.md) |

## Deliberate v0.4 boundaries

Automexia v0.4 does not claim a public extension SDK, third-party extension
downloads, Wasm sandboxing, remote image fetching, SVG/PDF preview, shared live
PTY views, complete Ghostty action parity, provider SDK authentication, preinstalled first-party DevOps aliases, trusted-workspace actions, secret
expansion, or exact Quick Action launch. CP2.2 reviewed insert/copy, CP3.0 pure
compilation, and CP3.1 explicit user-alias persistence are locally implemented;
their stable release claims remain gated by native and controlled evidence.
Those omissions are deliberate security and product boundaries, not hidden
features. The managed SSH/multi-cloud [Connection Hub](CONNECTION-HUB.md) is a
fully specified D5/D6 plan but is likewise not a shipped v0.4 surface. Its
reusable profiles and typed actions are specified in
[SSH connections and automation](SSH-CONNECTION-AUTOMATION.md). The complete
planned command/leader/picker replacement for GUI-oriented remote-management
workflows is [Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md).
It is a roadmap contract, not a shipped `automexia`/`ax` command claim. See the
[roadmap](ROADMAP.md) for sequencing and the
[decision index](DECISIONS.md) for rationale.

BSD is source-compatible/best-effort where Unix code paths apply, but the
release assurance matrix certifies Windows, Linux, and macOS. See
[Platform support](PLATFORMS.md) for the exact claim.
