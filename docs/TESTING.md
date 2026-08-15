# Testing and verification

## Complete local gate

Run the complete contributor gate and produce a smoke-tested debug executable
with one command:

```text
cargo ready
```

Use `cargo dev` to run that same gate and launch Automexia when it passes. Use
`cargo automexia` for a fast incremental build, version smoke, and launch when
the full gate has already passed. These commands are Cargo aliases backed by
`tools/xtask`, so they are identical on Windows, macOS, and Linux.

`cargo dev` opens no application window until its complete verification phase
passes. Its workspace-test phase uses a fresh isolated target and can spend
several minutes compiling Rust, WGPU, and native shader dependencies on a cold
run. Compiler progress remains visible; this is active verification, not a
launch hang. Use `cargo automexia` for ordinary day-to-day launches.

The launcher returns after a successful spawn, leaving Cargo available for the
next command while Automexia continues running. Both launch paths run the
cross-platform shell-provisioning phase immediately before the spawn:

- Windows installs or repairs PowerShell, CMD, and WSL Bash/Zsh support;
- macOS/Linux installs or repairs Bash/Zsh support and user-local terminfo;
- an unchanged installation exits through a fingerprinted fast path without
  rewriting profiles or starting WSL;
- a provisioning failure aborts launch, so a successful command never opens an
  unintentionally unintegrated terminal.

`cargo ready`, `cargo check`, and `cargo xtask ci` remain non-mutating. The
platform installers support direct `--force`/`-Force` invocation for focused
maintainer diagnosis, but normal development never requires it.

The complete gate also parses every repository PowerShell source, exercises the
Windows install/repair fast path in an isolated LocalAppData fixture, and executes
the PowerShell formatter/prompt contract on Windows. On Unix it syntax-checks
Bash and Zsh, runs ShellCheck, exercises automatic installation twice in an
isolated home, repairs a deliberately changed installed file, and executes both
shell-integration suites.
`cargo xtask ci` runs the same non-launching gate; neither command leaves its
isolated exhaustive build artifacts behind.

## Phase 0 evidence gate

Use the deeper evidence gate before a release or when changing renderer layout,
PTY ownership, concurrency, control-string parsing, or security boundaries:

```text
cargo qa
cargo qa --bundle
```

These aliases run `cargo xtask qa --full [--bundle]`. The full profile first
tests its own timeout, process-tree cleanup, redaction, log-cap, host-manifest,
and bundle-privacy contracts. It then checks formatting, locked metadata,
identity/provenance/architecture/packaging policy, repository formats, shell
contracts, warning-denied Clippy, pinned Nextest/JUnit, separate Cargo doctests,
deterministic resize/session suites, the finite Loom model, and cargo-deny. Every
subprocess has a named hard deadline; timeout kills the complete Windows process
tree or POSIX process group and is recorded as a required failure. The command
writes an atomic report below `target/qa/<UTC-run-id>/`; `--bundle` adds a ZIP
beside that directory.

Install the pinned contributor runner once if `cargo xtask doctor` reports it
missing:

```text
cargo install cargo-nextest --version 0.9.137 --locked
```

The product launcher never installs QA tools or changes machine-wide verifier
state. AppVerifier/WPR and native GUI runs are opt-in because they require an
interactive/elevated controlled host:

```powershell
$env:AUTOMEXIA_QA_NATIVE = '1'
$env:AUTOMEXIA_QA_COVERAGE = '1'
$env:AUTOMEXIA_QA_APPVERIFIER = '1'
$env:AUTOMEXIA_QA_WPR = '1'
cargo qa --bundle
```

Set `AUTOMEXIA_QA_COVERAGE=1` on Windows to build LLVM coverage in an isolated
target, enforce the recorded global and changed-owned-line thresholds, retain
only the path-free JSON summary, and delete both the raw LCOV and instrumented
target. Set `AUTOMEXIA_QA_BENCHMARKS=1` only on named stable hardware. The
report marks unavailable native, benchmark, 30-day, cross-platform GPU, and
screen-reader work as `external`, never as passed.

Logs are capped at exactly 2 MiB each; overlong untrusted lines are suppressed,
and workspace/home roots, escaped Windows paths, and token-like values are
redacted. Portable files are capped at 16 MiB and the uncompressed bundle at
64 MiB with an included/excluded manifest. Reports never enumerate the
environment or capture terminal content, clipboard data, credentials, or user
configuration. WPR ETL, raw LCOV, and live-terminal PNG captures are private and
excluded from the ZIP; only bounded structured summaries belong in portable
evidence. The allowlisted host record contains OS/architecture, safe shell and
WSL versions, primary display/DPI, GPU/driver, observed renderer status, and
power-scheme GUID without host name, username, environment values, or paths.

