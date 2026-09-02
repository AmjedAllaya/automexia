# Testing

Automexia uses layered evidence for its public free-terminal behavior. This page
does not document or authorize unreleased advanced features or commercial
products.

A test proves only the exact behavior, artifact, platform, and revision it ran.
Cross-compilation is not native runtime evidence, and a retry does not erase the
first failure.

## Fast documentation and policy gate

For documentation-only changes, run the repository's documentation hygiene,
coverage, link, confidentiality, and alignment checks, followed by:

```text
git diff --check
python tools/ci/validate_repository.py
```

Review the complete diff and verify that only documentation files authorized by
the task changed. Scan changed and untracked documentation for credentials,
machine-local paths, hostnames, account identifiers, and private product plans.

## Complete local gate

For changes that affect Rust code or build behavior, start with:

```text
cargo fmt --all --check
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
python tools/ci/validate_repository.py
git diff --check
```

Use the narrower package and test owner first, then expand to the workspace.
The contributor command `cargo ready` is the final local gate when applicable.
Use the complete CI-equivalent command only when the change or policy requires
it.

## Evidence ladder

1. Unit tests for pure parsing, state, policy, layout, and limits.
2. Integration tests for crate, PTY, shell, persistence, and platform boundaries.
3. Property tests for invariants and hostile structured input.
4. Fuzzing for parsers, protocols, archives, configuration, and terminal input.
5. Deterministic concurrency tests for ordering, cancellation, saturation,
   stale generations, restart, and shutdown.
6. Renderer-neutral geometry, hierarchy, focus, semantic, and clipping tests.
7. Exact controlled raster comparisons for visible behavior.
8. Native workflow and accessibility evidence on each claimed platform.
9. Performance, resource, storage-growth, and cleanup measurements.
10. Packaged-artifact, upgrade, rollback, and uninstall verification.

A narrow pass never overrides a known real-workflow failure.

## Native platform ownership

Windows, Linux/BSD, and macOS claims require native runs on the named platform
for PTY/process behavior, windowing, input, clipboard, IME, accessibility,
packaging, and credential integration as applicable. Cross-compilation and
mock adapters are supporting evidence only. Unrun platforms remain explicit
external gates and are never reported as passing.

## Terminal conformance

Terminal tests cover:

- CSI, OSC, DCS, APC, hyperlinks, titles, and alternate-screen behavior;
- UTF-8, Unicode widths, combining marks, emoji, CJK, bidi, and controls;
- zero, boundary, maximum, oversized, fragmented, malformed, and repeated input;
- scrollback, reflow, selection, search, cursor, and viewport state;
- Sixel, Kitty, and iTerm2 image limits and lifecycle; and
- parser recovery after invalid or interrupted sequences.

Independent oracles compare exact bytes, cells, cursor state, modes, visible
rows, semantic events, and final images where appropriate.

## PTY and process lifecycle

PTY tests exercise real supported adapters with exact executable and argument
arrays. They cover launch, ordered input, resize storms, output storms,
interrupt, EOF, exit status, cancellation, close, child trees, and application
shutdown.

Assertions include forbidden side effects: no shell evaluation for structured
actions, no implicit Enter, no cross-session input, no stale publication, no
orphan child, and no leaked handle or worker.

Unix PTY results do not prove ConPTY behavior, and Windows results do not prove
Unix process-group cleanup. Native claims name the operating system and
architecture that actually ran.

## Windows, tabs, panes, and input

Model and integration tests cover independent windows and PTYs, global tabs,
pane-local tabs, fresh and cloned splits, route isolation, focus movement,
divider resize, pointer routing, selection, clipboard, IME, search scope,
command navigation, and modal isolation.

Test every transition with one and multiple panes/tabs, rapid closure, stale
route IDs, shutdown, tiny through high-resolution viewports, and keyboard-only
operation.

## Configuration and persistence

Persistence tests cover:

