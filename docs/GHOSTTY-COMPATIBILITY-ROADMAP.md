# Full Ghostty compatibility roadmap

## Scope and status

Automexia currently provides a tested subset of Ghostty-compatible default
shortcuts. It does **not** yet provide Ghostty's complete keybinding language,
a selectable compatibility profile, or every Ghostty action. The exact
implemented shortcut matrix is maintained in
[Ghostty keyboard compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md).

This roadmap reconciles the proposed full-compatibility design with the
current source tree. It uses these status values:

- **Implemented**: present in code and covered by focused tests.
- **Partial**: useful behavior exists, but it does not satisfy the complete
  compatibility contract.
- **Planned**: should be implemented in the stated order.
- **Deferred**: valuable, but depends on more fundamental work or has a high
  lifecycle/security cost.
- **Excluded**: should not be implemented as a default or automated behavior.

Full Ghostty compatibility is a separate compatibility track, not a v0.4.0
release criterion. Stable v0.4 must not claim an exact Ghostty profile.

## Current implementation audit

| Roadmap capability | Status | Current evidence and remaining work |
|---|---|---|
| Platform-specific Ghostty-style defaults | Implemented | Separate macOS and Windows/Linux/BSD constructors cover every pinned default whose action exists. Host-independent collision tests build both tables. |
| Shell ownership of `Ctrl+R` and `Ctrl+D` | Implemented | Both chords reach PowerShell/Readline/ZLE for history search and EOF/logout. Automexia cloning uses `Ctrl+Alt+R` and `Ctrl+Alt+D`. |
| Automexia-specific non-colliding shortcuts | Implemented | Pane-local tabs, cloning, link hints, quake mode, appearance, and additional aliases are documented and collision-tested. |
| Logical and physical key triggers | Partial | Runtime bindings support logical keys, key location, and physical scancodes. The config schema exposes only string keys and does not provide Ghostty-compatible typed trigger atoms or portable physical-key serialization. |
| Mode predicates | Partial | Application cursor/keypad, alternate screen, vi, search, and keyboard-protocol modes exist as bitflags. There are no named platform/profile predicates, scope objects, or compiled predicate diagnostics. |
| User override precedence | Partial | A user binding removes matching default triggers and is then appended. There is no explicit `unbind`, origin metadata, precedence report, or profile-layer merge model. |
| Typed action model | Partial | The frontend has a Rust `Action` enum, but configuration parses action strings at runtime and several parameterized actions use ad hoc regular expressions. There are no stable action IDs, schemas, aliases, capability metadata, or reverse lookup index. |
| Compiled binding registry | Planned | Active bindings remain a flat `Vec` scanned for every key event. There is no direct lookup map, sequence trie, table stack, origin table, or reverse action index. |
| Versioned profiles | Planned | There is no `automexia`, `ghostty`, or `ghostty-<version>` profile selector and no checked-in generated Ghostty fixture. |
| Runtime configuration reload | Partial | Reload updates existing windows and rebuilds bindings. Invalid configuration currently substitutes defaults instead of retaining the last known-good registry, so reload is not atomic. |
| Performable/unconsumed dispatch | Planned | Matching bindings are executed during one list scan and the function returns only whether text input should be suppressed. It cannot distinguish handled, unavailable, unconsumed, or fallthrough outcomes. |
| Multi-key sequences, tables, chains, and `catch_all` | Planned | Kitty keyboard escape-sequence encoding is unrelated to a user key-sequence language. No pending-prefix state, table stack, chained action, or catch-all binding model exists. |
| Central action/shortcut registry | Planned | Command-palette shortcut labels are platform-specific constants duplicated from the binding tables. They are not generated from active bindings. |
| Config editor and raw terminal input | Partial | The config editor and arbitrary escape-string actions exist. They require typed registry metadata and safer parsing before strict profile import. |
| Clear screen/history semantics | Partial | `ClearHistory` clears saved history. `ClearScreen` currently clears both the visible screen and saved history. Distinct visible-only `ClearScreen`, history-only `ClearHistory`, and combined `ClearScreenAndHistory` actions are still required. |
| Selection and search actions | Partial | Select-all, copy, clear selection, terminal search, vi selection, and result navigation exist. Directional extension, boundary/page extension, scroll-to-selection, and search-from-selection do not. |
| Screen export actions | Planned | Visible-screen export to a secure temporary file, paste/copy path, and explicit open are absent. |
| Tab/window/split semantics | Partial | Automexia has explicit window, window-tab, pane-local-tab, split, clone, close, move, and sequential focus actions. Geometric split focus, zoom, and equalize are absent. |
| Inspector | Deferred | There is no terminal inspector action or redacted inspector surface. Its privacy boundary must be designed first. |
| Undo/redo closed surfaces | Deferred | Closed windows, tabs, and splits are destroyed; no bounded parked-PTY lifecycle exists. |
| Exact generated platform profiles | Planned | Current tables are manually maintained against pinned Ghostty commit `d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0`. There is no generated Linux/macOS fixture or deterministic Windows transform. |
| Keybinding/action CLI | Planned | `automexia --list-keybinds`, `--list-actions`, explain/collision output, and config migration commands do not exist. |
| Dedicated `xtask` compatibility commands | Planned | The contributor gate runs current collision and palette tests, but there are no `ghostty-sync`, `keybindings-check`, or generated-doc verification commands. |
| Generated user documentation | Planned | The compatibility matrix is hand-maintained. Reference tables are not generated from an action registry or compiled profile. |
| Fuzz and performance coverage for the binding engine | Planned | Current deterministic binding tests cover mappings and collisions. Parser/config fuzzing does not yet target sequences, profile compilation, or dispatch latency. |

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
8. The missing stateless actions: three clear variants, selection extension,
   search/scroll from selection, geometric split focus, split zoom/equalize,
   and secure visible-screen export.
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
search, selection, session, export, and later undo operations. The binding
crate must not initialize a PTY, window, font system, renderer, or GPU. This
keeps profile compilation deterministic and cheap to test.