The Windows native stress writes an atomic resource report into QA evidence
when `AUTOMEXIA_QA_NATIVE=1` and otherwise keeps direct focused runs
non-mutating. It enforces explicit ceilings for handles, threads, private bytes,
working set, and descendant processes. It always validates a topmost, client-region capture of the composited final
frame for usable dimensions, sample count, color diversity, and luminance
spread. `-FrameCapture <private-path>` explicitly retains a PNG for local human
review; omission keeps terminal pixels in memory only. The portable QA bundler
defensively excludes PNG and ETL files. The controlled wrappers are:

```powershell
tests/integration/appverifier-windows.ps1 -OutputDirectory target/native/appverifier
tests/integration/wpr-windows.ps1 -OutputDirectory target/native/wpr -DeleteTraceAfterManifest
```

Both validate the exact `automexia.exe` target. AppVerifier refuses to overwrite
pre-existing verifier state and always removes settings it created. WPR cancels
a recording it started on failure and can delete the private ETL after hashing
and recording its size/host manifest.

Focused tab-scope regressions can be run while iterating:

```text
cargo test -p automexia-terminal bindings::tests::ctrl_t
cargo test -p automexia-terminal layout::pane_tab_tests
cargo test -p automexia-terminal renderer::island::tests::local_tab_rail
cargo test -p automexia-terminal renderer::command_palette::tests::window_window_tab
cargo test -p automexia-terminal renderer::command_palette::tests::pane_and_local_tab_navigation
cargo test -p automexia-terminal renderer::session_footer::tests
cargo test -p automexia-terminal pane_footer_reservation
```

These checks cover shortcut scope, local order and last-tab retention, distinct
select/close/add hit targets, command-palette labels, the absence of obsolete
workspace/footer action hit targets, passive footer routing, session-aware
LF/CRLF selection, fixed-width clock formatting, DPI-stable grid reservation,
footer collapse at extreme pane heights, preservation of the vertical chrome
origin, exact single-pane edge connection, and gap-free adjacent split-footer
tiling. The
full frontend and workspace gates additionally cover PTY route isolation and
teardown behavior.
Focused clipboard-input regressions can be run with:

```text
cargo test -p automexia-terminal --bin automexia --locked ctrl_c_copies_only_a_nonempty_selection_and_otherwise_remains_interrupt
cargo test -p automexia-terminal --bin automexia --locked secondary_click_copies_and_clears_selection_or_pastes_clipboard_exclusively
cargo test -p automexia-terminal --bin automexia --locked default_mouse_clipboard_bindings_preserve_primary_selection_ownership
cargo xtask verify architecture
```

These checks prove that bare `Ctrl+C` copies only a non-empty terminal
selection, still encodes ETX (`0x03`) with no selection, and consumes the
matching key release instead of leaking a Win32 input event. They also prove
that secondary click chooses exactly one copy-and-clear-or-paste action, middle-click
retains primary-selection paste, and no left-click binding can paste. Search,
Vi-mode, user binding, application mouse-reporting, bracketed-paste filtering,
and empty-clipboard behavior remain owned by their established paths. Manual
native verification should additionally cover drag selection, touchpad
secondary click, a running command interrupted with no selection, and a
mouse-reporting TUI.

Focused regressions for the 2026-08 upstream correctness adaptation are:

```text
cargo test -p rio-vt --lib
cargo test -p rio-backend --all-targets
cargo test -p automexia-terminal --all-targets
cargo check -p sugarloaf --no-default-features
cargo check -p sugarloaf --features wgpu
cargo clippy -p rio-vt -p rio-backend -p sugarloaf -p automexia-terminal --all-targets --all-features -- -D warnings
cargo xtask test resize-stress
```

These cover Kitty query/placement honesty and memory release, explicit
background intensity, combining-mark damage, bulk parser parity, synchronized
updates, viewport/history invariants, IO-free image-path discovery, safe
link/preview click latching, pinned arrow navigation, and both CPU-only and
product GPU renderer configurations. The
exact upstream hashes and Automexia-specific adaptations are recorded in
`UPSTREAM.md`.
Image protocol and local quick-look changes have a focused gate:

```text
cargo xtask test image-rendering
cargo xtask test image-rendering --native-gui
cargo test -p automexia-image --locked
cargo test -p automexia-terminal image_preview --locked -- --test-threads=1
cargo test -p automexia-terminal bindings --locked
cargo test -p automexia-terminal command_palette --locked
cargo test -p rio-vt --features graphics bounded_decoder --locked
cargo xtask test image-decoder-fuzz --seconds 120
cargo xtask verify architecture
```