- canonical round trip and supported predecessor formats;
- missing, corrupt, truncated, duplicate, oversized, and unknown input;
- read-only, permission-denied, disk-full, and interrupted writes;
- temporary-file creation, flush, atomic replacement, and last-known-good
  recovery;
- concurrent readers/writers and compare-and-swap where used;
- private permissions and link/replacement checks;
- migration, rollback, disable, uninstall, and restart; and
- storage-tree and digest comparison before and after cleanup.

Fixtures use fictional stable values or isolated temporary paths. Never snapshot
the real user profile, host, environment, Git identity, or shell history.

## Shell integration

Supported-shell tests verify session-local provisioning, prompt boundaries,
status/duration/path/Git metadata, object-preserving listings, disable/remove
behavior, missing-resource fallback, quoting, Unicode, and startup cleanup.

The native shell remains the independent oracle for command editing, history,
completion, quoting, and pipeline objects.

## Images

Image tests bound input bytes, dimensions, decoded pixels, frame counts, cache
size, and lifetime. Local preview tests cover regular files, links, replacement,
permissions, unsupported formats, malformed data, cancellation, stale results,
and cleanup.

Protocol images are tested through parser state, terminal state, visible
viewport, renderer data, and exact controlled pixels.

## OpenSSH interoperability and inventory

Public SSH tests use an authorized loopback fixture or ordinary manual system
OpenSSH. They verify that terminal input, paste, resize, search, selection, exit,
and cleanup behave like local sessions while OpenSSH keeps credential and
network ownership.

Inventory tests use only explicitly selected synthetic configuration files.
They cover includes, duplicates, invalid syntax, oversized files, links,
replacement, revocation, public metadata, last-known-good recovery, and absence
of credential persistence or passive network work.

## Extension contract runtime

Public extension-maintenance tests verify version negotiation, strict bounded
messages, capability denial, route/session isolation, cancellation, queue
saturation, stale generation rejection, disable, uninstall, worker restart, and
shutdown.

Optional code cannot obtain ambient filesystem, network, credential, process,
terminal-history, clipboard, or PTY authority. Extension failure must not break
the basic terminal.

## Accessibility and visual assurance

Visible changes require three separate layers:

1. renderer-neutral geometry, hierarchy, focus, relationships, clipping,
   z-order, contrast, reduced motion, and semantic assertions;
2. deterministic controlled raster goldens with exact comparison; and
3. native frames plus accessibility tree/event evidence on each claimed
   platform.

Cover small through high-resolution viewports, 100–300% scale,
light/dark/high-contrast themes, long and localized text, Unicode, IME, empty
and error states, modal stacking, keyboard and pointer input, focus restoration,
and motion enabled/disabled.

Automated accessibility checks do not replace current Narrator/NVDA, VoiceOver,
and Orca evidence for releases claiming those environments.

## Performance and resources

Measure the real owning path with same-host baselines and noise-aware
thresholds. Record latency distributions, throughput, allocations, CPU, memory,
GPU memory when observable, handles, threads, child processes, queues, caches,
storage growth, cancellation latency, long-session stability, and final cleanup.

Interactive benchmarks cover input-to-publication latency, PTY throughput,
resize coalescing, parser/state updates, snapshot publication, renderer enqueue,
search, large scrollback, image pressure, multiple panes, and shutdown.

## Fuzzing and property testing

Every parser and hostile structured-input boundary has a maintained corpus and
bounded fuzz target. Record engine, seed, corpus digest, duration, executions,
sanitizer, target, platform, and replayed crashes. A smoke run proves only that
campaign.

Property tests use independent invariants or reference behavior. Historical
failures stay in the seed corpus.

## Checker assurance

Repository checkers are assurance code. Their mutation tests prove that missing
owners, deleted scenarios, weakened limits, reordered lifecycle steps, stale
paths, relaxed thresholds, fabricated evidence, and accidental private-data
publication fail closed.

