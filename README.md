# Automexia Terminal v0.3.13 — rustfmt-normalized release pipeline


## v0.3.13 generated-source normalization

v0.3.13 fixes the release-gate failure discovered on the real Windows checkout: Automexia's semantic patcher produced valid Rust whose whitespace/import ordering was not identical to the pinned `rustfmt` output, so the intentionally strict `cargo fmt --check` gate stopped the build before compilation. The source transformation and release verification stages are now separated explicitly:

```text
apply semantic patches
        ↓
FORMAT-WINDOWS.ps1
(cargo fmt --all using the pinned toolchain)
        ↓
source/architecture verification
        ↓
BUILD/CHECK/DEV use FORMAT-WINDOWS.ps1 -CheckOnly
        ↓
Cargo check / Clippy / tests / release
```

`BOOTSTRAP-WINDOWS.ps1` and direct `APPLY-AUTOMEXIA.ps1` normalize generated Rust whenever Cargo is available. `BUILD-WINDOWS.ps1`, `CHECK-WINDOWS.ps1`, and `DEV-WINDOWS.ps1` remain non-mutating gates: they only verify formatting and fail if a caller bypassed the generation/normalization stage. This keeps formatting ownership with source generation rather than hiding unrelated source edits during a release build.


## v0.3.12 live prompt layout

v0.3.12 fixes the resize/fullscreen/first-paint failure of the semantic prompt UI. The live DevOps context is no longer positioned from a cached visible-row scan. Automexia recomputes the active context row from the current cursor geometry on every frame and gates it with shell-published prompt lifecycle metadata. Historical prompt rows may still use OSC-133 scrollback anchors.

The working directory is deliberately **not** a DevOps segment anymore. PowerShell, Bash/WSL and Zsh/macOS render the current path in the normal command prompt line, followed by `λ` and editable input. The row above is reserved for environment/session context such as Ubuntu version, Docker, Git, Kubernetes, cloud, user and production state.

Expected rhythm:

```text
[Ubuntu] 24.04 │ [Docker] default │ [Git] feature/x │ [User] amjed
/mnt/d/workstation/project λ command
```

PowerShell uses its native Windows path in the same location. No custom CLI wrappers are installed.

v0.3.12 keeps the unified visual direction and fixes prompt reliability across first paint, resize and fullscreen. The live environment row is cursor-anchored; the current path is restored to the normal shell prompt line; Docker/Git/Kubernetes/cloud context enriches asynchronously without requiring another command.

Highlights:

- one Automexia ANSI/named-color palette on Windows, WSL/Linux and macOS;
- semantic error/warning/success/info/debug colors use the same palette;
- DevOps context is a compact native segmented line on the blank semantic prompt row above each command, with vector-drawn icons, current-path context and value-only labels;
- WSL/Ubuntu, Docker, Kubernetes, Git, cloud, Terraform, user/environment and production context remain extension-owned;
- optional PowerShell/Bash/Zsh shell integration gives the same lambda prompt/editor colors and emits OSC 7/133 metadata;
- PowerShell launch normalization adds `-NoLogo` when appropriate so the shell opens directly into the Automexia experience;
- no `ax`, `kgp`, Docker/Kubernetes wrappers, prompt command aliases, or runtime CLI execution were added.

The terminal engine remains pinned to audited Rio SHA `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2`; Automexia application/theme/extensions remain above that boundary.

## Architecture

```text
Automexia application/platform
├── commands, keybindings, market
├── extension runtime + capability manifests
├── bounded extension worker
├── cached UI models
└── renderer adapters
        │
        │ narrow snapshots/contracts
        ▼
Pinned Rio-derived terminal engine
├── PTY / ConPTY
├── VT parser + terminal state
├── font shaping/rasterization
├── screen damage/render snapshots
└── GPU/window backends
```

Read these before making structural changes:

- `docs/ARCHITECTURE.md` — authoritative subsystem/dependency specification;
- `docs/THREADING.md` — latency/threading rules;
- `docs/EXTENSION-PLATFORM.md` — extension/capability/sandbox plan;
- `docs/UPSTREAM-STRATEGY.md` — how Rio updates are audited without destabilizing releases;
- `docs/ROADMAP.md` — path from v0.3 to standalone Automexia;
- `docs/SOURCE-OWNERSHIP.md` — contributor dependency/ownership rules;
- `docs/QUALITY-GATES.md` — performance, correctness and release gates;
- `docs/VISUAL-SYSTEM.md` — unified palette, semantic prompt/context-line and shell-integration rules;
- `docs/DEVOPS-EXTENSION.md` — DevOps behavior and discovery contract;
- `docs/DEVOPS-TROUBLESHOOTING.md` — HUD/WSL diagnostics without custom CLI integration;
- `docs/REFERENCE-ARCHITECTURE-REVIEW.md` — design review and professional-terminal reference findings;
- `docs/adr/` — accepted architectural decisions.

