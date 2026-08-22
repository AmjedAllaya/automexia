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
next command while Automexia continues running. Normal launch never installs,
repairs, or rewrites a shell profile. On Windows, the launcher passes the
validated source integration directory only to the child Automexia process;
PowerShell and interactive CMD sessions load those resources for that process
tree. Unix profile integration remains an explicit installer operation.

`cargo ready`, `cargo check`, `cargo xtask ci`, and ordinary application launch
remain non-mutating. Persistent changes require the explicit
`automexia shell-integration install` command (or direct platform installer),
and PowerShell execution policy is never bypassed by the application or local
verification commands.

The complete gate also parses every repository PowerShell source, exercises the
explicit Windows install/repair/uninstall paths in isolated profile and
LocalAppData fixtures, and executes the PowerShell formatter/prompt contract on
Windows. On Unix it syntax-checks Bash and Zsh, runs ShellCheck, exercises the
explicit installer twice in an isolated home, repairs a deliberately changed
installed file, and executes both shell-integration suites.
`cargo xtask ci` runs the same non-launching gate; neither command leaves its
isolated exhaustive build artifacts behind.

### Native platform ownership

Cross-platform behavior is accepted on the operating system that owns the
adapter; compiling an Apple target from Windows is not a substitute for a
macOS run because Apple's SDK, window server, signing policy, and GPU stack are
host-provided. The required evidence is:

| Surface | Required host and checks |
|---|---|
| Portable Rust, metadata, configuration, bindings, and renderer-neutral layout | Every PR runs locked, all-feature Clippy, Nextest, and doctests on native Windows, Ubuntu Linux, and macOS. |
| PowerShell, CMD, ConPTY, window ownership, WGPU/CPU rendering, and Windows shell formatting | Native Windows runs `tools/ci/test_powershell.ps1`, `cargo xtask test resize-stress --native-gui`, `cargo xtask test session-clone --native-windows`, and the focused image GUI lifecycle. |
| Bash/Zsh install, repair, prompt metadata, and listing behavior | Native Linux and macOS run `bash tools/ci/test_shell_sources.sh`; the script uses only Bash 3.2/BSD-compatible temporary-file semantics and tests an isolated home. |
| Linux display adapters | Ubuntu checks the frontend separately with X11-only, Wayland-only, and combined features. Release jobs additionally validate DEB and RPM metadata/install behavior; this does not imply that every downstream Linux distribution has been manually certified. |
| WSL launch and clone routing | Native Windows plus an installed WSL distribution runs `cargo xtask test session-clone --native-wsl`; Linux source/build artifacts stay on the WSL filesystem rather than `/mnt/<drive>`. |
| macOS windows, Metal/WGPU, universal application, signing, and notarization | Native Intel/Apple-Silicon macOS jobs own compilation and tests. Controlled macOS hardware owns GUI, VoiceOver, Gatekeeper, notarization, and final artifact evidence. |

The native CI job intentionally enables every Cargo feature on all three host
families. Platform-specific code must use target configuration, not rely on a
feature being absent from one host. A platform result is reported as
`external` or `not run` when its required host, credentials, display server, or
hardware is unavailable; it must never be inferred from a different OS.

## Enforced feature assurance ledger

`tests/assurance/feature-matrix.json` is the machine-readable ownership and
evidence ledger for every shipped v0.4 product surface. It maps all Cargo
workspace members plus shell integration, packaging, workflows, and contributor
automation to these mandatory dimensions:

- correctness;
- security and hostile-input behavior;
- performance;
- process, thread, handle, memory, queue, cache, and GPU-resource lifetime;
- persistent and temporary storage hygiene;
- failure, cancellation, resize, reload, and teardown resilience;
- accessibility;
- rendered visual quality.

Each feature also owns non-empty `guide`, `reference`, and `explanation`
Markdown links. The validator resolves every file and heading, so an
implemented feature cannot satisfy quality/platform evidence while leaving its
user workflow, exact contract, or design rationale undocumented. The
[documentation contribution guide](DOCUMENTATION.md) defines canonical page
ownership and the website-ready information architecture.

Every entry also declares Windows, Linux, and macOS evidence. Evidence levels
are intentionally different: `pr` is deterministic contributor coverage,
`nightly` owns fuzz/sanitizer/soak work, `controlled` requires named hardware or
privileges, and `external` is an explicit unobserved gate rather than a pass.
`not_applicable` is accepted only with a feature-specific rationale. Linux
evidence means the declared Ubuntu/X11/Wayland and DEB/RPM contracts; it never
claims that every downstream distribution or driver has been certified.

Run the ledger and workflow mutation contracts directly with:

```text
python tools/ci/check_feature_assurance.py
python tools/ci/test_feature_assurance.py
python tools/ci/check_documentation_coverage.py
python tools/ci/test_documentation_coverage.py
python tools/ci/check_phase_implementation_audit.py
python tools/ci/test_phase_implementation_audit.py
python tools/ci/test_pr_policy.py
python tools/ci/check_platform_coverage.py
python tools/ci/test_platform_coverage.py
```

The phase-audit contract also compares the implementation audit with every
canonical roadmap and the main roadmap's status-first feature register. The
register permits exactly **Fully done**, **Partially done**, or **Not done** and
must match every phase and normalized implementation status in the executive
matrix. The checker also requires an explicit status for each declared phase,
links to all roadmap sources, a pinned audited source baseline, and the shared
correctness, security, performance, resource, storage, resilience, retry,
cross-platform, accessibility, visual, test, benchmark, fuzz, coverage, and
release evidence vocabulary. Its mutation suite proves that stale, missing,
duplicated, or nonstandard roadmap statuses and missing phases, sources, or
evidence dimensions fail closed.

The pull-request policy separately treats source, configuration, test, workflow,
asset, and packaging changes as documentation-relevant. It rejects such a pull
request unless an affected `docs/*.md` file changes in the same diff; a
changelog fragment alone does not satisfy the rule. Its unit suite covers source,
test/workflow/configuration, asset/packaging, and documentation-only cases.

The canonical ledger, phase audit, and platform matrix are part of repository
validation, so `cargo ready`, `cargo ci`, and every pull request fail when a
workspace member, roadmap phase, required repository surface, quality
dimension, native host, shell contract, display feature, architecture check,
package validator, or referenced evidence path/job/heading loses ownership. A
pull request that adds or materially changes a feature or phase must update the
ledger, affected documentation, main-roadmap status register, phase audit, and
changelog in the same change; none of those records replaces the tests it
references.

On Windows, the contributor gate scopes RustSec's Git fetch to Git for
Windows' `schannel` backend when the caller has not supplied an explicit
`GIT_CONFIG_COUNT`. This preserves TLS verification while using the operating
system certificate store (including managed enterprise roots); an explicit
caller Git configuration remains authoritative.

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

### Non-activated session-launch review boundary

The pull-request policy is part of this fail-closed boundary. It classifies the
terminal context/process owner, extension API/runtime, SSH extension,
session-launch fixtures/checkers, security ADRs, workflow, and policy checker as
protected. Approval normalization rejects the author, bots, case-only
duplicates, malformed records, and approvals whose reviewed commit does not
match the exact pull-request head. Review JSON is capped at 4 MiB/10,000
records, and newline/delimiter injection in logins or 40/64-hex commit IDs is
rejected before writing the GitHub environment. Run its regression suite with:

```text
python tools/ci/test_pr_policy.py
```

This local check is defense in depth. Production activation still requires two
actual independent human approvals and server-side rules that prevent the
workflow/checker from weakening itself.

The proposed D3 broker is compiled only by frontend tests. Run its versioned
D0/D3 contract, mutation checks, and capability/package constructors with:

```text
cargo test -p automexia-extension-api --lib --locked
python tools/ci/check_session_launch_d0.py
python tools/ci/test_session_launch_d0.py
cargo test -p automexia-terminal --bin automexia --locked context::launch_broker::tests
cargo xtask verify architecture
```

The checker locks schema-1 immutability and the active schema-2 contract:
production-disabled activation, trusted-loader digest source/size, exact package
version/contract/verification, manual-shell and missing-client behavior, grant
and audit fields, strict defaults, nine trust boundaries, four-platform
resolution, the authority ceiling, and nineteen required native scenarios.

