# Modern terminal compatibility and Ghostty migration roadmap

## Scope and status

This is the canonical source of truth for the Ghostty G0-G6 track. Automexia
keeps `automexia` as the implicit profile and provides explicit pinned
`ghostty-1.3` and moving `ghostty` selectors. The local source/runtime contract
is implemented through the accepted local G6 boundary; broader release and
lifecycle claims remain bounded by the status table below.

The stable G0-G6 identifiers remain the machine-readable roadmap keys. Each
item is also classified by intent:

- **TC - terminal capability**: product behavior Automexia owns independently;
- **GM - Ghostty migration**: explicit profile/import compatibility for users
  moving from Ghostty;
- **PS - persistent-session lifecycle**: Automexia-owned parking and restore
  semantics; compatibility mappings do not create lifecycle authority.

Only these status labels are used:

- **Fully done**: the stated local implementation contract and deterministic
  owner tests are complete.
- **Partially done**: useful implementation exists, but a stated behavior,
  native-platform, lifecycle, accessibility, performance, or release gate
  remains.
- **Not done**: no authoritative implementation owner and contract tests exist.

| Phase | Status | Implemented evidence | Remaining exit work |
|---|---|---|---|
| G0 — source lock and safety | **Partially done** | Ghostty 1.3.1 Linux/BSD source/binary/checksum provenance, normalized bindings/actions, deterministic Windows adaptation, classic golden, generator/verifier, properties, and accepted ADR 0026 | Native macOS fixture and native Linux/macOS release smoke |
| G1 — typed registry | **Fully done** | Pure bounded crate, stable schemas, typed triggers/predicates/scopes/origins/policies, allocation-free indexed lookup, reverse lookup, trie/table storage, classic adapter, registry-derived palette | Cross-platform release evidence is tracked under G5 |
| G2 — profiles and reload | **Fully done** | Default/moving/pinned profiles, bind/unbind layers, strict diagnostics, immutable last-known-good publication, transactional global/palette update, safe migration | Native release matrix is tracked under G5 |
| G3 — dispatch language | **Fully done** | Structured outcomes, performable/unconsumed fallthrough, exact sequences and cancellation, bounded tables/catch-all/chains, route isolation, all-surface snapshots | Native IME/layout evidence is tracked under G5 |
| G4 — stateless actions | **Fully done** | Clear, selection/search, topology, inherited independent splits, exact logical resize, transactional zoom/equalize, private bounded export and cleanup | Controlled visual/resource proof is tracked under G5 |
| G5 — tooling and release assurance | **Partially done** | CLI/explain/JSON, dry-run migration, host-independent xtask generation/verification, generated docs, two nightly fuzz targets, properties, Criterion coverage, a mutation-tested repository gate, and strict private native-evidence validation/QA wiring | Controlled Windows/Linux/macOS layout, visual, AT, resource, package, and lifecycle evidence plus an activated comparable 30-day baseline |
| G6 — high-lifecycle features | **Partially done** | Accepted redaction/lifecycle ADRs; modal renderer inspector; redacted parked count/list; newest restore; two-step clear; bounded parked-PTY undo/redo for complete closed top-level tabs | Individual split, pane-local-tab, and native-window closure history plus controlled native lifecycle evidence |

Modern terminal capability, explicit Ghostty migration, and session lifecycle
remain
separate concerns. Exhaustive Ghostty parity is not a v0.4 release
criterion. The [Ghostty keyboard compatibility guide](GHOSTTY-KEYBOARD-COMPATIBILITY.md)
documents usable behavior, safe rollback, and deliberate deviations. Generated
[action](generated/ghostty-1.3-actions.md) and
[keybinding](generated/ghostty-1.3-keybindings.md) references are canonical for
the bundled registry.

## Shared assurance prerequisites

Compatibility reuses the repository's Nextest, doctest, fuzz, snapshot,
Criterion, QA, and release infrastructure. Normal startup/build/test paths are
offline and never execute Ghostty. A compatibility-profile release must add the
remaining native fixture, keyboard-layout, rendered-frame, assistive-technology,
resource-cycle, packaging, and comparable 30-day benchmark evidence through the
strict private evidence-manifest contract. QA publishes only its bounded
redacted summary; synthetic platform tables and a Windows-only run cannot close
those gates.

## Decisions

### Implement

The following capabilities are needed for a maintainable compatibility layer:

1. A reviewed, checked-in baseline fixture containing normalized Ghostty
   actions and default keybindings, with upstream tag/commit, generation tool
   version, platform, and checksum metadata.
2. A private, renderer-independent `automexia-keybindings` crate containing
   typed triggers, predicates, action IDs and schemas, origins, compilation,
   conflict analysis, direct lookup, a sequence trie, and reverse lookup.
3. Versioned `automexia`, `ghostty`, and explicit `ghostty-<version>` profiles.
   `automexia` remains the default; compatibility is never enabled silently.
4. Explicit user override and unbind layers, plus a read-only migration report
   and an explicit apply step for legacy bindings.
5. Atomic reload: parse and compile off the event path, report all diagnostics,
   swap one immutable registry only on success, and retain the last known-good
   registry on failure.
6. Structured action outcomes for handled, unavailable, unconsumed, and error
   states, followed by sequence/table/chain support with pinned Ghostty prefix
   and fallthrough rules.
7. A single action/shortcut registry used by dispatch, the command palette,
   CLI inspection, collision diagnostics, generated docs, and tests.
8. The remaining stateless actions: three clear variants, page/home/end and
   line-boundary selection extension, search/scroll from selection, split
   zoom/equalize, and secure visible-screen
   export. Geometric split focus is already implemented by the Automexia
   profile and will be reused by the compiled registry.
9. CLI and contributor tooling to list/explain actions and effective bindings,
   verify fixtures, detect collisions, preview migration, and regenerate docs.
10. Deterministic, property, fuzz, native-platform, and latency tests for the
    compiled registry and each newly implemented action.

## Target ownership

The compatibility engine belongs in a private, renderer-independent
`automexia-keybindings` crate. It owns action and key IDs, parsing, profile
composition, trigger resolution, compilation, diagnostics, formatting, and
compatibility manifests. Suggested internal modules are `action`, `binding`,
`compiler`, `diagnostics`, `manifest`, `parser`, `profile`, `registry`,
`resolver`, `sequence`, and `table`.

The frontend remains responsible for platform event normalization and concrete
effects. Its input/action adapters own clipboard, configuration, layout,
search, selection, session, export, inspector, and bounded topology-history operations. The binding
crate must not initialize a PTY, window, font system, renderer, or GPU. This
keeps profile compilation deterministic and cheap to test.

### Constrained follow-up

The implemented registry/actions activated only the lifecycle scope accepted by
ADRs 0027 and 0028. Remaining compatibility work is deliberately bounded to:

- a native macOS 1.3.1 fixture and the controlled release-evidence matrix;
- individual split, pane-local-tab, and native-window closure history after
  owner/resource/native proof (complete top-level closed tabs already work);
- a broad user-facing key-table ecosystem only after real-world profile and
  migration evidence is stable; and
- exact claims for a newer Ghostty release only after its fixture is regenerated
  and reviewed rather than copied manually.

### Exclude

Automexia will not:

- install Ghostty's deprecated and ineffective `close_all_windows` default;
- run Ghostty or access the network during normal builds, tests, startup, or
  profile selection;
- silently switch an existing user to a compatibility profile or overwrite
  their configuration during migration;
- claim an official Ghostty Windows profile where upstream provides none;
  Windows must be documented as a deterministic Automexia transform;
- make two panes share one live PTY as an implementation of cloning;
- execute exported terminal contents or infer commands from them;
- expose secrets, clipboard contents, or hidden terminal data in an inspector;
- make complete Ghostty compatibility a blocker for the v0.4 stable release.

An imported deprecated action may be recognized only to produce a precise
diagnostic; it must not become an active default.

## Delivery order

### G0 — source lock and safety prerequisites — Partially done

The Linux/BSD and Windows-adapted fixture/provenance path is implemented; the
native macOS fixture and native release evidence remain.

Implemented locally:

- pinned Ghostty v1.3.1 commit
  `22efb0be2bbea73e5339f5426fa3b20edabcaa11` after source/tag review;
- generated raw Linux/BSD bindings and actions with source, binary, Zig, and
  normalized-output checksums;
- checked-in `linux.json`, `windows-adapted.json`, `actions.json`,
  `provenance.json`, `deviations.json`, `manifest.json`, and the Automexia
  classic Windows golden under `tests/fixtures/keybindings/ghostty/1.3.1/`;