## Recommended Windows setup

From the extracted v0.3 directory:

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force

$Package = (Get-Location).Path
$Automexia = "D:\workstation\projects\business-project\custom_terminal\automexia-terminal-source"

.\BOOTSTRAP-WINDOWS.ps1 -ProjectRoot $Automexia -SkipBuild
```

The source bootstrap now performs package regression checks, applies the architecture transactionally, verifies all reported warning cleanups (including the original 19 sites and later DevOps cleanup regressions) and runs the architecture verifier against the actual checkout.

Expected architecture gate:

```text
PASS: Automexia engine/app/extension/render boundaries, asynchronous extension IO, capabilities, single Automexia namespace, and exact fresh-checkout Rio pin verified
```

### Fast development loop

Use the dedicated development gate so normal iteration does not run the test and release profiles:

```powershell
Set-Location $Package
.\DEV-WINDOWS.ps1 -ProjectRoot $Automexia -Run
```

Equivalent manual loop:

```powershell
Set-Location $Automexia
cargo fmt --all
cargo check -p rioterm
cargo run -p rioterm --bin rio
```

### Full release gate

```powershell
Set-Location $Package
.\BUILD-WINDOWS.ps1 -ProjectRoot $Automexia
.\START-WINDOWS.ps1 -ProjectRoot $Automexia
```

`BOOTSTRAP-WINDOWS.ps1` normalizes generated Rust with the pinned `rustfmt`. `BUILD-WINDOWS.ps1` then runs the Python integration/regression/architecture checks, warning verification, non-mutating rustfmt verification, `cargo check`, Clippy with warnings denied, tests, release build and release-binary `--version` smoke test before copying:

```text
<automexia-source>\dist\AutomexiaTerminal.exe
```

## Unified shell UX

The renderer palette works immediately after bootstrap. To also unify prompt/editor behavior, install the optional shell integration. It only changes prompt metadata/colors; it does not add custom commands.

The context line requires the optional shell integration because the shell emits standard OSC 133 prompt boundaries; without it, Automexia still uses the unified ANSI palette and semantic output highlighting but does not invent prompt rows.

Windows + default WSL distro:

```powershell
.\INSTALL-SHELL-INTEGRATION-WINDOWS.ps1
```

Linux/macOS:

```bash
./install-shell-integration.sh
```

Restart the shell after installation. PowerShell uses PSReadLine colors; Bash/Zsh use the same lambda prompt and OSC 7/133 metadata.

## `/market`

Press **Ctrl+Shift+P**, type `/market`, and select **/market · Browse extensions**.

The v0.3 market remains a safe first-party catalog. Install/remove changes activation state for code already compiled into the binary. Arbitrary downloaded native code is intentionally unsupported.

## Automexia DevOps

The bundled first-party extension detects bounded local context for Kubernetes, Docker, AWS, Azure, GCP, Terraform, Git and environment/production indicators. It does not execute those CLIs and does not use network/process-spawn capabilities.

v0.3.2 fixes nested-shell discovery on Windows. Automexia snapshots the raw terminal/OSC title as generic session metadata; when the active child shell is WSL and exposes the common `user@host:/path` title, the extension passively maps WSL local config through `\\wsl.localhost`/`\\wsl$` without injecting commands. Host Docker/Kubernetes/cloud config remains a fallback. Docker Desktop's existing `.docker` root is recognized as the configured `default` context even if `config.json` does not name `currentContext`.

The newest prompt is live. OS/version and user context render immediately with no filesystem IO; bounded async discovery then adds Docker/Git/Kubernetes/cloud context to that same row. The shell independently renders its current working directory on the command line. When the next prompt starts, the previous context row is frozen into bounded history.

## Upstream policy

Do not merge or rebase Rio `main` into a stable Automexia release just because upstream changed. v0.3 is reproducibly pinned. A new Rio base must go through a compatibility branch and the full verification matrix before a new Automexia release updates the pin. Use `AUDIT-UPSTREAM-WINDOWS.ps1 -ProjectRoot $Automexia` to inspect new upstream commits without merging them.

## Standalone Automexia direction

v0.3 establishes the boundary inside the current Rio-derived checkout. The next major migration is to publish the complete source tree as the Automexia repository so bootstrap patching is no longer the product workflow. Required upstream MIT attribution/provenance remains preserved while user-facing identity and maintenance move to Automexia.
