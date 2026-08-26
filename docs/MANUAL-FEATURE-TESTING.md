# Complete implemented-feature manual testing guide

This guide is the end-to-end manual acceptance workbook for the features present
in the Automexia Terminal source tree. It is written for a tester starting with
a clean machine as well as for a contributor who already has a development
checkout. It explains what each feature does, how to prepare every optional
dependency used by the scenarios, exactly what to do, what to observe, and what
counts as a failure.

The guide was reconciled on 2026-08-26 with the machine-enforced
[`feature-matrix.json`](../tests/assurance/feature-matrix.json), the
[`feature-test-reinforcement-v1.json`](../tests/assurance/feature-test-reinforcement-v1.json)
contract, the [feature catalog](FEATURES.md), and the [phase implementation
audit](PHASE-IMPLEMENTATION-AUDIT.md). The commit being tested must still be
recorded at test time; documentation dates never substitute for an exact source
or package digest.

## Read this status boundary first

Automexia contains product behavior at several different assurance levels. The
expected result depends on the level:

| Label used here | Meaning | Correct manual expectation |
|---|---|---|
| **Available now** | The v0.4 terminal behavior is intended to work in a normal source build. | Exercise the real UI or shell workflow. |
| **Implemented locally / release-gated** | Source, deterministic tests, and a local product path exist, but named native, accessibility, package, or longitudinal evidence is still required. | Exercise the available path and record the exact host; do not generalize to untested platforms. |
| **Implemented internally, not activated** | Models, adapters, or review surfaces exist, but process, network, credential, provider, managed-SSH, or execution authority is intentionally off. | The correct product result is a clear disabled/pending state and no external side effect. Run the focused source tests for deeper coverage. |
| **Preview disabled** | Accepted suggestion/ecosystem source exists, but the user-facing runtime must remain unavailable. | Native completion must continue working; no popup, helper activation, PTY write, or implicit Enter may appear. |
| **Controlled/external** | A release claim needs real signed packages, services, accounts, hardware, assistive technology, or a long campaign. | Record as **not run** unless that exact controlled environment was actually used. |
| **Planned only** | Documentation or research exists without a product feature. | Do not invent a manual pass. Verify absence only when the boundary has an explicit nonactivation checker. |

Three especially important consequences follow:

1. The read-only Connection Hub may inventory explicitly selected SSH files, but
   it must not connect, log in, open a listener, or create a managed PTY.
2. AWS, Azure, Google Cloud, Kubernetes, OpenShift, and Teleport adapters have
   internal source contracts and cached review, but Automexia must not run their
   CLIs, read their credential caches, or change global context.
3. CP5 suggestions and D7/CP6 ecosystem runtime activation remain disabled. A
   visible suggestion popup or downloaded/running extension is a failure in the
   current source boundary, not a successful test.

## Safety rules for every test

- Use a disposable Automexia configuration root. Never test migration,
  recovery, aliases, Quick Actions, workspace persistence, or corrupted files
  against your normal profile.
- Use disposable local accounts, virtual machines, containers, clusters, and
  cloud subscriptions/projects when a controlled scenario calls for them.
- Never paste tokens, passwords, private keys, client secrets, device codes,
  certificates, private hostnames, or production output into a screenshot,
  issue, or test log.
- Prefer loopback (`127.0.0.1`) for OpenSSH and tunnel exercises. Do not expose a
  test SSH server to an untrusted network.
- Quick Actions and workspace reviews must insert or preview only. Review the
  command and press Enter yourself only in scenarios that explicitly say to run
  the shell-owned command.
- Stop at the first unexplained failure. Retrying may help investigation, but it
  does not erase the first result.
- Do not classify a cross-compile, screenshot, source test, or WSL run as a
  native Linux/macOS/accessibility/package pass.

## Manual evidence record

Create one record per host and renderer. Fill every field before testing:

```text
Tester:
Date and local timezone:
Repository commit:          git rev-parse HEAD
Branch:                     git branch --show-current
Working tree state:         git status --short
Binary/package SHA-256:
Operating system/build:
CPU architecture:
CPU and logical cores:
RAM:
GPU and driver:
Display resolution/scale:
Desktop/session:            Windows / macOS / X11 / Wayland
Renderer:                   WGPU / CPU fallback
Shell and version:
WSL distribution/version:
Assistive technology/version:
Docker/runtime version:
Kubernetes/oc/tsh/cloud CLI versions:
Scenario IDs run:
Pass/fail/not-run result:
Private artifact locations:
Cleanup result:
First failure and reproducer:
```

Record screenshots only when they contain synthetic test content. Keep native
accessibility transcripts, terminal-content screenshots, performance traces,
and path-bearing reports private unless they have been reviewed and redacted.

## Complete feature-to-scenario index

This table covers every entry in the machine-enforced feature matrix. The two
planned-only entries are included so they cannot be mistaken for usable
features.

| Feature matrix ID | Implemented result | Manual scenarios |
|---|---|---|
| `identity-config-migration` | Product identity, isolated configuration, live transactional reload, non-destructive Rio migration, coexistence | CFG-01 through CFG-05 |
| `terminal-protocols-grid-history` | VT protocols, Unicode grid, selection, search, scrollback, reflow | TERM-01 through TERM-06 |
| `pty-scheduler-process-lifecycle` | PTY/ConPTY launch, resize, ordered input, exit, teardown | PTY-01 through PTY-05 |
| `renderer-fonts-responsive-ui` | GPU/CPU rendering, fonts, graphemes, responsive panes, branded surfaces, footer/chrome | UI-01 through UI-07 |
| `windows-tabs-sessions-input` | Windows, global tabs, pane-local tabs, fresh/clone splits, mouse, clipboard, command jumps | INPUT-01 through INPUT-09 |
| `ghostty-compatibility-g0-g6` | Versioned profile, typed bindings, migration, inspector, bounded topology history; macOS fixture/release evidence partial | GHOST-01 through GHOST-08 |
| `prompt-context-devops-semantics` | OS/user/path/Git/container/Kubernetes/cloud/Terraform/status/duration context | SHELL-01 through SHELL-05 and EXT-01 |
| `openssh-inventory-persistence` | Explicit bounded static SSH inventory, public metadata, last-known-good refresh | HUB-01 through HUB-07 |
| `extension-contract-runtime` | Versioned bounded extension contracts and fail-closed application runner | DEV-03 |
| `image-protocols-local-preview` | Kitty/Sixel/iTerm2 protocol images and bounded local Quick Look | IMG-01 through IMG-05 |
| `shell-integration-listings` | Session-only PowerShell/CMD/WSL/Bash/Zsh/Fish integration and icon-aware listings | SHELL-01 through SHELL-08 |
| `packaging-release-provenance` | Package metadata/checks, signing/SBOM/provenance policy; final signed native evidence external | PKG-01 through PKG-04 |
| `command-productivity-cp0-policy` | Accepted nonactivating architecture and pure CP3.0 projection compiler | QA-02 and ACT-06 |
| `command-productivity-cp2-persistence` | Private bounded Quick Action store, CAS, recovery, last-known-good | ACT-01 through ACT-04 |
| `command-productivity-cp22-quick-actions` | Search/review/placeholders/import/export/insert/copy without Enter | ACT-01 through ACT-07 |
| `command-productivity-cp4-provider-actions` | Cached provider-aware rows with route revalidation; provider refresh/execution off | CP4-01 through CP4-04 |
| `command-productivity-cp31-persistent-aliases` | Explicit dry-run-first generated aliases with rollback/uninstall | ALIAS-01 through ALIAS-08 |
| `command-productivity-cp32-devops-packs` | Eleven reviewed static packs and 33 disabled-by-default actions | PACK-01 through PACK-05 |
| `command-productivity-cp33-native-imports-workspace` | Selected alias imports and trusted `just`/Task/mise workspace bridges | IMPORT-01 through IMPORT-07 |
| `command-productivity-cp1-native-completion` | Native-shell adapters, explicit provider refresh, health, disable/remove | COMP-01 through COMP-07 |
| `command-productivity-cp50-research` | **Planned/research only; not a user feature** | ABS-01 |
| `command-productivity-cp51-proposal` | Authenticated native-editor source implemented; preview disabled | SUG-01 through SUG-05 |
| `situation-aware-production-operations-po0-proposal` | **Documentation/checker proposal only; PO1-PO8 not implemented** | ABS-02 |
| `ecosystem-d7-cp6-proposal` | Accepted signed/sandboxed source boundary; download/activation/provider calls off | ECO-01 through ECO-05 |
| `contributor-automation-quality-policy` | One-command quality, architecture, identity, dependency, QA and release policy | QA-01 through QA-06 |
| `stabilization-release-assurance-s1-s2` | Deterministic policy and fail-closed release ratchet; native matrices and 30 days external | ASSURE-01 through ASSURE-05 |
| `ffi-wasm-embedding` | Native C API and WebAssembly embedding source surfaces | EMBED-01 through EMBED-03 |
| `connection-hub-f2-models` | Capability-free records, reducers, dry-run planner and renderer-neutral Hub | HUB-01, HUB-08 and DEV-04 |
| `connection-hub-f3-catalog` | Bounded search/filter/group/virtualization | HUB-02 through HUB-05 |
| `connection-hub-f3-composition` | Generation-safe composition and last-known-good catalog | HUB-06 and DEV-04 |
| `connection-hub-m1-product` | Read-only modal, exact file grants, public favorite/tag edits | HUB-01 through HUB-07 |
| `connection-hub-f3-library` | Private profiles/recipes/workspaces/preferences with transactional storage | WORK-01 and DEV-05 |
| `connection-automation-m6-workspaces` | Review-only typed automation, restore/recipe/broadcast planning; execution off | WORK-01 through WORK-07 |
| `provider-auth-m7-capsules` | Provider-neutral public capsules and reviewed requests without authority | CLOUD-01 and DEV-06 |
| `direct-openssh-m3-review` | Reviewed routes/trust/tunnels/lifecycle source; managed launch off | SSH-01 through SSH-09 |
| `provider-aws-m8-source` | AWS official-CLI contract and cached review; execution off | AWS-01 through AWS-04 |
| `provider-azure-m9-source` | Azure official-CLI contract and cached review; execution off | AZ-01 through AZ-04 |
| `provider-gcp-m10-source` | Named gcloud configuration contract and cached review; execution off | GCP-01 through GCP-04 |
| `provider-kubernetes-openshift-m11-source` | Bounded kubeconfig/private transient contracts and cached review; execution off | KUBE-01 through KUBE-09 |
| `provider-teleport-m12-source` | Bounded `tsh` public-status/review contract; execution off | TP-01 through TP-04 |

## Part I: prepare a clean test environment

### 1. Obtain and identify the source

Use a normal local filesystem, not a cloud-synchronized or network directory.
On WSL, use a Linux-native path such as `~/src/automexia-terminal`, not
`/mnt/c/...` or `/mnt/d/...`.

```text
git clone <reviewed-repository-url> automexia-terminal
cd automexia-terminal/standalone
git status --short
git branch --show-current
git rev-parse HEAD
```

Expected result:

- `git rev-parse HEAD` prints one 40-character commit ID;
- the branch and commit match the artifact or review under test; and
- `git status --short` is empty for a release candidate. If it is not empty,
  record every path and do not call the run clean-source evidence.

### 2. Install source-build prerequisites

