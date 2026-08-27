# U10 UI branding assurance audit

Status: **repository implementation fully done; U10 partially done pending
controlled external evidence**

Audited: 2026-08-27

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
| Current Windows WGPU and CPU automated evidence | **Fully done for this local host** | Both complete drivers passed eight base PowerShell cases, seven dynamic viewport-boundary cases, CMD, resize, history, fullscreen, multi-window, timestamp identity/label, glyph-pixel, contrast, and resource assertions | Preserve private captures/reports; independent visual review and the controlled matrix remain external |
| Deterministic visual and evidence infrastructure | **Fully done** | One-pixel comparator, five exact 9,216-case visual-suite matrices, 28-suite policy, mutation tests, redaction/freshness/commit binding, controlled workflow, and stable-tag `--require-complete` enforcement | Preserve fail-closed policy and exact current-commit binding |
| Current Windows WGPU and CPU complete visual/resource run | **Partially done** | Both automated drivers passed on this Windows x86_64 host and retained private reports/captures; the captures have not received independent human approval | Review the exact frames and rerun the controlled Windows GPU/RDP matrix for release evidence |
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

- 508 `rio-vt` unit tests and 3 conformance tests passed;
- 13 direct command-result tests and the 15-case broad `result_` renderer
  filter passed;
- strict focused Clippy passed for `rio-vt` and `automexia-terminal` with all
  targets and all features;
- the 512-row overflow benchmark measured 192.69-202.72 microseconds and
  50.384-53.005 MiB/s across 20 samples. It is a first local sample, not a
  controlled performance-regression comparison; and
- the current WGPU and CPU drivers each passed eight base and seven dynamic
  overflow cases plus CMD, resize, history, fullscreen, multi-window, resource,
  and cleanup assertions. All 15 completions carried timestamps and all 14
  painted results exposed the frozen datetime label. The private captures still
  require independent human review and do not replace the controlled matrix.

The final repository-wide gate and commit/push evidence belong in the delivery
handoff rather than this stable audit page.

## Exact external completion checklist

- [x] Run and retain local current-commit Windows WGPU and CPU suites,
  including result captures, resource ceilings, image workflow, fullscreen,
  search, caption controls, startup, modals, splits, panes, tabs, and cleanup.
- [ ] Independently review the exact local WGPU and CPU frames.
- [ ] Run and retain the controlled Windows GPU/RDP suites for release evidence.
- [ ] Retain Windows Intel, AMD, NVIDIA, and RDP resource suites.
- [ ] Retain elevated Application Verifier Basics and separate low-resource
  results with no stop, plus the redacted WPR summary.
- [ ] Retain Linux X11 and Wayland native/visual suites plus Intel, AMD, and
  NVIDIA resource suites.
- [ ] Retain macOS Intel and Apple Silicon native/resource/visual suites.
- [ ] Approve all five exact 9,216-capture visual matrices for the same commit.
- [ ] Approve Narrator, NVDA, VoiceOver, Orca X11, and Orca Wayland tasks.
- [ ] Ensure visual/accessibility reviewers are independent of the operator and
  use the required HTTPS review records.
- [ ] Validate one clean, fresh, exact-commit manifest with
  `python tools/ci/s1_assurance.py validate --require-complete`.
- [ ] Link the controlled workflow result from the release review.
