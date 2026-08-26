# CP5.1-CP5.6 suggestion source and release testing

Contract and source checks:

    python tools/ci/check_command_productivity_cp51.py
    python tools/ci/test_command_productivity_cp51.py
    python tools/ci/check_command_productivity_cp56.py
    python tools/ci/test_command_productivity_cp56.py
    cargo test -p automexia-command-productivity --locked --test suggestions_contract
    cargo test -p automexia-command-productivity --locked --test suggestion_sources
    cargo test -p automexia-command-productivity --locked --test suggestion_app_reply
    cargo test -p automexia-command-productivity --locked --test suggestion_helper_protocol
    cargo test -p automexia-command-productivity --locked --test suggestion_helper_shell_response
    cargo test -p automexia-ui-model --locked --test suggestions_ui
    cargo test -p automexia-terminal --locked --test suggestions_broker
    cargo test -p automexia-terminal --locked --test suggestion_helper_bootstrap
    cargo test -p automexia-terminal --locked --test suggestion_helper_endpoint
    cargo test -p automexia-terminal --locked --test suggestion_helper_runner
    cargo test -p automexia-terminal --locked --test suggestion_helper_session
    cargo test -p automexia-terminal --locked --test suggestion_helper_transport
    cargo test -p automexia-terminal --locked --test suggestion_publication
    cargo test -p automexia-terminal --locked renderer::suggestions
    cargo test -p automexia-terminal --locked automexia::suggestions::platform::windows
    cargo build -p automexia-terminal --locked --bin automexia-suggestion-helper
    pwsh -NoLogo -NoProfile -NonInteractive -File tools/ci/test_cp5_native_powershell_bridge.ps1
    python3 tools/ci/test_cp5_native_shell_bridge.py
    python3 tools/ci/test_cp5_native_shell_adapters.py
    cargo bench -p automexia-command-productivity --bench suggestions --locked --no-run
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
| Shell adapters | Inert helper binary; bounded bootstrap/transport/session/endpoint; exact 2,176-byte response envelope; four bidirectional adapters; version/preview/collision/disable guards; local Windows PowerShell handle/status cleanup and WSL Bash/Zsh/Fish Unicode replacement/stale/hostile paths | Reviewed restricted-handle launcher, signed/attested artifact, WSL relay, interactive PowerShell insertion, successful hosted Linux/macOS/Windows shell runs, profile preservation and exact uninstall |
| Source broker | Explicit source opt-in, shell-owned history, cwd/executable, frequency-ID, cached-public-provider and action precedence; no-network/no-secret negatives | Rapid typing/cancellation, slow filesystem, stale cache, worker loss, offline and multi-pane/session storms |
| Pane UI | Renderer-neutral placement plus bounded condition-variable publication, exact candidate reconstruction, authenticated status/replacement, route saturation/supersession/kill, cursor/IME/footer/tab/sibling/modal avoidance, tiny-to-8K and 100–300% models | Live screen composition, native GPU screenshots, high-contrast/reduced-motion, menu/confirmation z-order, long-running resize/reflow storms |
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
intervals were 21.397-22.498 microseconds for 32 candidates, 88.145-89.524
microseconds for 128, 367.21-381.40 microseconds for 512, 102.40-112.17
microseconds for near-limit UTF-8 frame encoding, and 100.32-104.86
microseconds for an isolated frame-decoding rerun. The first decode comparison
reported a 103.01-106.12 microsecond interval and a 6.4% regression signal; the
unchanged decode hot path and immediate isolated rerun indicate host noise, but
the first signal is retained rather than discarded. Authenticated reply encode
measured 681.32-690.43 nanoseconds and reply decode 1.1085-1.1576 microseconds.
These intervals are not p95 latency, did not measure peak memory, and do not
replace the named-hardware/native/30-day release gates.
PowerShell 7.6.4 plus PSReadLine 2.4.5 passed preview gating, inherited anonymous-
pipe handle ownership, pure response parsing, invalid UTF-8/C0/C1/bidi rejection,
bounded status parsing, and cleanup on Windows. Failure-first WSL cases exposed
invalid `FF` acceptance in Bash 5.2.21 and Zsh 5.9 before the fix. The corrected
Bash, Zsh, and Fish 3.7 adapters passed real enable, request framing, Unicode byte
spans, one native replacement, stale/unknown/extra/oversized status rejection,
invalid RFC 3629 (stray, overlong, truncated, surrogate, and above-U+10FFFF), C0/C1, U+061C, and bidi rejection, valid 2/3/4-byte replacement, and callback return. Fish also uses
an enforced 2,176-byte response cap and streaming 4-KiB/512-item native-completion
bounds. These fixtures prove the inert source bridge on
the named local environments. They do not prove interactive PowerShell
replacement, the WSL host relay, signed/attested package launch, live screen
composition, profile preservation, successful hosted three-OS jobs, or release
activation.

