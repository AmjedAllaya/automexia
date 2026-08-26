# CP5.0 native autocomplete research

Date: 2026-08-21
Decision: CP1 remains the complete solution; P2 is deferred.
Machine contract:
<code>tests/fixtures/command-productivity/cp50-research-contract-v1.json</code>

> **Historical phase record.** This page preserves the CP5.0 decision as it
> stood on 2026-08-21. Accepted
> [ADR 0025](../adr/0025-authenticated-native-editor-suggestion-bridge.md) later
> authorized disabled CP5 source work: CP5.1-CP5.4 are now complete at their
> source/local boundaries, CP5.5 is complete as an inert helper/adapter bridge,
> and CP5.6 plus live product activation remain partial. Current status lives in
> the [implementation audit](CP51-CP56-IMPLEMENTATION-AUDIT.md) and
> [roadmap](../ROADMAP.md).

This report closes CP5.0 at its research boundary. It does not activate a
suggestion surface, add a product dependency, inspect terminal cells, retain
command buffers, change a profile or keybinding, start a process from typing,
or grant PTY, network, credential, history, provider, or execution authority.

## Outcome and acceptance criteria

The phase is complete when evidence identifies the native editor owner for
every supported shell family, exercises a bounded editor-state and insertion
contract without runtime activation, compares the current matcher with the
focused candidate, evaluates serious build/wrap/adopt alternatives, and records
a reviewable P2 decision. The following conditions are now true:

- the seven-row shell matrix distinguishes supported rich APIs from native-only
  fallback and never claims CMD parity;
- the pure prototype rejects stale generations, invalid UTF-8 spans, control
  characters, oversized buffers/candidates/batches, and implicit execution;
- the matcher comparison covers 32, 128, and 512 candidates, eight ASCII,
  Unicode, combining-character, common-prefix, and stale-generation queries,
  20 warmups, and 200 samples per row;
- <code>nucleo-matcher</code> is pinned only in a standalone research
  workspace; the root manifest, root lockfile, libraries, application, and
  release binaries do not contain it;
- CP1 remains enabled, reversible, removable, and the authoritative fallback;
- at this CP5.0 checkpoint, CP5.1 through CP5.6 were not implemented and
  required a separate accepted bridge ADR and machine threat contract.

Out of scope are an editor transport, shell adapter activation, product ranking
changes, history ingestion, provider refresh, a renderer popup, new shortcuts,
profile installation, and process or PTY work. F4/D3 also remains externally
blocked by ADR 0003 exact-head approvals/server enforcement, attestation, and
native evidence after ADR 0012 owner acceptance.

## Evidence ledger

| Item | Before CP5.0 | Result | Evidence and exit |
|---|---|---|---|
| CP1 native completion | Fully implemented; hosted release evidence partial | Preserved | CP1 adapters, disable/uninstall contracts, shell conformance, and the 50 ms post-warmup adapter gate remain unchanged |
| Native shell/API matrix | Partially implemented | Fully done at the research/source boundary | Official APIs, local PowerShell/CMD/Bash/WSL versions, startup samples, and explicit external native gates are recorded below |
| Editor-state prototype | Not implemented | Fully done as a non-runtime prototype | The pure checker model owns only bounded ephemeral data and returns a replacement-only request to the native editor |
| Matcher comparison | Not implemented | Fully done for the 32/128/512 corpus | Standalone locked benchmark, Unicode tests, three repeated release runs, stale-generation guard, compile time, and binary size are recorded |
| Reedline/Carapace evaluation | Planned prose only | Fully done for CP5.0 | Reedline remains a UX reference; Carapace remains a separately installed external adapter candidate |
| P2 suggestion bridge | Not implemented | Not done by decision | No uniform safe bridge exists across the supported matrix; a future proposal needs new authority and protected review |
| F4 process/PTY activation | External prerequisite | Still blocked | ADR 0012 is accepted; protected exact-head approvals, attestation, native evidence, and activation remain |

## Shell and version decision matrix