### Defer

These features remain desirable but should follow the stateless registry and
action work:

- the terminal inspector, until its data model has an explicit redaction
  policy for environment values, clipboard data, hidden output, and secrets;
- undo/redo for closed windows, tabs, and splits, until a bounded parked-PTY
  lifecycle defines process ownership, resource limits, expiry, failure
  recovery, and user-visible state;
- a broad user-facing key-table ecosystem, until the base profile compiler,
  prefix handling, consumption semantics, and migration tooling are stable;
- exact compatibility claims for a new Ghostty release until its fixture is
  regenerated and reviewed rather than copied manually.

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

### G0 — source lock and safety prerequisites

Before expanding shortcut coverage:

- verify the proposed Ghostty v1.3.1 source tag and exact commit, then generate
  rather than hand-copy the fixture; until that review lands, the current
  `d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0` audit remains the shipped baseline;
- capture normalized `linux.json`, `macos.json`, `windows-adapted.json`,
  `actions.json`, `provenance.json`, and `deviations.json` under
  `tests/fixtures/keybindings/ghostty/1.3.1/`;
- record tag, commit, Ghostty binary hash, generation commands, retrieval date,
  platform, source links, schema version, and fixture checksums;
- generate the source fixture explicitly from `ghostty +list-keybinds
  --default` and `ghostty +list-actions`; only the maintainer generation command
  may execute an external Ghostty binary;
- add a normalized fixture schema and checksum verification;
- add a golden manifest for Automexia's current effective defaults;
- replace ADR 0010 only after a new ADR accepts profile precedence,
  compatibility versioning, the private crate boundary, and Windows adaptation;
- fix invalid runtime reload so it retains the active configuration;
- correct the XTGETTCAP terminal-name response from the inherited `rio` value;
- bound OSC, APC/graphics, and XTGETTCAP control-string accumulation, discard
  oversized payloads safely, recover at the terminator, and fuzz the limits.