- deterministic Windows transformation and explicit deviations;
- accepted ADR 0026 for profile precedence, versioning, pure-crate ownership,
  and Windows adaptation;
- exact offline regeneration/checks, bounded parser/property coverage, and the
  existing Automexia identity/control-string safety gates.

Remaining:

- generate and review the native macOS fixture on macOS; it is intentionally not
  synthesized from Linux and the selector fails closed until it exists; and
- add the native Linux/macOS fixture smoke and provenance result to the release
  evidence bundle.
Exit gate: fixtures are reproducible and reviewable; invalid reloads do not
change active behavior; hostile control strings cannot grow memory without a
bound; product identity responses are Automexia-owned.

### G1 — typed registry without behavior changes — Fully done

The following registry contract is implemented and covered by owner tests.

- add the private `automexia-keybindings` crate;
- introduce stable action IDs, parameter schemas, aliases, and capability
  metadata while adapting the current frontend `Action` enum;
- model a trigger as one or more atoms covering logical Unicode keys, W3C
  physical keys, named keys, modifiers, and `catch_all`;
- model focused-surface, all-surface, and operating-system-global scopes;
- model search, vi, alternate-screen, application-cursor,
  application-keypad, and Kitty-keyboard mode predicates;
- preserve built-in, profile, Windows-adaptation, user, imported, and legacy
  user origins plus deterministic priority for every binding;
- model `consumed`, `unconsumed`, and `performable` policies without changing
  their runtime behavior until G3;
- compile single-chord bindings into direct lookup and reverse-action indexes;
- reserve a prefix trie and separate registry per active key table;
- adapt the frontend to use the registry while keeping the existing defaults;
- derive command-palette labels from the active registry.

Exit gate: current mappings, user overrides, shell-owned controls, and palette
behavior are byte-for-byte equivalent; collision and lookup tests pass on all
platform tables; dispatch does not regress the recorded latency baseline.

### G2 — profiles, overrides, migration, and atomic reload — Fully done

The following profile, layering, migration, and publication contract is implemented.

- add `keyboard.profile = "automexia" | "ghostty" | "ghostty-1.3"`;
- bind `ghostty-1.3` permanently to the reviewed 1.3.1 fixture and make the
  moving `ghostty` alias resolve only to the newest bundled, verified profile;
- keep existing installations on `automexia`; profile detection may suggest
  compatibility but may never enable it;
- define explicit binding, unbind, and priority layers;
- apply user bindings/unbinds after profile entries and preserve them on reload;
- accept a Ghostty-compatible string syntax while translating legacy
  `[bindings].keys` into entries marked with a legacy-user origin;
- do not inject Automexia cloning or pane-local-tab defaults into strict
  Ghostty mode; keep those actions available through the palette/user config;
- compile configuration into an immutable registry off the event path;
- report all invalid actions, triggers, collisions, and unavailable actions;
- retain the previous registry after any failed reload;
- add read-only legacy migration output and a separate explicit apply command;
- warn clearly when a newer moving alias differs from a pinned profile;
- prepare global-hotkey changes before the swap, update every window's palette
  and profile indicator after it, and unregister obsolete globals only after
  the candidate is valid;
- never recreate PTYs or leave partially changed global registrations during
  reload.

Exit gate: fresh, reloaded, invalid, partially invalid, repeated, and concurrent
configuration cases are deterministic; no failed reload changes active input.

### G3 — consumption, sequences, tables, and chains — Fully done

The following structured dispatch contract is implemented and bounded per route.

- give every action `can_perform` and `execute` contracts;
- add structured `ActionOutcome` values for performed/consumed state, terminal
  damage, layout/window/config changes, and user-facing errors;
- implement unconsumed fallthrough to lower-priority bindings or the PTY;
- preserve the implemented terminal-level `Ctrl+C` rule (copy a non-empty
  selection, otherwise forward ETX) and secondary-click copy/paste rule while
  generalizing all registry-driven copy/paste fallthrough through `ActionOutcome`;
- make Escape end search only while search is active;
- make tab/split navigation fall through at topology boundaries and make split
  zoom/resize fall through when no split target exists;
- guarantee bare `Ctrl+R` and `Ctrl+D` reach the PTY in strict Ghostty mode;
- run all-surface actions against a stable route-ID snapshot and merge chained
  damage into at most one redraw;