A checker must validate semantics, not merely file existence or self-reported
counts.

## Packaging and release

Package verification covers identity, version, license notices, checksums, SBOM,
provenance, signatures, install, launch, upgrade, rollback, uninstall, startup
hooks, user-data retention, and final cleanup.

Controlled signing, notarization, store accounts, protected runners, hardware,
native assistive technology, and long-duration campaigns remain explicit
external gates when unavailable locally.

## Manual verification

Use [the public manual feature-testing guide](MANUAL-FEATURE-TESTING.md).
Record exact revision/package, environment, scenarios, first failure, redacted
evidence, cleanup, and reviewer. Do not convert a skipped or unavailable
scenario into a pass.

## Documentation verification

Before handoff:

- load every public Markdown file;
- verify UTF-8, balanced code fences, one clear title, and valid relative links;
- scan for machine-local or confidential values;
- scan for advanced, commercial, pricing, market, and unreleased-plan language;
- verify privately archived originals remain available and ignored;
- confirm public indexes name only public baseline features; and
- inspect the complete documentation diff for accidental loss.

The publication boundary is defined by
[Private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Command Productivity Cp50 Research

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub F2 Model And Golden Contract

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub F3 Application Composition

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub F3 Catalog Contract

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub F3 Private Profile Recipe And Preference Library

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub M1 Read Only Product Activation

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp20 Cp22 Quick Action Assurance

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp30 Pure Projection Compiler

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp31 Persistent Alias Publication And Activation

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp32 Reviewed Devops Action Packs

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp33 Native Imports And Trusted Workspace Task Bridges

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### D7cp6 Accepted Source And Release Gate

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M13 Provider Aware Quick Actions

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M3 Direct Openssh Review Contract

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M4 Ssh Routes And Trust Contract

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M5 Typed Openssh Tunnels And Native Release Evidence

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M6 Typed Automation And Multi Environment Workspaces

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M7 Provider Neutral Authentication And Capsule Isolation

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M8 Aws Adapter Source Contracts

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M9 Azure Adapter Source Contracts

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Nightly And Release Depth

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## Command-productivity CP1

CP1 assurance covers the native completion adapter lifecycle: bounded generated
artifacts, native-definition ownership, shell-specific parsing, provider-free
typing/startup paths, explicit refresh/remove, disabled fallback, hostile
metadata, exact executable arguments, filtered subprocess context, deadline and
output limits, repeated lifecycle cleanup, and native platform evidence.

Focused checks:

```text
python tools/ci/check_command_productivity_cp1.py
python tools/ci/test_command_productivity_cp1.py
python tools/ci/test_measure_completion_adapter.py
```

A passing source check does not replace supported-shell native runs or prove an
unpublished package. Record the exact revision, shell, platform, artifact, first
failure, performance sample, cleanup result, and remaining external gates.

## Verification plan

Command-productivity evidence is phase-owned but cumulative. CP2.2 includes the
`quick_action_search_1024` boundary and verifies search, copy, editor handoff,
and insert-without-Enter without launch. CP3.0 checks deterministic shell
projection; CP3.1 checks transactional opt-in alias persistence; CP3.2 checks
reviewed static packs; and CP3.3 checks capability-separated native imports and
trusted workspace task bridges.

### Command-productivity CP5.0 research

Run `python tools/ci/test_command_productivity_cp50.py` with the CP5.0 checker.
The result proves the reviewed source decision only; native shell, transport,
privacy, performance, resource, accessibility, package, disable, uninstall,
and rollback evidence remain separate gates.

The complete alias scenario inventory is maintained in
[DEVOPS-ALIASES.md#verification-plan](DEVOPS-ALIASES.md#verification-plan).

### Session-launch D0

D0 is governed by ADR 0012. Tests preserve system OpenSSH ownership, exact
argument boundaries, no implicit Enter, route/session isolation, cancellation,
descendant cleanup, and ordinary manual SSH fallback.