The required PR command covers every enabled raster codec, exact RGBA and
straight-alpha behavior, supported/unsupported extensions, URL/control/symlink
rejection, quoted and bare paths, WSL mapping, file/dimension/pixel/allocation
limits, malformed/truncated/mutated input storms, no small-image upscale,
tiny/large/edge geometry, route-generation stale-result rejection, bounded
queue/cache replacement, exact cache byte accounting, 1,000 repeated warm
lookups, source-handle release, and the absence of generated sidecar files. It
also runs Sugarloaf CPU/GPU-resource accounting, VT/backend protocol regressions,
and compiles the release benchmark.

On Windows the `--native-gui` form adds real WGPU and CPU windows. Each backend
runs 16 hover/open/dismiss cycles, proving active route pixels/overlay/texture
counts and exact GPU bytes, zero active resources after dismissal, empty worker
queue/mailbox state, bounded thumbnail retention, and bounded process
handle/thread/private-memory growth. The native capture samples transparent and
opaque fixture regions, rejects black or card-obscured pixels, and compares
WGPU/CPU dimensions and luminance distributions. Protocol rendering and local
quick look remain separate contracts; native Linux/macOS evidence follows the
matrix in [image previews](IMAGE-PREVIEWS.md).

The decoder fuzz command installs/uses explicit nightly on Unix. On Windows it
uses WSL because cargo-fuzz/libFuzzer does not support native Windows; this
avoids misleading `clang_rt.asan_dynamic` DLL failures. It fuzzes both bounded
decode and visible-path tokenization. The runner copies the current source tree
once from Windows into a disposable WSL-native `/tmp` workspace, excluding
`.git`, the workspace target, and generated fuzz target, corpus, and
artifact directories; all Cargo build, corpus, and target I/O then
stays under `/tmp`. It caps RSS/input time and removes the complete staged
campaign on exit. Nightly CI separately installs nightly, invokes every target
with `cargo +nightly fuzz`, and runs pure decoder tests under ASan and TSan.
The 2026-08-14 Windows-to-WSL decoder campaign completed 544,609 executions
over 121 seconds without a crash or sanitizer finding (2,504 coverage edges,
5,383 features, 1,161 final corpus entries, and 357 MiB peak RSS). These are
local evidence for the corrected runner, not a substitute for recurring hosted
nightly results.
The 2026-08-15 post-hardening Windows-to-WSL campaign compiled exclusively from
the staged `/tmp` source and completed 228,879 executions in six seconds
without a crash or sanitizer finding (1,982 coverage edges, 4,227 features,
1,049 final corpus entries, and 410 MiB peak RSS). The cleanup trap left no
`automexia-image-fuzz.*` directory or generated Windows-tree state.

Control-string and reload hardening has a focused local gate:

```text
cargo test -p rio-vt performer:: --locked
cargo test -p automexia-terminal application::custom_chrome_tests --locked
cargo test -p automexia-terminal global_hotkey::tests --locked
cargo xtask verify architecture
cargo xtask verify identity
```

OSC retains at most 1 MiB, APC/graphics 96 KiB, and XTGETTCAP 4 KiB. Tests
exercise exact-limit and limit-plus-one input, fragmented and unterminated
state, repeated attacks, memory bounds, non-dispatching CAN/SUB cancellation,
and valid recovery. `fuzz/fuzz_targets/control_string_bounds.rs` drives mixed,
fragmented oversized streams and is part of the nightly fuzz matrix. Sixel data
streams through its dimension-bounded decoder, and synchronized-update storage
keeps its existing 2 MiB cap.

Reload tests require malformed config, malformed theme, missing path, missing
font, and global-hotkey registration failures to leave the logical
last-known-good generation active. Hotkey tests inject addition/removal failures
and verify reverse-order rollback. The application event loop serializes reload
events; OS-level rollback failures are surfaced explicitly because desktop
hotkey APIs do not provide an atomic transaction.

On Windows, `cargo xtask test resize-stress --native-gui` creates a real
pane-local PowerShell tab, proves independent route/PID and preserved launch
intent, navigates previous/next within only that pane, closes its inactive
sibling without losing the source, creates a right split, focuses left/right by
rendered geometry without changing either route, then runs the multi-pane
resize storm and requires automatic full-path restoration. Binding-table tests
separately prove the platform chords dispatch those tested actions.

The same native run opens the command palette and close confirmation through
feature-gated renderer controls, requires exactly one modal owner at a time,
waits for a later presented frame, checks nonblank WGPU and CPU captures, then
dismisses the overlay and proves no hidden input-blocking state remains.
Sugarloaf unit coverage enforces the physical order from base primitives and
base labels through modal primitives and modal labels, including a
load-preserving WGPU modal pass; pane borders, footers, scrollbars, and ordinary
labels therefore cannot render over either modal.