The default AddressSanitizer fuzz target previously compiled with cargo-fuzz
0.13.1, but a bounded run could not start on this Windows host because the
Clang ASan runtime DLL is unavailable (`STATUS_DLL_NOT_FOUND`). During this
final audit, the isolated `suggestion_bridge` target first failed while the D:
drive was full; that failure is retained as infrastructure evidence. After
removing only generated Cargo caches, `cargo check --manifest-path
fuzz/Cargo.toml --bin suggestion_bridge --locked -j 2` completed in 3 minutes
47 seconds. The generated fuzz target was removed afterward. No sustained fuzz
campaign is claimed. The strict property corpus passed, and exact frame
round-trip plus native-replacement revalidation previously passed under Miri
0.1.0. Hosted Linux sanitizer fuzz and a bounded corpus campaign remain required
evidence.
## 2026-08-25 local full-gate evidence

The final source audit passed the exact contributor commands below on Windows
11 x86_64 with rustc/cargo 1.96.1:

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo nextest run --workspace --locked --profile ci
    cargo test --workspace --doc --locked
    python3 tools/ci/qa.py --full
    cargo ready

The first full nextest run found two packaging/keybinding regressions: adding a
second binary made bare `cargo run` ambiguous. `default-run = "automexia"` now
preserves the established application entry point; both focused regressions
then passed and the complete rerun executed 2,189 tests with 2,189 passing and
seven declared skips. Workspace documentation testing passed 64 executed tests;
three `rio-window` platform examples remained intentionally ignored.

Full QA passed its self-tests, mutation, policy, native PowerShell bridge,
repository, performance, Clippy, nextest, doctest, resize-stress, session-clone,
Loom, and dependency-policy stages. Its report is under the ignored generated
`target/qa` evidence tree. The QA result truthfully keeps controlled S1 and
OpenSSH manifests, interactive Windows GPU/AppVerifier/WPR runs, named-hardware
benchmarks, isolated LLVM coverage, and the 30-day baseline external.

The first `cargo ready` attempt stopped at its 12 GiB storage preflight because
a running Automexia process retained one generated D: executable; the process
was not terminated. A fresh C: target with 21.18 GiB free then passed repository
policy, a clean non-incremental workspace check, warning-denied Clippy, all
unit/integration/documentation tests, dependency policy, the persistent
application build, and the `automexia 0.4.0` smoke. Its 9.52 GiB verification
generation and outer temporary target were removed afterward. The retained D:
cache remains rebuildable local state owned by the running application, not a
passing or failing CP5 release claim.

CP5 still requires the reviewed launcher, restricted inherited handles,
signed/attested helper artifact, interactive PowerShell insertion, native
Linux/macOS endpoint runs, WSL host relay, live screen composition, package
rollback, accessibility-tool sessions, 1,000-cycle real endpoint/resource
campaigns, successful hosted three-OS jobs, and longitudinal evidence before
runtime activation or a stable-release claim. Those gates are not silently
treated as passing.
