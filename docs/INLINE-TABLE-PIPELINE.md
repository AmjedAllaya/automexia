# Generic inline-table pipeline: implementation and verification

## Scope and evidence

This change is based on Automexia source at
`7718a1587374c33bfb58a3c41b00afcddaabe9d1`.
It does not recognize Docker, Kubernetes, executable names, or particular header
words. Fictional test data exercises seven columns, a multiword header, a long
value, an empty middle field, and a final column beyond the first screen row.

The script author did not have Rust, a native Automexia build, or a reachable
clone in the authoring environment. Rust compilation and native tests MUST run
on the target checkout before merging. Script-engine/Python tests are not Rust
or native-terminal evidence. This patch does not certify a fix for the supplied
screenshot: lost ConPTY wrap provenance remains a hypothesis until the native
fixture measures the actual failing path.

## Implemented changes

1. `Crosswords::bounds_to_display_string_bounded` validates the input range and
   native-cell budget, then emits presentation text once. It preserves spaces
   across known soft wraps (including blank continuation segments), expands tabs
   from actual grid positions, respects wide/combining cells, and bounds emitted
   UTF-8 bytes. It does not first allocate an unused clipboard string. Normal
   selection/clipboard serialization is unchanged.
2. Snapshot capture records capture outcomes and incomplete prefix/suffix
   boundaries. It does not promote a known clipped logical-row tail to a header.
   Prompt-only separators do not allocate unused style maps.
3. A header-candidate predicate is no longer the sole membership predicate.
   An established whitespace schema can admit a sparse value aligned in a
   non-leading column. A lone value in column zero remains ambiguous and is not
   swallowed as a record. This is intentionally conservative, not a complete
   parser for arbitrarily hard-wrapped or malformed tables.
4. Complete replacement surfaces are prepared locally and published together
   with their source snapshot and diagnostics. Existing source-position mapping
   and `RowProjection` remain the only coordinate owners. No asynchronous task,
   extra PTY parser, second terminal, per-cell identity, or new dependency is added.
5. Detection has one global ceiling of 32 model invocations per refresh, shared
   with sparse-row probes, in addition to existing per-block and storage bounds.
6. Failures have inspectable reasons. Only changed diagnostic decisions are
   logged at debug level, at most once per second per desktop pane; text,
   commands, paths, and credentials are not logged.
7. Existing model, focused viewer, and renderer tests remain enabled. New tests
   cover initial widths, chunking, reflow, mapping, bounds, negative cases,
   disabled behavior, and native transport. New cases are included in the
   existing renderer test module. The existing Criterion target gains capture,
   preparation, unchanged-snapshot, and disabled-path benchmarks.

## Not implemented or claimed

No patch changes CRLF semantics, guesses missing native soft-wrap flags, joins
arbitrary indented hard lines, reconstructs truncated producer output, rewrites
commands, fetches structured JSON, or weakens style safety checks. Unsupported
styles still use native rendering, now with a diagnostic reason.

This change does not overhaul the renderer's global damage architecture or add
an output protocol. Identical snapshots reuse surfaces; source capture is still
bounded snapshot-based. Further revision-indexed incremental capture, optional
worker offload, and more permissive sparse-row handling require measured evidence
and separate changes. A global count ceiling is not a wall-clock latency promise.

A green native-host test proves the tested system-shell/PTY path only. Running
Linux tests inside WSL does NOT exercise a Windows-host ConPTY session launching
WSL. Physical input, GPU/compositor latency, accessibility, and all supported
native transports remain separate evidence requirements.

## Checks

From the repository root after applying:

```sh
python tools/ci/test_check_inline_pipeline.py
python tools/ci/check_inline_pipeline.py
```

The checker uses the existing `tools/ci/qa_process.py` process owner. Each step
has a deadline and a 16 MiB output ceiling. A zero-test result, timeout, failed
cleanup, or compilation error fails the run. It runs existing tests as well as
the new tests. It does not change assertions to accommodate failures.

On a Windows host, explicitly test the installed WSL distro you use:

```powershell
python tools/ci/check_inline_pipeline.py --wsl-distro Ubuntu
```

The opt-in WSL test launches only a fictional `printf` fixture in that distro;
it does not access Docker, a cluster, credentials, or application configuration.
It starts the selected distro, so do not request a production/unknown distro.

```sh
python tools/ci/check_inline_pipeline.py --bench --ready
cargo bench --locked -p automexia-terminal --bench automexia_services -- inline_pipeline
```

`--bench` runs correctness smoke, not a performance certification. Compare actual
Criterion measurements on the same pinned hardware/build/power profile; collect
input and frame latency independently. Never change a threshold simply to pass.
The optional `cargo ready` gate may reveal pre-existing or assurance-inventory
failures; fix or document them rather than silently bypassing them.

## Debugging a remaining native failure

Inspect `InlineTables::diagnostics()` or debug-level
`inline table presentation decision` events. Check captured logical row count,
incomplete boundaries, unsupported styles, schema confidence, and geometry
limits. Capture raw PTY bytes only with explicit user consent and bounded,
private retention: arbitrary terminal output may contain secrets.

If the native fixture cannot reconstruct original logical rows, investigate
`teletypewriter` / ConPTY / `rio-vt` at the first point of information loss.
Do not implement a Docker special case or force the native test to pass by
pretending every hard newline is a soft wrap.

## Review and rollback

The installer preflights all five existing owner-file fingerprints and refuses
unknown changes. New paths must not contain unrelated content. It refuses a
merge/rebase/cherry-pick in progress, preserves unrelated files and line-ending
style, and never stages, commits, pushes, installs packages, or merges branches.

Originals and a manifest are kept under the repository's Git metadata directory
in `automexia-inline-pipeline-v1`. File replacement is atomic per file, not across
the entire set. The journal supports rollback after interruption. Post-apply test
failures leave the patch and logs available for diagnosis, return a nonzero exit
status, and do not claim success. Rollback refuses to overwrite subsequent edits.