To retain native modal screenshots for human visual review, set the report path
before running the canonical command:

    $env:AUTOMEXIA_NATIVE_RESOURCE_REPORT = "$PWDartifacts
ative-modalwgpu.json"
    cargo xtask test resize-stress --native-gui

The command writes WGPU and CPU palette/confirmation PNGs under the adjacent
modal-captures directory. These artifacts are local evidence and must not be
committed.

## Build-artifact lifecycle and storage

### Keep every toolchain on its native filesystem

Windows Cargo/MSVC, ConPTY, WGPU, and MSI work belongs in the NTFS checkout.
Linux Cargo, Unix PTY, sanitizer, and Linux GUI work inside WSL belongs in a
separate clone under the Linux filesystem, such as
`~/src/automexia-terminal`. Building from `/mnt/c` or `/mnt/d`
causes expensive cross-filesystem metadata traffic.

`cargo xtask doctor` reports `host-native/ok`, `WSL-native/ok`,
or an actionable mounted-drive advisory. The compilation-heavy project commands
fail early when WSL source or `CARGO_TARGET_DIR` is on a mounted Windows
drive. `AUTOMEXIA_ALLOW_SLOW_WSL_MOUNT=1` exists only for deliberate
one-off diagnosis and is not valid CI, benchmark, fuzz, or release evidence.

The complete setup, synchronization, target-placement, fuzz-staging, and
troubleshooting procedure is in
[Windows and WSL development](WSL-DEVELOPMENT.md).

The workflow has three deliberately separate artifact classes:

- `target/debug` is the persistent incremental application build used by
  `cargo automexia`; it makes ordinary edits and launches fast.
- `target/automexia-verification-v1-<pid>-<generation>` is the unique,
  non-incremental target used by
  `cargo xtask check`, `cargo ci`, `cargo ready`, and the verification phase
  of `cargo dev`. It is removed after success or ordinary failure, so
  all-target checks, Clippy, and tests cannot accumulate separate incremental
  graphs. Concurrent gates use different directories, so one run cannot clean
  another run's build products. Cargo's native `cargo check` command remains
  available for focused incremental diagnosis.
- `target/automexia-runtime` contains generation-specific launch copies. On
  Windows this lets the running process own its copy while Cargo's canonical
  `target/debug/automexia.exe` remains replaceable. Stale, unlocked copies are
  removed automatically before each launch.

Inspect the current target without changing it:

```text
cargo storage
```

The report shows the resolved target, filesystem capacity, total target size,
and its twelve largest direct children. The default warning is 12 GiB. To
recover all build space, first close every Automexia window and then run:

```text
cargo purge
```

`cargo purge` is the cross-platform alias for Cargo's built-in `clean` and
honors `CARGO_TARGET_DIR`. It removes rebuildable artifacts, never configuration
or source files.

An exhaustive verification gate needs at least 12 GiB free before it starts; a
persistent application build needs 4 GiB. This prevents a predictable build
from filling the volume halfway through. The following integer-GiB environment
variables exist for unusual build hosts:

- `AUTOMEXIA_VERIFY_MIN_FREE_GIB` (default `12`);
- `AUTOMEXIA_BUILD_MIN_FREE_GIB` (default `4`);
- `AUTOMEXIA_TARGET_WARN_GIB` (default `12`);
- `AUTOMEXIA_KEEP_VERIFY_TARGET=1` retains verification output for deliberate
  diagnosis instead of deleting it.

Keep the defaults on contributor machines. CI, nightly, and release workflows
set `CARGO_INCREMENTAL=0` and cache only downloaded Cargo registry/Git content,
not `target` build products.

To validate another checkout or filesystem, provide an absolute or
invocation-relative `CARGO_TARGET_DIR`; build, smoke, launch, storage preflight,
and cleanup all resolve the same directory consistently.

The local gate validates all checks that can run on the current host. GitHub CI
keeps separate native and cross-platform jobs for operating-system matrices,
coverage, CodeQL, dependency review, fuzzing, sanitizers, and controlled
hardware that one contributor machine cannot reproduce.

Cargo creates a test executable for each applicable library, binary, integration
target, and documentation target. Therefore, `test result: ok. 0 passed; 0
failed` is expected for a target that defines no tests; it does not mean the
workspace test suite was skipped. The command is successful only when every
target completes and `cargo ready` prints its final `PASS` line.
`cargo ready` summarizes successful harness output to keep the normal workflow
readable and prints the complete captured diagnostics automatically on failure.
Running `cargo test` directly retains Cargo's normal per-target output.

