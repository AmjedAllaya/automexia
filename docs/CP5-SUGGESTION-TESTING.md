# CP5.1-CP5.6 suggestion source and release testing

Contract and source checks:

    python tools/ci/check_command_productivity_cp51.py
    python tools/ci/test_command_productivity_cp51.py
    python tools/ci/check_command_productivity_cp56.py
    python tools/ci/test_command_productivity_cp56.py
    cargo test -p automexia-devops --locked --test suggestions_contract
    cargo test -p automexia-devops --locked --test suggestion_sources
    cargo test -p automexia-ui-model --locked --test suggestions_ui
    cargo test -p automexia-terminal --locked --test suggestions_broker
    cargo test -p automexia-terminal --locked renderer::suggestions
    cargo test -p automexia-terminal --locked automexia::suggestions::platform::windows
    cargo bench -p automexia-devops --bench suggestions --locked --no-run
    cargo +nightly fuzz build --fuzz-dir fuzz suggestion_bridge

ADR 0025 authorizes source implementation while `runtime_activation` stays
false. The local gates cover strict framing, route/capability/generation/span
revalidation, Windows/Unix endpoint source policy, deterministic sources/ranking,
fragmented streams, lifecycle controls, pane placement/accessibility semantics,
renderer projection, shell syntax/disabled fallback, hostile mutations, fuzz
build, and benchmark compilation. The PR/nightly split remains:

| Layer | PR evidence | Nightly/release evidence |
|---|---|---|
| Pure model/ranking | Deterministic ordering, precedence, limits, Unicode/grapheme, hostile labels, replacement-span properties, stale-generation rejection | Criterion 32/128/512-candidate and low-end reference baselines; dependency-size/startup comparison |
| Bridge protocol | Frame/schema/route/capability/generation validation, malformed/oversize/replay/cross-pane/downgrade rejection, memory-only redaction | Native Windows named-pipe and Unix-socket churn, crash/restart/sleep/resume, ACL/mode and handle/socket leak campaigns |
| Shell adapters | Inert PowerShell/Bash/Zsh/Fish request-scaffold syntax, version/preview/collision/disable guards, zero default sourcing and truthful CMD/native fallback | Signed helper and response/replacement adapters; native Windows/WSL/Linux/macOS insertion for spaces/quotes/selections/multiline without Enter; profile preservation and exact uninstall |
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

Local source evidence on 2026-08-25 used Windows 11 10.0.26200 x86_64, an AMD
Ryzen 5 5600H (12 logical processors), 27.9 GiB RAM, rustc/cargo 1.96.1, and
Criterion's 3-second warmup plus 100-sample default collection. Confidence
intervals were 21.657-22.151 microseconds for 32 candidates, 89.790-90.614
microseconds for 128, 374.29-404.40 microseconds for 512, 100.01-101.97
microseconds for near-limit UTF-8 frame encoding, and 99.816-100.50 microseconds
for decoding. These intervals are not p95 latency, did not measure peak memory,
and do not replace the named-hardware/native/30-day release gates.

PowerShell 7.6.4 parsing and disabled health passed on Windows; native WSL Bash,
Zsh, and Fish syntax plus `preview-disabled` CP1/native fallback health passed.
These fixtures prove inert adapter syntax and truthful fallback only. They do not
prove the absent signed helper, response reader, native replacement, profile
preservation, WSL relay, packaging, or activated UI.

The default AddressSanitizer fuzz target compiled successfully with cargo-fuzz
0.13.1 in 17 minutes 48 seconds. A bounded 30-second run could not start on this
Windows host because the Clang ASan runtime DLL is unavailable
(`STATUS_DLL_NOT_FOUND`); neither the nightly sysroot nor an installed Clang
provided it. A no-sanitizer fallback was attempted, but the duplicated global
fuzz dependency graph exhausted the drive before linking. Its generated
`fuzz/target` directory was cleaned (11,126 files, 8.0 GiB); source and corpus
were not removed. Therefore no sustained fuzz-run result is claimed. The strict
property corpus still passed, and exact frame round-trip plus native-replacement
revalidation each passed under Miri 0.1.0. Hosted Linux sanitizer fuzz and a
bounded corpus campaign remain required evidence.
## 2026-08-25 local full-gate evidence

The final source audit passed the exact contributor commands below on Windows
11 x86_64 with rustc/cargo 1.96.1:

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo nextest run --workspace --locked --profile ci
    cargo test --workspace --doc --locked
    python3 tools/ci/qa.py --full
    cargo ready

Nextest ran 2,164 tests: 2,164 passed, seven were skipped by their declared
platform/fixture conditions, and the two keybinding assurance tests completed in
95.1 seconds. Workspace documentation testing passed 64 executed tests; three
`rio-window` platform examples remained intentionally ignored. Full QA passed
its mutation, policy, shell, performance, repository, Clippy, nextest, doctest,
resize-stress, session-clone, Loom, and dependency-policy stages. `cargo ready`
then independently passed a clean non-incremental workspace check, warning-denied
Clippy, unit/integration/documentation tests, dependency policy, application
build, and `automexia --version` smoke check in an isolated temporary target;
the 9.0 GiB verification tree and outer temporary target were removed afterward.

The full QA result still classifies controlled S1 evidence, native OpenSSH
evidence, an interactive Windows GPU run, Application Verifier, WPR, named-host
benchmarks, isolated LLVM coverage, and the 30-day baseline as external. CP5 also
requires the signed helper/native editor replacement, native Linux and macOS
endpoint runs, a WSL relay, package/signing/rollback proof, accessibility-tool
sessions, and 1,000-cycle endpoint/resource campaigns before runtime activation
or a stable-release claim. Those gates are not silently treated as passing.