The mutation suite also locks the hermetic loopback protocol, isolated
disposable credential/`known_hosts`/agent state, bounded probes and exact
timeouts, DNS/connect/auth cancellation, platform activation rules, cleanup
invariants, evidence metadata, artifact policy, and nine redaction surfaces.
Rust tests cover Windows volume/file-index and Unix device/inode identity,
fixed platform roots, unverified/mismatched package denial, one literal
destination argument, option/environment/secret isolation, fail-closed
override resolution, authorization-owned cwd fallback, decision expiry/scope,
capsule rebind, replay, nonce exhaustion, redaction, revocation, stale/sibling
leases, and zero-retained-state 1/10/50 pure cycles.

These tests do not spawn OpenSSH and are not evidence that managed SSH is
available. Protected ADR approval, real package-loader attestation/revocation
binding, process/PTY/route ownership, cancellation/teardown, PID reuse,
application close, atomic native check-to-spawn, execution of host-key/auth/
tunnel/hostile-output scenarios, and 1/10/50-session process/PTY/renderer
results remain activation gates listed in
[ADR 0012](adr/0012-first-party-ssh-and-session-launch-boundary.md) and the
[broker contract](SESSION-LAUNCH-BROKER.md).

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

Focused keyboard-selection regressions can be run with:

```text
cargo test -p automexia-terminal --bin automexia --locked keyboard_selection
cargo test -p automexia-terminal --bin automexia --locked forwarded_input_exits_selection
cargo test -p automexia-terminal --bin automexia --locked selection_actions_parse
cargo test -p automexia-terminal --bin automexia --locked user_binding_can_override_shift_left_selection
cargo test -p rio-vt --lib --locked keyboard_
cargo check -p rio-vt --bench vt_input --locked
cargo xtask test resize-stress --native-gui
```