Known incompatible transitive dependency generations are maintained as an
exact, reasoned baseline in `deny.toml`. They do not print repetitive warnings.
Any newly introduced duplicate is denied, while advisories, banned crates,
licenses, and dependency sources continue to be checked independently.

## Every pull request

Every PR runs policy checks regardless of changed paths:

- Cargo metadata/lock consistency and `rustfmt --check`;
- TOML, YAML, JSON, XML, shell, PowerShell, documentation, and link validation;
- product identity, provenance/license, architecture graph, package metadata,
  and brand-manifest verification;
- warning-denied workspace Clippy and workspace tests on Windows, Linux, and
  macOS;
- Linux X11-only, Wayland-only, and combined checks;
- Windows MSVC x64 tests and ARM64 cross-check;
- macOS x64 and ARM64 compile checks;
- `cargo deny`, dependency review, CodeQL, and secret-safe fork permissions;
- LLVM coverage with a non-decreasing recorded global baseline and at least 80%
  line coverage on changed Automexia-owned lines.

An inherited engine file is exempt from the changed-line threshold only while
untouched. Any engine change requires a focused regression test.

To evaluate the exact local working tree, including uncommitted and untracked
Rust sources, generate coverage and select the checker's explicit worktree
mode:

```powershell
$env:BASE_SHA = "HEAD"
$env:HEAD_SHA = "WORKTREE"
$env:COVERAGE_PLATFORM = "windows-x86_64-msvc"
cargo llvm-cov --workspace --locked --lcov --output-path lcov.info
python tools/ci/check_coverage.py
```

For pull requests, CI continues to set `BASE_SHA` and `HEAD_SHA` to the exact
base and head commit IDs. The checker anchors Git and report paths to the
repository root, so invoking it from a wrapper or a different directory cannot
silently inspect the wrong checkout.

## Terminal conformance

Fixtures cover fragmented and malformed VT/CSI/OSC/DCS sequences, OSC 7, OSC
133, OSC 1337 user variables, title/prompt lifecycle, Unicode graphemes,
combining marks, emoji width, and cursor position. PTY suites cover ConPTY and
Unix lifecycle, resize, child exit, teardown, and throughput.

Renderer-neutral goldens cover prompt anchors, clipping, segment truncation,
selection/search precedence, stable OSC prompt identities, metadata-only
incremental snapshots, repeated command transitions, and shrink/grow reflow.
Pane-footer checks prove that it has no action regions, passive clicks route to
the exact pane, PowerShell/CMD and Unix/WSL sessions report their expected line
ending convention, clock formatting remains stable, its physical grid
reservation is DPI-stable, tiny panes recover the reserved row space, and the
terminal scrollbar never enters the footer.
Extension tests cover unavailable,
disconnected, busy, stale, malformed, and oversized inputs plus multi-window
session isolation. Shell tests cover syntax, idempotency, exit status, history
handlers, monotonic prompt identities, UTF-8 lambda handling, and uninstall
behavior. The Windows contract additionally executes a generated native CMD
integration in-process, proves a zero-argument wrapper cannot become one empty
native argument, verifies that `cmd /c` is untouched, requires BOM-free ASCII
batch deployment, validates repeatable CMD shell/user/executable and OSC 7/133
metadata, and runs the explicitly UTF-8 category-aware listing helper against
Unicode and sensitive/source fixtures.

The conformance suite is included in `cargo ready`. For focused diagnosis only,
run it directly with `cargo xtask test conformance`.

Prompt and resize resilience has a dedicated deterministic gate:

```text
cargo xtask test resize-stress
```

It feeds the real Automexia OSC 7/133/1337 byte stream through the parser at
every fragmentation boundary, performs 2,000 fixed-seed one-column through
8K-equivalent reflows, interleaves editing and command transitions, and checks
cursor bounds, row widths, prompt ordering, stable `aid` ownership, completed
history, and exact logical path/lambda counts after every resize. The active
prompt snapshot is immutable for one generation, every scalar/ASCII/Unicode
writer path propagates its `aid`, and the final effective resize repairs a
missing terminal-owned context even when no later PTY byte arrives. It also
proves that a hard shell newline ends context-row ownership, so incomplete or
legacy prompt markers cannot attach later command output to the active prompt.
The same gate proves that 1,000 queued PTY resizes collapse to the final size while input and
shutdown remain ordering barriers. A recording PTY sink verifies exact
delivery order, duplicate suppression, final size, and retry behavior after a
transient resize error. Full-screen clear/home/line-erase shell-editor repaints
must remove stale cells without absorbing lambda/input into the context
snapshot; combining-mark and emoji paths survive tiny viewports, scrollback,
and automatic restoration at a usable size.