| Shell family | Supported baseline and local evidence | Native owner and smallest prototype | CP5.0 decision |
|---|---|---|---|
| PowerShell 7 | PowerShell 7.2+ and PSReadLine 2.2.2+; local 7.6.4 / 2.4.5 | PSReadLine owns buffer, insertion, prediction view, history, and keys. A documented ICommandPredictor plug-in can participate only inside the existing host. | Native-only. Do not install a module, mutate a profile, or replace PSReadLine. |
| Windows PowerShell | 5.1; local 5.1.26100.9168 / PSReadLine 2.0.0 | No supported predictor plug-in contract comparable to PowerShell 7.2+. | Native-only with no rich-parity claim. |
| Bash | Bash 5.x/Readline; local Bash 5.2.21 through WSL | Readline programmable completion exposes COMP_LINE and COMP_POINT during a completion invocation. | Native-only. A persistent external bridge would need shell wiring and new per-edit authority. |
| Zsh | Zsh 5.8+ with ZLE/compsys; not installed on this host | ZLE/compsys owns BUFFER, CURSOR, widgets, context, quoting, and insertion. | Native-only; native interaction remains an external runner gate. |
| Fish | Fish 3.6+; not installed on this host | Fish already owns autosuggestions, syntax feedback, pager filtering, and insertion. Buffer access is inside Fish functions/bindings. | Native-only; a second popup would duplicate a mature accessible interaction. |
| CMD | Supported Windows Command Prompt; local Windows 10.0.26200.9168 | No supported rich completion/editor-state API. | Native-only and explicitly no rich parity. |
| WSL | WSL2 plus a supported guest shell; local WSL2 and Bash evidence | The guest Bash/Zsh/Fish editor remains owner. The Windows host must not infer its buffer from terminal cells. | Delegate to CP1 in the guest; no host bridge. |

Primary contracts reviewed:

- [PowerShell PSReadLine and native buffer APIs](https://learn.microsoft.com/en-us/powershell/module/psreadline/about/about_psreadline?view=powershell-7.6)
- [PowerShell predictor plug-in contract](https://learn.microsoft.com/en-gb/powershell/scripting/learn/shell/using-predictors?view=powershell-7.4)
- [GNU Bash programmable completion](https://www.gnu.org/software/bash/manual/html_node/Programmable-Completion.html)
- [Zsh completion system](https://zsh.sourceforge.io/Doc/Release/Completion-System.html)
- [Zsh completion widgets](https://zsh.sourceforge.io/Doc/Release/Completion-Widgets.html)
- [Fish autosuggestions and completion pager](https://fishshell.com/docs/current/interactive.html)
- [Fish responsiveness design](https://fishshell.com/docs/current/design.html#the-law-of-responsiveness)

## Native startup and UX baseline

The local startup probe used typed executable paths and exact argument arrays,
disabled profiles/AutoRun, performed five warmups and twenty samples, and
required a zero exit status. These values characterize this Windows host only;
they are not product SLOs and do not replace native Linux/macOS measurements.

| No-profile process | p50 | p95 | Notes |
|---|---:|---:|---|
| PowerShell 7.6.4 | 228.572 ms | 270.754 ms | NoLogo, NoProfile, NonInteractive |
| Windows PowerShell 5.1 | 144.976 ms | 150.214 ms | NoLogo, NoProfile, NonInteractive |
| CMD | 26.158 ms | 28.533 ms | AutoRun disabled with /d |
| Bash 5.2.21 via WSL | 111.018 ms | 135.025 ms | No profile or rc; WSL was warm |
| WSL2 guest Bash | 101.606 ms | 127.582 ms | Exact --exec Bash launch; WSL was warm |

The first Bash warmup exceeded an intentionally narrow two-second cold-start
ceiling. The failure was investigated as a WSL cold-start condition and the
bounded probe was repeated with a ten-second process ceiling; every measured
warm sample then completed. The first timeout is retained here rather than
being treated as a passing sample.

Completion/prediction behavior, typing, cancellation, resize, accessibility,
and disable evidence is classified as follows:

- PowerShell, Bash, Zsh, Fish, CMD, and WSL retain their native UI, focus,
  cursor, IME, quoting, history, accessibility, resize, and key semantics.
- CP1 already verifies managed adapter startup at most 50 ms p95 after five
  warmups, native precedence, timeout/cancellation, disable, repair, removal,
  quoting, and provider-offline behavior.
- CP5.0 ships no per-key code, queue, cache, worker, window, transport, or
  persistent record. Its typing latency and resident-memory delta are therefore
  zero in the product.
- Controlled Zsh/Fish native runs, screen-reader combinations, and three-OS
  interactive evidence remain external evidence for any future P2 proposal,
  not evidence silently claimed by this Windows research run.

## Editor-owned prototype

The research-only model in
<code>tools/ci/check_command_productivity_cp50.py</code> represents four inputs:
an ephemeral UTF-8 buffer, byte cursor, shell-returned replacement span, and
pane generation. A candidate batch carries the same generation and bounded
replacement text. The output contains only start/end bytes and replacement
text, and fixes execution to false.

The model deliberately has no transport, persistence, renderer, process, PTY,
history, profile, keybinding, or terminal-grid adapter. It validates before
insertion and returns ownership to the native editor. Tests cover stale
generation, a byte offset inside a multi-byte character, CR/LF and control
injection, invalid selection, byte/count ceilings, and the no-Enter invariant.

This is sufficient to prove the smallest safe data contract. It is not an
activation authority or proof that a persistent editor bridge is feasible.

## Matcher comparison

The standalone tool at
<code>tools/research/cp5-matcher-benchmark</code> mirrors the existing
Quick Action subsequence/word-boundary scorer locally and compares it with
<code>nucleo-matcher 0.3.1</code>. It is a separate Cargo workspace with an
exact lockfile and cannot enter root workspace builds or release artifacts.

Method:

- release profile on the same Windows host;
- 12 deterministic DevOps seed families expanded to 32, 128, and 512 records;
- eight queries per sample, including accented precomposed text, combining
  characters, Greek, CJK, common prefixes, and token/subsequence searches;
- identical sorted result materialization for both matchers;
- 20 warmups and 200 measured samples per corpus size;
- three repeated benchmark executions for noise;
- generation checked before each candidate with deterministic cancellation
  before the eighth candidate;
- 16 ms p95 ceiling for the complete eight-query sample.

Observed p95 range across the three repeated executions:

| Candidates | In-tree p95 range | nucleo-matcher p95 range | Result |
|---:|---:|---:|---|
| 32 | 0.183-0.205 ms | 0.275-0.443 ms | Both pass; in-tree is faster on this product corpus |
| 128 | 0.593-1.074 ms | 1.130-1.353 ms | Both pass; no user-visible candidate benefit |
| 512 | 2.283-2.412 ms | 3.993-4.150 ms | Both pass; in-tree is about 1.7x faster at the largest corpus |

Checksums are stable within each implementation across all repetitions.
Different checksums are expected because Nucleo intentionally uses different
normalization and scoring. The Unicode tests verify searchability; they do not
claim ranking equivalence. Cancellation stopped after seven candidates and
before stale publication in every run.

The isolated first release build, after registry sources were available, took
4.137 seconds and produced a 378,368-byte Windows executable containing both
matchers and the reporting harness. Twenty-five exact startup probes of that
isolated executable measured 17.812 ms p50 and 21.220 ms p95. These are research
tool measurements, not a product binary delta. Because the dependency is
rejected for runtime adoption, the shipped compile, binary, startup, resident
memory, and background-resource delta is zero.

Low-end hardware remains an external replication gate. The same-host result is
strong enough for the current decision because Nucleo provides no measured
benefit even before its dependency, licensing, and integration costs are
charged to the product.

## Candidate dependency review

<code>nucleo-matcher 0.3.1</code> is the smaller low-level crate recommended by
its upstream when the managed picker is unnecessary. Its default features are
Unicode normalization, case folding, and segmentation; the research manifest
names those features explicitly. The crate declares MPL-2.0, which is allowed
by this repository's license policy, depends on memchr and optionally
unicode-segmentation, and does not declare a Rust-version field. The standalone
tool pins Automexia's Rust 1.96.1 MSRV; that proves only this research build,
not an undocumented lower upstream MSRV. Upstream
describes the matcher as stable and used by Helix, but also labels its published
benchmark comparison incomplete. GitHub reported no published advisories on
the review date, while the repository had no SECURITY.md; absence of a
published advisory is not proof of absence of vulnerabilities.

Sources:

- [Nucleo source and upstream benchmark caveat](https://github.com/helix-editor/nucleo)
- [nucleo-matcher 0.3.1 API and usage warning](https://docs.rs/nucleo-matcher/latest/nucleo_matcher/)
- [exact crate manifest and features](https://raw.githubusercontent.com/helix-editor/nucleo/master/matcher/Cargo.toml)
- [upstream security page](https://github.com/helix-editor/nucleo/security)
- [MPL-2.0 license text](https://raw.githubusercontent.com/helix-editor/nucleo/master/LICENSE)

No product adoption occurs, so no MSRV, redistribution notice, SBOM, binary,
startup, unsafe-code, migration, or customer rollback obligation is introduced.
A future adoption proposal must repeat advisories/provenance, unsafe and MSRV
review, three-OS builds, same-host product benchmarks, binary/startup/resource
deltas, cargo-deny/SBOM checks, and uninstall/replacement analysis.

## Build, wrap, or adopt decision

| Candidate | Decision | Reason |
|---|---|---|
| Existing in-tree matcher | Build/retain | Already owned, deterministic, dependency-free, under 2.5 ms p95 for the measured 512-record corpus, and sufficient for current local search |
| nucleo-matcher | Research only; reject runtime adoption | Strong library and better Unicode model, but slower on this realistic bounded corpus and no user-visible benefit justifies product cost |
| Reedline | Reference only | It is a complete line editor with history, hints, menus, and keybindings; embedding it would compete with each shell's editor authority |
| Carapace | Explicit external adapter only | It may complement an installed user workflow, but must never be bundled, invoked per keystroke, or outrank native/provider completion |
| Uniform Automexia editor bridge | Defer | No common supported mutation-free persistent API exists across the matrix; CMD and host-to-WSL parity would be misleading or unsafe |

Reference implementations:

- [Reedline](https://github.com/nushell/reedline)
- [Carapace](https://github.com/carapace-sh/carapace-bin)

The product therefore builds and retains its current small matcher, wraps only
existing shell-native CP1 completion contracts, and adopts no CP5 dependency.
A future P2 is a new protected design, not the automatic continuation of this
research phase.

## Privacy and authority delta

The shipped privacy and authority delta is zero:

- no buffer, cursor, span, history, candidate, path, host, or provider value is
  persisted or logged;
- no local pipe, socket, sideband, shell process, PTY reader, terminal-cell
  inference, network client, provider refresh, credential access, or worker is
  introduced;
- no profile, completer, predictor, widget, function, keybinding, prompt,
  environment, or shell startup file is changed;
- no suggestion can press Enter or execute a command;
- CP1 disable/remove behavior and the terminal's existing safe fallback are
  unchanged.

The machine contract fixes all capabilities to false, bounds prototype bytes and
candidate counts, and rejects any root manifest or lockfile adoption of
<code>nucleo-matcher</code>. The repository validator and mutation tests guard
that result.

## External evidence

The following items were unavailable locally and are intentionally not reported
as passing:

- controlled native Zsh and Fish interaction, cancellation, resize, and memory
  measurements;
- controlled Windows, Linux, and macOS screen-reader assessment;
- low-end hardware replication of shell startup and the matcher corpus;
- protected human review of any future bridge ADR and machine threat contract.

These gates do not block the decision to retain CP1 because CP5.0 ships no
runtime feature. They become prerequisites if a future proposal attempts to
approve P2.

## Reproduce and rollback

Run the focused evidence with:

    python tools/ci/test_command_productivity_cp50.py
    python tools/ci/check_command_productivity_cp50.py
    cargo fmt --manifest-path tools/research/cp5-matcher-benchmark/Cargo.toml -- --check
    cargo clippy --manifest-path tools/research/cp5-matcher-benchmark/Cargo.toml --all-targets --locked -- -D warnings
    cargo test --manifest-path tools/research/cp5-matcher-benchmark/Cargo.toml --locked
    cargo run --release --manifest-path tools/research/cp5-matcher-benchmark/Cargo.toml --locked --quiet
    python tools/ci/validate_repository.py

Rollback is documentation/tooling-only: remove the standalone research
workspace, CP5.0 machine contract/checker/tests, and this report, then restore
the roadmap status to Not done. No user configuration, migration, persistent
record, release binary, shell profile, or runtime state needs recovery.