The last two items are v0.4 identity/security hardening and have priority over
new compatibility features.

Exit gate: fixtures are reproducible and reviewable; invalid reloads do not
change active behavior; hostile control strings cannot grow memory without a
bound; product identity responses are Automexia-owned.

### G1 — typed registry without behavior changes

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

### G2 — profiles, overrides, migration, and atomic reload

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

### G3 — consumption, sequences, tables, and chains

- give every action `can_perform` and `execute` contracts;
- add structured `ActionOutcome` values for performed/consumed state, terminal
  damage, layout/window/config changes, and user-facing errors;
- implement unconsumed fallthrough to lower-priority bindings or the PTY;
- make copy/paste fall through when there is no non-empty selection/text;
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

### G4 — missing stateless actions

Implement one focused action family per pull request:

1. typed configuration/raw-terminal actions: open/reload config, text, ESC,
   CSI, cursor-key normal/application variants, terminal reset, and
   parameterized font-size changes;
2. visible-only clear, history-only clear, and combined clear;
3. directional, page, home/end, and line-boundary selection extension that
   respects graphemes, wide cells, wrapped lines, combining marks, viewport
   boundaries, and active selection direction;
4. scroll-to-selection, search-from-selection, search start/end, and next/
   previous match with correct mode-dependent fallthrough;
5. typed window/tab/pane-local-tab/split/close actions and an explicit split
   launch policy for a fresh default shell, inherited shell/profile/cwd, or a
   `SessionLaunchDescriptor` clone;
6. geometric split focus, exact logical-pixel resize, split zoom/restore, and
   recursive equalization. Directional focus uses pane rectangles, overlap,
   distance, and stable route-ID tie-breaking; zoom preserves the split tree,
   hidden PTYs, exact dimensions, and focus while showing an indicator;
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

### G5 — tooling, generation, and release verification

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

Exit gate: generated artifacts are reproducible, the working tree stays clean,
normal tests are offline, and profile claims are backed by native evidence.

### G6 — high-lifecycle features

After the preceding gates are stable, design and implement:

- a renderer-owned inspector showing grid/viewport dimensions, terminal modes,
  active profile/binding origin, pending table/sequence state, PTY identity,
  route, and recent parser diagnostics while excluding environment secrets,
  clipboard contents, and output beyond what is already visible;
- bounded undo/redo transaction history for new/closed windows, tabs, and
  splits using parked independent PTYs that retain exact topology placement;
- per-topology isolation plus cleanup on count, timeout, scrollback/memory
  pressure, shutdown, expiry, failed restore, and redo invalidation.

These features require their own ADRs and are not implied by shortcut parity.

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

Native coverage includes PowerShell, CMD, WSL, Windows-key interception,
clipboard, and US/French/German layouts on Windows; X11/Wayland, primary
selection, Super shortcuts, and available XDG global shortcuts on Linux; and
Command shortcuts, native text controls/fullscreen, global permissions, and
undo/redo on macOS. Synthetic tables remain mandatory on every host; real GUI
injection belongs in nightly and release jobs.

Normal CI must never depend on a Ghostty installation or network access.

## Documentation contract

Until G1 lands, the hand-maintained
[compatibility matrix](GHOSTTY-KEYBOARD-COMPATIBILITY.md) remains canonical for
current defaults. Once the registry can generate stable output, generated
action and binding references become canonical and this roadmap will link to
them. Any new profile, action, trigger syntax, migration behavior, or platform
transform must update configuration, compatibility, testing, contributor, and
release documentation in the same pull request.

When the profile system becomes user-facing, add generated `KEYBOARD-PROFILES`,
`KEYBINDINGS`, compatibility, and migration guides and expose it first through
an opt-in beta. Onboarding may offer the profile only after fixture parity,
native-platform evidence, backward compatibility, and all mandatory gates
pass.

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
- existing Automexia users remain backward compatible; and
- unit, PTY, native GUI, layout, migration, fuzz, and performance gates pass.