On an interactive Windows machine with a working GPU, the native driver adds a
real-window storm, validates renderer-neutral JSON snapshots, and samples the
actual composited pixels in Automexia's client region after the final repaint:

```text
cargo xtask test resize-stress --native-gui
```

The driver waits for one complete first prompt without sending input, executes
240 real window moves, restores a usable viewport, and waits for a matching
post-reflow snapshot before asserting. It then opens local quick look through
real hover/click/arrow input, samples only the published image rectangle, and
rejects a present-but-black or card-obscured preview using color-bucket,
luminance-spread, mean-luminance, and bright-pixel thresholds. The complete
native contract runs once on WGPU and once on the CPU fallback. WGPU swap-chain pixels are not reliably
available through `WM_PRINT`, so the driver converts the exact client origin to
screen coordinates, temporarily places only the target window topmost, copies
that bounded region with `BitBlt`, and restores normal z-order in `finally`. A
strict five-second presentation deadline rejects a zero-sized, blank, or
insufficiently varied frame. The same driver enters and exits the real F11/
Alt+Enter borderless-fullscreen path, requires exact display coverage, proves a
successful per-window Windows `DisplayRequired` request through non-privileged
test-only state, and requires release on exit. It samples the composited frame
before, during, and after the transition; the dominant color bucket must remain
identical, which prevents application-side alpha, gamma, or HDR regressions.
Newly visible secondary windows must also present
a varied frame before the one-shot custom-close click is tested, preventing an
HWND-visible/application-not-ready race. The opt-in `native-gui-test-hooks`
build feature is enabled only by that command. Product builds perform no
snapshot or capture I/O.

Before the pane-local coverage, the native gate exercises the exact top-level
tab lifecycle used by `Ctrl+T`. It requires the new tab to be selected exactly
once, expose its launch profile before shell output, inherit the live window
viewport rather than the reduced PTY extent, place its footer against the pane
bottom, and publish one complete prompt without synthetic keyboard input.
The same real ConPTY then enters bare `cmd`, requires CMD identity and the
complete lambda/path prompt without a second keypress, renders folder and Rust
icons directly beside fixture names, exits, and requires PowerShell identity to
return on the first parent prompt. Native snapshots are decoded explicitly as
UTF-8, so mojibake cannot satisfy the glyph assertions.
The driver sends the already-tested CSI Up encoding through Automexia's input
queue, avoiding nondeterministic desktop foreground-lock policy while retaining
the real frontend queue, ConPTY, PSReadLine, VT, damage, and renderer path. Rust
window-input tests separately prove the Windows physical-key metadata and CSI
encoding.
At the end of the same native gate, Automexia creates a second OS window through
the action bound to `Ctrl+Shift+N`, clicks its real custom-chrome close target,
and requires the original HWND, process, panes, and PTYs to survive. It repeats
the assertion with a native Windows `WM_CLOSE` request. Unit coverage separately
proves intermediate/last-window confirmation policy, the `Ctrl+Shift+N`
binding, timer cleanup, and distinct window-close/process-quit actions. The
native window-count assertion proves that test controls cannot recursively
create additional windows. The native driver delivers the custom-close move,
press, and release synchronously, eliminating posted-message reordering from
the isolation assertion.

CI runs the deterministic gate on every pull request; nightly runs the native driver when the protected
`automexia-gpu` self-hosted runner is enabled.

Session cloning has its own deterministic gate:

```text
cargo xtask test session-clone
```

It covers action names, user overrides, Search/Vi exclusions,
classic `Ctrl+R`/`Ctrl+D` cloning, explicit `Ctrl+Alt+R`/`Ctrl+Alt+D`
shell-control passthroughs,
PowerShell/pwsh, nested/direct CMD, and native Bash/Zsh descriptors, direct
and nested WSL descriptors, incomplete metadata, spaces/Unicode, environment
overrides, unknown/invalid logical directories, safe profile fallback, and
CreateProcess-compatible quoting. On a Windows GPU workstation,
the full native clone-plus-resize storm is:

```text
cargo xtask test session-clone --native-windows
```

That driver first executes a unique PowerShell command and requires both Up
Arrow recall and raw `Ctrl+R` reverse search to repaint through ConPTY within
the 1.5-second native budget. On the audited Windows host, three consecutive
full native runs measured Up at 1.049-1.101 seconds and reverse search at
0.972-0.981 seconds; a renderer-free ConPTY probe measured the same roughly
one-second floor with Windows PowerShell 5.1 and PSReadLine 2.0. Ordinary
printable input remains independently capped at 500 ms, so a slow shell action
cannot be misreported as frontend/PTY input lag. `cargo xtask doctor` reports
that legacy host/module combination and recommends PowerShell 7 or a supported
current stable PSReadLine without modifying the machine. The raw control uses
the user-facing `Ctrl+Alt+R` passthrough because classic `Ctrl+R` is owned by
session cloning. The Windows-only PTY regression negotiates Win32 input-record
mode and checks the same shell operations without the renderer, keeping
protocol, shell, and render latency separable.
It then creates three independent PowerShell clones (four panes total),
verifies unique routes and ConPTY child PIDs, proves clone-only input/output
cannot contaminate the source, and interleaves the final clone plus active-pane
changes into 240 resize transitions before checking prompt/path restoration. A
controlled WSL runner additionally executes:

```text
cargo xtask test session-clone --native-wsl
```

Set `AUTOMEXIA_TEST_WSL_DISTRO` to the installed test distro. The suite proves
that distro, user, executable, profile, and Linux directory survive cloning and
that no PowerShell fallback is opened. These native hooks are feature-gated and
are absent from product builds.

For native liquid-hacker UI and prompt regressions, run:

```text
cargo test -p rio-vt semantic
cargo test -p automexia-terminal renderer::island::tests
cargo test -p automexia-terminal renderer::devops_status::tests
powershell -NoProfile -File tools/ci/test_shell_integration.ps1
```

The DevOps/runtime suites cover bounded WSL probe parsing, session-local
Docker/Kubernetes/cloud/Git/Terraform/user models, truthful badge visibility,
default Docker labeling, snapshot-publication-before-redraw ordering,
originating-route wake-up, strictly equivalent five-second snapshot reuse,
rejection of cross-path/unintegrated/stale reuse, and the 100 ms fallback/
three-second steady refresh cadence. Renderer tests additionally enforce every
semantic color anchor, independent AWS/Azure/GCP/unknown-cloud mapping,
pairwise default-theme distinction, shared live/history resolution, and
quantized 4.5:1 contrast on dark, light, low-contrast, and custom backgrounds.
Release smoke testing must additionally
confirm a real WSL Docker context appears at initial launch and after switching
shells without typing, opening a new prompt, or restarting the terminal.

The Windows shell integration test proves that synchronous first-prompt style
installation does not remove filesystem icons, change native `ls` object
semantics, block the first command/history repaint, or require user input. Its
listing fixtures cover the four native metadata columns, composite
folder-badge/name adjacency, directory suffixes,
spaces and Unicode, narrow-width truncation, and real `DirectoryInfo`/`FileInfo`
values after filtering and sorting. CMD coverage proves its interactive launcher
uses the existing PTY rather than a detached process, direct configured CMD
profiles receive integration once, nested CMD clones retain `%ComSpec%` and the
live directory, `ls`/`ll` keep icon/name adjacency, and built-in `dir` remains
unmodified.

The Bash suite feeds representative eza 0.18.x ANSI output through the bundled
TTY compatibility filter and checks configuration and source folder badges,
category colors, and an unchanged unclassified folder. Bash/Zsh integration
tests also assert the deterministic root/cyan/violet/blue/lime path hierarchy.
`cargo test -p rio-fonts` parses the embedded Symbols Nerd Font and verifies
that every declared composite folder codepoint has a real glyph.

These cover Windows-drive versus WSL title classification, custom chrome hit
targets and resize edges, the absence of workspace-action paint and hit targets,
conditional pane-local tab-rail reservation, pane/sibling isolation, HiDPI hit
testing, terminal-content displacement, native snapshot restoration, bundled Nerd icon
codepoints, explicit shell identity, terminal-owned full-path three-row prompts,
per-command context snapshots, OSC command status/timing, and context/result
survival through shrink/grow reflow.

Responsive regressions exercise the supported 300×200 minimum, compact and
comfortable breakpoints, transient invalid dimensions, 4K/8K HiDPI logical
equivalence, very large grid counts, tab/control non-overlap, hidden-control
hit targets, adaptive palette row counts, overlay containment, Unicode-safe
label elision and split-layout underflow. Run the focused set with:

```text
cargo test -p automexia-terminal renderer::responsive::tests
cargo test -p automexia-terminal renderer::island::tests
cargo test -p automexia-terminal renderer::command_palette::tests
cargo test -p automexia-terminal layout::compute_tests
```

## Nightly and release depth

Nightly jobs fuzz VT, bounded OSC/APC/XTGETTCAP streams, bounded raster
decoding, OSC metadata,
configuration migration, semantic classification, and label sanitization. Suitable pure crates run Miri and
ASan/TSan. Criterion cases exist for parser throughput, row rebuild, prompt layout,
cache access, worker submission, and cold/warm image quick look. Hosted nightly
always compiles them; a named self-hosted runner executes and retains Criterion
evidence when AUTOMEXIA_BENCHMARK_RUNNER=1. Run the commands below for local
measurements. The controlled 30-day comparison baseline is not complete.