The frontend tests prove all six common shortcuts are unique, disabled during
search/Vi ownership, configurable, and collision-free beside platform pane and
tab shortcuts. VT tests cover reversal, row/scrollback boundaries, Unicode
words, decomposed Unicode, soft wraps, wide graphemes, malformed anchors,
vertical spacer avoidance, and exhaustive small-grid endpoint bounds.
`keyboard_selection_word_motion_4k` records long-identifier latency and
`keyboard_selection_word_motion_120k_scrollback` covers adversarial retained
history, both without per-key allocation. The feature-gated native Windows
storm types into a real PowerShell/ConPTY session and asserts the exact terminal
selection and renderer highlight from renderer-neutral state. It also posts a
real unmodified Left Arrow and sends text through the shared input seam, proving
both clear VT/render selection state before reaching PowerShell. Unit coverage
reproduces an empty mouse-click anchor at a different cell from the terminal
cursor and proves keyboard selection chooses the cursor. The same shared input,
binding, and VT code is compiled and tested by native Linux and macOS CI.
Physical keyboard-layout and assistive-technology checks remain host-owned
release evidence.

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
cargo test -p rio-vt --lib --locked control_string_buffers_allocate_lazily_and_release_large_high_water
cargo test -p rio-vt --lib --locked osc_buffer_is_bounded_and_releases_attack_high_water
cargo test -p automexia-terminal application::custom_chrome_tests --locked
cargo test -p automexia-terminal global_hotkey::tests --locked
cargo xtask verify architecture
cargo xtask verify identity
cargo bench -p rio-vt --bench vt_input --locked -- processor_default_lazy_control_buffers --quick --noplot
```

OSC retains at most 1 MiB, APC/graphics 96 KiB, and XTGETTCAP 4 KiB. Tests
exercise exact-limit and limit-plus-one input, fragmented and unterminated
state, repeated attacks, memory bounds, non-dispatching CAN/SUB cancellation,
and valid recovery. `fuzz/fuzz_targets/control_string_bounds.rs` drives mixed,
fragmented oversized streams and is part of the nightly fuzz matrix. Sixel data
streams through its dimension-bounded decoder, and synchronized-update storage
keeps its existing 2 MiB cap. A new processor performs no control-string heap
allocation. Completed unusually large sequences release their high-water
allocation; ordinary OSC/APC/synchronized-update traffic retains only 64 KiB,
8 KiB, and 64 KiB respectively for reuse. Tests verify lazy allocation,
overflow-safe size arithmetic, ordinary-buffer reuse, repeated attack cleanup,
and recovery. The construction Criterion case detects accidental restoration
of eager per-pane allocation; its result is informational until the controlled
30-day baseline is ratified.

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

To retain native screenshots for human visual review, set the report path
before running the canonical command:

    $env:AUTOMEXIA_NATIVE_RESOURCE_REPORT =
        Join-Path $PWD 'artifacts\native-gui\wgpu.json'
    cargo xtask test resize-stress --native-gui

The command writes clean four-pane WGPU and CPU workspace PNGs under the
adjacent typography-captures directory, plus palette and close-confirmation
PNGs under modal-captures. The report also records each pane's effective font
size, zoom-reset baseline, scaled size, and line height, and rejects blank
or low-detail frames. Retained screenshots temporarily enter per-monitor DPI
awareness so 125%-225% Windows scaling cannot crop the visual evidence. These
artifacts are local evidence and must not be
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

### Embedded `librio` input boundary

The private Rust/C embedding boundary has platform-independent tests for SGR
and legacy mouse press/release encoding and null-pointer rejection. Native Unix
tests additionally enable DEC 1000, 1002, 1003, and X10 modes through real VT
input, then verify application ownership, motion rules, invalid-button
rejection, and Shift-to-local-selection bypass. The curated C smoke source calls
the button and motion symbols so a future ABI build cannot silently omit them.

Focused commands are:

```text
cargo test -p librio --lib --locked mouse
cargo test -p rio-vt --lib --locked reclaim_purges_interning_lookup
bash tools/ci/test_librio_c_api.sh
python tools/ci/test_platform_coverage.py
```

The last command also locks the macOS weak-framework linker contract alongside
the native platform and package matrix.

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
Renderer-neutral geometry additionally proves that a visible context tag starts
on at least a 1.22-row rhythm from the preceding row origin, no context produces
no spacing geometry, and tiny or unusually tall rows remain bounded without
overlapping the complete path below. Completion timing shares the same origin.
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

The application shell unit suite also launches a disposable unsigned
integration fixture in real Windows PowerShell subprocesses. RemoteSigned must
load its validated normal local path; AllSigned must remain authoritative while
falling back without stderr or a startup exception. A separate path test
requires safely representable canonical roots to omit the Windows verbatim
prefix, and runtime-trust mutation tests fail if either canonicalization,
literal-path lookup, or the narrow policy-exception fallback is removed. These
tests never change a persistent execution-policy scope or unblock a file.

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
configuration migration, semantic classification, and label sanitization. All
libFuzzer commands install and invoke nightly explicitly. Suitable pure crates
run Miri and ASan/TSan; the sanitizer matrix includes the extension worker's
restart, saturation, recovery, and shutdown paths while excluding Loom's own
scheduler models. Sanitizer jobs install nightly `rust-src` and do not
cancel the second sanitizer when the first fails. Miri selects scalar UTF-8,
base64, parser-transcode, and bounded Kitty temporary-file transport regressions
instead of running the entire VT suite or calling native `simdutf` FFI. The
optimized SIMD path remains enabled in production. The explicit Miri suite has
a 30-minute job timeout; filesystem isolation is disabled only on the ephemeral
hosted runner so the two bounded temporary-file cases can execute. Criterion
cases exist for parser throughput, row rebuild, prompt layout, cache access,
worker submission, cold/warm image quick look, PTY startup/clean exit, and
sustained PTY output/clean exit. Hosted nightly compiles every declared target
under a 45-minute hard job limit; a named self-hosted runner executes and
retains every Criterion result under a 180-minute hard job limit when
AUTOMEXIA_BENCHMARK_RUNNER=1. Run the commands below for local
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

The real PTY lifecycle benchmark is available on Windows, Linux, and macOS:

```text
cargo bench -p teletypewriter --bench pty_io --locked -- --noplot
```

It measures shell process startup through first output and clean exit, then a
1 MiB sustained-output route through clean exit. The adjacent lifecycle test
repeats create, extreme resize, route-specific output, child exit, and drop six
times. These checks catch lost output and lifecycle regressions; process handle,
thread, and memory deltas remain owned by native controlled resource runs.

The renderer-neutral `row_rebuild_full_snapshot` and
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
focus, pane-local tab cycling, global-tab separation, explicit shell
passthroughs, and the four mnemonic app-surface launchers. They exercise stable
configuration names, user overrides, mode suppression, and intentional
compound actions, and reject shortcut collisions, blank palette labels, and
duplicate visible palette labels. Pure layout
tests cover all four directions, uneven/nested grids, perpendicular-beam
preference, deterministic ties, edge stopping, and local-tab wraparound. The
planned compiled-profile suite—including fixture provenance,
origins and shadowing, atomic reload, fallthrough, sequences/tables/chains,
generated docs, fuzzing, and hot-path latency—is specified in the
[full Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md) and must
not be reported as implemented until those gates exist and pass.

The focused feature-shortcut contract is:

```powershell
cargo test -p automexia-terminal palette_shortcuts_are_complete_and_unique --locked
cargo test -p automexia-terminal feature_surface_actions_parse_with_stable_configuration_names --locked
cargo test -p automexia-terminal automexia_windows_defaults_restore_the_classic_workflow --locked
cargo test -p automexia-terminal automexia_unix_defaults_restore_the_classic_workflow --locked
cargo test -p automexia-terminal automexia_macos_defaults_restore_the_classic_workflow --locked
```

These tests prove table construction and pure dispatch contracts on any host;
they do not replace native macOS/Linux keyboard-layout or controlled assistive-
technology runs.

Feature-shortcut completion evidence recorded on native Windows x64 on
2026-08-21:

- `cargo test -p automexia-terminal --bin automexia --locked` passed all 391
  frontend tests.
- `cargo fmt --all -- --check` and warning-denied workspace Clippy passed.
- `cargo nextest run --workspace --locked --profile ci` passed 1,757 tests;
  seven explicitly skipped cases remained skipped.
- `cargo test --workspace --doc --locked` passed 64 documentation tests with
  three explicitly ignored examples.
- `python3 tools/ci/qa.py --full` passed repository, shell, architecture,
  dependency, resize-stress, session-clone, concurrency, workspace, and
  documentation gates. Its local report is under
  `target/qa/20260821T180812Z-19980/report.html` and is not committed.
- `cargo xtask test resize-stress --native-gui` passed on both native WGPU and
  CPU renderers. The inspected 1750x1080 command-palette capture kept the new
  `Ctrl+F4` and `Ctrl+Shift+F4` keycaps aligned and unclipped; WGPU/CPU pixels
  agreed within the harness tolerance.
- `cargo ready` passed its cold isolated all-target check, warning-denied
  Clippy, unit/integration/documentation tests, dependency policy, persistent
  application build, and `automexia 0.4.0` smoke check. The command removed its
  7.76 GiB disposable verification target after success.

Native Linux/BSD and macOS keyboard-layout checks, controlled assistive-
technology runs, elevated Application Verifier/WPR work, and controlled
benchmark campaigns remain external gates; the Windows run does not claim
those results.

Nightly builds unsigned installers for every artifact target. The Windows x64
MSI uses cargo-packager/WiX 3; ARM64 uses the pinned repository-owned WiX 5
source because WiX 3 has no ARM64 MSI support. Linux package jobs install the
exact nFPM version without a semver-incompatible `v` prefix.

CodeQL runs in no-build Rust mode. Public repositories upload SARIF to GitHub
code scanning. When the repository is private without GitHub Code Security,
the same analysis runs with upload disabled and retains its SARIF as a private
14-day workflow artifact for maintainer review instead of failing on an
unavailable entitlement.

Stable release requires WSL, real-GPU, clean-install, upgrade, uninstall,
signature, notarization, URL handler, terminfo, and migration smoke tests on
controlled hardware/self-hosted runners.

### Release trust and antivirus evidence

Release trust has a deterministic PR layer and a credentialed controlled layer.
Run the PR layer with:

```text
python tools/ci/release_trust.py --check-policy
python tools/ci/test_release_trust.py
python tools/ci/check_platform_coverage.py
python tools/ci/test_platform_coverage.py
python tools/ci/check_runtime_trust.py
python tools/ci/test_runtime_trust.py
```

The hostile mutation suites reject missing architectures, unexpected/raw
artifacts, symlinks, empty or oversized packages, checksum tampering, policy
drift, excessive workflow permissions, unsigned downloads, non-isolated signing
inputs, automatic profile provisioning, execution-policy bypass, encoded WSL
shell transport, elevated Windows manifests, unsigned distributed scripts,
stale or extra portable resources, semantically empty SBOMs,
release-version/publisher/byte-evidence mismatches, missing Defender evidence,
missing immutable-release/reproducibility dependencies, missing attestations,
and unsafe macOS deep-signing.
The manifest implementation hashes with a fixed-size buffer, writes atomically,
and records digest throughput in `release-trust-benchmark.json`; that benchmark
measures release IO only and adds no application runtime overhead.

The controlled Windows gate requires signed final artifacts and the exact
publisher, extracts portable ZIPs under traversal/expansion/entry-count limits,
requires the exact five root files plus bounded integration tree and embedded
release version, checks all MSI/executable signatures plus eight signed
PowerShell assets per ZIP, verifies current Defender
state/intelligence, and runs `MpCmdRun` without remediation under a hard timeout.
Its redacted evidence includes engine/intelligence versions, signature count,
package names/sizes/SHA-256 digests, and scan time. Final publication independently
binds that evidence to the exact Windows packages, version, and configured
publisher. Controlled GUI/PTY and WSL smoke use the final packaged Windows and
Linux portable archives. macOS release CI independently proves
hardened runtime, absence of `get-task-allow`, accepted notarization, staple
validation, and Gatekeeper acceptance. A release-only Linux job compares two
fresh cold builds byte for byte and records durations/hash/size; publication
also requires repository immutable releases and refuses pre-existing assets.
These native/external trust decisions cannot be claimed from local unsigned
builds. The complete contract and false-positive response are in
[Release trust](RELEASE-TRUST.md).

## OpenSSH inventory foundation

The D4 package is verified independently of any managed connection feature:

    cargo test -p automexia-devops-ssh --all-targets --locked
    cargo clippy -p automexia-devops-ssh --all-targets -- -D warnings
    cargo bench -p automexia-devops-ssh --bench openssh_inventory -- --noplot

The benchmark is also compiled by the nightly benchmark-build job and executed
with a 7,200-second process ceiling by controlled QA when
AUTOMEXIA_QA_BENCHMARKS=1. Repository validation prevents the benchmark from
being declared without both owners.

PR-native jobs execute immutable-ceiling, bounded-grant, race-resistant
no-follow read, active cancellation, scanner-derived watcher, bounded
serialization, parser, model, refresh, and platform persistence tests
on Windows, Linux, and macOS. Nightly runs openssh_inventory under libFuzzer
with explicit nightly, duration, per-input timeout, and RSS limits. The
10,000-alias Criterion target protects the reviewed maximum-cardinality path.
Architecture verification rejects added process/network authority, evaluator
calls, missing resource ceilings, weakened private permissions, or loss of
fuzz/benchmark ownership. Full threat model, limits, and manual interpretation
are in [OpenSSH inventory](SSH-INVENTORY.md).

## Connection Hub F2 model and golden contract

The F2/D5.0 capability-free baseline is implemented and remains non-activated.
It is reproduced locally with:

    cargo test -p automexia-devops --tests
    cargo test -p automexia-ui-model --tests
    python tools/ci/check_connection_hub_f2.py
    python tools/ci/test_connection_hub_f2.py
    cargo bench -p automexia-devops --bench connection_planning --no-run
    cargo test --manifest-path fuzz/Cargo.toml --no-run
    cargo xtask verify architecture

The DevOps tests cover every top-level strict/versioned record, sealed validated
wrappers, document limits, duplicate/missing/cyclic dependencies, duplicate
review identities, cross-recipe variable collisions, pre-allocation plan-step
ceilings, hostile controls/bidi in documents and plan overrides, secret-bearing
and command-shaped input, option confusion, policy/retry combinations,
redaction, deterministic plan fingerprints, active-operation authentication
correlation, late-generation rejection, canonical event IDs, all authentication/
result states and transitions, exactly 64 steps, order-independent capability/
executable inputs, panic-free sequence conversion, and the all-false process/
network/provider/credential/PTY/listener authority ceiling.

The UI-model tests cover the ten-provider and all-auth fixture matrices, every
Hub empty/loading/failure state, wide/medium/narrow projection at 100-400% text
scale, bounded row virtualization, modal/background-inert/topmost behavior,
keyboard navigation, managed composite focus with stale-selection fallback,
opener focus restoration, route-aware modal tab cycles, reading order, live
loading progress, status text independent of color, high contrast, reduced
motion/transparency, all seven Connection Review sections, blocking decisions,
and value-redacted human dry-run action narration. The structured files under
`tests/fixtures/connection-hub/goldens` are semantic/layout contracts, not a
claim about native pixels or a shipped dialog.

`tools/ci/check_connection_hub_f2.py` freezes schemas, ceilings, provider/state/
layout matrices, 30 required regressions, sealed validation, panic-free planning,
evidence owners, forbidden authority primitives, and disabled execution/modal
invariants. Its mutation suite proves those checks fail when limits, authority,
states, validation construction, panic primitives, required tests,
accessibility, or projected-value privacy invariants drift.
Nightly owns the bounded `connection_planning` fuzz target; controlled QA owns
execution of the 64-step validation/resolution benchmark. The benchmark build
is a PR/nightly ownership check, not a longitudinal performance ratchet.

A local Windows release-profile diagnostic on 2026-08-17 first used ten samples:
validation measured 3.8865-4.5138 us and resolution measured 87.246-89.835 us.
Criterion reported an apparent 10.635% validation regression, but that sample
contained a severe high outlier. The required 50-sample investigation then
measured validation at 3.6726-3.7202 us and resolution at 85.880-87.687 us;
validation improved and resolution remained within noise. Both paths are far
below the 10 ms reviewed ceiling. These local measurements prove the current
bounded implementation, not the controlled named-hardware or 30-day baseline.

The fuzz package and `connection_planning` target compile on this Windows host.
Native campaign execution remains external here: the sanitizer build could not
load `clang_rt.asan_dynamic-x86_64.dll`, while the `--sanitizer none` fallback
failed in the MSVC linker on the libFuzzer sanitizer-coverage start/stop symbols.
Nightly's supported sanitizer runner remains the execution owner; this local
host limitation is not recorded as a successful fuzz campaign.

No F2 test requires an account, network, process, filesystem persistence, PTY,
window server, or GPU. Resource-lifetime and storage evidence are not applicable
because F2 is synchronous pure modeling with bounded owned collections and no
resource/persistence owner. This does not waive those gates for D5.1/D5.2.

### M3 direct OpenSSH review contract

The M3 preparation and Connection Review slice is application-wired but
deliberately non-activated:

    cargo test -p automexia-devops --test direct_openssh_review --locked
    cargo test -p automexia-ui-model --test direct_openssh_review --locked
    cargo test -p automexia-ui-model --test connection_hub --locked
    cargo test -p automexia-terminal --lib automexia::connections --locked
    cargo test -p automexia-terminal --bin automexia renderer::connection_hub::tests --locked
    cargo clippy -p automexia-devops -p automexia-ui-model -p automexia-terminal --all-targets --all-features --locked -- -D warnings
    cargo bench -p automexia-devops --bench connection_planning --locked -- direct_openssh_prepare_selected --warm-up-time 1 --measurement-time 2 --sample-size 20
    cargo xtask verify architecture

Eleven devops-model tests cover inventory-typed concrete aliases, exact
single-argument preservation, canonical `ssh`, exact `session.launch`, the
all-false authority ceiling, complete canonical no-recipe F2 planning,
pending preparation without a fabricated executable identity, wrong source/
executable/capability denial, non-applicable trust denial, misleading targets,
option-like/empty/whitespace/control/bidi/oversized/ambiguous hostile values,
M4 user/port/jump deferral, profile-identifier/destination/reference-redacted
debug and errors, observation readiness and
fingerprint invalidation. Two UI-model review tests cover all nine decisions,
disabled action, accessibility names, redaction, and application-preparation
projection. Twelve Hub-model tests include keyboard Back focus restoration.

Eight application connection tests cover stable D4-record mapping, invalid and
unsupported records, current/unknown/stale runtime selection, one bounded
immutable controller-owned preparation, generation invalidation, redacted
diagnostics, Enter-to-review, and Escape-to-results. Six renderer tests cover
the three icon-and-color groups, route-specific chrome, an inert disabled
primary action, pointer Back, truthful route-specific status, readable group
labels at 360x280, and bounded geometry at 1280x720 and 7680x4320 at 200
percent scale.

The contract is synchronous and bounded to one selected record, a 512-byte
destination, and existing F2 collection ceilings. It adds no worker, file,
process, socket, PTY, listener, route, credential, persistence, or GPU resource.
The existing runtime clones the selected public record under its catalog lock;
pure preparation and projection occur after the lock is released. Review state
is discarded when the selection, metadata, catalog generation, or route changes.

On the native Windows development host on 2026-08-22, the focused suites,
strict owner Clippy, architecture verification, repository validation,
`cargo fmt --all -- --check`, full workspace Clippy, the 1,775-test CI-profile
nextest run (7 skipped), workspace documentation tests, full QA, and
`cargo ready` passed. `cargo ready` also passed a clean isolated workspace
check/Clippy/test build, dependency policy, persistent application build, and
version smoke test. The disposable QA evidence report is under
`target/qa/20260822T001824Z-2240/report.html` (UTC timestamp).

The same-host Criterion sample for `direct_openssh_prepare_selected` measured a
95 percent interval of 6.3520-6.6011 microseconds with 2 of 20 outliers; this is
local bounded-preparation evidence, not a stable-hardware release threshold.

No native OpenSSH process, PTY, prompt, host-key exchange, network operation,
renderer pixel capture, screen-reader run, Application Verifier/WPR campaign,
or manual before/after `ssh` run occurred. Protected ADR approval, real
executable identity/attestation, the atomic native check-to-spawn adapter,
process/PTY/route ownership, cancellation and descendant teardown, PID-reuse
coverage, Linux/macOS native execution, and controlled accessibility/GPU/
long-session evidence remain M2/M3 activation or release gates.
### Connection Hub F3 catalog contract

The first D5.1 slice adds a pure catalog contract without activating D4 or any
process, network, authentication, PTY, listener, or provider authority:

    cargo test -p automexia-ui-model --locked
    cargo bench -p automexia-terminal --bench connection_catalog --no-default-features --no-run --locked
    cargo bench -p automexia-terminal --bench connection_catalog --no-default-features --locked -- --sample-size 20 --measurement-time 3

The catalog tests cover deterministic combined text/favorite/recent/tag/source
filtering, source-revision preservation, contiguous bounded grouping, truthful
empty versus filtered-empty states, hostile controls and bidirectional format
characters, query/tag/record/memory ceilings, 10,000-record projection, 300%
text scale, 8K layout, 32-row virtualization, one managed selected row, and the
all-disabled execution/PTY boundary. On this Windows host on 2026-08-17, the
release-profile rapid-filter benchmark measured 7.0513-7.4408 ms for complete
10,000-record filter/group projections, below the 16 ms reviewed target. This
is local implementation evidence, not the controlled multi-platform baseline.

A fresh M0 rerun on 2026-08-21 at revision
`074bb49e6c93bac231a8acb408e99992f557c25d` plus the documentation-only diff
measured 8.1839-8.8694 ms (8.4996 ms estimate) with 20 samples and a three-second
measurement window. The Windows 11 `10.0.26200` host used an AMD Ryzen 5 5600H
(6 cores/12 logical processors) and 27.9 GiB RAM. The result remains below the
16 ms target; it is a same-host spot measurement, not longitudinal enforcement.

The catalog benchmark does not by itself prove persistence, rendering, or
native accessibility. Application composition and private persistence have
their own contracts below; native product rendering and platform/screen-reader
evidence remain D5.1 gates.

### Connection Hub F3 transactional OpenSSH metadata

The D4 metadata owner now supplies the persistence semantics required by the
read-only Hub's favorites, tags, and recent-use fields:

    cargo test -p automexia-devops-ssh --locked
    cargo test -p automexia-devops-ssh --test metadata_store --locked
    cargo clippy -p automexia-devops-ssh --all-targets --locked -- -D warnings

The focused contract covers legacy revision-zero documents, CAS increments,
stale-writer rejection, one validated previous generation, malformed-primary
fallback with an explicit recovery origin, reviewed recovery with a new
revision, unnecessary-recovery refusal, process-local writer contention, and
Unix no-follow recovery paths. The full D4 suite retains bounded serialization,
atomic replacement, interrupted staging, redacted malformed/oversized input,
exact removal, Unix mode, and Windows current-user DACL coverage. Application
composition and the private profile/recipe/preference library are separate F3
contracts below; the product editor and rendered metadata controls remain open.

### Connection Hub F3 application composition

The native application owns explicit D4 grants, generation-scoped background
refresh, public snapshot/metadata composition, last-known-good health, and
static platform setup guidance without activating connection authority:

    cargo test -p automexia-terminal --test connection_hub_runtime --no-default-features --locked
    cargo clippy -p automexia-terminal --test connection_hub_runtime --no-default-features --locked -- -D warnings

The focused contract proves that opening the runtime does not scan; only exact
reviewed grants start work; metadata and OpenSSH records produce a bounded
public catalog; newer generations supersede older work; failures retain the
last good catalog with a path-free diagnostic code; metadata writes use reviewed
revision CAS; and explicit shutdown cancels and joins the single worker. The
private profile/recipe/preference library and product adapter are covered below.

### Connection Hub M1 read-only product activation

The application Router owns the service, each Screen owns one route-local
controller, and Sugarloaf owns the topmost modal. The native picker is invoked
only by an explicit product action and passes selected files to the review/
worker boundary; it never becomes an ambient scanner or persistent grant.

    cargo test -p automexia-terminal --test connection_hub_runtime --locked
    cargo test -p automexia-terminal --test connection_hub_controller --locked
    cargo test -p automexia-terminal --bin automexia connection_hub --locked
    cargo test -p automexia-terminal --bin automexia command_palette --locked
    cargo test -p automexia-devops-ssh --locked
    cargo test -p automexia-ui-model --locked
    cargo bench -p automexia-terminal --bench connection_catalog --locked -- --noplot
    cargo build -p automexia-terminal --bin automexia --features native-gui-test-hooks --locked
    cargo deny check
    cargo build -p automexia-terminal --release --locked

Windows 11 results on 2026-08-21: 10 runtime, 8 controller integration, 3
targeted worker/controller/cache unit, 5 renderer, 46 palette, 33 D4, 33
UI-model, and 5 library tests passed. The contracts cover
no-scan-on-open, memory-only grant revocation, review tokens, stale generation
rejection, publish-before-wake, explicit joined shutdown, last-known-good state,
metadata CAS success/conflict/reload, read-only recent, hostile tags/search,
keyboard/pointer/IME, focus restoration, inert modal stacking, distinct filter
pointer actions, and bounded tiny/normal/ultrawide/8K geometry. Connect, Login,
provider refresh, recipe execution, process, network, authentication, listener,
and PTY authority remain disabled.

`cargo deny check` passed advisories, bans, licenses, and sources. `rfd` 0.17.2
is the only new direct dependency and `pollster` is its only new transitive
package. The release 10,000-record projection measured 7.1790–7.7931 ms versus
the below-16-ms target. The earlier same-host range was 7.0513–7.4408 ms; this
single run is not treated as a statistical regression comparison. The release
executable is 22,670,336 bytes, 650,752 bytes (2.96%) above the 22,019,584-byte
same-host pre-M1 baseline.

For the current visual review, the feature-gated native test control first
waited for the renderer-neutral prompt-active signal. Only then did it open
Connection Hub, avoiding the startup race caused by sending a shortcut before
the terminal was ready. A direct native-window capture at 1600x950 physical
pixels and 125% scale was inspected on Windows 11. The complete frame showed a
centered 760x480-logical setup surface, one clear primary action, complete
bounds, distinct hierarchy, restrained semantic color, code-native icons with
redundant text, and no setup-only search/filter toolbar or verbose disabled-
action footer. The hook is excluded from normal builds.

Renderer-neutral tests remain the evidence for tiny-to-8K responsive geometry.

Native macOS/Linux picker and static permission/recovery runs plus controlled
Narrator/NVDA, VoiceOver, and Orca verification remain external. Local semantic,
geometry, and limited Windows frame evidence do not substitute for those runs.

### Connection Hub F3 private profile, recipe, and preference library

The application crate owns a private, versioned Connection Library store for
saved connection profiles, automation recipes, and user preferences. M1
initializes it once and exposes only a non-executing snapshot/recovery state:

    cargo test -p automexia-terminal --lib automexia::connections::library::tests::injected_read_only_and_disk_full_fail_before_replacing_primary --no-default-features --locked
    cargo test -p automexia-terminal --test connection_library --no-default-features --locked
    cargo clippy -p automexia-terminal --lib --test connection_library --no-default-features --locked -- -D warnings

The 16 MiB document boundary validates every imported or loaded value before
publication. Persistence uses an expected-revision compare-and-swap contract,
same-directory staging and atomic replacement, one validated previous
generation, and explicit reviewed recovery. No-follow file opening rejects
link substitution where the platform adapter supports it. Redacted transfer
omits opaque credential references, last-used timestamps, and private local
state; import generates fresh identifiers. Deterministic tests cover hostile
controls, oversized and unknown data, stale writers, concurrent contention,
interrupted and corrupt writes, recovery, canary preservation, and injected
read-only/disk-full failures before primary replacement.

On 2026-08-21, at source revision
`074bb49e6c93bac231a8acb408e99992f557c25d` plus the documentation-only M0
reconciliation, Windows 11 `10.0.26200` on an AMD Ryzen 5 5600H recorded one
focused failure-path unit test passing and all five Windows-applicable
integration tests passing. The Unix symlink substitution case was compiled but
not executed on Windows; native Linux/macOS runs remain external evidence.

#### M0 local reconciliation evidence

The documentation-only reconciliation used these final local gates:

    cargo clippy -p automexia-terminal --lib --test connection_library --no-default-features --locked -- -D warnings
    python tools/ci/check_feature_assurance.py
    python tools/ci/check_documentation_coverage.py
    python tools/ci/check_platform_coverage.py
    python tools/ci/validate_repository.py
    cargo fmt --all --check
    cargo ready
    git diff --check

Focused Clippy, all four Python policy/coverage validators, Rustfmt, and the diff
check passed. The follow-up completion audit also ran 105 application library
tests and all five Windows-applicable Connection Library integration tests with
no failures. Repository validation counted 41 TOML, 14 YAML, 25 JSON, 6 XML,
one desktop file, 199 Markdown files, and 24 assurance entries.

`cargo ready` passed its repository validation, PowerShell integration,
licensing/provenance, architecture/trust, formatting, isolated all-target
workspace check, warnings-as-errors workspace Clippy, full workspace unit/
integration/documentation tests, `cargo deny`, persistent debug application
build, and `automexia 0.4.0` executable smoke check. The isolated 7.75 GiB
verification target was removed after success. Optional packaging tools that
were reported missing are not required by this local contributor gate; hosted
native/release evidence remains separate.

### Remaining Connection Hub activation assurance

D5.1 behavior is fully implemented locally. Its remaining release evidence is
native macOS/Linux picker and permission/recovery coverage plus controlled
Narrator/NVDA, VoiceOver, and Orca verification. D5.2 managed launch and D6
provider work remain non-activated and must add their own deterministic,
native, controlled-provider, security, performance, privacy, persistence, and
resource evidence before authority is enabled.

At minimum, D5.2 must prove exact capability approval/revocation, exact Windows/
Unix launch arguments, host-trust behavior, session isolation, and bounded
1/10/50 process/PTY/tunnel teardown. D6 must separately prove provider expiry,
MFA, cancellation, offline, denial, cache isolation, and external credential
custody for each provider. Synthetic fixtures remain mandatory but never
substitute for controlled native evidence.
## CP2.0-CP2.2 Quick Action assurance

The capability-free schema/parser/validator, activation index, and CP3.0
projection compiler remain in the exact five-file `automexia-devops/src/actions`
boundary. CP2.1/CP2.2 use exactly
eight reviewed app sources under `automexia::quick_actions` for bounded private no-follow storage,
atomic primary/one-previous recovery, nonblocking cross-process lock/CAS,
immutable fingerprinted last-known-good snapshots, exact watch filtering,
bounded coalescing/periodic reconciliation, CRUD, and redacted errors. The
checker rejects process, network, environment discovery, clipboard, provider,
shell-profile, UI, VT, PTY, async-runtime, execution, and unsafe code outside the
reviewed platform permission adapter.

CP2.2 starts one joined application worker and provides bounded layered search,
per-route latest-query coalescing with a 32-route ceiling and fair multi-pane
publication, activation-time model revalidation, responsive
placeholder/risk/conflict/health review, explicit empty/error states, dry-run administration and
digest-checked transfer, plus explicit copy or bracketed-paste insertion without
Enter. Secret and exact operations are rejected before placeholder input. CP2.2
itself adds no alias projection, provider work, trusted-workspace activation,
secret expansion, or exact execution. Native-host and controlled accessibility
evidence remains as described in [DevOps Quick Actions and persistent
aliases](DEVOPS-ALIASES.md#verification-plan).

Run the focused CP2.0-CP2.2 gate with:

```powershell
cargo test -p automexia-devops --all-targets --locked
cargo test -p automexia-terminal --lib quick_actions --locked
cargo test -p automexia-terminal --test quick_action_persistence --locked
cargo test -p automexia-terminal --test quick_action_transfer --locked
cargo test -p automexia-ui-model quick_actions --locked
cargo clippy -p automexia-terminal --lib --locked -- -D warnings
cargo bench -p automexia-devops --bench quick_actions --locked -- --noplot
cargo bench -p automexia-terminal --bench quick_action_store --locked -- --noplot
python tools/ci/check_devops_alias_spec.py
python tools/ci/test_devops_alias_spec.py
python tools/ci/check_command_productivity.py
python tools/ci/test_command_productivity.py
python tools/ci/check_command_productivity_cp22.py
python tools/ci/test_command_productivity_cp22.py
```

The active schema-1 CP2.2 contract now names 13 mandatory evidence categories,
including multi-route fairness/capacity, visible health/empty state,
activation-time revalidation, and pre-prompt secret denial. The
`automexia-devops` target owns both `quick_action_search_1024` and
`quick_action_expand_and_quote`; controlled QA executes the same target so these
measurements cannot become orphaned compile-only benchmarks.

The focused Windows evidence is 26 unit cases and four public integration /
property cases, including a protected single-entry DACL assertion, concurrent
writers, rollback/tamper/corruption,
Unicode/spaced paths, 1,000 read-handle cycles, 64 writes, 24 watcher lifecycles,
1,000 coalesced events, redaction, and periodic cross-window reconciliation.
A native Ubuntu 24.04/WSL run passes 27 unit cases, the same four public tests,
and warnings-denied Linux Clippy, including Unix modes, links, sync, and inotify.
Hosted native Windows/Linux/macOS CI plus the named-hardware 30-day benchmark
remain mandatory before cross-platform or release-performance claims.

### CP3.0 pure projection compiler

CP3.0 compiles only validated GlobalUser/ShellUser aliases into deterministic
in-memory PowerShell, Bash, Zsh, Fish, or CMD artifacts. Complete bounded
collision/completion/tool observations are mandatory. Native definitions win
unless matching User consent names the exact owner fingerprint. A claimed
same-action owner must also match the compiler's deterministic fingerprint;
another or forged Automexia owner cannot be replaced. The compiler recomputes
canonical source identity, retains degraded tool health in decisions, and
verifies source, tool, owner, previous-artifact, structured-manifest, and body
identities while always recording activation disabled. No compiler or test path
reads/writes a profile or executes a provider.

Run its focused gate with:

```powershell
cargo test -p automexia-devops --test quick_action_projection --locked
cargo bench -p automexia-devops --bench quick_actions --no-run --locked
cargo check --manifest-path fuzz/Cargo.toml --bin quick_action_projection
python tools/ci/check_command_productivity_cp30.py
python tools/ci/test_command_productivity_cp30.py
```

The thirteen Rust cases cover five serializers, typed/fixed/forwarded arguments,
unsafe eligibility, CMD's nine required typed positions, collision ownership and
fingerprinted consent, source identity mismatch, truthful completion/tool health,
bounded complete inventories, disabled-completion collisions, inventory-order
determinism, structured/text tamper verification, hostile quoting, 256
deterministic bindings, and available native shell parsing, exact capture, and
wrapper exit status. On Windows the native path exercises PowerShell exact argv/
exit status and CMD macro-file loading; Unix hosts syntax-check and capture
arguments/exit status with each installed Bash/Zsh/Fish. The nightly libFuzzer
target generates bounded valid actions across all five shells, argument policies,
completion/tool degradation, collisions, and tampering. The controlled 30-day
benchmark measured 2.7919-2.9846 ms locally for 256 Bash bindings after the
integrity hardening. Treat that result as diagnostic only; the named-hardware
30-day baseline and hosted all-platform results remain release evidence.

The gate must exercise PowerShell 5.1/7+, Bash, Zsh, Fish, CMD, WSL, Windows,
Linux, and macOS while proving that native definitions win, generated files are
fully removable, no alias silently executes provider/network/secret work, and
new shells restore enabled aliases without duplicating profile hooks. Synthetic
serializer fixtures are required for every PR; controlled native shells remain
mandatory before a release claim. CP2/CP3 cannot be marked complete merely
because a generated file parses or an alias works in one interactive shell.

### CP3.1 persistent alias publication and activation

CP3.1 adds two application-owned sources to the eight-file CP2 persistence
boundary. It publishes all five shell artifacts as one private immutable
content-addressed generation, verifies SHA-256 plus exact source/shell/compiler
identity, journals source-CAS/pointer-last changes, retains and verifies one
rollback generation, and recovers a crash to all-old or all-new. Verification
rejects unexpected topology and checks artifact permissions even when hashes
match. Read-only commands never create, lock, repair, or clean live state.
Startup executes no action/provider/network operation and does not rewrite
canonical source. Native definitions win; exact consent is accepted only for
still-observed authenticated owner identity.

Run the focused contract, mutation, Rust, CLI, and benchmark-build gates with:

```powershell
python tools/ci/check_command_productivity_cp31.py
python tools/ci/test_command_productivity_cp31.py
python tools/ci/check_devops_alias_spec.py
python tools/ci/test_devops_alias_spec.py
cargo test -p automexia-terminal --lib quick_actions::aliases::tests --locked -- --nocapture
cargo test -p automexia-terminal --lib cli::tests --locked -- --nocapture
cargo test -p automexia-terminal --lib aliases_cli::tests --locked -- --nocapture
cargo clippy -p automexia-devops -p automexia-terminal --all-targets --locked -- -D warnings
cargo bench -p automexia-terminal --bench quick_action_store --no-run --locked
```

Run native activation, reload, collision, tamper, disabled-state, startup p95,
and exact uninstall preservation/refusal with:

```powershell
pwsh -NoLogo -NoProfile -File tools/ci/test_shell_integration.ps1
bash tools/ci/test_shell_integration.sh
bash tools/ci/test_shell_sources.sh
```

Linux/macOS CI installs and executes the Bash/Zsh/Fish aggregate; Windows CI
executes PowerShell and CMD. Configured nightly and release WSL runners now run
the same full aggregate, not only session-clone tests. The fixtures prove one
exact-compiler verified generation activates in a fresh shell, a self-consistent
wrong-compiler generation and tampering fail closed while last-known-good stays
active, late native owners win, disable affects only the pointer, and uninstall
removes only validated generated aliases while preserving `actions/actions.toml`.
PowerShell/Bash/Zsh/Fish reload removes only unchanged Automexia-owned definitions;
CMD truthfully requires a new session. Bash and Zsh take twenty-five reloads
(five warmups and twenty measured samples); Fish takes five warmups followed by
twenty shell-native timing samples of three reloads each. Every adapter enforces
the provisional 50 ms p95 locally. Fish batches fixed-path metadata and digest
verification into one bounded constant helper invocation, with no provider,
network, action execution, or per-alias subprocess. This diagnostic budget is
not the controlled named-hardware 30-day release baseline. A local Windows
release-profile run on 2026-08-17 measured the 256-binding five-shell compile at
24.228-26.100 ms, durable publish-and-verify at 35.476-37.391 ms, and read-only
doctor at 8.580-9.228 ms. Publish and doctor intentionally authenticate every
active/rollback artifact ACL and exact directory entry; publication also validates
the rollback pointer both before mutation and during cleanup. These values are
local diagnostic evidence, not the pending named-hardware baseline.

Twenty-one owned alias-store regressions span common, three Windows-native, and two
Unix-permission/link cases (19 run on Windows and 18 on Unix), with three focused
CLI detail regressions and seven CLI parser cases. A hostile-manifest property,
cross-process lock contention, the schema-1 CP3.1 contract, eight mutations,
workflow evidence checks, and the aggregate CP2-CP3.3 checker prevent publication-
order, capability, security, UX, lifecycle, and documentation drift. Local WSL
also passes the Unix unsafe-artifact-permission case. Published hosted native
Windows/Linux/macOS/WSL results and the 30-day resource baseline remain release
evidence. CP3.2 static actions remain disabled and unaliased. CP3.3 native
imports and trusted task bridges are fully implemented locally, insert-only,
unaliasable, explicitly selected/trusted, and separately revocable.
## Command-productivity CP0 contract

CP0 is a non-runtime policy boundary. It does not enable managed completion,
aliases, or Quick Actions. Run the focused contract and mutation suite with:

    python tools/ci/check_command_productivity.py
    python tools/ci/test_command_productivity.py
    cargo xtask verify architecture

The checker validates the accepted five-shell discovery and ownership matrix,
eleven-provider matrix, deterministic precedence and fallback, fourteen exact
resource ceilings, eleven conflict fixtures, sixteen threats, and seven trust
boundaries. Canonical fingerprints and exact nested schemas make every accepted
field review-visible. Bounded no-symlink scanning covers shell startup and all
runtime workspace crates to reject premature provider/runtime activation or
terminal-grid command inference. Twenty-two policy tests, including a versioned
eleven-case hostile corpus, prove that weakened contracts, threat controls,
activation status, source boundaries, and CI wiring fail closed.

The machine-readable sources are
[the compatibility fixture](../tests/fixtures/command-productivity/cp0-contract-v1.json),
[the threat fixture](../tests/fixtures/command-productivity/cp0-threats-v1.json),
and [the hostile corpus](../tests/fixtures/command-productivity/cp0-hostile-mutations-v1.json).
Human interpretation belongs in the
[compatibility baseline](COMMAND-PRODUCTIVITY-COMPATIBILITY.md) and
[threat model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md). The following CP1 gate
supplies the separately reviewed runtime and native-shell evidence.

## Command-productivity CP1 native completion

CP1 activates only shell-native completion adapters and explicit local provider
generation. Run its deterministic policy, lifecycle, and shell contracts with:

```text
python tools/ci/check_command_productivity_cp1.py
python tools/ci/test_command_productivity_cp1.py
cargo test -p xtask --locked
powershell -NoProfile -File tools/ci/test_powershell.ps1
bash tools/ci/test_shell_sources.sh
```

The Rust suite covers provider/shell policy, strict argument parsing, fixed
artifact names, invalid UTF-8/NUL/C0/bidirectional output, sanitized diagnostics,
stdout/stderr bounds, timeout kill, successful capture, cross-platform
descendant/process-tree termination, linked destinations and managed directories,
early leader exit with inherited pipes, provider executable replacement,
destination preflight, bounded doctor validation, absolute/local platform roots,
atomic replacement, two-digest interruption recovery, ambient-secret environment
isolation, stable SHA-256, and immutable limits. Shell suites cover native
definition precedence, steady-state and transitional digest verification, tamper
fallback, disable behavior, repeated sourcing, prompt/editor ownership,
install/no-op/repair/uninstall, Unicode surrounding profile content, stale
owned-block repair, simulated canonical macOS installation, relative/overlong
root rejection, Windows UNC rejection, parent-link/reparse substitution,
pre-mutation validation, exact owned-file removal, and 20-sample post-warmup
adapter p95 enforcement against the 50 ms
registration budget. Because non-interactive Fish does not update
`CMD_DURATION` per sourced command, a bounded monotonic harness measures 20
exact baseline/source process pairs after five warmups and computes p95 from
paired registration overhead. Linux/macOS CI
installs Bash/Zsh/Fish validators; Windows CI runs PowerShell/CMD integration.

Windows source contracts also prohibit module-dependent hashing on installer
integrity checks and the PowerShell completion startup path; both use the
platform SHA-256 API directly.

Windows profile-path coverage classifies the complete Microsoft Cloud Files tag
family separately from name-surrogate tags, accepts a real OneDrive-backed
profile directory when the host exposes one, exercises isolated profile
install/no-op/repair/uninstall, and proves that a parent junction is rejected
before the profile target is created or modified.
The Windows host additionally validates a real user-distribution stdin smoke
when WSL is available; source contracts require a bounded canonical Base64
payload, redirected input/output/error, a fixed decoder command, and no payload
bytes in process arguments.

For a controlled real-generator smoke, use an isolated config root and one
installed CLI, then inspect and remove the fixed artifact:

```bash
AUTOMEXIA_CONFIG_HOME="$(mktemp -d)" cargo xtask completion refresh --provider kubernetes --shell bash
cargo xtask completion doctor
cargo xtask completion remove --provider kubernetes --shell bash
```

`doctor` must not start providers. A refresh must finish within 750 ms or fail
closed, retain no child process, and never run on startup/typing/render paths.
Doctor reports an artifact healthy only when its bounded regular artifact,
digest, metadata, provenance header, PowerShell consent marker, and managed
directory chain all agree; missing and invalid states are separate.
The authoritative runtime fixture is
[`cp1-contract-v1.json`](../tests/fixtures/command-productivity/cp1-contract-v1.json).
Native hosted macOS and clean-runner evidence remains required before a stable
release; local WSL success is not represented as macOS evidence.

## Command-productivity CP5.0 research

CP5.0 has a deterministic, non-activating evidence gate:

    python tools/ci/check_command_productivity_cp50.py
    python tools/ci/test_command_productivity_cp50.py
    cargo fmt --manifest-path tools/research/cp5-matcher-benchmark/Cargo.toml -- --check
    cargo clippy --manifest-path tools/research/cp5-matcher-benchmark/Cargo.toml --all-targets --locked -- -D warnings
    cargo test --manifest-path tools/research/cp5-matcher-benchmark/Cargo.toml --locked
    cargo run --release --manifest-path tools/research/cp5-matcher-benchmark/Cargo.toml --locked --quiet

The policy tests fix false runtime capabilities, seven shell-family decisions,
UTF-8 byte-span validation, stale-generation rejection, count/byte ceilings,
control-character rejection, replacement-only insertion, dependency isolation,
and roadmap/documentation wiring. The locked standalone benchmark covers
32/128/512 candidate corpora, Unicode and combining text, long prefixes, and
cancellation after a deterministic generation change. It is not a root
workspace or release dependency.

The local Windows report includes PowerShell 7/5.1, CMD, Bash/WSL startup,
three repeated matcher runs with 20 warmups and 200 samples, compile/binary/
startup cost, and the first cold WSL timeout. Native Zsh/Fish, Linux/macOS
interaction, controlled screen readers, and low-end hardware replication are
reported as external, not passing. See
[CP5.0 native autocomplete research](research/CP5-AUTOCOMPLETE-RESEARCH.md).

## Planned CP5.1-CP5.6 Shell Completion and Suggestions gate

CP5.1-CP5.6 have no runtime test command because implementation is forbidden
until a separate bridge ADR, threat amendment, compatibility version, and
machine contract are accepted. When activated, PR and nightly ownership cover:

| Layer | PR evidence | Nightly/release evidence |
|---|---|---|
| Pure model/ranking | Deterministic ordering, precedence, limits, Unicode/grapheme, hostile labels, replacement-span properties, stale-generation rejection | Criterion 32/128/512-candidate and low-end reference baselines; dependency-size/startup comparison |
| Bridge protocol | Frame/schema/route/capability/generation validation, malformed/oversize/replay/cross-pane/downgrade rejection, memory-only redaction | Native Windows named-pipe and Unix-socket churn, crash/restart/sleep/resume, ACL/mode and handle/socket leak campaigns |
| Shell adapters | Supported PowerShell/PSReadLine, Bash/Readline, Zsh/ZLE, Fish and CMD fallback fixtures; spaces, quotes, selections, multiline and no-Enter insertion | Native Windows/WSL/Linux/macOS shell/version matrix, unsupported/downgrade, profile preservation and exact uninstall |
| Source broker | Explicit source opt-in, shell-owned history, cwd/executable, frequency-ID, cached-public-provider and action precedence; no-network/no-secret negatives | Rapid typing/cancellation, slow filesystem, stale cache, worker loss, offline and multi-pane/session storms |
| Pane UI | Renderer-neutral placement, cursor/IME/footer/tab/sibling/modal avoidance, tiny-to-8K and 100–300% scale goldens, focus/dismissal | Native GPU screenshots, high-contrast/reduced-motion, menu/confirmation z-order, long-running resize/reflow storms |
| Accessibility | Listbox/option names, keyboard-only navigation, coalesced announcements, icon-plus-text semantics | Controlled NVDA/Narrator, VoiceOver and Orca sessions before stable activation |
| Resources/security | Bounded allocation/queue/cache/message tests, fuzz/property corpus, dependency policy, no log/crash/telemetry/extension leakage | ASan/TSan/Miri where supported, sustained fuzz, process/task/pipe/socket/file/GPU/storage leak and 30-day performance baselines |

After each bridge operation, tests assert the shell/editor buffer, cursor,
selection, quoting mode, generation, and history remain authoritative; the
terminal grid is never the source. Accepting a candidate must revalidate the
route/generation/replacement span, insert exactly once using shell-native
escaping, and never send Enter. Dismissal, typing, focus change, modal opening,
resize, pane/tab close, clone, reconnect, and shutdown must cancel obsolete work
and leave no popup, private payload, task, endpoint, process, or cache entry.

The initial performance gates are warm local display <= 50 ms p95, popup update
<= 8 ms p95, cancellation <= 50 ms p95, local-source deadline <= 250 ms with
native/stale fallback, process-wide completion cache <= 8 MiB, and exactly zero
provider/network/authentication work during startup and typing. Benchmarks must
publish distribution, machine identity, corpus, cold/warm state, sample count,
and peak memory; a single fast local run is not release evidence.

## External-tool and adopted-dependency assurance

The canonical ownership policy is
[Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md).
Every OpenSSH, provider CLI, Mosh, Git, Upterm, SOPS/age, task-runner, transfer,
policy, collaboration, or AI adapter must use the one core ExternalToolRunner.
Each adapter's protected milestone adds all applicable evidence below.

| Layer | Required deterministic evidence |
|---|---|
| Core runner | Canonical executable identity; exact argv; validated cwd; allowlisted environment; null/protected stdin; byte/line caps; startup/idle/total deadlines; cancellation; descendant termination; redacted events; version state; session/extension/operation ownership |
| Extension adapter | Fake executable contract; supported/unsupported version fixtures; truncated, malformed, oversized, hostile, and Unicode output; bounded parser; typed normalization; no shell concatenation; no direct spawn; no ambient credential/environment; extension-disable fallback |
| Lifecycle/state | Offline, locked, expired, MFA, denied, cancelled, timeout, crash, stale/last-known-good, ambiguous outcome, repeated create/drop, owner close, and cross-pane/window/process isolation |
| Native integration | One controlled real-tool job on every claimed OS; platform-specific agent/keychain/PTY/process-tree behavior; clean absence and unsupported-version behavior; no silent fallback |
| Performance/resources | Cold and warm launch, output parsing, search/cache refresh, cancellation, cleanup, peak memory/handles/threads/files, binary-size delta, queue saturation, and 10/50/100-session evidence where relevant |
| Security/privacy | Secret canaries, hostile executable/path/argument/output, log and bundle redaction, capability denial/revocation, production review, no startup/typing/render/resize invocation, and no retained raw provider output |
| UI/accessibility | Renderer-neutral state, stable item identity, focus restoration, keyboard-only operation, screen-reader roles/names/states/actions, responsive goldens, stale/error/permission presentation, and reduced-motion/high-contrast behavior |

Parsers for external JSON, safe SSH inventory, route graphs, policy input, file
operations, log frames, and collaboration messages receive property and fuzz
coverage. Pure security decisions and state machines receive scoped mutation
testing. cargo-vet is introduced only with a named audit owner, trusted-import
policy, explicit criteria, ratcheted exemptions, and renewal process; it
complements rather than replaces cargo-deny, dependency review, CodeQL, SBOMs,
attestations, or release signing. Diagnostic Nextest retries remain reported
flaky failures.

An adopted runtime crate also needs pinned minimal features, license/source/
advisory/provenance review, an update owner, platform declaration, measured
startup/binary/resource cost, cargo xtask doctor classification, and tests
showing that disabling or uninstalling its feature leaves ordinary terminal
behavior intact. Naming a crate in a roadmap does not satisfy this gate.

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

## CP3.2 reviewed DevOps action packs

CP3.2 is fully done at the local source boundary. Its focused validation is:

```text
cargo test -p automexia-devops --test quick_action_packs --lib
cargo test -p automexia-terminal cli::tests::pack_ --lib
cargo test -p automexia-terminal packs_cli --lib
cargo check --manifest-path fuzz/Cargo.toml --bin quick_action_packs
cargo bench -p automexia-devops --bench quick_actions --no-run
python tools/ci/check_command_productivity_cp32.py
python tools/ci/test_command_productivity_cp32.py
python tools/ci/check_devops_alias_spec.py
python tools/ci/test_devops_alias_spec.py
```

The pack suite covers the exact 11-provider/33-action inventory, HTTPS/version/
sort validation, disabled and unaliased defaults, explicit materialization,
effect/risk alias denial, forged built-in provenance, missing/unsupported/
completion/ready health, hostile version text, overlay preservation, deprecation
replacement, functional update detection, correct version-only unchanged
classification, stale-overlay rejection, and immutable-version advancement.
Registry unit mutations reject enabled defaults, wrong provenance, duplicate
actions, inconsistent completion policies, and malformed HTTPS documentation
URLs. Five CLI parser/rendering/preflight cases prove enable is a dry run, apply
needs revision CAS, stale revisions fail before store creation, exact argv/risk/
effect/documentation are reviewable, and registry doctor never claims provider
readiness.

A short local Windows Criterion run measured full 11-pack/33-action validation at
272.31-283.98 us and all 11 ready health evaluations at 289.48-310.79 us. This
proves the benchmark and current bounds, not the controlled 30-day baseline.
The Criterion targets validate all 11 manifests/33 actions and evaluate all 11
health reports. Nightly fuzzes bounded hostile version observations and both
valid/stale overlay digests. The contract freezes the complete reviewed registry
digest plus 17 named integration/CLI regressions; eight mutation cases freeze
inventory, capability denial, exact preview UX, alias safety, lifecycle, source,
tests, benchmark, fuzz, CI wiring, and eight CP3.2 documents. Hosted
cross-platform and
30-day comparable measurements remain release evidence.
## CP3.3 native imports and trusted workspace task bridges

CP3.3 is fully done at the local source boundary. Run the focused evidence with:

```text
cargo test -p automexia-devops --test quick_action_imports --locked
cargo test -p automexia-terminal --test quick_action_native_import --locked
cargo test -p automexia-terminal --test quick_action_workspace_trust --locked
cargo test -p automexia-terminal trusted_workspace_tasks_are_cached_off_thread_and_revocation_fails_closed --lib --locked
cargo test -p automexia-terminal cp33_native_import_and_workspace_mutations_are_explicit_and_cas_guarded --lib --locked
cargo check --manifest-path fuzz/Cargo.toml --bin quick_action_imports
cargo bench -p automexia-devops --bench quick_actions --no-run --locked
python tools/ci/check_command_productivity_cp33.py
python tools/ci/test_command_productivity_cp33.py
python tools/ci/check_command_productivity.py
python tools/ci/test_command_productivity.py
python tools/ci/check_devops_alias_spec.py
python tools/ci/test_devops_alias_spec.py
```

The 14 named cross-platform regressions comprise five capability-free parser/
task/trust cases, three application import cases, four workspace/trust cases,
one background runtime revocation case, and one CLI parser case. Twelve execute
on Windows; the two additional Unix cases create real linked inventory,
workspace, and trust sources and prove no-follow rejection. Coverage includes
all six native formats, exact just/Task/mise argv, secret/control/bidi/path/
substitution/pipeline/metacharacter rejection, explicit selection, stable rename,
conflict/replace, portable export, source preservation, dry-run/CAS, private
path-free receipts, source-change invalidation, revocation, removal, read-only
no-side-effect lookup, bounded background cache authorization, and final
review/insertion failure UX.

The schema-1 contract freezes six formats, three runners, six commands, 18
security assertions, 12 lifecycle assertions, ten reviewed source files, all
named tests, twelve synchronized documents, the benchmark, and the fuzz target.
Seven mutation tests weaken inventories, security/lifecycle values, parser/
trust/runtime/UI guards, dry-run/CAS dispatch, test evidence, benchmark, fuzz,
duplicate keys, and nightly registration. The aggregate CP2-CP3.3 checkers keep
the pure and application capability allowlists exact.

A short local Windows release-profile Criterion run on 2026-08-17 measured
1,024 Bash alias records at 9.7948-12.291 ms and exact workspace trust
verification at 20.081-21.105 us (10 samples, one-second measurement window).
These measurements prove the target and current bound; they do not replace the
controlled named-hardware 30-day baseline or hosted native/accessibility gates.
