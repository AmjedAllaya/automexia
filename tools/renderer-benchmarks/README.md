# Renderer and application performance

This contributor tooling has two independent layers. Neither layer is a product
telemetry service or an FPS scoreboard. Application measurements do not replace
the existing [S2 performance evidence policy](../ci/performance_assurance.py).

## Microbenchmarks

The unpublished `automexia-renderer-benchmarks` Cargo package keeps its explicit
Criterion `text_fit` target and includes the actual private renderer functions.
There is no copied renderer or production dependency on the harness.

```sh
cargo bench --locked -p automexia-renderer-benchmarks --bench text_fit
```

Use these measurements to explain a change in fitting, geometry or other local
work. A microbenchmark improvement is not proof of faster terminal interaction.
Retain Criterion's raw output, compiler identity and selected features. Its
compilation boundary is documented in [ADR 0047](../../docs/adr/0047-private-renderer-benchmark-boundary.md).

## Application measurements

`application/benchmark.py` launches the real application with an isolated config,
new PTYs and a fixed Python child workload. It reuses the repository's contained
QA process owner and bounded JSON reader/writer. It does not execute user shell
profiles, commands, connections, history, extensions or recovery sessions.
The application observer is compiled only with `application-benchmarks` and
activated only by the collector's explicit environment. Normal builds have no
observer. The observer records bounded numeric events in memory and writes once
at shutdown. Existing polling GUI-test hooks are rejected for timing campaigns.

| Metric | Endpoint and scope |
| --- | --- |
| `startup_window` | Immediately before process creation to return from the native window visibility request. Includes process startup; not compositor acknowledgement. |
| `startup_terminal_frame` | Process creation to first non-dropped frame submission containing a terminal pane. This is not shell-ready time or physical scanout. |
| `idle_rss` | Main application resident working set, sampled 20 times at 250 ms intervals after a fixed five-second warmup. Excludes child/collector memory. |
| `idle_cpu` | Main application's user + kernel CPU time divided by the measured idle interval; 100% means one logical core. |
| `idle_gpu` | Optional NVML device-0 busy percentage over the idle interval, with exact GPU/driver identity. Includes other device users, so requires an otherwise idle controlled GPU. Missing/unsupported sensors are unavailable, never zero. |
| `input_present` | Fixed `x` key dispatch in the application to its next non-dropped frame submission, 50 spaced probes per fresh process. Not hardware-keyboard-to-photon latency. |
| `output_1m` | Exactly 1,000,000 fixed 64-byte ASCII/CRLF lines written in at most 64 KiB chunks, from fixture write start through submission of the final sentinel row. Includes PTY transfer, parsing and visible rendering; does not render or retain one million distinct frames/scrollback rows. |
| `resize_reflow` | Native resize event dispatch to submission at the matching dimensions, 12 alternating resizes. Includes application resize/reflow work visible at this boundary. |
| `search` | Existing synchronous query update: regex construction, bounded match navigation and visible-match recalculation. Twelve incremental query characters over 1,000 fixed history lines; not an unbounded full-history search. |
| `four_pane_rss` | Main application RSS after three native split actions and four seconds of settling. Child Python memory is excluded. Compare with `idle_rss` from the same campaign. |
| `four_pane_input_present` | 50 input probes requiring a matching submitted frame with exactly four panes. |
| `image_kitty`, `image_iterm2`, `image_sixel` | Write start through first submitted completion marker with a decoded image and composed overlay. Fixed 256×256 opaque image; protocol payloads differ. This includes parsing, decoding, placement and submission, not physical scanout. |
| `image_rss_delta` | Nonnegative main-process RSS increase around each fixed image workload; the maximum of the three protocol deltas for each independent run. It is an observed process delta, not an allocation count. |

The fixture's fixed warmup and driver pacing are part of the workload identity.
Key probes that overlap or never reach a frame fail instead of dropping slow
samples. Every run uses a fresh process. Native input stops on focus/ownership
loss. Children exit through their fixed `q` protocol; supervisor deadlines retain
cleanup ownership on interruption. Forced-PTY-close stress belongs to native
lifecycle tests, not this campaign. Wall/monotonic clock disagreement above 1 ms
invalidates a run. Search/input/resize intervals use one monotonic clock.

## Running a campaign

Build a measurement executable with the normal release optimization:

```sh
cargo build --release --locked -p automexia-terminal --bin automexia --features application-benchmarks
python -m unittest discover -s tools/ci -p 'test_application_*.py'
```

Prepare a private copy of [the profile template](application/profile.example.json).
Replace every placeholder with reviewed public hardware/software labels. Pin the
OS version, CPU and logical cores, GPU/driver, compositor, power mode, display
resolution/refresh/scaling, Rust toolchain, renderer, feature list and build
profile. Do not use hostnames, device serials, user names, paths or credentials as
labels. Reserve the machine, keep its power/thermal state stable, use the same
display and keyboard layout, and stop other builds or GPU workloads.

Hash every font file the pinned configuration can resolve, including system
fallbacks. `--font-file` is repeatable; paths remain local. For example:

```sh
python tools/renderer-benchmarks/application/benchmark.py fingerprints --font-file sugarloaf/src/font/resources/CascadiaCode/CascadiaCodeNF.ttf --font-file sugarloaf/src/font/resources/CascadiaCode/CascadiaCodeNFItalic.ttf --font-file rio-fonts/resources/SymbolsNerdFontMono/SymbolsNerdFontMono-Regular.ttf
```

