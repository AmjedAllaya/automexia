# Automexia Terminal roadmap

## v0.3 — architecture foundation

- exact audited terminal-engine pin;
- Automexia application-platform module;
- renderer/extension IO separation;
- bounded extension worker and snapshot generations;
- extension capability manifests;
- namespaced extension state with v0.2 migration;
- architecture verification gate;
- authoritative architecture/threading/upstream docs.

## v0.4 — standalone fork/product identity

- publish complete source tree as Automexia Terminal;
- remove bootstrap-as-product workflow;
- Automexia binary/package/config/application identity;
- preserve required upstream license/copyright provenance;
- release automation, issue templates, contribution guide, code of conduct, security policy;
- keep Rio as a reference remote, not a runtime/build dependency.

## v0.5 — internal modularization

- gradually rename product-facing crates/modules;
- create explicit Automexia core/application/platform crates where the source structure benefits from it;
- isolate platform adapters and renderer adapters;
- add performance benchmarks and terminal conformance suites.

## v0.6 — extension SDK preview

- versioned extension manifest schema;
- sandbox/Wasm runtime;
- capability broker;
- package signatures/hashes and transactional installation;
- extension SDK/examples;
- marketplace registry protocol.

## v0.7+ — ecosystem hardening

- extension crash isolation and resource budgets;
- API compatibility tooling;
- marketplace trust/reputation/update model;
- developer tools and diagnostics;
- accessibility/platform-native integration audits.

## v1.0 criteria

- independently buildable Automexia source repository;
- stable terminal behavior/conformance baseline;
- repeatable signed releases;
- documented extension API compatibility policy;
- safe third-party extension execution model;
- Windows daily-driver quality plus defined Linux/macOS support policy;
- measured performance budgets and regression gates.

## v0.3.1 DevOps integration hardening

- native default-enabled first-party DevOps HUD;
- split manifest/model/context/semantics ownership;
- Windows-aware local context paths;
- semantic INFO/DEBUG plus shell/build/container failure coverage;
- no custom CLI commands, command wrappers, network probes or process spawning.


## v0.3.2 session-aware DevOps

- raw terminal-title session facts for nested-shell detection;
- passive WSL session recognition (the early UNC-filesystem bridge experiment was later superseded by non-blocking shell metadata + `/mnt/<drive>` mapping);
- Docker Desktop default-context fallback;
- per-terminal-session context cache and completion revisions;
- guaranteed bounded redraw polling until async discovery completes.


## v0.3.3 warning-clean maintenance

- Keep the DevOps extension public module surface minimal.
- Remove unused `KubernetesContext` / `WslContext` re-exports while retaining the internal model types.
- Gate the warning cleanup so future overlays cannot silently reintroduce the Rust `unused_imports` warning.


## v0.3.7 unified visual system

- Move DevOps context from fixed application chrome onto blank OSC-133 semantic prompt rows so it appears immediately above each command and scrolls with terminal history.
- Render DevOps/session context as native vector icon + value segments (`24.04`, `default`, current Git branch, context/namespace) rather than category-name pills.
- Apply one Automexia named/ANSI palette across Windows PowerShell, WSL/Linux and macOS while preserving explicit application truecolor.
- Add optional PowerShell/Bash/Zsh integration for a shared two-row lambda prompt, OSC 7/133 session metadata and PowerShell PSReadLine syntax colors; the first row is a native context surface and no command wrappers are installed.
- Normalize the default Windows PowerShell launch to suppress the stock logo/banner without using `-NoProfile`.
- Propagate Automexia terminal identity through `WSLENV` so nested WSL integration activates only inside Automexia sessions.
- Add visual-system quality gates for the palette, vector icons, shell integration and real application call sites.


## v0.3.8 reliable shell integration

- source the installed PowerShell integration directly for interactive Automexia shells while preserving normal profiles;
- use actual PowerShell profile paths instead of guessed Documents locations;
- install WSL shell hooks in one non-login invocation;
- remove blocking `\\wsl.localhost` / `\\wsl$` provider enumeration from DevOps refresh;
- publish distro/version once through shell metadata;
- allow semantic error highlighting to recolor neutral white/default rows while preserving non-neutral application colors.

## v0.3.9 UTF-8 shell integration hardening

- Generate the prompt glyph from U+03BB at shell runtime rather than depending on a literal Unicode byte sequence.
- Transfer Windows-to-WSL integration payloads as UTF-8 bytes encoded in ASCII base64, avoiding Windows PowerShell 5.1 native-pipeline OEM conversion.
- Keep prompt startup constant-time and free of WSL UNC enumeration.

## v0.3.12 live prompt reliability + release audit

- Resolve the active DevOps context row from the live OSC-133 Prompt/PromptContinuation pair on every frame, using cursor geometry only as a first-paint fallback, so resize/fullscreen/reflow cannot orphan it.
- Publish generic shell/prompt lifecycle metadata (`automexia_shell`, `automexia_prompt_active`) through OSC 1337 rather than inferring prompt state from stale visible-row snapshots.
- Keep historical semantic prompt rows bounded/cached, but never let scrollback anchors drive the live prompt position.
- Move current working directory out of the DevOps extension and back into the normal PowerShell/Bash/Zsh prompt line.
- Preserve the unified Automexia palette and native vector environment icons.
- Preserve user PowerShell history handlers when installing the prompt-active hook.
- Prevent a full capacity-one extension queue from leaving a pane permanently stale; add timeout/retry semantics.
- Preserve existing Bash `PROMPT_COMMAND` ordering and observed `$?` while appending Automexia metadata hooks idempotently.
- Remove all runtime WSL UNC-provider access and keep `/mnt/<drive>` mapping plus host context fallback non-blocking.
- Audit every overlay file as an Automexia-managed bootstrap path so repeated/resumed integrations do not reject Automexia's own `shell.rs`/`theme.rs`.
- Add strict source-quality, shell-behavior, Cargo metadata/fmt/check/Clippy/test/release/smoke gates.


## v0.3.13 generated-source normalization

- normalize semantic-patcher output with the checkout-pinned rustfmt during bootstrap/direct apply;
- keep BUILD/CHECK/DEV formatting gates non-mutating via `FORMAT-WINDOWS.ps1 -CheckOnly`;
- fail early with a clear normalization error instead of dumping a large rustfmt diff at release time;
- keep generated-source formatting as an explicit generation responsibility.