Use the Rust toolchain pinned by `rust-toolchain.toml`. The official Rust
installation path is [rustup](https://www.rust-lang.org/tools/install); Windows
also needs Visual Studio C++ Build Tools and a Windows SDK.

Common prerequisites:

```text
git --version
python --version
rustup --version
rustc --version
cargo --version
python -m pip install PyYAML
cargo install --locked cargo-deny --version 0.20.2
```

Windows:

1. Install Git and Python 3 from their official distributions.
2. Run the x64 `rustup-init.exe` from the Rust site.
3. Install Visual Studio Build Tools with **Desktop development with C++**, the
   current MSVC toolset, and the Windows 10/11 SDK.
4. Open a fresh Developer PowerShell or ordinary PowerShell after installation.

Ubuntu/Debian:

```text
sudo apt update
sudo apt install build-essential pkg-config libasound2-dev libfontconfig1-dev libxkbcommon-dev libwayland-dev libx11-xcb-dev glslang-tools python3 python3-pip git
```

macOS:

```text
xcode-select --install
```

Then install rustup using the command shown on the official Rust page.

From the repository root run:

```text
cargo xtask doctor
```

Expected result: the report identifies the Rust tools, native shell, packaging
prerequisites, target location, storage, and WSL placement. Treat `missing`, an
unsupported toolchain, or a target under `/mnt/<drive>` as a setup failure. Do
not lower the storage guard to hide insufficient disk space.

### 3. Create an isolated Automexia profile

Close every Automexia process first. Create a dedicated root that can be deleted
after the test.

PowerShell:

```powershell
$AutomexiaManualRoot = Join-Path $env:TEMP 'automexia-manual-test'
New-Item -ItemType Directory -Force -Path $AutomexiaManualRoot | Out-Null
$env:AUTOMEXIA_CONFIG_HOME = $AutomexiaManualRoot
Write-Output $env:AUTOMEXIA_CONFIG_HOME
```

Bash/Zsh/Fish-compatible environment setup:

```sh
export AUTOMEXIA_CONFIG_HOME="${TMPDIR:-/tmp}/automexia-manual-test"
mkdir -p "$AUTOMEXIA_CONFIG_HOME"
printf '%s\n' "$AUTOMEXIA_CONFIG_HOME"
```

Expected result: the printed path is the disposable directory, not the normal
Windows `%LOCALAPPDATA%`, macOS Application Support, or Linux XDG configuration
folder. Keep the invoking shell open so the variable remains active.

Create the starter configuration without overwriting anything:

```text
automexia --write-config
```

If the installed binary is not on `PATH`, build first and use the exact binary
under `target/debug`.

Expected result: one starter `config.toml` appears below the isolated root.
Running the same command again refuses to overwrite it. That refusal is a pass.

### 4. Build, verify, and launch

The first source run needs at least 12 GiB of free build space:

```text
cargo dev
```

Expected result:

1. the complete contributor preflight finishes successfully;
2. the debug binary is built and its version smoke passes;
3. shell support is prepared only after verification; and
4. the Automexia window opens after the checks, never before them.

After one successful full run, use:

```text
cargo automexia
```

Expected result: the incremental build/smoke completes and the application
opens. `cargo ready` performs the full gate without launching and is used later
in QA-05.

### 5. Prepare synthetic content

Create a folder containing only public test data. Examples below use
`automexia-manual-fixture`.

PowerShell:

```powershell
$AutomexiaFixture = Join-Path $env:TEMP 'automexia-manual-fixture'
New-Item -ItemType Directory -Force -Path $AutomexiaFixture | Out-Null
1..300 | ForEach-Object { "AUTOMEXIA-HISTORY-$($_.ToString('000'))" } |
    Set-Content -Encoding utf8 (Join-Path $AutomexiaFixture 'history.txt')
Copy-Item 'assets\brand\automexia-terminal-source-512.png' $AutomexiaFixture
Set-Location $AutomexiaFixture
```

Unix shells:

```sh
AUTOMEXIA_FIXTURE="${TMPDIR:-/tmp}/automexia-manual-fixture"
mkdir -p "$AUTOMEXIA_FIXTURE"
seq -f 'AUTOMEXIA-HISTORY-%03g' 1 300 > "$AUTOMEXIA_FIXTURE/history.txt"
cp assets/brand/automexia-terminal-source-512.png "$AUTOMEXIA_FIXTURE/"
cd "$AUTOMEXIA_FIXTURE"
```

Expected result: the folder contains `history.txt` and the Automexia PNG. Do not
use personal files in image, search, clipboard, or screenshot scenarios.

### 6. Optional: install WSL for cross-shell testing on Windows

Microsoft's current [WSL installation guide](https://learn.microsoft.com/en-us/windows/wsl/install)
uses an elevated PowerShell:

```powershell
wsl --install
```

Restart if requested, launch Ubuntu, and create the Linux username/password.
Then verify from PowerShell:

```powershell
wsl --version
wsl --list --verbose
```

Expected result: an installed distribution appears with version `2`. If WSL is
already installed, use `wsl --update`; do not unregister an existing distribution
for this test. Install optional shells inside a disposable Ubuntu distribution:

```sh
sudo apt update
sudo apt install zsh fish openssh-server
bash --version
zsh --version
fish --version
```

### 7. Optional: install Docker and a disposable Kubernetes cluster

#### Windows and macOS Docker Desktop

Follow the current [Docker Desktop installation guide](https://docs.docker.com/desktop/).
On Windows, the WSL 2 per-user installation is the normal least-privilege choice.
Review Docker's license requirements before using it in a company. Start Docker
Desktop and wait for the engine to become ready.

#### Linux Docker Engine

Use Docker's official distribution-specific [Engine installation guide](https://docs.docker.com/engine/install/).
Do not use an unreviewed convenience script on a production workstation. Treat
membership in a Docker socket group as host-equivalent authority.

Verify without Automexia-specific behavior:

```text
docker version
docker run --rm hello-world
```

Expected result: client and server sections appear, then the container prints a
Docker hello message and exits successfully. A daemon connection error is an
environment failure, not an Automexia result.

#### Install kubectl

Use the official [Kubernetes tool installation page](https://kubernetes.io/docs/tasks/tools/)
and select the instructions for your OS. Verify:

```text
kubectl version --client
```

Expected result: the client version is printed without needing a cluster.

#### Install kind and create a disposable cluster

Follow the official [kind quick start](https://kind.sigs.k8s.io/docs/user/quick-start/).
Place the reviewed release binary on `PATH`, then run:

```text
kind version
kind create cluster --name automexia-manual --wait 5m
kubectl cluster-info --context kind-automexia-manual
kubectl get nodes --context kind-automexia-manual
kubectl create namespace automexia-manual --context kind-automexia-manual
kubectl get namespace automexia-manual --context kind-automexia-manual
```

Expected result: the cluster becomes ready, one or more nodes report `Ready`,
and namespace `automexia-manual` is `Active`. Keep explicit `--context` flags so
the test never depends on or silently changes an unrelated current context.

Cleanup after KUBE scenarios:

```text
kind delete cluster --name automexia-manual
```

Expected result: kind reports deletion and `kubectl config get-contexts`
contains no `kind-automexia-manual` context.

### 8. Optional: install OpenShift tools

For CLI-only tests, download the `oc` client from the official Red Hat/OpenShift
client downloads associated with your cluster and place it on `PATH`:

```text
oc version --client
```

Expected result: an OpenShift client version is printed. A real local OpenShift
test normally uses Red Hat OpenShift Local (`crc`), requires supported
virtualization, substantial memory/disk, a Red Hat account and pull secret, and
must follow the current [OpenShift Local documentation](https://docs.redhat.com/en/documentation/red_hat_openshift_local/).
Do not put a pull secret in this repository or a public evidence bundle.

Typical disposable sequence after official installation:

```text
crc setup
crc start
crc status
crc console --credentials
```

Use the printed ephemeral credentials only on the local host, then run the
specific KUBE-09 checks. Cleanup:

```text
crc stop
crc delete
```

Expected result: the VM stops and is deleted. Keep Red Hat credentials and pull
secrets out of all Automexia logs and screenshots.

### 9. Optional: install cloud CLIs

These tools test ordinary terminal/shell interoperability. They do **not**
activate Automexia's internal provider adapters.

- AWS: follow the official [AWS CLI v2 installation guide](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html), then run `aws --version`.
- Azure: follow the official [Azure CLI installation guide](https://learn.microsoft.com/en-us/cli/azure/install-azure-cli), then run `az version`.
- Google Cloud: follow the official [Google Cloud CLI install guide](https://cloud.google.com/sdk/docs/install), then run `gcloud version`.

Expected result: each installed tool prints its version. Do not authenticate for
the base scenarios. Controlled real-provider scenarios require disposable,
least-privilege accounts and organization approval; root/owner credentials are
never appropriate.

### 10. Optional: install Teleport `tsh`

Follow Teleport's official [`tsh` installation guide](https://goteleport.com/docs/connect-your-client/teleport-clients/tsh/).
Install the same major version as the disposable Teleport cluster, then run:

```text
tsh version
```

Expected result: the client version is printed. A controlled organization test
needs a disposable Teleport cluster or approved test tenant. Do not publish the
proxy name, username, roles, certificates, or profile directory.

### 11. Optional: prepare assistive technology

- Windows Narrator is built in. Start/stop it with `Win+Ctrl+Enter`; use
  Microsoft's [Narrator guide](https://support.microsoft.com/en-us/windows/complete-guide-to-narrator-e4397a0d-ef4f-b386-d8ae-c172f109bdb1).
- NVDA must come from [NV Access](https://www.nvaccess.org/download/) and should
  run with a fresh portable/test profile.
- macOS VoiceOver is built in. Use `Cmd+F5` or Accessibility settings and the
  [VoiceOver guide](https://support.apple.com/guide/voiceover/welcome/mac).
- GNOME Orca can be started with `Super+Alt+S` or `orca`; use the official
  [Orca guide](https://help.gnome.org/orca/).

Record the exact assistive technology version, speech settings, keyboard layout,
desktop session, scale, and whether full keyboard access is enabled. Do not call
a visual keyboard-only pass a screen-reader pass.

## Part II: terminal, renderer, input, and shell scenarios

### CFG-01 — identity and zero-configuration startup

**Covers:** `identity-config-migration`.

1. Keep the isolated `AUTOMEXIA_CONFIG_HOME` active.
2. Temporarily move the isolated `config.toml` out of that directory.
3. Launch Automexia.
4. Open the About/version surface or run `automexia --version` from another
   shell.

Expected result:

- Automexia opens with tested defaults even though no config exists;
- the product/window/application identity says Automexia, not Rio;
- the terminal is usable and no first-run path or private configuration path is
  rendered; and
- `automexia --version` prints `automexia <version>` and exits successfully.

Failure signals: a panic, a required-config error, old public Rio identity, a
local path shown on the welcome surface, or a config file created implicitly.

Restore the isolated config only after closing Automexia.

### CFG-02 — starter config is non-overwriting

1. Run `automexia --write-config` against the empty isolated root.
2. Save the file's hash.
3. Run the same command again.
4. Recalculate the hash.

PowerShell hash command:

```powershell
Get-FileHash (Join-Path $env:AUTOMEXIA_CONFIG_HOME 'config.toml') -Algorithm SHA256
```

Expected result: the first command creates a valid starter file; the second
refuses to overwrite; both hashes are identical. Any silent replacement is a
failure.

### CFG-03 — valid live reload and visible setting change

1. Start with the generated config.
2. Change one obvious, reversible setting documented in [configuration
   reference](reference/configuration.md), such as a font size or theme.
3. Save the file.
4. Use the configured `ReloadConfig` action, or restart if no reload binding is
   installed.

Expected result: the new value appears in every surface owned by the setting;
terminal contents and the running PTY are preserved; no second PTY is created;
focus remains usable.

### CFG-04 — invalid candidate keeps last-known-good configuration

1. While Automexia is running with the visibly valid CFG-03 setting, introduce
   an invalid TOML line into the isolated config.
2. Request reload.
3. Observe the notification and existing window.
4. Repair the TOML and reload again.

Expected result: the invalid candidate is rejected, the CFG-03 appearance stays
active, the terminal remains usable, and an actionable path-safe error appears.
After repair, reload succeeds. A crash, partial setting application, blank
window, PTY restart, or fallback to an unrelated default is a failure.

### CFG-05 — non-destructive Rio migration and coexistence

Use only the isolated roots described in [migration](MIGRATION.md).

1. Create a small synthetic Rio config in the source fixture location.
2. Record its hash and modification time.
3. Launch Automexia with the isolated empty destination.
4. Review the migration result.
5. Compare the source hash/time, Automexia destination, and application IDs.
6. Repeat startup.

Expected result: the Rio source is never modified; supported values are copied
once into Automexia's root; unsupported values receive an explicit explanation;
the second launch is idempotent; Rio and Automexia identities/config roots can
coexist. If the source is linked, oversized, malformed, or permission-denied,
migration fails closed and normal defaults remain usable.

### TERM-01 — Unicode, grapheme, and width rendering

In the selected pane print synthetic text containing ASCII, combining marks,
emoji, CJK wide characters, and right-to-left script without embedding bidi
override controls:

PowerShell:

```powershell
Write-Output 'ASCII | café | 😀 | 日本語 | العربية | END'
```

Bash/Zsh/Fish:

```sh
printf '%s\n' 'ASCII | café | 😀 | 日本語 | العربية | END'
```

Expected result: no panic or replacement-box cascade; adjacent rows align;
combining marks stay attached; double-width cells do not overwrite neighbors;
selection and copy return the original Unicode sequence. Test at 100%, 150%,
200%, and 300% display scale where the OS supports it.

### TERM-02 — keyboard and pointer selection

1. Print `alpha beta gamma`.
2. Drag across `beta`; copy with the platform shortcut and paste into a plain
   text editor.
3. Clear selection, move the shell cursor to a known position, and press
   `Shift+Left/Right`.
4. Extend by word with `Ctrl+Shift+Left/Right`.
5. Reverse direction through the starting point.
6. Press an unmodified arrow and then type one character.

Expected result: pointer selection copies only selected cells; keyboard
selection starts at the insertion cursor, extends/reverses by cell or Unicode
word, and exits before normal input. The selection motion never appears as
escape bytes in the shell.

### TERM-03 — scrollback, history navigation, and reflow

1. Print `history.txt` so at least 300 labeled rows enter scrollback.
2. Use wheel scrolling and `Shift+PageUp/PageDown`.
3. Use `Shift+Home` and verify row `AUTOMEXIA-HISTORY-001` is reachable.
4. Resize the window narrow, then wide, then restore it.
5. Search for `AUTOMEXIA-HISTORY-150`.

Expected result: rows remain ordered and searchable; resize reflows text without
duplicating or dropping labels; the cursor returns to the live prompt when
scrolling to bottom; no input is sent during host-owned scrolling.

### TERM-04 — continuous pane/all-pane search

1. Create two visible splits.
2. Print `LOCAL-ONLY-A` in pane A and `GLOBAL-SHARED` in both panes.
3. Select pane A and press `Ctrl+F` (`Cmd+F` on macOS).
4. Type `GLOBAL-SHARED` and note scope/count/focus.
5. Press `Ctrl+Shift+F` (`Cmd+Shift+F`) without closing search.
6. Press `Ctrl+F` again, then repeat `Ctrl+F` once more.
7. Tab to the `PANE`/`ALL PANES` group, use an arrow to switch, then click the
   other chip.
8. Press Escape.

Expected result:

- one search session moves between the selected-pane footer and safe
  workspace-bottom position;
- the query and query focus are preserved in both directions;
- counts are recomputed for exactly the visible panes;
- repeating the active shortcut only refocuses the query and does not reset it;
- exactly one scope is selected and the keyboard/pointer controls work;
- the surface does not overlap or activate window close controls; and
- no query byte reaches either shell prompt.

Switching focus to another pane while pane scope is active must close that local
search rather than silently changing ownership. Hidden pane-local tabs and other
window-level tabs must not contribute to all-visible-pane results.

### TERM-05 — search limits and failure states

1. Search for a string with zero matches.
2. Use `Ctrl+U` to clear it.
3. Enter two queries and use `Ctrl+P`/`Ctrl+N` to traverse search history.
4. Enter an invalid pattern if regex mode is exposed by the build.
5. Paste more than 4 KiB of synthetic text into the search box.

Expected result: zero and invalid states are explicit; query history affects
only search; oversized input is bounded/rejected; result badge caps at `999+`;
Escape or `Ctrl+C` closes search without sending an interrupt to the PTY while
the search surface owns input.

### TERM-06 — alternate-screen ownership

Run an installed full-screen terminal application such as `vim`, `less`, or
`top`.

1. Verify its keys and mouse work normally.
2. Try app-surface shortcuts that are documented as inactive during alternate
   screen ownership.
3. Exit the application normally.

Expected result: the alternate-screen application remains the input owner;
Automexia does not open hidden modal surfaces or leak shortcut letters into the
application. After exit, the original primary-screen history and prompt return.

### PTY-01 — shell launch, ordered input, and exit status

1. Launch PowerShell, CMD, WSL Bash, or a native Unix shell using Automexia's
   configured shell or `automexia -e <program>`.
2. Run three commands that print `ONE`, `TWO`, and `THREE`.
3. Run a command that exits with code 7 and inspect the next prompt metadata.

Expected result: output order matches input order; no duplicated/missing bytes;
the shell remains interactive; supported semantic integration shows failure and
exact native status where available. Unsupported CMD status stays explicitly
neutral rather than inventing success.

### PTY-02 — resize storm and cursor integrity

1. At an empty prompt, type `AUTOMEXIA-RESIZE-SENTINEL` without pressing Enter.
2. Repeatedly drag the window through tiny, normal, tall, wide, and maximized
   sizes for 30 seconds.
3. Create and resize a split during the storm.
4. Press Enter.

Expected result: the sentinel remains exact and in order, cursor position is
correct, prompt/footer do not overlap, no renderer hang occurs, and the command
is submitted once.

### PTY-03 — interrupt and EOF ownership

1. Run a long process: `ping -t 127.0.0.1` on Windows or `ping 127.0.0.1` on
   Unix.
2. With no selection, press `Ctrl+C`.
3. In a Bash/Zsh/Fish pane, use the displaced EOF chord documented in the
   keyboard reference when testing Automexia clone bindings.

Expected result: `Ctrl+C` reaches the application as interrupt when no selection
exists; with a selection it copies instead. The long process ends and the shell
returns to a prompt. Clone shortcuts must not steal the documented replacement
control-byte chord.

### PTY-04 — child teardown on pane close

1. Start a loopback ping or synthetic Python process in a disposable pane.
2. Note its PID using the shell's normal process tools.
3. Close that pane/split using the documented shortcut.
4. Verify from another shell that the process and its descendants are gone.

Expected result: the exact pane closes; unrelated panes remain; the child tree
is terminated/joined within the platform's bounded lifecycle; no orphan keeps
the executable or temporary files open.

### PTY-05 — high-throughput output remains responsive

Run a bounded output burst, not an infinite flood:

```text
python -c "import sys; [sys.stdout.write(f'AUTOMEXIA-STORM-{i:05d}\\n') for i in range(10000)]"
```

Expected result: the app remains responsive, final row `AUTOMEXIA-STORM-09999`
is reachable, input works after completion, memory does not continue growing
after quiescence, and closing the pane releases the process.
### UI-01 — branded startup and window controls

1. Launch at normal size, 800×600-equivalent, ultrawide, and maximized.
2. Inspect welcome/startup hierarchy, title, minimize, maximize/restore, and
   close controls.
3. Hover and keyboard-activate each available control.
4. Repeat at 100%, 200%, and 300% scale and in light/dark appearance.

Expected result: the welcome copy describes saving time, effort, and improving
flexibility without presenting SSH as the product's main purpose. At comfortable
density the top shelf is 42 logical pixels, the terminal begins after a 46-pixel
reservation, the active tab is at most 184 pixels by default, and each caption
card is 30 pixels inside a 40-pixel hit target. Controls remain separate rather
than inside a grouped border, with no short underline beneath any control at
rest, hover, press, or inactive-window state. Icons, hover/pressed/focus states,
and hit targets remain distinct; no search/modal covers the close target;
maximize changes to restore; no clipping or overlap occurs. Repeat the pointer
check at the outer edges of every 40-pixel target, not only over the visible card.

### UI-02 — responsive panes, footer, and chrome priority

1. Create a 2×2 split layout.
2. Shrink until space is constrained, then expand to 4K/8K-equivalent size.
3. Observe selected outlines, local-tab rails, footer, scrollbar, context, and
   modal placement.

Expected result: active pane is clear without color alone; low-priority chrome
omits itself before terminal rows become unusable; footer stays at pane bottom
when space permits and disappears cleanly when it does not; no control has a
negative/offscreen hit area.

### UI-03 — font size, glyph list, and appearance

1. Use an isolated config root from the clean-machine setup and set `[fonts]`
   `size = 18.0` in `config.toml`.
2. Launch Automexia, create two panes and a second OS window, then use
   `Ctrl/Cmd+=` twice. Do not type or press Enter after the shortcut.
3. Confirm every existing pane/window reaches 20 pt and that the prompt line is
   byte-for-byte unchanged. Create another pane; it must inherit 20 pt.
4. Wait for `state/user-preferences-v1.toml` to appear. It must be no larger
   than 16 KiB and contain schema 1 plus `font-size = 20.0`; it must not contain
   terminal text, paths, history, environment values, credentials, or provider
   data.
5. Close Automexia normally and reopen it with the same isolated root. The
   first window and every newly created pane must start at 20 pt without a
   visible 18-to-20 pt flash.
6. Toggle appearance with the platform shortcut, close, and reopen. The same
   light/dark choice must be active on the first frame and on all branded
   surfaces.
7. Edit an unrelated config value, reload config, and confirm it applies while
   the saved 20 pt override remains. Use `Ctrl/Cmd+0`; every pane must return to
   the configured 18 pt size. Restart once more; 18 pt must remain.
8. Open registered fonts with `Ctrl/Cmd+Shift+L` and render the TERM-01 Unicode
   line at the 6 pt lower bound, default, and 100 pt upper bound. Additional
   decrease/increase attempts must stop at the exact bound.
9. Recovery: with Automexia closed, first create two valid generations by
   changing the size twice. Replace only
   `state/user-preferences-v1.toml` with malformed text and relaunch. Automexia
   must warn, recover the `.previous.toml` value, preserve `config.toml`, and
   still accept Reset.
10. Failure isolation: make the state directory read-only or hold the
    `user-preferences-v1.lock` from a second process, change the size, and wait
    for the warning. The live terminal must keep accepting input and rendering;
    the last durable primary file must remain parseable and unchanged. Restore
    permissions/release the lock, change the setting again, and verify a normal
    restart.

Expected result: size stays within the documented 6–100 point bound; reset is
deterministic and returns to `config.toml`; font and appearance survive restart
and stay application-wide; the font browser is reachable and dismissible;
light/dark state updates all branded surfaces with readable contrast; grapheme
layout remains stable; preference activity emits no PTY bytes, never rewrites
`config.toml`, and storage failure cannot block the terminal.

### UI-04 — command palette and modal input isolation

1. Type `DO-NOT-SUBMIT` at a prompt without Enter.
2. Open the command palette.
3. With the unfiltered list overflowing, confirm that a slim vertical indicator
   is already visible. Scroll down and up with a mouse wheel; repeat with a
   precision trackpad when available at 100%, 200%, and 300% display scale, and
   reach both exact list boundaries without scale-dependent speed changes.
4. Reverse trackpad direction after a partial movement, mix wheel movement with
   Up/Down keys, type and clear a query, then resize from a one-row palette to
   the normal ten-row layout and back.
5. Search, dismiss with Escape, reopen, and activate a harmless UI action.
6. Repeat with Connection Hub, quit confirmation, assistant, and compatibility
   inspector beneath or above the palette where the UI permits it.

Expected result: underlying terminal is dim/inert; typed modal query never joins
`DO-NOT-SUBMIT`; wheel/trackpad movement changes only the palette result window,
keeps one selected row visible, never focuses another pane, never moves terminal
scrollback, and sends no PTY bytes. The indicator position follows the bounded
offset, remains subdued before/after activity, and brightens while scrolling;
short, empty, filtered, and enlarged lists do not show an unnecessary thumb or
blank trailing rows. Escape restores exact pane focus; only the selected UI
action runs; stacked or hidden modals cannot receive input.

Automated native evidence: the Windows driver injects `WM_MOUSEWHEEL` messages
at the real palette center in both directions and requires offset `0 -> 3 -> 0`
while selected index `3` remains inside the visible result window. Independent
oracles preserve the active route, terminal display offset, cursor position,
and raw cursor line, so pane selection, terminal scrolling, and PTY input cannot
masquerade as a palette pass. Controlled WGPU and CPU runs each capture and
inspect a 1750 x 1080 palette frame; the reports retain offsets, selected index,
terminal-state invariants, color buckets, luminance spread, and artifact
identity. Physical mouse/precision-trackpad hardware, non-Windows backends, and
native accessibility scroll announcements remain release evidence.

### UI-05 — completed-command result separation matrix

Run every applicable command below in a PowerShell pane, waiting for a new
prompt after each:

```powershell
Write-Output 'AUTO-SINGLE-OK'
'AUTO-LINE-1','AUTO-LINE-2'
cmd.exe /D /C "echo AUTO-CMD-OK"
Write-Error 'AUTO-FAIL-ERR'
ls -ll
Get-Item .
& cmd.exe /D /C "echo AUTO-NATIVE-ERR 1>&2 & exit /b 7"
Get-Item (Join-Path $env:TEMP 'automexia-file-that-does-not-exist') -ErrorAction SilentlyContinue
```

Also run Bash/Zsh/Fish/WSL equivalents:

```sh
printf '%s\n' AUTO-SINGLE-OK
printf '%s\n' AUTO-LINE-1 AUTO-LINE-2
printf '%s\n' AUTO-STDERR >&2; false
true
```

For each output-producing command expect:

- a persistent low-contrast result tint that excludes the next prompt;
- additional whitespace plus a horizontal end rule separating output from the
  next command line;
- a status icon/duration when the shell proves them, or explicit neutral
  treatment when it cannot;
- a stable local completion datetime at the right edge. A normal/wide pane uses
  `YYYY-MM-DD HH:MM:SS`; narrower panes drop duration before compacting to
  `MM-DD HH:MM`, without overlapping the left prompt-context tags;
- one visible 540 ms lightening, roughly three times the original short effect,
  without repeated blinking or layout movement;
- real output glyphs still visible independently of decoration; and
- no vertical stroke bar.

`ls -ll` in PowerShell must show its parameter-binding error inside a failure
surface. The silent final command must not create an empty rectangle or borrow
the previous command's surface. Failure to group any output-producing command,
including `ls -ll`, is a failure even if the end rule alone is visible.
Stock CMD keeps status and duration neutral but must still show its terminal-
owned completion datetime. Change the OS timezone between two harmless commands:
the earlier label must remain unchanged and the later label must use the new
local timezone. Repeat across a DST boundary in a controlled VM when claiming
timezone-transition evidence.

### UI-06 — result separation across overflow and scrollback

1. Record the current row count from the footer.
2. Run bounded commands producing approximately `rows-2`, `rows-1`, `rows`,
   `rows+1`, `2*rows-1`, `2*rows`, and `2*rows+1` lines with a unique final
   token.
3. At each boundary, wait for the next prompt, scroll up/down, resize, and search
   for the final token.
4. Produce enough later output to evict the original command prompt while its
   output/following prompt boundary remains visible.

Expected result: exactly one separation belongs to each completion; no duplicate
or stale surface appears; output remains grouped even when the source prompt is
offscreen or fully evicted; repaint/reflow does not lose the boundary or change
its datetime.

### UI-07 — reduced motion and accessibility redundancy

Enable the OS reduced-motion preference, relaunch, and repeat UI-05.

Expected result: persistent tint, whitespace, end rule, and text/icon status
remain, while the temporary lightening is suppressed or reduced. Meaning never
depends on motion or color alone.

### INPUT-01 — windows, global tabs, and pane-local tabs

1. Create an OS window, two window-level tabs, and two pane-local tabs.
2. Cycle forward/backward through each hierarchy.
3. Select numbered window tabs where supported.
4. Close one local tab, then one global tab, then the secondary OS window.

Expected result: each shortcut affects only its documented hierarchy; local-tab
close never removes the split; `Ctrl+F4` never removes a split; title/focus and
PTY remain isolated.

### INPUT-02 — fresh versus cloned splits

1. Change to the synthetic fixture directory and set a harmless shell-local
   variable.
2. Create a fresh right split and record its shell/profile/directory.
3. Create a cloned down split and record the same properties.
4. Run distinct output in all panes.

Expected result: fresh split starts the configured default shell using fresh
launch context; clone preserves the active shell/profile/directory descriptor
but creates an independent PTY. Input/output and process exit in one pane never
appear in a sibling.

### INPUT-03 — geometric focus and divider resize

Create an asymmetric three-pane layout. Use nearest-pane shortcuts, F6 cycling,
and divider-resize shortcuts.

Expected result: geometric navigation selects the nearest pane in the requested
direction; cycle order is stable; only the intended divider moves; minimum sizes
are respected; no pane becomes unreachable.

### INPUT-04 — hover-to-scroll selects the pointer pane

1. Fill two visible panes with distinguishable scrollback.
2. Select pane A.
3. Move the pointer over pane B without clicking and scroll.
4. Type one harmless character after the wheel event.

Expected result: pane B becomes selected when scrolling starts and only B's
history moves. The later character goes to B. If A scrolls because it was the
previous selection, the regression has returned.

### INPUT-05 — Ctrl+V in local, WSL, and SSH sessions

Copy the synthetic string `AUTOMEXIA-PASTE-NO-ENTER` and paste into PowerShell,
CMD, WSL Bash, and a manual system-SSH session where available.

Expected result: the exact string appears in the selected line editor once,
without newline/Enter; bracketed paste and control filtering remain active;
another pane receives nothing. Press Escape or clear the line rather than
executing it.

### INPUT-06 — copy versus interrupt

1. Select text and press `Ctrl+C`; verify clipboard contents.
2. Clear selection, start loopback ping, and press `Ctrl+C`.

Expected result: selection copies in the first case; interrupt reaches the child
in the second. Neither action produces both effects.

### INPUT-07 — right/middle/left mouse ownership

Test right-click with and without a selection, middle-click where primary
selection exists, left-click focus, link activation, and a mouse-reporting
application with the documented Shift override.

Expected result: right-click copies-and-clears an existing selection or pastes
when none exists; middle-click uses primary selection only where supported;
left-click never pastes; mouse-reporting apps retain ownership unless the host
override is used.

### INPUT-08 — custom binding replacement and rollback

1. Add one harmless custom binding from [keyboard reference](reference/keyboard.md),
   reload, and use it.
2. Add an unknown action, reload, and retry the prior registry.
3. Remove the custom entry and reload.

Expected result: the valid custom trigger replaces only its exact matching
default; the unknown action is rejected and last-known-good registry remains;
removal restores the normal default without recreating a PTY.

### INPUT-09 — jump between commands in the selected pane

1. Start a supported session-only-integrated PowerShell, CMD, Bash, Zsh, Fish,
   or WSL shell and create enough retained history to exceed one viewport. Use
   at least five distinguishable commands: a short success, a silent command
   such as `true` (or `$null = 1` in PowerShell), a multiline command, a
   failure, and a command producing more than one screen of output.
2. Leave a harmless unsubmitted token such as `DO_NOT_RUN_73491` in the live
   editor. Record the selected pane, visible token, and live prompt.
3. Press `Ctrl+Shift+Up` on Windows/Linux/BSD or `Cmd+Shift+Up` on macOS once,
   then repeatedly. Confirm each press anchors the immediately preceding
   retained command at the top and eventually stops at the oldest mark.
4. Press the corresponding Down shortcut repeatedly. Confirm it advances one
   command at a time and stops at the live prompt without overshooting.
5. Create a second pane with different output. Select pane A, use both command
   shortcuts, and verify pane B does not move. Select pane B and repeat.
6. Open pane search and press the command-jump chord; close search. Enter Vi
   mode and repeat; leave Vi mode. Run an alternate-screen program such as
   `less`, `vim`, or `nvim`, repeat, then exit it.
7. If available, launch a deliberately unintegrated custom shell that emits no
   OSC 133 marks and try both shortcuts.
8. Open the command palette and invoke **Jump to Previous Command** and **Jump
   to Next Command**. Confirm they match the direct shortcut behavior.

Expected result: only the selected pane's viewport moves, exactly one retained
semantic command per successful press. Wrapped/multiline prompts count once;
adjacent prompts after a silent command remain two separate targets.
The unsubmitted token, raw command line, prompt identity, process, and sibling
pane remain unchanged; no command is recalled, edited, rerun, submitted, or
written to the PTY. First/last boundaries and an unintegrated shell are clean
no-ops. Search, Vi, and the alternate-screen program keep key ownership. The
command-palette and direct-shortcut paths produce the same viewport result.

### SHELL-01 — integration appears before first input

Launch each available shell as a new session without pressing a key.

Expected result: supported sessions show shell/OS/user context and complete
working path before first input; no installer prompt, WSL provisioning, or
profile rewrite occurs during launch.

### SHELL-02 — prompt status, duration, and path updates

1. Change directories twice.
2. Run a success, a failure, and a command lasting at least one second.
3. Inspect the next prompt after each.

Expected result: path/context update at prompt boundaries; success/failure and
duration are attached to the command that completed; a stale native
`LASTEXITCODE` cannot turn a newer shell failure into success.

### SHELL-03 — Git context

In a disposable directory:

```text
git init
git status
```

Create and modify one synthetic file.

Expected result: Git context appears/updates without entering command output,
without running provider/network work on every key, and without delaying shell
input. Leaving the repository removes the context at the next supported prompt.

### SHELL-04 — icon-aware listings preserve pipeline objects

1. Run the integration's interactive `ls`/`ll` wrapper and inspect icons/colors.
2. Pipe listing objects to a serializer or count operation appropriate to the
   shell.
3. Compare with native `dir`/explicit command behavior documented for the shell.

Expected result: interactive listing is easier to scan, but piped data remains
the native typed/object or byte stream. No decorative icon contaminates a pipe,
file, or redirected output.

### SHELL-05 — Docker/Kubernetes/cloud/Terraform context

With only disposable/public contexts configured, run `docker version`, the kind
commands from Part I, and installed CLI version commands. If Terraform is
installed, create only a disposable local workspace.

Expected result: available public context tags update at prompt boundaries,
include text/icon redundancy, and do not expose credentials. Missing tools show
no false-ready context. Opening or typing in Automexia must not itself call any
provider.

### SHELL-06 — persistent integration doctor/install/uninstall

1. Hash/back up the disposable shell profile.
2. Run `automexia shell-integration doctor`.
3. Run `install`, repeat it, then `install --force`.
4. Start a nested shell and inspect integration.
5. Run `uninstall`, repeat it, and compare the profile with its original state.

Expected result: doctor is read-only; install is explicit and idempotent; force
repairs only Automexia-owned markers; nested shell works; uninstall removes only
owned blocks/files and preserves unrelated profile content. Windows execution
policy is respected and never bypassed.

### SHELL-07 — missing integration resources fail safely

Against a disposable copied binary/resource layout, make the integration
resource tree unavailable and launch.

Expected result: the real unmodified shell still opens; Automexia does not fetch,
install, rewrite profiles, trust an inherited arbitrary resource path, or fail
the terminal core.

### SHELL-08 — WSL Bash/Zsh/Fish isolation

Open separate WSL sessions for Bash, Zsh, and Fish. Repeat SHELL-01, SHELL-02,
TERM-04, INPUT-05, and UI-05.

Expected result: shell-native editing/history/completion remain authoritative;
integration does not stack when sourced repeatedly; result grouping works for
stdout, multiline, stderr, silent, and overflow cases; one distribution/session
does not alter another.

### EXT-01 — Docker and Kubernetes ordinary terminal interoperability

Inside Automexia, run the Part I Docker/kind/kubectl commands and a bounded watch:

```text
kubectl get pods --all-namespaces --context kind-automexia-manual
kubectl get events --all-namespaces --context kind-automexia-manual --sort-by=.metadata.creationTimestamp
```

Expected result: output, Unicode, colors, resize, search, copy, and interrupt act
like the native shell; no Automexia provider adapter claims ownership; explicit
context remains unchanged after the test.

### IMG-01 — local image hover preview

1. List the synthetic PNG path in a supported shell listing.
2. Move the pointer over the path.
3. Move away without clicking.

Expected result: a bounded branded preview appears without upload/network work,
shows filename/dimensions/size text, stays inside the viewport, and disappears
when hover ownership ends.

### IMG-02 — pin, browse, and dismiss

1. Add two synthetic raster images to the fixture folder.
2. Click one preview to pin it.
3. Use arrow keys to browse visible image paths.
4. Press Escape.

Expected result: pinning survives pointer movement, browsing changes only among
visible eligible paths, Escape closes, and underlying terminal input remains
inert while the overlay owns keys.

### IMG-03 — keyboard Quick Look

Select a displayed local image path and press `Ctrl+Alt+I` (`Cmd+Alt+I` on
macOS).

Expected result: the selected/pointer-targeted local raster opens once; no Enter
or path text reaches the PTY; focus restores after dismissal.

### IMG-04 — invalid, oversized, linked, and replaced images

Try a text file renamed as PNG, a file beyond documented limits, a symlink, and
a file replaced between listing and click.

Expected result: each fails with a bounded actionable local error; no partial
decode, unbounded allocation, path traversal, stale preview, crash, or upload
occurs. Normal terminal use continues.

### IMG-05 — terminal image protocols

Use known-safe local fixtures/tools for Kitty, Sixel, and iTerm2 protocol images.
Do not use untrusted arbitrary payloads.

Expected result: supported protocol state renders within terminal ownership;
scroll/reflow/clear and alternate-screen transitions do not leak images into
unrelated panes; malformed/oversized payloads are rejected without losing text.
## Part III: Ghostty-compatible keyboard and lifecycle scenarios

### GHOST-01 — inspect pinned provenance without starting the GUI

Run:

```text
automexia --list-actions --aliases --unavailable
automexia --list-keybinds --profile ghostty-1.3 --platform windows --effective --shadowing
automexia --list-keybinds --profile ghostty-1.3 --platform linux-bsd --json
automexia --list-keybinds --profile ghostty-1.3 --explain <reviewed-trigger-or-action>
```

Expected result: every command exits before GUI initialization and starts no
Ghostty/shell/PTY. Output identifies the pinned Ghostty 1.3.1 provenance,
origins/policies/diagnostics and registry counts. Windows is explicitly labelled
an Automexia adaptation. The macOS selector fails closed until its external
native fixture exists; it must not synthesize a passing profile.

### GHOST-02 — explicit profile selection and live transactional reload

In the isolated `config.toml`, set:

```toml
[keyboard]
binding-profile = "ghostty-1.3"
```

Launch, verify a documented binding, then change to `ghostty`, then back to
`automexia`. Add one harmless user binding and one `unbind`; save after each
change. Finally add an invalid strict binding.

Expected result: profile changes are explicit and visible; `ghostty` currently
resolves to the pinned profile; user bind/unbind layers apply after the profile;
valid complete registries publish atomically. The invalid reload reports a
bounded diagnostic and leaves the previous complete registry active. Existing
users are never silently migrated.

### GHOST-03 — tabs, splits, shell fallthrough, and scope semantics

Using the generated [Ghostty bindings](generated/ghostty-1.3-keybindings.md),
test window-level tabs, independent-PTY splits, focus/resize/zoom/equalize,
selection/search and two-step clear. Verify bare `Ctrl+R` performs shell history
search and bare `Ctrl+D` remains shell EOF/fallthrough. Exercise one supported
sequence/table/chain and one visibly unavailable action.

Expected result: every supported chord invokes exactly its documented Automexia
semantic action once, with correct focused/all-surface/platform-global scope.
Independent splits have fresh PTYs. Unsupported actions are unavailable, not
mapped to a vaguely similar behavior. Escape/cancel restores the prior owner and
no modal chord leaks to PTY.

### GHOST-04 — physical/logical key, layout, IME, AltGr, and dead-key matrix

On each native keyboard layout available, test physical, named and logical keys;
Shift/Ctrl/Alt/Super combinations; AltGr; dead keys; composed characters; IME
preedit/commit; NumLock; repeated keys; and focus loss/restore. Repeat after a
profile reload and in two panes.

Expected result: precedence is physical → named → logical as documented; one
physical event cannot trigger two actions; unconsumed/fallthrough bytes remain
exact and ordered; IME/dead-key text reaches only the editor/PTY owner; a modal
never consumes composition intended for its active text field incorrectly.
Record actual native layouts—synthetic event tests do not replace them.

### GHOST-05 — dry-run migration, confirmation, atomic apply, and rollback

Create a small disposable Ghostty configuration containing bounded keybinding
and include directives only. Run:

```text
automexia migrate ghostty --input <fixture> --dry-run --json
automexia migrate ghostty --input <fixture> --output <isolated-config> --apply --confirm
```

Before apply, hash/copy the destination. Repeat apply against an existing typed
section and test a missing, linked, cyclic, oversized, invalid UTF-8, hostile
control/bidi and unsupported-action input.

Expected result: dry-run writes nothing and clearly reports supported,
unavailable and rejected mappings. Confirmed apply validates the complete
result, creates a backup and publishes atomically. Existing typed configuration,
stale/replaced input and hostile/unsupported content fail without partial edit.
No shell, Ghostty process, include evaluation or arbitrary path expansion runs.
Restore the backup and verify the original profile still loads.

### GHOST-06 — redacted compatibility inspector and modal isolation

Open the compatibility inspector from its command-palette action. Inspect active
profile, origins/diagnostics, aggregate session/topology counts and unavailable
items at normal, narrow and 300% layouts. Navigate entirely by keyboard and with
a screen reader; type while the inspector is open.

Expected result: roles/names/states and focus order are clear; content is
bounded and redacted—no clipboard, terminal text, command, host, credential,
private path or hidden selection appears. Keys never reach PTY. Escape closes and
restores exact prior focus. Closing a route invalidates its transient inspector
state.

### GHOST-07 — parked top-level-tab restore, two-step clear, and bounded history

Create a complete top-level tab with a harmless running local process, close it
through the compatibility action, inspect the newest-first parked list, and
restore the newest item. Close several tabs to the documented bound, then invoke
clear once and cancel; invoke clear again and confirm. Exercise undo/redo where
available.

Expected result: only complete closed top-level tabs use the current history;
individual split, pane-local-tab and native-window history remain unimplemented
rather than silently substituted. Restore preserves the parked owned topology
without duplicating PTY ownership. History is memory/time/count bounded,
newest-first, redacted and cleared only after the explicit second step. App exit
cleans parked processes/resources.

### GHOST-08 — generated artifacts, fuzz compilation, benchmark, and native gate

Run:

```text
cargo xtask test keybindings
cargo xtask generate keybindings --version 1.3.1
cargo xtask generate keybindings --check
cargo xtask verify keybindings
python tools/ci/check_ghostty_compatibility.py
python tools/ci/test_ghostty_compatibility.py
python tools/ci/test_ghostty_native_evidence.py
cargo check --manifest-path fuzz/Cargo.toml --bins --locked --offline
cargo bench -p automexia-keybindings --bench registry --locked -- --noplot
```

Expected result: source fixtures, generated manifests/references, compiler,
registry, frontend, migration, inspector, topology, selection, fuzz compilation
and benchmark all pass without artifact drift. Native Windows/Linux/macOS
layout/render/accessibility/resource/package evidence and the macOS fixture are
separate controlled gates; missing/incomplete/wrong-commit evidence must fail or
remain **not run**.

## Part IV: Connection Hub, system OpenSSH, routes, trust, and tunnels

### SSH-01 — install a disposable loopback OpenSSH fixture

The Connection Hub itself needs only synthetic config files. Actual connection,
host-key and tunnel tests require system OpenSSH. Prefer a disposable VM whose
firewall/network adapter cannot accept external traffic.

Verify tools first:

```text
ssh -V
ssh-keygen -t ed25519 -f <fixture-directory>/id_ed25519 -N ""
```

Windows: follow Microsoft's [OpenSSH installation guide](https://learn.microsoft.com/en-us/windows-server/administration/openssh/openssh_install_firstuse)
from an elevated PowerShell. Install `OpenSSH.Client~~~~0.0.1.0`; install the
Server capability only on the disposable host. Before starting `sshd`, edit its
reviewed `%ProgramData%\ssh\sshd_config` to use a high test port such as 22222,
`ListenAddress 127.0.0.1`, public-key authentication, password authentication
off, and the exact test user. Apply Microsoft's required owner/ACL rules, place
only the generated public key in that test user's authorized-keys file, validate
with `sshd -t`, set startup to Manual, then start the service. Verify the
listener is loopback-only with `Get-NetTCPConnection -LocalPort 22222`.

Ubuntu/Debian disposable host:

```text
sudo apt update
sudo apt install openssh-client openssh-server
```

Create `/etc/ssh/sshd_config.d/99-automexia-manual.conf` with:

```text
Port 22222
ListenAddress 127.0.0.1
PubkeyAuthentication yes
PasswordAuthentication no
PermitRootLogin no
AllowUsers <exact-test-user>
```

Append the generated `.pub` line to that user's `~/.ssh/authorized_keys`, set
`.ssh` to mode 700 and the file to 600, run `sudo sshd -t`, restart the service,
and verify `ss -ltnp` reports only `127.0.0.1:22222`. On macOS, Remote Login can
expose a network listener; use a disposable VM or separately reviewed loopback-
only sshd configuration rather than enabling it on a shared workstation.

Expected result: the fixed client/server versions are recorded, the server
accepts public-key authentication only on loopback, and no private key is copied
into the repository or evidence. Any non-loopback listener is a setup failure:
stop the service before continuing.

### HUB-01 — first-run no-scan and setup hierarchy

Launch with the isolated configuration root and synthetic SSH files located
outside normal `~/.ssh` paths. Open the Hub with `Ctrl+Shift+H`
(`Cmd+Shift+H` on macOS) or **Connection Hub (read-only)** in the palette.

Expected result: the shared-brand first-run card is compact and shows one short
description, one **Local review only · nothing connects** safety line, and the
two primary choices **Enter host** (`L`) and **Choose files** (`F`). There is no
inner grouping panel, duplicate bottom status bar, or candidate configuration
path. The header shows distinct 40-pixel **Connections C**, **Workspaces W**,
**Providers P**, and close targets without overlap. Search/filter/catalog
controls are hidden and unfocusable until records exist. No standard SSH path
is scanned, no file/process/network/agent/keychain/PTY is touched, and terminal
input is inert until the modal closes. Escape restores exact previous pane
focus.

### HUB-02 — exact-file picker, canonical review, and revocation

Create a synthetic file `ssh-config-fixture`:

```sshconfig
Host automexia-loopback
    HostName 127.0.0.1
    User <exact-test-user>
    Port 22222
    IdentityFile <absolute-fixture-private-key>
    IdentitiesOnly yes

Host automexia-routed
    HostName 127.0.0.1
    User <exact-test-user>
    Port 22222
    ProxyJump jump-a,jump-b
```

Choose **Choose files**, select only that exact file, inspect the canonical-path
review using keyboard/pointer/Page/Home/End, then cancel. Return to setup, press
`F`, and verify that the same parented native picker and review appear. Repeat
and confirm scan. Restart Automexia. While Search is focused, type `f`; while a
nested review/editor is open, press `F`; also try Ctrl/Alt/Super-modified `F`.

Expected result: cancel revokes the selection and no read occurs; confirmation
scans only the reviewed canonical regular file within limits and presents public
aliases. The grant is memory-only and gone after restart. Automexia never edits
the file or persists its path. Links, directories, replaced files, invalid UTF-8,
oversized/many files, controls/bidi and ambiguous dynamic directives fail with a
path-free diagnostic and no partial publication. Pointer and unmodified `F`
produce the same file-review path; Search receives literal `f`, nested/modified
keys do not open a picker, and no shortcut byte reaches the selected PTY.

### HUB-03 — transient literal host editor and paste/IME validation

Press `L`. Enter `127.0.0.1`, the test user and `22222` in separate fields. Use
Tab/Shift+Tab through Host → User → Port → Review → Cancel; review and return.
Repeat with missing optional fields and with invalid inputs: leading hyphen,
URI, raw IPv6, wildcard, whitespace, Unicode/control/bidi, option/shell text,
out-of-range/nondecimal port, jump and tunnel syntax. Paste and IME-commit an
invalid value.

Expected result: the valid transient target reaches the same compact review but
is classified conservatively/production until trusted profile data exists. Each
invalid value is rejected atomically—no partial paste. Focus never escapes or
reaches PTY; small/high-scale layouts show one nonoverlapping Cancel; cancel,
surface close or successful preparation clears editor contents. Nothing is
persisted or connected.

### HUB-04 — search, filters, grouping, favorites, tags, and metadata CAS

After HUB-02, exercise `/` or Ctrl/Cmd+F, `G`, `V`, `R`, `S`, `X`, Space and `T`.
Use case variants, Unicode-safe public text, no-match text, pointer controls and
IME. Review/save one favorite and normalized tags; cancel another edit. Simulate
or use tests for a concurrent stale revision.

Expected result: Search/filter/grouping is bounded and stable; **Clear** appears
only with an active filter. Search/tag fields receive mnemonic letters as text.
Tags deduplicate case-insensitively and reject controls/bidi. Saves show before/
after and use reviewed CAS; stale revision reloads/conflicts rather than
overwriting. Favorites/tags never edit ssh_config or grant authority. Recent
remains read-only until a successful future managed connection.

### HUB-05 — loading, ready, filtered, stale, unavailable, and last-known-good

Start a confirmed scan and observe Loading → Ready. Apply a no-match filter. Then
make only the copied fixture invalid/unreadable and explicitly request a newer
refresh. Restore it and refresh again.

Expected result: generation publishes before its route wake; filtered state
retains the catalog and offers clear. Failed newer refresh reports bounded
Stale/unavailable recovery while preserving the previous immutable catalog; a
successful newer generation replaces it. No private path/error text is exposed,
no half-updated rows appear, and obsolete work cannot overwrite current state.

### HUB-06 — route/screen ownership, saturation, and shutdown

Open two Automexia windows/screens with independent Hub controllers. Start a file
review in one and attempt to confirm/cancel/display it from the other. Trigger
rapid successive reviewed grants up to and beyond the bounded worker inbox, then
close one route and finally the application.

Expected result: review tokens, generation, focus and state remain screen-local.
The other screen cannot act on the review. Capacity returns an immediate
path-free busy state rather than blocking input or growing a queue. Obsolete work
is cancelled/rejected; shutdown joins the one worker and creates no post-close
wake, lingering file handle, child or thread.

### HUB-07 — responsive, keyboard, pointer, IME, and accessibility audit

Exercise Setup, Loading, populated, filtered, stale, literal editor, file review,
connection review and metadata editor at 320, 1,920 and 5,120 px-equivalent
widths and 100–300% scale. Navigate without pointer, then with pointer and IME,
then a native screen reader.

Expected result: concise title/status/help, colored icon plus redundant text,
clear hierarchy and bounded detail; no overlap with close controls, clipping or
hidden focus. Roles/names/states/position/count/actions are announced, focus is
trapped only while modal and restored on close, disabled prerequisites are
explained, and no interaction reaches terminal input. At the standard viewport,
the first-run card remains at or below 680×380 logical pixels; each section,
setup action, and close hit target remains at least 40×40 logical pixels. Verify
the C/W/P/L/F keycaps at 100%, 200%, and 300% scale and confirm long/localized
text cannot cover another action.

### HUB-08 — complete Hub model/runtime/product source evidence

Run:

```text
cargo test -p automexia-devops-ssh --locked
cargo test -p automexia-devops-ssh --test metadata_store --locked
cargo test -p automexia-ui-model --locked --test connection_hub
cargo test -p automexia-terminal --test connection_hub_runtime --locked
cargo test -p automexia-terminal --test connection_hub_controller --locked
cargo test -p automexia-terminal --bin automexia connection_hub --locked
cargo test -p automexia-terminal --bin automexia command_palette --locked
cargo bench -p automexia-terminal --bench connection_catalog --locked -- --noplot
```

Expected result: every process exits 0; no-scan opening, grants/revocation,
parser/metadata limits, CAS/recovery, generation/LKG, controller isolation,
input/focus, responsive semantics and cached-frame performance pass. This does
not prove native picker portals, pixels/screen readers, real SSH, or activation.

### SSH-02 — managed direct review and three explicit decisions

Open the valid literal or inventory alias review. Inspect package/launcher,
typed target, route, risk, host-trust/public-identity evidence, requested
`session.launch`, 60-second authorization, exact operation and PTY scope. Invoke
`A`, `S`, and `D` separately; also use pointer/focused Enter.

Expected result: Allow once/session exercise only the review policy and show
**Protected security review is still pending**; no process/network/PTY starts.
Deny returns safely. Decisions are focus-aware, one-time/session semantics are
clear, stale/expired/changed reviews are rejected, and no mnemonic reaches PTY.
The production activation constant remains false and linked candidate
unverified.

### SSH-03 — exact command copy and manual recovery boundary

Press `C` in a current review, paste into a plain text editor first, inspect it,
then—only for the loopback fixture—paste into the terminal without executing.

Expected result: one exact reviewed system-SSH command is copied with no secret,
implicit newline/Enter, shell evaluation or automatic run. It uses a canonical
executable/defensive exact arguments appropriate to the review and cannot be
copied after source/route/trust/executable/generation change. Ordinary manual
`ssh` remains available even though managed activation is off.

### SSH-04 — direct and static ProxyJump route validation

Review the direct and `automexia-routed` aliases. Test `ProxyJump none`, one hop,
8 hops, over 8 hops/2 KiB, duplicate/empty/option-like hop, multiple conflicting
values, `ProxyCommand`, `Match exec`, shell text, remote command, URI/raw IPv6,
tunnel and KEX override fixtures.

Expected result: direct stays direct; the first unambiguous static canonical
comma-separated jump list becomes one exact `-J` argument. Dynamic, ambiguous,
oversized or code-capable forms fail closed with path-free diagnostics. The
review explains the full route; source/route change invalidates it. Automexia
never runs `ssh -G`, evaluates Match/ProxyCommand, or overrides OpenSSH KEX/
weak-crypto policy.

### SSH-05 — host trust, identity readiness, and changed-key denial

Use source fixtures for unknown/first-use/trusted/changed/revoked/malformed host
keys and ready/missing/locked/expired agent/key/certificate/hardware identity
states. With the loopback server, connect manually once to a disposable
`known_hosts` file, then replace only the disposable server host key and retry
manually.

Expected result: the review displays full public algorithm/fingerprint and
strict state with redundant text/icon/color. Changed/revoked is **BLOCKED** and
cannot form a launch binding. Identity state is actionable but contains no
private key, certificate/token, agent contents or path. System OpenSSH owns the
actual prompt and known_hosts update; Automexia never auto-accepts or edits it.
Restore/remove only disposable trust files.

### SSH-06 — typed tunnel review, strong confirmation, and lifecycle source

Use source fixtures for local (`-L`), remote (`-R`) and dynamic (`-D`) tunnels;
loopback and non-loopback binds; production risk; collision; planned/starting/
ready/failed/cancelled/closed transitions; endpoint drift and stale events.

Expected result: compact Safety row shows direction, exact public endpoints,
session lifetime, OpenSSH ownership, state and redundant risk. Local/dynamic
default to `127.0.0.1`. Remote, non-loopback or production requires Allow once;
Allow for session is disabled, unfocusable, pointer-inert and `S` is rejected.
One route/session lease owns bounded state; stale/terminal reversal is rejected;
route end closes nonterminal state. Current product opens no listener or child.

### SSH-07 — actual manual system-SSH and loopback tunnel interoperability

Use only the SSH-01 fixture and a disposable known-hosts file:

```text
ssh -F <synthetic-config> -o UserKnownHostsFile=<disposable-known-hosts> automexia-loopback
```

Verify interactive input, resize, Ctrl+C, Unicode, output search/selection and
clean `exit`. For a tunnel, start a harmless loopback service on the remote test
host, then in a separate Automexia pane run an explicit system command such as:

```text
ssh -N -F <synthetic-config> -o UserKnownHostsFile=<disposable-known-hosts> -L 127.0.0.1:18080:127.0.0.1:<remote-test-port> automexia-loopback
```

Connect to `127.0.0.1:18080`, stop SSH with Ctrl+C, and verify the listener is
gone. Repeat only with reviewed nonproduction endpoints.

Expected result: system OpenSSH—not managed Automexia—owns prompts,
authentication, child and sockets; terminal behavior remains correct. Closing
the pane/route or app leaves no ssh descendant/listener. Never bind `0.0.0.0`,
use production, disable host-key verification, or expose a test server.

### SSH-08 — complete D0/D3/M3/M4/M5 deterministic evidence

Run:

```text
cargo test -p automexia-extension-api --lib --locked
cargo test -p teletypewriter --locked
cargo test -p automexia-connectivity --test direct_openssh_review --locked
cargo test -p automexia-connectivity --test direct_openssh_tunnels --locked
cargo test -p automexia-devops-ssh --locked
cargo test -p automexia-ui-model --test direct_openssh_review --locked
cargo test -p automexia-terminal direct_openssh --locked
cargo test -p automexia-terminal connection_hub --locked
python tools/ci/check_session_launch_d0.py
python tools/ci/test_session_launch_d0.py
python tools/ci/native_openssh_evidence.py --check-repository
python tools/ci/test_native_openssh_evidence.py
python tools/ci/test_pr_policy.py
cargo xtask verify architecture
```

Expected result: all deterministic owners exit 0; schemas/immutability,
exact argv, trust/routes/tunnels, broker/runner/PTY ownership, review input,
receipts/reconnect, 1/10/50 lifecycles, redaction, mutation/policy and synthetic
manifest pass while production remains disabled. The safe prerequisite probe
may truthfully report missing `sshd`; that is external-prerequisite evidence,
not a managed-SSH pass.

### SSH-09 — protected real-native F5 evidence and cleanup

Follow [F5 native OpenSSH assurance](F5-NATIVE-OPENSSH-ASSURANCE.md) on one
ephemeral JIT runner per Windows/macOS/Linux OS/architecture with the exact
fixed `ssh`, `ssh-add`, `ssh-keygen`, `sshd`, clean commit and hashed application/
package/client. Dispatch the protected manual workflow only after independent
review. Run all 23 required direct/routed/trust/identity/tunnel/hostile-output/
cancellation/1-10-50/resource/manual-client/disable/uninstall scenarios.

Expected result: the private strict manifest binds real host/tools/artifacts and
records zero descendants/listeners/handles/tasks/routes/temp files after each
case; the path-free summary passes exact-commit validation. WSL is separately
denied as native release evidence. Missing runner/server/signature/approval,
version/hash drift, linked/changing file, synthetic result, redaction canary,
nonzero cleanup or failed scenario blocks activation. Stop the disposable sshd,
remove only fixture accounts/keys/known-hosts/config and verify ordinary manual
SSH still works before recording completion.

## Part V: completion, Quick Actions, aliases, and workspace-task scenarios

All mutations in this part must use the isolated `AUTOMEXIA_CONFIG_HOME`. Keep a
copy of each preview's revision/generation and use it only once.

### ACT-01 — empty-store doctor and read-only commands

Run before creating any action:

```text
automexia actions doctor --json
automexia actions list --json
automexia actions show missing.action
```

Expected result: doctor/list report a valid empty or absent store without
creating it; revision/count are bounded and redacted; showing a missing ID is an
actionable not-found error; no provider, PTY input, profile, or network work
occurs.

### ACT-02 — preview and apply one typed action

Create `one-action.toml` in the synthetic fixture root:

```toml
schema_version = 1
revision = 0

[[actions]]
id = "shell.git-status"
display_name = "Show Git status"
description = "Insert a reviewed Git status command without executing it."
tags = ["git", "read-only"]
scope = "global-user"
shells = ["powershell", "bash", "zsh", "fish", "cmd"]
risk = "read-only"
execution = "insert"
enabled = true

[actions.template]
kind = "typed-argv"
executable_id = "git"
arguments = [{ kind = "literal", value = "status" }]

[actions.working_directory_policy]
kind = "inherit"

[actions.provenance]
kind = "user"
```

Run:

```text
automexia actions put one-action.toml
automexia actions list
automexia actions put one-action.toml --apply --expected-revision 0
automexia actions list
automexia actions show shell.git-status
```

Expected result: preview validates exactly one action and writes nothing; first
list is still empty; apply reports a newer revision; second list shows metadata
but omits command template; explicit `show` reveals the reviewed typed argv. A
repeat without `--replace`, stale revision, linked file, extra action, unknown
field, secret-like value, control/bidi text, or malformed TOML fails without
changing the current revision.

### ACT-03 — Quick Action search, placeholders, insert, and copy

1. Open Quick Actions with `Ctrl+Shift+O` (`Cmd+Shift+O` on macOS).
2. Search for `Show Git status` and inspect source, shell, risk, scope, health,
   exact expansion, and conflict state.
3. Choose **Insert without Enter**.
4. Verify the line editor contains `git status` but has not run it; clear it.
5. Reopen and choose copy; inspect clipboard in a plain editor.
6. Add the repository's known-valid placeholder fixture through the normal
   preview/import flow and verify public placeholder entry and quoting.

Expected result: search ranking is stable; typed values remain in shell-native
editor ownership; insert/copy is exact once, contains no implicit newline, and
never sends Enter. Placeholder controls reject control/bidi/oversized text.
Closing/focus change cancels obsolete work and affects no sibling pane.

### ACT-04 — persistence, concurrent CAS, tamper, and recovery

1. Close/restart Automexia and list the saved action.
2. Open two shells with the same isolated root. Preview two different edits at
   the same revision; apply the first, then the second.
3. Corrupt or truncate only a disposable copy of `actions.toml` after recording
   its hash; run doctor/list and restart.
4. Review the available previous generation and use
   `automexia actions recover <revision>` without and then with `--apply` only
   when the primary is rejected.

Expected result: restart loads the action; first writer wins and the stale second
writer is rejected; malformed/oversized/tampered reload retains an immutable
last-known-good snapshot with a redacted diagnostic; preview recovery writes
nothing; applied recovery is monotonic and cannot replace a valid primary.

### ACT-05 — export, import, conflict, and machine-path consent

1. Export to a new exact file; inspect permissions and content.
2. Preview import into another isolated root.
3. Apply with the displayed revision.
4. Reimport to create a conflict, first without and then with separately reviewed
   conflict consent.
5. Test overwrite and machine-path flags only with public synthetic paths.

Expected result: export is private, atomic, digest/versioned, and excludes
machine paths by default; import is bounded/no-follow/dry-run-first; apply needs
CAS; conflicts and machine paths need separate consent; session/built-in actions,
secrets, and unknown schema never transfer.

### ACT-06 — negative fixture and policy matrix

Run the parser/policy against the checked-in hostile fixtures under
`tests/fixtures/command-productivity/cp2-quick-actions/`.

Expected result: duplicate IDs, unknown fields, secret defaults/aliases, missing
placeholders, shell mismatch, destructive or unreviewed aliasing, raw-insert
aliasing, unsafe bidi, and unsupported command-alias arguments each fail for the
named reason. No fallback turns rejected text into executable shell code.

### ACT-07 — responsive, keyboard, accessibility, and no-PTY audit

Repeat ACT-03 with zero, one, and maximum-sized fixture catalogs; tiny through
8K-equivalent viewport; 100–300% scale; long Unicode; high contrast; reduced
motion; keyboard; pointer; IME; and available screen readers.

Expected result: loading/empty/unavailable/conflict/confirmation states are
explicit; focus order and accessible name contain risk/source/status in text;
destructive actions require a second confirmation; hidden or clipped controls
cannot receive input; opening/searching/reviewing sends zero PTY bytes.

### PACK-01 — list the immutable reviewed packs

Run:

```text
automexia packs list
automexia packs list --json
automexia packs doctor
```

Expected result: eleven immutable pack records and their disabled-by-default
state are discoverable; JSON is strict; registry doctor says
`registry-ready` only and does not claim providers are installed or healthy; no
tool/provider is started.

### PACK-02 — inspect one pack and action

Choose one `PACK_ID` from PACK-01:

```text
automexia packs show <PACK_ID>
automexia packs doctor <PACK_ID> --missing
```

Expected result: the manifest shows reviewed provenance/version, HTTPS docs,
tool family, completion policy, exact action metadata and risk. The supplied
`--missing` observation produces a missing-tool state without probing the host.

### PACK-03 — enable remains preview-first

Choose a read-only `ACTION_ID` from the shown pack:

```text
automexia packs enable <PACK_ID> <ACTION_ID>
automexia packs enable <PACK_ID> <ACTION_ID> --apply --expected-revision <N>
```

Expected result: preview displays exact argv tokens, effect, risk, docs, alias
eligibility, registry digest, and revision and writes nothing. Apply creates one
user action only at the reviewed revision, refuses overwrite, leaves alias
disabled, and never executes it.

### PACK-04 — risk and provenance enforcement

Review one inspection action, one context-changing/authentication action, and
one destructive/privileged action.

Expected result: risk floors and effects match immutable pack metadata.
Context-changing, authentication, destructive, and privileged actions are
ineligible for persistent aliases. Digest/provenance drift or version regression
fails instead of silently updating.

### PACK-05 — remove and fallback

Remove the materialized user action through preview/CAS. List packs again.

Expected result: the user copy disappears, immutable disabled pack remains
discoverable, external tool configuration is unchanged, and ordinary shell
commands continue working.
### ALIAS-01 — dry-run compiler and native parser checks

With `shell.git-status` saved, run:

```text
automexia aliases list
automexia aliases preview --show-source
automexia aliases test
automexia aliases doctor --json
```

Expected result: list/preview/test/doctor are read-only; all five shell
projections are deterministic; generated source appears only with
`--show-source`; each available native parser is tested in an isolated temporary
root; no provider/action runs and no profile changes.

### ALIAS-02 — enable with exact source and generation CAS

Run the preview form appropriate to the exact `--help` output:

```text
automexia aliases enable shell.git-status --name gst-auto --shell powershell --shell bash --shell zsh --shell fish --shell cmd
```

Review collision, completion, tool identity, risk, source revision and candidate
generation. Apply by repeating the command with the displayed
`--apply --expected-revision <N> --expected-generation <DIGEST>` values.

Expected result: preview writes nothing; apply publishes one private immutable
generation atomically and changes the activation pointer last. Native aliases,
functions, macros, executables, and completers win by default. A stale revision
or generation fails without partial publication.

### ALIAS-03 — native shell lifecycle and exact arguments

Open fresh PowerShell, CMD, Bash, Zsh, and Fish sessions as applicable. Run
`gst-auto` inside a disposable Git repository. Pass an unexpected argument if
the action's policy is `None`.

Expected result: alias invokes only exact `git status`; unexpected arguments are
rejected rather than appended; shell exit status/history/editor/completion remain
native; startup does not run the action, provider, or network. If CMD requires a
new session, reload output says so explicitly.

### ALIAS-04 — collision and exact override review

Create a native alias/function named `gst-auto` before Automexia startup and
retry enable/reload.

Expected result: native definition wins. An override is impossible without the
specific explicit policy flags printed by `--help`, exact owner fingerprint,
matching shell/name, User provenance, and still-current observation. Forged or
changed ownership fails closed.

### ALIAS-05 — rename, disable, regenerate, reload, and disable-all

Preview and apply each command with current CAS values:

```text
automexia aliases rename shell.git-status gst-automexia
automexia aliases disable shell.git-status
automexia aliases regenerate
automexia aliases reload --shell powershell
automexia aliases disable-all
```

Expected result: every mutation is preview-first; activation switches all-old or
all-new, never mixed; reload prints the native reload action but does not start a
shell; disable-all changes only the activation pointer and preserves canonical
actions/generations.

### ALIAS-06 — rollback and last-known-good

Create two valid generations, then preview/apply rollback using the current
generation. Tamper with a disposable generated artifact and run doctor/reload.

Expected result: rollback swaps only authenticated current/previous generations;
tampered digest/permissions/topology are rejected; last-known-good native
behavior remains; doctor performs no repair.

### ALIAS-07 — startup/resource repetition

Open/close 50 fresh shells with aliases enabled, record startup latency and
process/file-handle growth, then repeat after disable-all.

Expected result: no provider/action runs at startup; generated artifact is read
once per shell; resources return near baseline after closure; disabled startup
matches native behavior. This manual sample is diagnostic, not the controlled
30-day baseline.

### ALIAS-08 — clean removal

Disable all aliases, uninstall persistent shell integration if installed solely
for this test, and inspect profiles/generated roots/canonical action store.

Expected result: Automexia-owned profile markers and activation are gone;
unrelated native definitions are byte-identical; canonical actions remain unless
the user separately removes them; unexpected topology makes uninstall refuse
before deleting anything.

### IMPORT-01 — preview a selected native alias inventory

Create a regular synthetic inventory containing only simple aliases, for example
Bash/Zsh lines `alias gst='git status'` and `alias kpods='kubectl get pods'`.
Preview one exact name:

```text
automexia actions import-aliases --source bash --input <inventory> --name gst
```

Expected result: Automexia reads only the supplied file, starts no shell/Git/
provider, selects exactly `gst`, and previews an Imported/Mutating/Insert action
without alias projection. The source file remains unchanged.

### IMPORT-02 — apply, rename, conflict, and reject hostile aliases

Apply IMPORT-01 with the displayed revision. Then test an explicit portable
action-ID rename and a deliberate existing-ID conflict. Add hostile examples
using pipes, redirects, substitution, Git `!`, secret-like values, machine paths,
controls/bidi, and metacharacters.

Expected result: apply needs CAS; rename is explicit; conflict needs
`--replace-conflicts`; hostile/complex entries fail closed; native inventory is
never edited or executed.

### IMPORT-03 — create a workspace task bridge

In a disposable workspace with an existing `just`, Task, or mise task that only
prints a public sentinel, run `automexia actions task-put` with explicit
workspace, runner, task, action ID, display name, and shell. Preview first, then
apply with the shown Quick Action revision.

Expected result: `.automexia/actions.toml` contains only an exact typed runner
plus task reference; action is Mutating/Insert/WorkspaceRoot/WorkspaceTask,
never an alias; Automexia does not discover/list/parse/copy recipes or execute
the task.

### IMPORT-04 — workspace trust, publication, and exact insertion

1. Run `workspace-doctor` before trust.
2. Preview/apply `workspace-trust` with the displayed trust revision.
3. Launch Automexia in that exact workspace and open Quick Actions.
4. Review and insert the task; do not press Enter.

Expected result: untrusted action is absent; trust receipt contains no path and
binds exact workspace identity/source digest/revision; trusted action appears
only for that route/workspace; insertion is exact and has no Enter. Lookup walks
at most 64 ancestors and stale route authorization expires quickly.

### IMPORT-05 — source change and revocation

Change the workspace task source after trust, then refresh/review. Preview/apply
`workspace-revoke` and `task-remove` with current revisions.

Expected result: any source identity/digest/revision change removes the layer
until fresh explicit trust; revoke affects only that workspace; removal is
preview-first; another workspace/session remains isolated.

### IMPORT-06 — links, replacement, WSL, and failure handling

Test linked workspace/source/trust files, replacement during review, malformed
TOML, lock contention, read-only storage, and unresolved WSL guest paths.

Expected result: every case fails closed with a redacted actionable code; no
source mutation, execution, implicit Enter, trust creation during read, or
cross-host path guess occurs; last-known-good unrelated actions remain.

### IMPORT-07 — focused CP3.3 evidence

Run:

```text
cargo test -p automexia-command-productivity --test quick_action_imports --locked
cargo test -p automexia-terminal --test quick_action_native_import --locked
cargo test -p automexia-terminal --test quick_action_workspace_trust --locked
python tools/ci/check_command_productivity_cp33.py
python tools/ci/test_command_productivity_cp33.py
```

Expected result: parser, selection, conflicts, trust, source change, revocation,
insertion authorization, no-follow and policy mutations pass. Hosted native,
accessibility, and 30-day evidence remain external.
### COMP-01 — native completion baseline

Before enabling Automexia-managed provider artifacts, test each shell's native
Tab/history completion with common installed commands and a user-defined
completer.

Expected result: PowerShell/PSReadLine, Bash/Readline, Zsh/ZLE, Fish, or CMD owns
editing, cursor, history, quoting and candidate display. Automexia adds no second
popup and never sends Enter.

### COMP-02 — completion doctor is read-only

Run:

```text
cargo xtask completion doctor
```

Expected result: supported shells, fixed provider/cache health, executable and
artifact status are reported without invoking provider completion definitions,
network, credentials, shell profiles, or user files.

### COMP-03 — explicit provider refresh

Choose only a provider ID and shell reported by doctor and review exact help:

```text
cargo xtask completion refresh --provider <ID> --shell <SHELL>
```

Expected result: refresh is explicit, bounded by process/output/time limits,
generates one private reviewed artifact, and does not authenticate or change
provider context. A PowerShell native override needs its separate flag and exact
collision review.

### COMP-04 — completion in empty/prefix/mid-token/quoted/Unicode contexts

Exercise native editor completion at an empty line, command prefix, mid-token,
quoted path with spaces, Unicode path, user completer collision, and maximum
bounded input.

Expected result: candidates and replacement spans are shell-correct; cursor/
selection/history remain native; duplicates/stale generations/controls/bidi/
oversized output are rejected; no provider work runs while typing.

### COMP-05 — disable, environment disable, remove, and fallback

1. Use `cargo xtask completion disable`, start a shell, and test native
   completion.
2. Re-enable, then launch one shell with `AUTOMEXIA_COMPLETION_DISABLED=1`.
3. Remove one provider/shell artifact with `completion remove`.

Expected result: each disabled/removed state falls back to normal native
completion; no broken key or empty second UI remains; only exact Automexia-owned
artifact is removed.

### COMP-06 — provider failure and lifecycle

Test missing executable, unsupported version, timeout, malformed/oversized
output, interrupted refresh, repeated refresh/remove, and shell shutdown.

Expected result: last-known-good or native fallback stays usable; no partial
artifact, orphan process, handle, socket, task, or growing cache remains;
diagnostics contain no credential/environment values.

### COMP-07 — focused completion evidence

Run the CP1 focused tests/checkers documented by `cargo xtask completion --help`
and [command productivity compatibility](COMMAND-PRODUCTIVITY-COMPATIBILITY.md),
then record p50/p95/p99 only from the controlled benchmark method.

Expected result: registration, candidates, collision, disable/remove,
startup/typing no-provider work, and lifecycle checks pass. A local smoke does
not replace native three-OS or 30-day evidence.

### SUG-01 — CP5 remains preview-disabled

Open normal PowerShell, Bash, Zsh, and Fish sessions; type history-like prefixes,
misspellings, Unicode and long commands; try common suggestion chords.

Expected result: no Automexia suggestion popup or public binding appears; no
helper starts through ordinary shell integration; native editor suggestions and
CP1 completion remain unchanged. `Ctrl+Space`, Tab, Right and Enter are not
reserved globally.

### SUG-02 — inert helper/version and bootstrap boundary

Build and run only the documented inert helper/version/bootstrap checks from
[CP5 suggestion testing](CP5-SUGGESTION-TESTING.md). Do not source generated
adapter scaffolds into a real profile.

Expected result: helper identifies its protocol/version and refuses activation
without authenticated endpoint/capability/session/generation context. Unsupported,
disconnected, killed, or disabled adapters return to native behavior.

### SUG-03 — protocol, route, span, and stale-result source evidence

Run:

```text
cargo test -p automexia-terminal --locked --test suggestion_helper_bootstrap
cargo test -p automexia-terminal --locked --test suggestion_helper_endpoint
cargo test -p automexia-terminal --locked --test suggestion_helper_runner
cargo test -p automexia-terminal --locked --test suggestion_helper_session
cargo test -p automexia-terminal --locked --test suggestion_helper_transport
cargo test -p automexia-terminal --locked --test suggestion_publication
cargo test -p automexia-terminal --locked --test suggestions_broker
```

Expected result: strict fragmented framing, authenticated endpoint, pane/route/
capability/generation/replacement-span ownership, stale/cross-pane/replay/
oversize/hostile rejection, cancellation/kill/shutdown, bounded publication,
and no Enter pass.

### SUG-04 — native adapter fixtures and lifecycle

Run the exact PowerShell and portable adapter fixtures from
[CP5 suggestion testing](CP5-SUGGESTION-TESTING.md), including invalid UTF-8,
C0/C1/bidi, Unicode replacement, stale/unknown/extra/oversized status, collision,
disable, and cleanup cases.

Expected result: PowerShell/Bash/Zsh/Fish editor buffer, cursor, selection,
quoting, history and generation remain authoritative; replacement occurs exactly
once only after revalidation; no PTY text/Enter, profile mutation, orphan helper,
handle/socket/task, or private log remains.

### SUG-05 — activation and external evidence stay blocked

Inspect config, keyboard list, palette, startup processes, network connections,
and isolated profile before/after SUG-01.

Expected result: no public setting/action/shortcut, signed runtime helper launch,
endpoint, WSL relay, popup, network/provider work, or persisted suggestion data
exists. Interactive PowerShell insertion, signed/attested package launch, native
Linux/macOS endpoints, live pixels, controlled screen readers, 1,000 cycles and
30-day preview evidence remain **not run** unless separately completed.

### ABS-01 — CP5.0 research is not a product feature

Run the CP5.0 research checker/fixtures named in [testing](TESTING.md#command-productivity-cp50-research)
and inspect the application for a CP5.0 setting or surface.

Expected result: reproducible research artifacts/checks may pass, but no product
feature, shortcut, provider, network request, or editor replacement appears.

### ABS-02 — Production Operations PO0 is documentation-only

Run:

```text
python tools/ci/check_production_operations_po0.py
python tools/ci/test_production_operations_po0.py
```

Expected result: the exact proposal digest and mutations pass while repository
nonactivation proves no provider capability, watcher, completion source,
investigation UI, live-log controller, managed diagnostic session, setting,
journal, model, process, network request, or execution authority exists. Do not
look for or claim PO1–PO8 manual product behavior.

### ECO-01 — product surface denies ecosystem activation

Open the Extensions marketplace shortcut/palette surface and inspect local
state.

Expected result: the current source may expose branded inspection/review
plumbing, but public download, install/activate, provider calls, grants and model
execution are unavailable. The modal sends no PTY input and ordinary terminal/
CP1–CP3 behavior remains usable.

### ECO-02 — accepted manifest, capability, consent, and package contracts

Run:

```text
python tools/ci/check_ecosystem_d7_cp6.py
python tools/ci/test_ecosystem_d7_cp6.py
cargo test -p automexia-ecosystem --locked
cargo test -p automexia-ecosystem-runtime --locked
```

Expected result: exact digest/acceptance, 28 limits, threats/gates, strict JSON,
portable paths, grant/revoke/generation, lifecycle/quarantine/queues, selected-
input redaction/consent/staleness, real signed ZIP/provenance/SBOM/license,
atomic disabled storage and exact uninstall pass. No public activation occurs.

### ECO-03 — disabled Component Model conformance host

Run:

```text
cargo test -p automexia-ecosystem-runtime --features component-host --locked
```

Expected result: public activation is denied; only private conformance permit can
run finite no-WASI fixtures; fuel/epoch/cancellation stop hostile guests;
oversized memory/tables and forbidden/mismatched imports fail; generated host
interfaces default to no data and denied publication.

### ECO-04 — selected-input privacy and failure campaign

Use only synthetic selected text containing redaction canaries. Exercise
malformed/oversized/stale responses, replay, capability revocation, wrong route,
model/provider substitution, cancel/kill/disable/uninstall, and offline state
through source tests.

Expected result: selected input is one-shot and generation-bound, never logged or
persisted; model output never executes or presses Enter; stale/cross-session data
is rejected; fallback leaves terminal and CP1–CP3 functional.

### ECO-05 — external ecosystem release evidence

If authorized, follow [ecosystem testing](ECOSYSTEM-PLATFORM-TESTING.md) for real
signed-package sandbox, malicious bundle/component corpus, Windows/Linux/macOS
accessibility/IME/focus/uninstall, publisher/revocation drills, benchmarks, 1,000
lifecycle cycles, and 30-day soak.

Expected result: every manifest binds exact commit/package/dependencies/OS/
hardware/fixtures/resources/cleanup/reviewer. Missing publisher roots, protected
approvals, native packages, privacy/legal review, or elapsed evidence is **not
run**, never a pass.

## Part VI: multi-environment workspaces, provider context, and multi-cloud

### WORK-01 — empty Connection Library and diagnostic state

Start Automexia with the isolated configuration root from Part I. In a separate
shell with the same `AUTOMEXIA_CONFIG_HOME`, run:

```text
automexia workspaces list
automexia workspaces list --json
automexia workspaces doctor
automexia workspaces doctor --json
```

Then open the Hub with `Ctrl+Shift+H` (`Cmd+Shift+H` on macOS), press `W`, and
inspect **Workspaces**.

Expected result:

- list reports no saved workspaces and a nonnegative library revision;
- JSON is valid and contains no credential, host secret, terminal text, private
  path, process ID, or environment dump;
- doctor reports the library/recovery state without starting a process, probing
  a tool, opening a network connection, or creating a terminal session;
- the UI shows one clear empty state, not an error or fabricated sample; and
- the primary activation control remains unavailable because D3 protected
  activation and M5 native lifecycle evidence are pending.

Keep Task Manager, Activity Monitor, or `ps` open if desired. No `ssh`, provider
CLI, or new Automexia child should appear merely because list/doctor/Hub ran.

### WORK-02 — prepare and inspect a valid populated workspace fixture

A workspace binding contains exact current profile and recipe fingerprints. Do
not invent hashes or edit the private Connection Library directly. Use one of
these safe preparations:

1. On a development checkout, run the repository-owned product fixture:

   ```text
   cargo test -p automexia-terminal --locked --test m6_workspace_product
   ```

   This deterministically constructs a `production-ops` workspace bound to the
   `production-api` profile and validates the product/CLI contract in an
   isolated temporary store.
2. For an interactive retained fixture, use a private copy of a Connection
   Library produced by the same reviewed schema owner. Point
   `AUTOMEXIA_CONFIG_HOME` only at that copy. Never transplant real credential
   references or edit fingerprint fields.

With a retained valid fixture, run:

```text
automexia workspaces list --json
automexia workspaces show production-ops --json
```

Expected result: metadata identifies the workspace, revision, environment and
bounded topology. `show` reports one exact current binding and contains no live
session, PTY, process, credential, token, tunnel socket, terminal history, or
resumable action. A missing/stale profile, revision, or fingerprint fails with
an actionable error instead of silently rebinding.

### WORK-03 — preview-first edit, compare-and-swap, and remove

Export an existing valid workspace with `show --json`, preserve all exact
binding fields, and change only a harmless public field such as
`description`. Save it as `workspace-review.json` in the disposable root. First
run preview only:

```text
automexia workspaces put workspace-review.json --json
```

Record the current library revision and workspace entity revision printed by
`list/show`. Then apply exactly the reviewed versions:

```text
automexia workspaces put workspace-review.json --apply --expected-revision <library-revision> --expected-entity-revision <workspace-revision> --json
```

Repeat the same apply using the old revisions. Preview removal, then—only in the
disposable fixture—apply it:

```text
automexia workspaces remove production-ops --entity-revision <current-workspace-revision> --json
automexia workspaces remove production-ops --entity-revision <current-workspace-revision> --apply --expected-revision <current-library-revision> --json
```

Expected result: preview performs no write; the exact CAS apply advances the
revision atomically; stale revisions are rejected; removal preview retains the
record; applied removal deletes only the named current entity. No command
launches a profile or sends terminal input.

Negative cases: add an unknown JSON field, duplicate a pane ID, make a parent
cycle, use a split ratio outside its allowed range, insert bidi/control text,
reference a nonexistent profile, or exceed a documented count/byte limit. Each
must fail closed and leave the last-known-good library unchanged.

### WORK-04 — responsive workspace catalog and restore review

With the valid retained fixture, open the Hub and press `W`. Test at a narrow
window, normal window, 300% display scaling, and an ultrawide window. Navigate
with pointer, Up/Down, Home/End, and Enter; press Escape to return.

Expected result:

- rows remain readable without overlap or clipped primary meaning;
- the selected row exposes workspace/environment/window/pane/connection counts;
- Enter opens an exact restore review for current bindings;
- the review explicitly says automatic reconnect and interrupted-action resume
  are off;
- **Activation gates pending** is disabled; and
- opening or closing the review creates no window, pane, PTY, child, connection,
  tunnel, command, or Enter event.

While Search owns input, type `W`; it must enter the query, not switch sections
or reach the PTY. If the library revision changes, the open review must be
invalidated rather than remaining actionable.

### WORK-05 — restore and typed recipe review

Run against a valid retained fixture:

```text
automexia workspaces restore production-ops --generation 11 --json
automexia workspaces recipe-plan --profile production-api --generation 12 --json
automexia workspaces recipe-plan --profile production-api --generation 13 --no-hooks --json
```

Expected result: restore lists fresh exact targets, requires review, and reports
execution/reconnect/resume as false. The normal recipe plan is ordered and typed;
`--no-hooks` contains only planner-owned steps and is clearly marked as explicit
recovery intent. Neither mode starts OpenSSH, a shell, a provider, a PTY, or any
recipe action. Generation reuse, stale binding, invalid context JSON, an unknown
variable, dependency cycle, unsafe action, or oversized input fails closed.

### WORK-06 — safe broadcast review

Create a regular UTF-8 file containing exactly one harmless line:

```text
uptime --pretty
```

Run:

```text
automexia workspaces broadcast production-ops --command-file <absolute-command-file> --arm-duration-ms 10000 --json
```

Expected result: the review identifies exact targets and production
confirmation, but `execution_enabled` and `enter_requested` remain false. The
command is not present in the process argument list and is redacted from debug
logs. No target receives bytes.

Negative cases: a second line, NUL/control/bidi text, more than 8 KiB, a link or
non-regular file, more than 50 targets, an arm duration above 60 seconds, stale
workspace revision, and a vanished file must all fail without partial delivery.

### WORK-07 — migration, recovery, isolation, and complete M6 source evidence

On a disposable schema-1 fixture, run migration preview, verify no disk change,
then apply with the exact current revision. Corrupt or remove only the copied
primary and review an available previous generation before recovery:

```text
automexia workspaces migrate --json
automexia workspaces migrate --apply --expected-revision <library-revision> --json
automexia workspaces recover <previous-revision> --json
automexia workspaces recover <previous-revision> --apply --json
```

Expected result: migration is explicit and CAS-bound; imported topology loses
unsafe bindings until locally rebound; recovery is allowed only when the primary
is absent/rejected and never overwrites a valid newer primary.

Run the deterministic evidence set:

```text
cargo test -p automexia-connectivity --locked --test connection_planning
cargo test -p automexia-connectivity --locked --test connection_automation_m6
cargo test -p automexia-connectivity --locked --test workspace_automation_m6
cargo test -p automexia-ui-model --locked --test connection_hub
cargo test -p automexia-terminal --locked --test connection_library
cargo test -p automexia-terminal --locked --test m6_workspace_product
cargo test -p automexia-terminal --locked --bin automexia workspace_
python tools/ci/check_connection_hub_f2.py
python tools/ci/test_connection_hub_f2.py
cargo xtask verify architecture
```

Expected result: every process exits 0. These tests prove deterministic local
contracts, not a real remote restore, broadcast, managed recipe, or native
OpenSSH lifecycle.

### CLOUD-01 — six-provider cached review and provider-neutral isolation

Open the Hub and press `P`. Inspect AWS, Azure, Google Cloud, Kubernetes,
OpenShift, and Teleport at narrow, normal, 300%, and ultrawide layouts. Switch
sections with `C`, `W`, and `P`; use pointer and keyboard to open each detail.

Expected result:

- exactly six independently labelled rows appear;
- with no validated in-memory capsule every row says **Choose a public provider
  context**;
- detail shows public state/recovery only and **Activation gates pending**;
- there is no login, refresh, connect, copy, or execute control;
- opening, searching, switching, and closing invoke no provider CLI, credential
  cache, browser/device flow, network, PTY, or global-context mutation; and
- unsupported OpenBao is absent/rejected because it has no accepted active
  implementation.

A source-fixture populated row may display only bounded public identity, scope,
freshness/auth state, risk, and recovery. Replacing it requires a new capsule,
new session, and higher revision; revoke/shutdown invalidates an open review.

### DEV-06 — provider-neutral authentication/capsule contracts

Run:

```text
cargo test -p automexia-connectivity --locked --test provider_auth_m7
cargo test -p automexia-extension-runtime --locked provider_context_rebind_requires_a_fresh_session
cargo test -p automexia-ui-model --locked --test connection_hub
python tools/ci/check_provider_auth_m7.py
python tools/ci/test_provider_auth_m7.py
```

Expected result: all processes exit 0. Fixtures cover available, refreshing,
browser, device, MFA, ready, expired, offline, denied, cancelled, stale and error
states; exact visible one-time capability review; provider/session/revision
isolation; cancellation; redaction; and shutdown. No test is evidence of a real
cloud login or secret-store integration.

### AWS-01 — install and identify the official AWS CLI

Follow the official [AWS CLI installation guide](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html).
Use a disposable, least-privilege account only when an organization has approved
real-provider testing. Verify:

```text
aws --version
aws configure list-profiles
```

Expected result: version and public profile names print in the ordinary shell.
Automexia must not install the CLI, invoke it on startup/typing/Hub open, or copy
its credential/configuration files. Do not run `aws configure` with production
keys for this guide.

### AWS-02 — ordinary-shell interoperability

In an Automexia pane, run a harmless CLI-owned identity query only if authorized:

```text
aws sts get-caller-identity --profile <disposable-profile> --output json
```

Expected result: the system AWS CLI owns authentication/MFA/network and prints
its normal JSON or actionable CLI error. The terminal renders/selects/searches
output normally. This tests terminal interoperability only; it does not activate
Automexia's internal AWS adapter.

### AWS-03 — nonactivation and global-state safety

Record `aws configure list-profiles`, the relevant environment-variable names
(with values redacted), process list, and configuration timestamps. Open/filter
the Hub Providers view repeatedly, switch panes, and type into the shell.

Expected result: no AWS process/browser/network is started, no cache/config file
changes, no environment value is exposed, and no profile/region becomes global.
A source-fixture AWS row may review exact SSO/STS/SSM/EKS intent but cannot run.

### AWS-04 — deterministic AWS adapter evidence

Run:

```text
cargo test -p automexia-devops-aws --locked
cargo clippy -p automexia-devops-aws --all-targets --all-features --locked -- -D warnings
cargo test -p automexia-terminal --locked aws_is_independently_registered_and_disabled
```

Expected result: all processes exit 0 and registration remains disabled.
Malformed/oversized/hostile public input, secret-bearing data, non-ready state,
scope drift, cancellation, stale session and unsupported versions fail closed.
No AWS account or network ran.

### AZ-01 — install and identify Azure CLI

Follow the official [Azure CLI installation guide](https://learn.microsoft.com/en-us/cli/azure/install-azure-cli), then run:

```text
az version
az account list --output table
```

Use `az login` only with a disposable approved tenant/subscription; the CLI owns
browser, Windows broker, device flow, MFA, token cache, and logout.

Expected result: ordinary CLI output renders correctly. Automexia does not
install Azure CLI, read its token cache, or run login automatically.

### AZ-02 — ordinary-shell subscription scoping

If authorized, use explicit scope rather than changing a global default:

```text
az account show --subscription <disposable-subscription-id> --output json
```

Expected result: Azure CLI prints scoped public account data or its own clear
error. Do not use `az account set` as an Automexia test; the adapter contract
forbids global subscription mutation.

### AZ-03 — nonactivation, AKS/Bastion safety, and cache isolation

Record Azure process/config/cache timestamps. Open the Providers catalog and any
source-fixture Azure review.

Expected result: only public subscription/tenant/cloud/state/identity-kind data
may appear. There is no `az` child, browser, token read, network call, `az account
set`, Bastion connection, or AKS kubeconfig path. AKS can reference only a future
M11-owned opaque private transient handle.

### AZ-04 — deterministic Azure adapter evidence

Run:

```text
cargo test -p automexia-devops-azure --locked
cargo clippy -p automexia-devops-azure --all-targets --all-features --locked -- -D warnings
cargo test -p automexia-terminal --locked azure_is_independently_registered_and_disabled
```

Expected result: all processes exit 0; exact subscription-bound review,
AAD-only Bastion, opaque AKS relation, limits, hostile input, redaction,
version/state/cancellation/drift and disabled registration pass without Azure
network or credentials.

### GCP-01 — install and identify Google Cloud CLI

Follow the official [Google Cloud CLI installation guide](https://cloud.google.com/sdk/docs/install), initialize only a disposable approved account/configuration, and run:

```text
gcloud version
gcloud config configurations list
```

Expected result: the official CLI prints its version and named public
configurations in the ordinary terminal. Automexia does not install gcloud,
activate a configuration, read its credential database, or invoke it on startup,
typing, or Hub navigation.

### GCP-02 — explicit named-configuration interoperability

If authorized, create/select a disposable named configuration using the official
CLI workflow. Query it without changing the active global configuration:

```text
gcloud --configuration <disposable-configuration> config list --format=json
gcloud --configuration <disposable-configuration> auth list --filter=status:ACTIVE --format=json
```

Expected result: gcloud owns authentication/network and prints its normal output
or actionable error. Every test command names the configuration. The terminal
renders the result, but Automexia receives no credential material and does not
activate the configuration.

### GCP-03 — nonactivation, IAP/GKE, and private-output boundaries

Record gcloud process/cache/configuration timestamps, then open/search/review the
Providers section repeatedly.

Expected result: only bounded public account/project/region/zone hints may be
shown by a source fixture. There is no gcloud/browser/federation/2FA/network,
configuration activation, OS Login key custody, IAP SSH process, or GKE cluster
call. Any future GKE kubeconfig must be an opaque M11-owned private transient,
not a user path or public catalog field.

### GCP-04 — deterministic Google Cloud adapter evidence

Run:

```text
cargo test -p automexia-devops-gcp --locked
cargo clippy -p automexia-devops-gcp --all-targets --all-features --locked -- -D warnings
cargo test -p automexia-terminal gcp_is_independently_registered_and_disabled --lib --locked
cargo bench -p automexia-devops-gcp --bench provider --locked
```

Expected result: tests/clippy/benchmark exit 0; exact named configuration,
public project/IAM, Workforce/Workload opaque references, IAP, M11-only GKE
intent, limits, hostile/secret input, versions, state, drift and redaction pass.
This is synthetic local evidence, not a real Google Cloud run.

### KUBE-01 — create a disposable local Kubernetes cluster

Complete the Docker, kubectl, and kind setup in Part I. Create and identify the
cluster:

```text
kind create cluster --name automexia-manual --wait 120s
kubectl config get-contexts
kubectl --context kind-automexia-manual cluster-info
kubectl --context kind-automexia-manual get nodes -o wide
```

Expected result: kind reports a ready control plane; the exact context is
`kind-automexia-manual`; kubectl reports at least one Ready node. If Docker is
not running, ports are unavailable, or the cluster is not ready within the
bound, stop and record setup failure rather than testing a production cluster.

### KUBE-02 — terminal interoperability with explicit context and namespace

Create an isolated namespace and a small disposable workload:

```text
kubectl --context kind-automexia-manual create namespace automexia-manual
kubectl --context kind-automexia-manual --namespace automexia-manual create deployment hello --image=nginx:alpine
kubectl --context kind-automexia-manual --namespace automexia-manual rollout status deployment/hello --timeout=120s
kubectl --context kind-automexia-manual --namespace automexia-manual get pods -o wide
```

Expected result: output is readable, searchable, selectable, separated from the
next prompt, and does not disturb input in another pane. Every command has an
explicit context and namespace. A registry/network failure is an environment
failure; capture the exact CLI message, do not weaken TLS or use production.

Open another pane and run a long `kubectl ... get pods --watch`; verify Ctrl+C
stops only that foreground command and both panes remain usable.

### KUBE-03 — cached Kubernetes/OpenShift Hub nonactivation

Open the Providers section with the kind cluster running. Also open it after
stopping Docker.

Expected result: Automexia does not discover the cluster, run kubectl/oc, read
`KUBECONFIG`, execute an auth plugin, contact the API server, or change current
context. Without an explicitly validated in-memory capsule both rows retain the
honest choose-context empty state. A synthetic cached row may show only public
cluster/context/namespace/provider/freshness/provenance/risk and remains disabled.

### KUBE-04 — private transient and cross-provider isolation

Run the app-owned lifecycle evidence:

```text
cargo test -p automexia-terminal --lib --locked provider_transients
cargo test -p automexia-terminal --test m8_m12_provider_product --locked
```

Expected result: tests exit 0. AWS EKS, Azure AKS and Google GKE private outputs
are validated before an opaque handle is published; handles are session,
provider, relation, revision and generation bound; expiry/revoke/shutdown/drop
removes files. Public/debug projections contain no path, token, certificate,
exec argument or secret canary.

### KUBE-05 — hostile kubeconfig and plugin-denial scenarios

Never corrupt the user's real kubeconfig. In a temporary directory, create
copies representing: invalid UTF-8; YAML duplicate/merge keys; controls/bidi;
more than the allowed byte/event/depth/item budgets; dangling references;
multiple files/contexts; HTTP non-loopback server; proxy; inline token/key/cert;
external credential path; `auth-provider`; and `exec` plugin.

Exercise these through the deterministic parser/runtime tests, not by pointing
kubectl at them:

```text
cargo test -p automexia-devops-kubernetes --locked
```

Expected result: forbidden/ambiguous inputs fail closed; no plugin or network
runs. A reviewed exec plugin can exist only as exact digest/argv/allowed
environment/interactivity/session/deadline/output/tree-cancellation metadata and
still has execution disabled at the current product boundary.

### KUBE-06 — namespace, lifecycle, and cleanup

Remove only the disposable resources, then delete the kind cluster:

```text
kubectl --context kind-automexia-manual delete namespace automexia-manual --wait=true --timeout=120s
kind delete cluster --name automexia-manual
kind get clusters
```

Expected result: the namespace and cluster disappear; no `automexia-manual`
entry remains in `kind get clusters`; no watch/kubectl child remains. Inspect
Docker containers and temporary kubeconfig copies and remove only verified test
artifacts. Do not edit unrelated kubeconfig entries.

### KUBE-07 — complete M11 deterministic evidence

Run:

```text
cargo test -p automexia-devops-kubernetes --locked
cargo test -p automexia-devops-openshift --locked
cargo clippy -p automexia-devops-kubernetes -p automexia-devops-openshift --all-targets --all-features --locked -- -D warnings
cargo test -p automexia-terminal kubernetes_is_independently_registered_and_disabled --lib --locked
cargo test -p automexia-terminal openshift_is_independently_registered_and_disabled --lib --locked
cargo bench -p automexia-devops-kubernetes --bench kubeconfig --locked -- --sample-size 50
cargo deny --locked --color never check --hide-inclusion-graph
```

Expected result: every process exits 0, adapters remain independently disabled,
and the maximum-valid parser workload stays bounded. Record same-host benchmark
results and outliers; do not compare unlike fixture sizes as a regression.

### KUBE-08 — Docker and Kubernetes prompt-context behavior

Inside the kind test environment, start a fresh integrated shell and `cd` into a
synthetic project directory. Run the explicit kubectl commands from KUBE-02 and
inspect prompt/context surfaces.

Expected result: any Docker/Kubernetes context shown is public, bounded and
clearly scoped; the prompt remains readable with Git/path/status/duration; no
secret, token, certificate, full kubeconfig path or provider cache is painted.
With integration disabled or unavailable, the ordinary shell remains functional
and no context is fabricated.

### KUBE-09 — optional OpenShift/CRC interoperability

For a real disposable local OpenShift test, install `oc` and Red Hat CRC using
the official [OpenShift documentation](https://docs.redhat.com/en/documentation/openshift_container_platform). Meet CRC CPU/RAM/virtualization requirements,
run `crc setup`, then `crc start`, and use only the generated local developer
credentials. Verify:

```text
oc version
oc whoami
oc config current-context
oc get projects
```

Expected result: `oc` owns login, certificates, network and context. Ordinary
output renders correctly. Automexia's OpenShift adapter remains nonactivated:
Hub navigation does not run `oc login`, `oc project`, `oc rsh`, read a token, or
contact CRC. Stop CRC and clean its disposable state per official instructions.
If hardware/licensing/account prerequisites are unavailable, mark this scenario
**not run**, not passed.

### TP-01 — install and identify the Teleport client

Install `tsh` using the official [Teleport client guide](https://goteleport.com/docs/connect-your-client/teleport-clients/tsh/), then run:

```text
tsh version
tsh status
```

Expected result: a supported client prints its version; status shows the
CLI-owned current public profile or a normal not-logged-in result. Automexia
must not install tsh, scan `~/.tsh`, copy certificates, add keys to an agent, or
contact a proxy.

### TP-02 — disposable ordinary-shell login and SSH interoperability

Only with an approved disposable Teleport cluster/account, use the exact proxy
and user flow provided by its administrator:

```text
tsh login --proxy=<test-proxy> --user=<test-user>
tsh status
tsh ssh <test-node>
tsh logout
```

Expected result: tsh owns browser/MFA/hardware-key/network/certificates and the
actual SSH process. Automexia renders the interactive program normally. Logout
removes/revokes CLI-owned session state according to Teleport policy. Do not
publish proxy, user, node or certificate details in evidence.

### TP-03 — cached review and agent/cache nonactivation

Before and after opening/filtering a synthetic Teleport provider row, inspect
processes, SSH-agent keys, and `~/.tsh` timestamps without recording secrets.

Expected result: no tsh/proxy/browser/network/access request/SSH/PTY is started;
no cache/certificate is read or changed; no key is added to the agent. A cached
row may retain bounded public proxy/cluster/user and freshness only. Expiry,
revocation, capsule/session/revision/proxy drift invalidates it.

### TP-04 — deterministic Teleport adapter evidence

Run:

```text
cargo test -p automexia-devops-teleport --locked
cargo clippy -p automexia-devops-teleport --all-targets --all-features --locked -- -D warnings
cargo test -p automexia-terminal --locked automexia::builtins::tests::teleport_is_independently_registered_and_disabled
cargo bench -p automexia-devops-teleport --bench status --locked -- --sample-size 100 --warm-up-time 5 --measurement-time 10
```

Expected result: all processes exit 0; status limits, hostile/sensitive input,
expiry, exact reviewed login/logout/SSH argv, agent/environment restrictions,
states, drift, cancellation and redaction pass with registration disabled. No
Teleport service ran.

### CP4-01 — provider-aware Quick Actions empty-state isolation

With no validated provider capsule, open Quick Actions and search provider terms
such as `aws`, `azure`, `gcp`, `kubectl`, `oc`, and `tsh`.

Expected result: no provider-context action is fabricated from terminal text,
environment, global CLI state, credential files, or live provider calls. Static
reviewed CP3 pack actions remain separately labelled and disabled-by-default as
configured; typing causes no provider process/network/authentication work.

### CP4-02 — cached provider publication and route ownership

Run the product publication fixture:

```text
cargo test -p automexia-terminal --test cp4_provider_product_publication --locked
cargo test -p automexia-ui-model --locked provider_context
```

Expected result: cached public provider context projects bounded rows for the
seven supported projection families, with exact route/session/provider/revision
ownership. Repeating the same publication is idempotent; a new revision replaces
rather than duplicates; revoke, route close, workspace change and shutdown
clear candidates. No command executes.

### CP4-03 — review, final revalidation, production confirmation, and insertion

Use deterministic source fixtures to exercise ready, stale, expired, offline,
revoked, wrong-route, wrong-session, replaced-revision, broker-required and
production-risk candidates. Search, select, review and request insertion.

Expected result: review exposes provider/account/project/subscription or
cluster/context/namespace plus region/zone, freshness, route and risk without
secrets. Final revalidation rejects every stale/mismatched candidate. Production
requires explicit confirmation. An accepted nonexecuting candidate may insert
reviewed text into the owning shell line but never sends Enter, runs a provider,
changes global context, crosses panes, or falls back to ambient state.

### CP4-04 — complete provider-aware Quick Action evidence

Run:

```text
cargo test -p automexia-command-productivity --test provider_quick_actions_cp4 --locked
cargo test -p automexia-command-productivity --test quick_action_activation --locked
cargo test -p automexia-terminal --test cp4_provider_product_publication --locked
cargo test -p automexia-terminal --lib --locked provider
cargo test -p automexia-terminal --bin automexia --locked provider
cargo test -p automexia-ui-model --locked provider_context
cargo check --manifest-path fuzz/Cargo.toml --bin provider_quick_actions
python tools/ci/check_provider_quick_actions_cp4.py
python tools/ci/test_provider_quick_actions_cp4.py
cargo xtask verify architecture
```

Expected result: every process exits 0; cached-only publication, search, final
revalidation, production confirmation, isolation, revocation, fuzz compilation,
no-keystroke provider work and nonactivation pass. Real refresh, login and exact
provider execution remain external/blocked rather than silently passing.

## Part VII: contributor, extension, package, and release assurance

### DEV-03 — extension contract/runtime fail-closed behavior

Run:

```text
cargo test -p automexia-extension-api --locked
cargo test -p automexia-extension-runtime --locked
cargo clippy -p automexia-extension-api -p automexia-extension-runtime --all-targets --all-features --locked -- -D warnings
cargo xtask verify architecture
```

Expected result: every process exits 0. Version/capability negotiation, bounded
records, exact application-runner requests, stale generations, cancellation,
redaction, and provider/session rebind isolation pass. Invalid versions,
unknown capabilities, oversized/hostile data and unavailable runner fail closed.
Extensions never receive renderer, PTY, arbitrary filesystem/network,
credential, ambient environment, or another extension's state.

Manual negative check: open every current extension-related review surface and
press keys/type text. It must consume its own input, never leak characters to a
pane, never run an extension/provider, and restore focus on close.

### DEV-04 — renderer-neutral Connection Hub model and composition

Run:

```text
cargo test -p automexia-connectivity --locked --test connection_records
cargo test -p automexia-connectivity --locked --test connection_planning
cargo test -p automexia-ui-model --locked --test connection_hub
python tools/ci/check_connection_hub_f2.py
python tools/ci/test_connection_hub_f2.py
```

Expected result: all processes exit 0. Capability-free records/reducers,
dry-run plans, exact modal routes, search/filter/group/virtualization,
responsive/accessibility projections, generation replacement, cancellation and
last-known-good behavior pass. No renderer/GPU/PTY/process/network authority
enters the model.

### DEV-05 — private Connection Library persistence

Run:

```text
cargo test -p automexia-terminal --locked --test connection_library
cargo test -p automexia-terminal --locked --test m6_workspace_product
```

Expected result: transactional 16 MiB bounded documents, compare-and-swap,
atomic publication, recovery, read-only/disk-full/link/invalid UTF-8/oversize/
duplicate/hostile cases, redacted transfer and fresh import IDs pass. A rejected
write preserves the primary and last-known-good snapshot. Debug/errors expose no
private target, opaque credential reference, terminal text or local path.

### QA-01 — fast repository validation before the full gate

From the exact repository root run:

```text
git status --short
git diff --check
python tools/ci/validate_repository.py
cargo xtask verify architecture
cargo xtask verify identity
cargo xtask verify keybindings
```

Expected result: diff check and every validator exit 0. A release candidate has
no unexplained worktree changes. The validators must reject—not rewrite—identity,
architecture, generated-keybinding, policy, documentation, workflow, limit or
feature-matrix drift.

### QA-02 — feature matrix, roadmap, policy, and mutation owners

Run:

```text
python tools/ci/check_phase_implementation_audit.py
python tools/ci/check_feature_test_reinforcement.py
python tools/ci/test_feature_test_reinforcement.py
python tools/ci/check_repository_protection.py
python tools/ci/test_repository_protection.py
```

Also run the named checker/test pair referenced by the roadmap phase under test
(for example CP4, M7, ecosystem, Ghostty, S1 or S2).

Expected result: all processes exit 0. Every matrix feature has an implementation
owner, tests, limits and truthful status; source/policy mutations are detected.
A checker passing proves the checked contract, not external service/hardware or
visual correctness.

### QA-03 — focused test-first reproduction

Before a change, run the smallest existing test that owns the behavior. For a
bug, add or identify a deterministic regression that fails for the reported
reason. Record the command, first failure, environment and artifact. After the
fix, rerun that test, its crate, affected policy/mutation tests, and every test
whose fixture/golden changed.

Expected result: the original regression fails before the fix and passes after
it; negative and boundary cases remain. A test that never exercises the real
ownership path, assertion, renderer-neutral state or native adapter is not valid
proof. Never hide a flaky first failure with retries.

### QA-04 — full Rust and documentation quality gate

Run exactly:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked --profile ci
cargo test --workspace --doc --locked
python3 tools/ci/qa.py --full
cargo ready
```

Expected result: every command exits 0. Formatting changes nothing; Clippy emits
no warning; Nextest reports no unexpected failure; doctests pass/only documented
ignores remain; full QA produces a bounded report; `cargo ready` completes
without launching the application. Record skipped/ignored tests and unavailable
native/external gates separately. A disk-full/linker/infrastructure failure is
still a failed attempt until diagnosed and the exact command passes.

### QA-05 — performance, fuzz, concurrency, and lifecycle selection

Select the affected owners from [Testing](TESTING.md) and run their exact
Criterion, fuzz compile/campaign, Loom/model, resize/output-stress and repeated
lifecycle tests. Representative commands are:

```text
cargo bench -p rio-vt --bench vt_input snapshot_visible_noop -- --noplot
cargo bench -p rio-vt --bench vt_input history_navigation_repaint_deep_scrollback -- --noplot
cargo bench -p teletypewriter --bench pty_io --locked -- --noplot
cargo check --manifest-path fuzz/Cargo.toml --bins --locked --offline
```

Expected result: benchmarks finish on a named stable host, fixtures are
identical to the baseline, and measurements/outliers are retained; fuzz targets
compile and any campaign has a fixed seed/time/artifact directory; concurrency
and lifecycle tests reach bounded readiness and zero owned cleanup. A local
microbenchmark is not the S2 30-day release baseline.

### QA-06 — failure triage and evidence integrity

For every failure:

1. preserve the first error and exact command;
2. classify product regression, test defect, unsupported host, missing external
   prerequisite, or infrastructure/resource failure;
3. inspect bounded redacted logs and owner state;
4. fix the cause without lowering a limit, disabling a test, widening authority,
   retrying silently, or altering expected output merely to pass;
5. rerun the focused owner and every affected later gate; and
6. mark unavailable controlled evidence **not run**.

Expected result: the final ledger includes both the first failure and verified
resolution. It contains no token, private host, home path, terminal history,
credential/cache content or unreviewed screenshot.

### PKG-01 — local package metadata and offline-ready preflight

From a clean exact commit run:

```text
cargo xtask package --check
cargo deny --locked --color never check
```

Expected result: metadata, icons/resources, app/desktop identifiers, URL handler,
terminfo, version, licenses, locked dependencies, advisories, bans and sources
pass without publishing. The check must reject placeholder/missing assets,
identity/version drift, unexpected resources, bad permissions or unreviewed
source dependencies.

### PKG-02 — build and inspect an unsigned development package

Use the platform packaging command documented by `cargo xtask package --help`
and the relevant CI workflow. Build only on a native host for the target format.
Inspect archive/package contents before installation:

- Windows: x86_64 and ARM64 ZIP/MSI contain the exact bounded executable and
  integration resource tree; ARM64 MSI uses the pinned repository WiX tool.
- macOS: a universal app/DMG contains correct bundle identifier, icons,
  architectures and resources; a release claim additionally needs Developer ID
  signing and notarization.
- Linux: x86_64/ARM64 DEB, RPM and tar.gz contain correct desktop, AppStream,
  icon, URL and terminfo files for X11/Wayland.

Expected result: version/architecture/content match the exact commit and no
source tree, credential, cache, debug database, private test artifact or
unbounded extra file is packaged. An unsigned development package must be
labelled unsigned and cannot satisfy stable-release trust.

### PKG-03 — signing, checksums, SBOM and provenance

This is a controlled release scenario. Follow [Releasing](../RELEASING.md) and
[Release trust](RELEASE-TRUST.md). Use protected environments and repository
secrets; never place a certificate/PFX password/token in a local command log.
For an exact annotated tag, verify:

- Windows Authenticode publisher/timestamp for each MSI, executable and required
  PowerShell/format resource;
- macOS Developer ID signature, hardened runtime, notarization and Gatekeeper;
- SHA-256 checksums for every final artifact;
- semantic CycloneDX/SPDX SBOMs matching package version/components/lockfile;
- GitHub provenance/SBOM attestations bound to the tag and final signed files;
- Linux reproducibility job's two cold same-path x64 outputs are byte-identical;
  and
- publication uses the exact allowlist and creates one new immutable draft
  without clobbering an existing release.

Expected result: every independent verifier succeeds. Missing credentials,
unknown/partial signing backend, digest/version drift, empty SBOM, unsigned
resource, unprotected runner, existing release or unattested artifact blocks
publication. Mark this **not run** without the controlled infrastructure.

### PKG-04 — clean install, upgrade, rollback, and uninstall

On disposable native Windows x64/ARM64, macOS Intel/Apple Silicon, Linux x64/
ARM64, and Windows+WSL hosts as available:

1. snapshot user config and installed files;
2. clean-install the final signed package;
3. verify `automexia --version`, application launch, icon, URL handler, terminfo,
   shell integration and one PTY session;
4. upgrade from the supported previous release while preserving config;
5. test a rejected/corrupt update and documented rollback;
6. uninstall using the platform's normal mechanism; and
7. verify product-owned binaries/resources/registrations are gone while user
   data is preserved or removed only by explicit documented choice.

Expected result: no elevation surprise, orphan child, broken handler, identity
collision, unrelated profile mutation or credential removal. Actual results are
per exact native package/OS/architecture; untested combinations remain **not
run**.

### ASSURE-01 — S1 source policy and private-manifest validation

Run the deterministic policy/mutation layer:

```text
python tools/ci/s1_assurance.py check-policy
python tools/ci/test_s1_assurance.py
```

Expected result: both exit 0 and reject missing suites, dirty/wrong commit,
stale/future/synthetic evidence, linked/changed files, unknown fields, leaked
paths/secrets, weak cleanup and absent independent review.

Only on the protected `automexia-assurance` runner, validate the private
current-commit manifest:

```text
python tools/ci/s1_assurance.py validate --manifest <private-manifest.json> --expected-commit <40-character-commit> --require-complete --output <public-summary.json>
```

Expected result: all 24 required native, resource, visual and accessibility
suites are complete, independently reviewed and cleanup-clean; the bounded
public summary contains no private artifact path or terminal content. A local or
fabricated manifest must fail and must never be substituted for controlled S1.

### ASSURE-02 — native visual and single-pixel-sensitive review

Prepare four policy environments from the S1 audit, including the required
renderer/OS combinations. For each required case exercise tiny through
8K-equivalent dimensions, 100–300% scale, normal/high-contrast/custom themes,
light/dark backgrounds, Unicode/emoji/combining/bidi-safe fixtures, long paths,
multiple tabs/panes, overlays, search, Connection Hub, Quick Actions, completion,
prompt/result separation, alternate screen and renderer fallback.

For every case:

1. wait for an explicit renderer-neutral ready signal—never an arbitrary sleep;
2. capture the native frame with the exact commit, OS, GPU/driver, scale, theme,
   dimensions and scenario ID;
3. compare the renderer-neutral snapshot/golden first;
4. run the repository visual comparator against same-size expected/actual PNGs:

   ```text
   cargo xtask visual-diff --expected <expected.png> --actual <actual.png> --config <reviewed-config.json> --diff <heatmap.png> --report <report.json>
   ```

5. inspect the full frame and heatmap at original resolution;
6. repeat focus, hover, animation/reduced-motion and open/close transitions; and
7. retain only redacted approved artifacts.

Expected result: exact regions configured with zero tolerance detect a one-pixel
change; anti-aliasing/dynamic regions use only reviewed masks and thresholds.
No changed pixel may be dismissed solely because aggregate ratio passes. Text,
icons, focus, borders, prompt/result boundaries, cursor and close controls are
aligned, unclipped, nonoverlapping and consistent. A mask/threshold/dimension
mismatch fails. The complete matrix is 1,600 cases; an incomplete local subset
is recorded as such, never as full S1.

### ASSURE-03 — keyboard-only and native assistive-technology review

Run once with Windows Narrator and once with NVDA where available, once with
macOS VoiceOver, and once with Linux Orca on X11 and Wayland where required.
Use the official setup links in Part I. Test startup, tab/pane creation and
close, split/resize, local/global search and live scope switching, Quick Actions,
Connection Hub sections/details, workspace/provider empty/error/disabled states,
completion, configuration errors and focus restoration.

For each surface verify:

- a stable role, concise accessible name, current value/state, position/count
  and available action;
- logical focus order; visible focus; pointer and keyboard parity;
- radio-like `PANE`/`ALL PANES` scope announcement and immediate retained-query
  switching;
- disabled actions announced as disabled with the reason, not silently missing;
- icons/color are backed by text/state; contrast and high-contrast remain clear;
- Escape returns focus to the exact prior owner; modal keys never reach PTY;
- updates such as match counts/status are polite, not repeated/noisy; and
- reduced motion removes decorative animation while preserving boundaries.

Expected result: the workflow is understandable without vision or pointer and
has no focus trap, duplicate/blank name, hidden actionable control, terminal
input leak or inaccessible error. Record AT/version/OS/transcript and every
manual issue. Renderer-neutral semantics passing is necessary but does not
replace these native sessions.

### ASSURE-04 — resource, stress, cleanup, and native platform behavior

Use a named stable machine on AC power with fixed performance mode and no
unrelated workload. Record baseline process/thread/handle/private-byte/working-
set/GPU/socket/file counts after a five-minute idle. Exercise:

1. repeated application/window/tab/pane create/close cycles;
2. six or more PTY create/extreme-resize/output/exit/drop cycles;
3. 1 MiB and sustained output, deep scrollback/search/selection/reflow;
4. rapid input, resize storms, hover-scroll ownership, clipboard/IME;
5. repeated Hub/Quick Action/search/completion open/switch/close;
6. library/provider snapshot replacement, cancellation and shutdown;
7. renderer device loss/fallback where safely supported; and
8. install/launch/exit/uninstall of the final package.

On Windows controlled hardware also run Application Verifier and WPR using the
opt-in instructions in [Testing](TESTING.md); on Linux inspect `/proc`, file
descriptors and X11/Wayland behavior; on macOS use native Instruments/Activity
Monitor and both Intel/Apple Silicon where required. Never leave verifier state
or traces enabled after the run.

Expected result: no crash, hang, lost output, cross-route input, post-exit
process tree, listener/socket, growing unbounded queue/cache/history, leaked
handle/thread/file/temp directory, or failure to return near the reviewed idle
baseline. Store raw traces privately and publish only bounded redacted metrics.
The exact same-host Criterion results and native deltas must be compared before
making a performance claim.

### ASSURE-05 — S2 30-day performance/release ratchet

Run the local policy tests first:

```text
python tools/ci/performance_assurance.py check-policy
python tools/ci/test_performance_assurance.py
```

Expected result: both exit 0; linked/changed/dirty/wrong-commit/stale/future,
weak-sample, unknown-metric and unreviewed evidence is rejected.

Controlled completion requires 30–90 consecutive comparable daily runs on the
same named runner/hardware/power/configuration and exact reviewed workload. Each
record binds commit, runner/operator, tool versions, fixture digest, latency,
throughput, allocation/memory/resource cleanup and review URL. Independently
review and activate the baseline using the protected S2 workflow, then evaluate
a tagged clean commit with `--require-active --expected-commit`.

Expected result: the collecting baseline never authorizes a stable tag. An
active baseline passes only when policy confidence/sample rules pass and latency
is no more than 5% worse and memory no more than 10% worse, unless an exact
commit/metric/baseline/maximum waiver has a bounded reason, independent HTTPS
approval and unexpired at-most-30-day lifetime. Missing days, runner drift,
changed fixture, failed cleanup or missing independent review restarts/blocks
the claim. Follow the [S2 completion audit](research/S2-RELEASE-RATCHET-COMPLETION-AUDIT.md)
for recovery, activation and rollback.

## Part VIII: native C and WebAssembly embedding compatibility

These inherited private crates are source compatibility surfaces. Automexia v0.4
does not publish a public C, npm, web, or extension SDK.

### EMBED-01 — native Rust/C API build and smoke

Run:

```text
cargo test -p librio --locked
cargo build -p librio --release --locked
```

On Linux/macOS or Git Bash with a C compiler, also run:

```text
bash tools/ci/test_librio_c_api.sh
```

Inspect `librio/include`, compile the curated header/modulemap against the
release static library, create an engine/surface/render state, feed `ls\r`, call
update, and destroy resources in the documented order.

Expected result: Rust tests and C smoke exit 0; ABI symbols/header agree; the
host-pulled render state updates dirty rows; callbacks wake the owning surface;
creation and repeated destruction leak no owned process/thread/allocation. The
crate draws no window itself and is not included as a public desktop SDK.

### EMBED-02 — WebAssembly build and host-owned transport

Install exact tools in a disposable development environment:

```text
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.106 --locked
cargo test -p librio-wasm --locked
cargo build -p librio-wasm --release --target wasm32-unknown-unknown --locked
wasm-bindgen --target web --out-dir <temporary-pkg-directory> target/wasm32-unknown-unknown/release/librio_wasm.wasm
```

Load the generated module in a minimal local web test host. Feed synthetic child
output, render returned state, send key/mouse input, and capture bytes emitted by
the `output` callback. Do not connect it to a real remote service for this smoke.

Expected result: module instantiates, `feed` updates terminal state, callbacks
return terminal response/input bytes, and cleanup removes the instance/listeners.
There is no browser PTY, ambient filesystem/network/credential authority or
published npm package; the host owns transport.

### EMBED-03 — hostile input, mouse ownership, and lifecycle isolation

For both native and Wasm owners test fragmented/oversized VT input, Unicode and
controls, repeated create/drop, callback after close, multiple independent
surfaces, keyboard/wheel/button/motion events, application mouse mode and Shift
selection bypass. Run the focused mouse boundary too:

```text
cargo test -p librio --lib --locked mouse
cargo test -p librio-wasm --locked
```

Expected result: bounds/errors are deterministic, surfaces do not share state,
callbacks cannot use a destroyed owner, and mouse input reports whether the
child owns it. Shift bypass preserves host selection. No panic crosses the C/
Wasm boundary and repeated lifecycle returns resources to baseline.

## Part IX: universal manual regression matrix

Apply this compact matrix to every **Available now** UI feature and every
**implemented locally** review surface after its feature-specific scenario:

| Dimension | Required cases | Pass condition |
|---|---|---|
| Viewport | smallest supported, normal, split panes, ultrawide, 4K/8K-equivalent | No clipping, overlap, hidden primary action, unsafe z-order or unusable target. |
| Scale | 100%, 125/150%, 200%, 300% | Stable layout/hit targets; crisp or correctly anti-aliased rendering; no one-pixel seam accepted without review. |
| Input | keyboard, pointer, hover-scroll, selection, Ctrl+V, IME, focus loss/restore | Exact owning pane/surface receives input once; modal input never reaches PTY. |
| Shell | PowerShell, CMD, WSL/Bash; Bash/Zsh/Fish on native Unix where available | Ordinary input/output survives unsupported integration and optional features. |
| Content | ASCII, long text, emoji, combining marks, CJK, RTL-safe fixture, controls | Correct grapheme/cell behavior; hostile controls never become UI authority or secret leakage. |
| State | first run, populated, empty, loading, stale, offline, cancelled, corrupt, read-only, disk full | Actionable redacted state; last-known-good where specified; no silent unsafe fallback. |
| Concurrency | two+ panes/tabs/windows, rapid switching, stale generations, shutdown | Route/session/generation isolation and complete owned cleanup. |
| Theme/accessibility | dark, light/custom, high contrast, reduced motion, native screen reader | Meaning not color-only; contrast/focus/semantics preserved. |
| Performance | cold/warm startup, output/resize/search storms, long session | No hot-path provider/filesystem/network work or unbounded resource growth. |
| Disable/recovery | feature disabled, uninstall, migration, rollback, recovery | Core terminal remains usable and user/unrelated external state is preserved. |

## Final acceptance and cleanup checklist

A feature is manually accepted only when all applicable boxes can be backed by
an evidence record:

- [ ] Exact commit/package digest, native host, architecture, renderer, scale,
      shell and external tool versions recorded.
- [ ] Feature-specific happy path, empty/error/cancel/stale/hostile/boundary,
      keyboard/pointer/accessibility and cleanup scenarios passed.
- [ ] Expected UI/output was observed; screenshots/goldens were inspected at
      original resolution for visible changes.
- [ ] No PTY/input leak, implicit Enter, unintended process/network/credential
      access, global provider mutation or cross-pane/session/generation state.
- [ ] Relevant focused tests, checker/mutation owner, benchmark/fuzz/concurrency
      owner and full contributor gate exited 0.
- [ ] First failures and their causes remain recorded; reruns are not substituted
      for the original evidence.
- [ ] Controlled native, signed-package, assistive-technology, real-provider and
      30-day evidence is exact, current and independently reviewed—or clearly
      marked **not run**.
- [ ] Test accounts, clusters, namespaces, containers, child processes, sockets,
      verifier settings, temporary files and isolated configuration were removed.
- [ ] User profiles, credentials, provider caches, kubeconfig, SSH configuration
      and unrelated worktree files remain unchanged.

Final cleanup for this guide:

1. close Automexia and verify all product-owned children exited;
2. delete only the verified `kind` cluster/CRC/test containers and disposable
   cloud/Teleport sessions created for this run;
3. remove only the isolated `AUTOMEXIA_CONFIG_HOME` and temporary synthetic
   fixtures after archiving approved redacted evidence;
4. clear the test-specific environment variable in the current shell;
5. verify no listener, background watch, provider/ssh child, WPR/AppVerifier
   state, mounted image, temporary private output or stale lock remains; and
6. run `git status --short` and compare with the starting record.

Do not call the feature, phase, platform or release fully validated when any
applicable scenario failed or was unavailable. Report the precise passing local
evidence and list every remaining external gate separately.

## Related authoritative guides

- [Product feature catalog](FEATURES.md)
- [Testing and CI](TESTING.md)
- [Feature-test reinforcement matrix](FEATURE-TEST-REINFORCEMENT.md)
- [Connection Hub and system SSH](user-guide/connection-hub-and-ssh.md)
- [Multi-cloud provider adapter testing](MULTI-CLOUD-PROVIDERS-TESTING.md)
- [Command productivity](COMMAND-PRODUCTIVITY.md)
- [Ghostty compatibility implementation](GHOSTTY-COMPATIBILITY-IMPLEMENTATION.md)
- [Ecosystem platform testing](ECOSYSTEM-PLATFORM-TESTING.md)
- [Packaging guide](../packaging/README.md), [release trust](RELEASE-TRUST.md), and
  [release procedure](../RELEASING.md)
- [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md)

If a specialized guide conflicts with this workbook on an exact limit, command,
platform, status or release prerequisite, stop and reconcile the source owner,
machine-enforced contract and documentation before testing. Never choose the
less restrictive interpretation merely to obtain a pass.