Copy the returned three hashes into the profile. Configuration identity hashes
the fixed template with logical placeholders, not temporary paths. Workload
identity includes the Python implementation/executable, collector, observer and
QA supervisor. Font identity hashes content, not file location. The collector
checks these before and after a campaign and verifies OS, architecture, core
count, optimization mode, scale and application artifact identity. Other hardware,
driver, display, power and font-resolution assertions require operator review;
the JSON profile is not hardware attestation.

Run at least five independent repetitions (Windows executable shown):

```sh
python tools/renderer-benchmarks/application/benchmark.py run --binary target/release/automexia.exe --profile .automexia-private/performance/profile.json --font-file sugarloaf/src/font/resources/CascadiaCode/CascadiaCodeNF.ttf --font-file sugarloaf/src/font/resources/CascadiaCode/CascadiaCodeNFItalic.ttf --font-file rio-fonts/resources/SymbolsNerdFontMono/SymbolsNerdFontMono-Regular.ttf --work-dir .automexia-private/performance/campaign-01 --output target/performance/candidate.json --runs 5 --desktop
```

On Linux/macOS use `target/release/automexia`. `--desktop` opts into real keyboard
and resize probes in the owned application window. Windows requires an unlocked
interactive desktop. Linux's input driver requires controlled X11 and `xdotool`;
Wayland input probes explicitly fail without that adapter. macOS uses AX/Quartz
and requires previously granted Accessibility access; the collector never changes
permissions or opens a permission prompt. Keyboard probes target the owned PID.
Native pointer probes verify focus, window identity and a system AX hit test before
posting their paired mouse events through the session event stream. Windows/Linux/macOS resource adapters
are separate; each platform still needs native validation before admitting its
baseline. NVML is optional; it is not an implementation of AMD/Intel/Apple GPU
counters. Noninteractive scenarios can run without a keyboard driver.

`--scenario` selects diagnostic scenarios; skipped metrics remain in the report
as unavailable. `--diagnostic` accepts a debug build and marks the evidence dirty,
so it cannot pass a regression comparison. Do not quote debug timing as product
performance. Failed native runs retain private bounded diagnosis and are never
represented as successful zero-duration samples.

## Raw data and regressions

`application-benchmark` schema 1 contains the source commit, dirty flag, binary
digest, UTC date, pinned cohort profile and all metrics. Measured entries contain
units and independent arrays of raw samples. Missing entries contain an explicit
reason. The separate summary reports nearest-rank p50/p95 and observed per-run
quantile ranges. These ranges are **not confidence intervals**.

```sh
python tools/renderer-benchmarks/application/benchmark.py summarize --input target/performance/candidate.json --output target/performance/summary.json
python tools/renderer-benchmarks/application/benchmark.py compare --baseline target/performance/baseline.json --candidate target/performance/candidate.json --output target/performance/comparison.json
```

Only clean evidence with exactly matching profiles and sample counts can pass.
Across-run spread above 10% is inconclusive. A regression requires separation of
the observed run ranges beyond a 5% duration/rate or 10% memory budget; throughput
uses the opposite direction. Idle utilization also has a one-percentage-point
absolute budget, so a zero baseline is meaningful. Borderline, missing or noisy
evidence is reported explicitly. Exit codes: 0 pass, 1 regression, 2 invalid,
incomplete, incomparable or inconclusive. No cross-machine leaderboard or
automatic baseline refresh is performed.

The [controlled application job](../../.github/workflows/nightly.yml)
is manual, main-branch-only and opt-in on an exclusive Windows benchmark runner.
Set `AUTOMEXIA_APPLICATION_BENCHMARK_RUNNER=1`, a reviewed JSON
`AUTOMEXIA_APPLICATION_PROFILE`, a JSON array of local font-file paths in
`AUTOMEXIA_APPLICATION_FONTS`, and optionally a local reviewed baseline path in
`AUTOMEXIA_APPLICATION_BASELINE`. A missing baseline is reported as
`baseline-not-configured`, not a passed regression check. A configured baseline
makes regression, noise, missing measurements and cohort changes fail the job.
Shared CI checks contracts/compilation only. Existing S2 release activation and
regression policy are unchanged.

Only candidate raw JSON, its summary and comparison are uploaded. Private traces,
configurations, process paths, fixture output and logs are not upload artifacts.
Keep the raw JSON and methodology publicly accessible at immutable revision URLs
before making any performance claim. GitHub artifact retention alone is not a
permanent public research archive. FPS may be exposed as a diagnostic counter;
it is not an acceptance metric or a substitute for these application measurements.


The [native Unix session scenarios](../../tests/integration/unix-session-ui.py)
also record bounded shell-ready and command-roundtrip diagnostic samples on
Linux (X11 and isolated Wayland) and macOS. The Linux CI matrix runs
CPU/Vulkan with the default shell, Bash, Zsh and Fish at 1× and 1.5×. Wayland
requires `sway`, `wtype`, `grim` and `Xvfb`; its private compositor disables Xwayland,
uses an owned virtual display for persistent input devices, and
never sends input to the user’s display. These samples include 50 ms polling, fixture setup and desktop
automation. Fixture commands are consumed on render and may wait for the passive
idle refresh; these are not input-to-present measurements or controlled regression
baselines. The macOS native workflow runs renderer microbenchmarks and retains
raw Criterion samples separately for each runner architecture. Shared hosted
hardware numbers must not be compared across machines or used for performance
claims. Use the pinned application profiles above for regression decisions.