- add multi-key sequences matching the pinned Ghostty contract: prefixes wait
  indefinitely until completion, cancellation, or an invalid continuation;
- retain the exact original encoded bytes for every pending prefix;
- flush invalid sequences in original order; `end_key_sequence` flushes only
  the preceding prefix; performable failure resets and forwards buffered input;
- apply precedence exactly: a later direct prefix removes an older sequence
  branch, while a later sequence replaces an older direct prefix action;
- add named table activation/deactivation and a bounded table stack;
- resolve tables from innermost to outermost and then the default table;
- deactivate one-shot tables after a non-`catch_all` action;
- run `catch_all` only when a table has no explicit entry;
- disallow all-surface/global sequences;
- add ordered action chains with explicit failure semantics;
- ensure IME, AltGr, dead keys, Kitty keyboard protocol, and shell input remain
  correct when a prefix is pending or abandoned;
- isolate pending trie node, encoded bytes, table stack, label, and cancellation
  reason per surface, with a compact renderer-owned pending/table indicator.

Exit gate: sequence ambiguity, indefinite waiting, cancellation, table nesting, chains,
fallthrough, and shell passthrough have deterministic and fuzz coverage.

### G4 — missing stateless actions — Fully done

The planned stateless action families below are implemented; external native
visual/resource evidence remains part of G5.

Implement one focused action family per pull request:

1. typed configuration/raw-terminal actions: open/reload config, text, ESC,
   CSI, cursor-key normal/application variants, terminal reset, and
   parameterized finite fractional font-size changes, including absolute set and
   the documented 6–100 point Automexia renderer adaptation;
2. visible-only clear, history-only clear, and combined clear;
3. reuse the implemented directional cell/row and word-boundary selection
   engine, then add page, home/end, and line-boundary extension plus generated
   Ghostty-profile bindings. Preserve its wide-cell, wrapped-line, viewport,
   scrollback, Unicode, and active-direction guarantees;
4. scroll-to-selection, absolute-row, line and fractional-page scrolling with
   Ghostty's positive-down sign convention, search-from-selection, search
   start/end, and next/previous match with correct mode-dependent fallthrough;
5. typed window/tab/pane-local-tab/split/close actions and an explicit split
   launch policy for a fresh default shell, inherited shell/profile/cwd, or a
   `SessionLaunchDescriptor` clone;
6. preserve the implemented geometric split-focus contract while adding exact
   logical-pixel resize, split zoom/restore, and recursive equalization.
   Directional focus uses pane rectangles, overlap, distance, centre alignment,
   and stable visual-order tie-breaking; zoom preserves the split tree, hidden
   PTYs, exact dimensions, and focus while showing an indicator;
7. secure screen export with restrictive permissions, collision-safe temporary
   files, bounded UTF-8 output, normalized line endings, explicit open,
   paste-path, and copy-path variants. Capture a consistent visible snapshot
   under a short lock, serialize afterward, exclude hidden scrollback, track
   files for exit/age cleanup, and never execute exported content.

Strict Ghostty tabs map to Automexia window-level tabs. Strict Ghostty splits
inherit the active launch context but always create an independent PTY. They
never share a live PTY. Pane-local tabs remain an Automexia extension.

Each action requires direct unit tests, dispatch tests, platform bindings,
palette/CLI discoverability, generated documentation, and security review when
it touches the filesystem or another process.

### G5 — tooling, generation, and release verification — Partially done

Local tooling, generation, host-independent synthetic tables, mutation-tested
repository policy, nightly fuzz/benchmark wiring, properties, and same-host
Windows benchmarks are implemented. The native and controlled release items
below remain open.

- generate exact Linux/macOS profiles from the pinned fixtures and generate
  Windows through a deterministic transform: Super becomes the Windows key,
  primary-selection paste becomes clipboard paste, and unsupported/global or
  OS-intercepted combinations receive explicit diagnostics;
- enforce strict-profile semantics: `Ctrl+Shift+T` is a window tab; bare
  `Ctrl+T`, `Ctrl+R`, and `Ctrl+D` are forwarded; `Ctrl+Shift+O/E` create
  inherited independent splits; `Ctrl+Alt+Arrow` focuses geometrically; and
  `Ctrl+Shift+Enter` toggles split zoom;
- add `automexia --list-actions` and `automexia --list-keybinds`, including
  `--profile`, JSON, platform, origin, aliases, shadowing, unavailable entries,
  and effective output before GUI initialization;
