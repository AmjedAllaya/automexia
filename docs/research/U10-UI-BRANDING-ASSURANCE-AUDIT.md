# U10 UI branding assurance audit

Status: **repository implementation fully done; U10 partially done pending
controlled external evidence**

Audited: 2026-08-26

## Outcome, authority, and scope

Automexia-owned renderer surfaces now share the documented liquid-hacker visual,
interaction, privacy, and responsive-layout contracts, and the last known source
defect in completed-command grouping is fixed across viewport and complete
source-prompt scrollback eviction. The authoritative feature status remains the
[UI branding roadmap](../UI-BRANDING-ROADMAP.md). The exact controlled native,
visual, resource, accessibility, privacy, review, freshness, and release
requirements are owned by the
[S1 policy](../../tests/assurance/s1-assurance-policy-v1.json) and the
[S1 assurance audit](S1-NATIVE-VISUAL-RESOURCE-ACCESSIBILITY-AUDIT.md).

U10 does not authorize a second renderer, product screenshot writer,
accessibility-tree shortcut, network service, credential, telemetry stream, or
PTY owner. Test hooks remain opt-in. Retained screenshots and path-bearing native
reports remain ignored, private evidence.

## Acceptance criteria and classification

| Requirement | Status | Evidence | Exit condition |
|---|---|---|---|
| Shared brand hierarchy, contrast, redundant meaning, targets, density, and responsive geometry | **Fully done** | `renderer/ui_theme.rs` and the U0-U9.4 roadmap owners; focused layout, hit-test, contrast, privacy, modal, and interaction tests | Preserve source and focused tests |
| Completed-command branding across short and long output | **Fully done** | Stable pane-local result IDs and following-prompt boundaries in `rio-vt`; visible-anchor deduplication in `renderer/mod.rs`; parser-to-visible regressions at seven viewport heights; full source eviction, silent, newline-only, repaint, reuse, and reflow tests | Preserve exactly-one boundary and no paint-time history scan |
| Current Windows WGPU result-stage evidence | **Fully done for the bounded result stage** | Eight base PowerShell cases and seven dynamic viewport-boundary cases passed fresh ownership, visible token, glyph-pixel, blank-surface, and exact boundary-ID assertions at the 0.099 resting tint | A clean complete driver run is still required for U10 release evidence |
| Deterministic visual and evidence infrastructure | **Fully done** | One-pixel comparator, exact 1,600-case visual-suite matrices, 24-suite policy, mutation tests, redaction/freshness/commit binding, controlled workflow, and stable-tag `--require-complete` enforcement | Preserve fail-closed policy and exact current-commit binding |
| Current Windows WGPU and CPU complete visual/resource run | **External prerequisite** | The current WGPU result stage passed, but an operating-system security dialog contaminated the composed desktop and the run later stopped in the unrelated image-hover stage; current CPU was not reached | Close all foreign overlays; rerun both backends on a controlled Windows host; retain private reports and result captures |
| Linux and macOS native/visual/resource evidence | **External prerequisite** | Policy and runners are specified; no current controlled artifacts were produced on this Windows host | Run Linux X11, Linux Wayland, macOS Intel, and macOS Apple Silicon suites for the exact commit |
| Assistive-technology delivery | **External prerequisite** | Renderer-neutral names, states, keyboard paths, focus, contrast, and limitations are documented; v0.4 does not claim a complete native accessibility tree | Independently review Narrator, NVDA, VoiceOver, Orca X11, and Orca Wayland sessions |
| Final U10 release decision | **External prerequisite** | Release validation is source complete and fail closed | One clean, current, independently reviewed manifest passes `python tools/ci/s1_assurance.py validate --require-complete` |

There are no remaining **Not implemented** repository rows in U10. External
prerequisites are deliberately not relabeled as passing or replaced by local
unit tests, cross-compilation, WSL, or contaminated screenshots.

## Design and trust-boundary result

The terminal is the only command-lifecycle authority. Each valid completion
receives a stable pane-local ID. Output-producing completions project one
content-free boundary onto the following prompt, carrying only the source
prompt generation and result ID. This metadata follows row reset, copy, recycle,
reflow, and repaint semantics. The renderer consumes visible anchors,
deduplicates by ID, and performs no retained-history scan in the paint path.
Silent commands do not create an empty band, and the boundary stores neither
command text nor output text.

This closes the visible ownership defect without creating the durable complete
output-region model reserved for future diagnostic-navigation design. Removal is
reversible: deleting boundary projection and its tests restores the previous
behavior without migrating user data, but would reopen the U9.1 regression and
must also revert the corresponding roadmap claim.

## Current-practice check

The implementation and evidence contract were checked against primary sources:

- [WCAG 2.2](https://www.w3.org/TR/WCAG22/) for 4.5:1 text contrast, non-color
  meaning, target sizing, reduced motion, and flashing limits;
- [Microsoft accessibility testing](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-testing)
  for keyboard, Narrator, UI Automation inspection, scaling, and high-contrast
  validation on Windows;
- [Apple accessibility testing](https://developer.apple.com/documentation/accessibility/performing-accessibility-testing-for-your-app)
  for audits, VoiceOver, keyboard, and native macOS inspection; and
- [AccessKit](https://github.com/AccessKit/accesskit) as the accepted future
  platform adapter under ADR 0013, not as an unreviewed U10 dependency.

Automexia builds the bounded renderer and terminal metadata changes with existing
owners, and wraps native platform tools through the S1 evidence policy. Adding a
second UI framework or accessibility dependency in U10 would duplicate authority
and does not supply the missing controlled human/native executions.

## Local evidence

On Windows x86_64 on 2026-08-26:

- 506 `rio-vt` unit tests and 3 conformance tests passed;
- 24 focused Automexia renderer tests passed;
- strict focused Clippy passed for `rio-vt` and `automexia-terminal` with all
  targets and all features;
- the 512-row overflow benchmark measured 192.69-202.72 microseconds and
  50.384-53.005 MiB/s across 20 samples. It is a first local sample, not a
  controlled performance-regression comparison; and
- the current WGPU command-result stage passed eight base and seven dynamic
  overflow cases. The foreign Windows Security dialog invalidates the retained
  composed frame as U10 visual evidence, and the complete WGPU/CPU run is not
  claimed.

The final repository-wide gate and commit/push evidence belong in the delivery
handoff rather than this stable audit page.

## Exact external completion checklist

- [ ] Close or suppress foreign desktop overlays on the controlled host.
- [ ] Run and retain clean current-commit Windows WGPU and CPU/RDP suites,
  including result captures, resource ceilings, image workflow, fullscreen,
  search, caption controls, startup, modals, splits, panes, tabs, and cleanup.
- [ ] Retain Windows Intel, AMD, NVIDIA, and RDP resource suites.
- [ ] Retain elevated Application Verifier Basics and separate low-resource
  results with no stop, plus the redacted WPR summary.
- [ ] Retain Linux X11 and Wayland native/resource/visual suites.
- [ ] Retain macOS Intel and Apple Silicon native/visual suites.
- [ ] Approve all four exact 1,600-capture visual matrices for the same commit.
- [ ] Approve Narrator, NVDA, VoiceOver, Orca X11, and Orca Wayland tasks.
- [ ] Ensure visual/accessibility reviewers are independent of the operator and
  use the required HTTPS review records.
- [ ] Validate one clean, fresh, exact-commit manifest with
  `python tools/ci/s1_assurance.py validate --require-complete`.
- [ ] Link the controlled workflow result from the release review.