The renderer-neutral application-service benchmarks are available with:

```text
cargo bench -p automexia-terminal --bench automexia_services -- --noplot
```

They measure the route-scoped extension snapshot cache and non-blocking bounded
worker submission path. The isolated production image decoder/cache benchmark
avoids benchmark-time calls into those unrelated services:

```text
cargo bench -p automexia-terminal --bench image_preview --locked -- --noplot
```

It compares cold 1600x1000 decode/downscale with a warm file-version-validated
lookup. Record both medians; the warm path must retain the same allocation and
render identity, and cache memory remains capped independently of timing.

The renderer-neutral row_rebuild_full_snapshot` and
`prompt_layout_resize_reflow` cases cover full visible-row materialization and
repeated narrow/wide semantic-prompt reflow.

The performance roadmap includes startup, sustained PTY throughput,
resize/reflow latency, idle/scrollback memory, and extension refresh latency.
The complete 30-day controlled baseline has not yet been collected. Once the
execution pipeline and baseline exist, results remain informational for 30 days;
afterward, regressions above 5% latency or 10% memory need a recorded
maintainer waiver.

For a focused optimized measurement of the most common unchanged-frame fast
path, run:

```text
cargo bench -p rio-vt --bench vt_input snapshot_visible_noop -- --noplot
```

The benchmark starts from a populated styled terminal and repeatedly requests
a no-damage snapshot. It protects the contract that cursor/UI-only activity
does not copy the resident style table or visible grid. Run the complete
`vt_input` benchmark before and after changes to parser, grid, or snapshot code;
record the machine, power mode, and median result in any performance waiver.

Shell-history repaint latency has a dedicated deep-scrollback benchmark:

```text
cargo bench -p rio-vt --bench vt_input history_navigation_repaint_deep_scrollback -- --noplot
```

It keeps 15,000 historical rows behind an active OSC 133 prompt and measures
the clear/repaint pattern emitted by PSReadLine, Readline, and ZLE for Up Arrow
and reverse-history search. Runtime must remain proportional to the live prompt
block, not the configured scrollback depth.

Current keybinding tests construct macOS, Windows, and Linux/BSD default
tables on every host, verify classic tab/split/clone scopes, geometric pane
focus, pane-local tab cycling, global-tab separation, and explicit shell
passthroughs, exercise user overrides and intentional compound actions, and
reject shortcut collisions and duplicate visible palette labels. Pure layout
tests cover all four directions, uneven/nested grids, perpendicular-beam
preference, deterministic ties, edge stopping, and local-tab wraparound. The
planned compiled-profile suite—including fixture provenance,
origins and shadowing, atomic reload, fallthrough, sequences/tables/chains,
generated docs, fuzzing, and hot-path latency—is specified in the
[full Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md) and must
not be reported as implemented until those gates exist and pass.

Nightly builds unsigned installers for every artifact target. Stable release
requires WSL, real-GPU, clean-install, upgrade, uninstall, signature,
notarization, URL handler, terminfo, and migration smoke tests on controlled
hardware/self-hosted runners.

## Assurance status and remaining expansion

The Phase 0 local baseline now includes pinned Nextest/JUnit/doctests, a
self-tested deadline/process-tree-safe and privacy-bounded `cargo qa --bundle`,
allowlisted host identity, isolated coverage summaries, shrinking viewport/DPI
properties with a persisted regression, a reviewed structured footer snapshot,
finite Loom models, Windows resource ceilings, and topmost client-region final-frame
smoke validation.
These are implemented commands and locally passing evidence, not release-host
claims.

The remaining roadmap work is deliberately separate:

- controlled expected/actual/diff raster goldens across viewport, theme, font,
  and DPI matrices plus native Linux/macOS frame evidence;
- broader pure-state Proptest/Loom models, longer persisted fuzz campaigns, and
  a separate Automexia-owned coverage baseline;
- executed and compared Criterion/startup/interaction/resource evidence on named
  stable hardware followed by the complete 30-day baseline;
- elevated Windows Application Verifier/WPR evidence and expanded controlled GPU
  resource tests on all supported operating systems;
- recorded v0.4 Narrator/NVDA, VoiceOver, and Orca smoke followed by the v0.5
  renderer-independent native accessibility model; and
- v0.5 scoped mutation testing and maintainable cargo-vet supply-chain audits.

The authoritative ordering, dependencies, exclusions, CI tiers, and acceptance
criteria are in the
[stabilization roadmap](STABILIZATION-ROADMAP.md#verification-infrastructure-plan).
A source implementation never substitutes for the hosted, elevated, signed, or
human-reviewed evidence named there.