- add `automexia migrate ghostty --dry-run|--apply`. It reads only keybindings,
  follows canonicalized includes with cycle detection, never executes Ghostty
  config, translates aliases/actions, preserves comments in its report,
  classifies exact/translated/unsupported/unsafe entries, rejects unsupported
  globals, defaults to dry-run, writes atomically only after confirmation,
  and creates a recoverable backup;
- add `cargo xtask verify keybindings`, `cargo xtask test keybindings`,
  `cargo xtask generate keybindings --version 1.3.1`, and
  `cargo xtask generate keybindings --check`;
- generate reference tables from the registry rather than hand-copying them;
- make the palette show active aliases, `Unbound`, profile, origin, support,
  platform limitations, active table/pending sequence, and profile/config
  commands; preserve accessibility and narrow-screen behavior;
- compare compiled profiles with checked-in fixtures on every pull request;
- fuzz binding/action syntax, sequences, table state, malformed migration UTF-8,
  include graphs, adversarial duplicates, and topology-changing chains;
- benchmark single-key resolution, 1,000-entry registries, four-level
  sequences, table lookup, invalid-sequence flushing, atomic compilation, and
  reverse lookup;
- require no full-vector scan, no ordinary shortcut allocation, no terminal
  lock before a match, no PTY recreation during reload, and one redraw per
  completed action chain; establish a 30-day baseline, then flag resolution or
  dispatch regressions above 10%;
- run native keyboard smoke tests for Windows, Linux/BSD, and macOS before a
  compatibility-profile release.
- run compatibility unit/integration tests through the shared Nextest groups,
  retain Cargo doctests, publish JUnit, and fail on timeout, process leak, or a
  retry that reveals flakiness;
- snapshot the typed registry, diagnostics, effective palette rows, profile
  indicator, pending sequence/table indicator, and accessibility metadata;
  capture controlled rendered frames for affected UI at compact, normal,
  split, HiDPI, and 200% text-scale sizes with expected/actual/diff artifacts;
- execute Criterion on stable runners, retain raw reports, compare only like
  hardware, and include lookup/dispatch/startup deltas in the QA bundle;
- add bounded Loom models where immutable-registry publication, concurrent
  reload, global-hotkey preparation, or per-surface pending state crosses
  threads; keep platform event/PTY/GPU FFI outside those models;
- add native resource assertions proving failed reloads, sequences, tables,
  repeated pane actions, and profile switches do not leak processes, handles,
  threads, registrations, memory, or renderer state.

Exit gate: generated artifacts are reproducible, the working tree stays clean,
normal tests are offline, and profile claims are backed by native evidence.

### G6 — high-lifecycle features — Partially done

Implemented:

- renderer-owned inspector with bounded grid/viewport/mode/profile/origin/
  pending/table/opaque-ID/diagnostic metadata and explicit exclusion of output,
  clipboard, environment, commands, paths, and credentials;
- memory-only parked-PTY undo/redo for a complete closed top-level window tab,
  retaining its contained splits, pane-local tabs, route IDs, and independent
  PTYs without relaunch;
- limits of 8 entries, 5 minutes, and 250,000 retained history lines per window,
  plus redo invalidation and cleanup on pressure, expiry, child exit, failed
  restore, and shutdown;
- a newest-first inspector projection containing only session count, aggregate
  history lines, and TTL, with no parked route IDs, titles, commands,
  destinations, or terminal content; and
- pointer and keyboard restore plus a two-step clear that explicitly warns it
  will end parked processes; every modal key press/release is consumed before
  the PTY input path.

Remaining:

- individual split and pane-local-tab closure history;
- whole native-window history; and
- controlled native lifecycle, resource, visual, and accessibility evidence.

The accepted ADRs make this narrower scope explicit; shortcut parity does not
implicitly authorize a broader parked-process lifetime.

## Test and review requirements

Every compatibility change must include the smallest applicable set of:

- normalized fixture and provenance checks;
- both platform-family default tables on every host;
- trigger, mode, scope, origin, priority, shadowing, and collision tests;
- logical/physical key, AltGr, IME, dead-key, and keyboard-protocol tests;
- invalid/partial/repeated/concurrent reload tests;
- sequence waiting/cancellation, fallthrough, table-stack, and chain tests;
- QWERTY, AZERTY, QWERTZ, non-Latin, IME, dead-key, and AltGr normalization;
- action-specific terminal, selection, split, filesystem, and process tests;
- command-palette/CLI/documentation parity checks;
- deterministic property tests and fixed-seed fuzz regressions;
- lookup/reload benchmarks with a recorded baseline and regression threshold;
- native Windows, Linux/BSD, and macOS smoke evidence for affected defaults.
- shrinking Proptest state machines for override/reload/sequence/table/action
  transitions, with persisted minimal failure cases;
- exact structured snapshots plus controlled rendered-frame diffs for visible
  palette/profile/pending/focus behavior; image changes require explicit review;
- Nextest ownership, timeout, leak, test-group, flaky-result, and JUnit output,
  with Cargo documentation tests retained;
- executed Criterion results and an environment-qualified comparison, not a
  compile-only benchmark job;
- a redacted QA bundle containing fixture/profile identity, commands, results,
  visual evidence, resource measurements, and explicit unsupported/skipped work;
- keyboard-only, focus, contrast, 200% scale, and native assistive-technology
  evidence for every new visible compatibility control.

Native coverage includes PowerShell, CMD, WSL, Windows-key interception,
clipboard, and US/French/German layouts on Windows; X11/Wayland, primary
selection, Super shortcuts, and available XDG global shortcuts on Linux; and
Command shortcuts, native text controls/fullscreen, global permissions, and
undo/redo on macOS. Synthetic tables remain mandatory on every host; real GUI
injection belongs in nightly and release jobs.

Normal CI must never depend on a Ghostty installation or network access.

Execution tiers are mandatory:

- pull requests run fixture parity, both synthetic platform tables, Nextest/
  JUnit, Cargo doctests, bounded deterministic Proptest cases, exact state
  snapshots, a small pinned offscreen visual set, and benchmark compilation;
- nightly runs expanded Proptest seeds, compatibility fuzz campaigns, bounded
  Loom models, executed Criterion, native rendered frames, resource lifetime,
  and keyboard-layout/platform automation;
- release runs the approved native Windows/Linux/macOS profile and visual
  matrix on controlled hardware and stores the redacted QA evidence bundle;
- manual review owns aesthetic approval and assistive-technology behavior that
  cannot be represented honestly by a synthetic event table.

## Documentation contract

The generated action and effective-binding references are canonical for the
bundled registry. Any profile, action, trigger syntax, migration, platform
transform, inspector, or topology-history change must regenerate them and
update configuration, keyboard, CLI, architecture, testing, release, feature,
roadmap/audit, navigation, and changelog ownership in the same change.

The profile remains explicit opt-in. Onboarding must not select it automatically,
and a full cross-platform claim waits for the remaining native fixture,
backward-compatibility, visual, accessibility, performance, resource, packaging,
and release gates.

## Final acceptance gate

Full compatibility is complete only when:

- `ghostty-1.3` exactly matches normalized Ghostty 1.3.1 Linux/macOS fixtures,
  while Windows matches only the documented deterministic adaptation;
- each binding resolves once to the same semantic action, with no unresolved
  collision, and user overrides/unbinds always win;
- performable, unconsumed, sequence, table, chain, and `catch_all` behavior
  matches the pinned contract;
- bare `Ctrl+R`/`Ctrl+D` remain shell-owned, Ghostty tabs use window-level tabs,
  and strict splits inherit context while owning independent PTYs;
- every planned stateless action is implemented and tested;
- palette, CLI, settings, and generated docs read one registry;
- reload failure preserves the complete last valid config/registry;
- deterministic registry snapshots and controlled rendered frames agree for
  every visible profile, palette, focus, pending-sequence, and table state;
- shrinking property tests, applicable Loom models, extended fuzz corpora,
  Nextest/JUnit/timeouts/leak checks, and Cargo doctests pass;
- executed benchmark comparisons satisfy the established lookup/dispatch
  policy on named hardware, and native resource tests show no lifecycle leak;
- keyboard/focus/contrast/scaling and the v0.5 accessibility-tree/native
  assistive-technology contracts pass for compatibility controls;
- the redacted QA bundle proves fixture, generator, profile, transform, native
  platform, visual, performance, and resource evidence without exposing user
  bindings or terminal data;
- existing Automexia users remain backward compatible; and
- unit, PTY, native GUI, layout, migration, fuzz, and performance gates pass.
