# S1 and S2 stabilization and release-assurance status

The former execution-plan narrative is no longer public documentation.
This page retains current implementation and evidence references only.

## Scope and authority

S1 source tooling and the inactive-by-default S2 ratchet exist. These are
assurance mechanisms, not proof that every native release gate has passed.

Current owners and external gates are documented in
[S1 native, visual, resource, and accessibility audit](S1-NATIVE-VISUAL-RESOURCE-ACCESSIBILITY-AUDIT.md)
and [S2 release-ratchet completion audit](S2-RELEASE-RATCHET-COMPLETION-AUDIT.md).

## Evidence ledger

| Area | Current source status | Remaining evidence |
|---|---|---|
| S1 | Versioned suite policy, evidence validation, deterministic visual fixtures, mutation coverage and controlled workflow integration exist. | Native hosts, controlled hardware, assistive technologies, elevated tools and human review remain explicit where unexecuted. |
| S2 | Bounded collection, comparison, waiver and release-policy tooling exist; enforcement is inactive by default. | Activation requires an explicitly accepted baseline covering 30 consecutive comparable UTC dates and all metric families. |

No local Windows run can establish Linux/macOS native behavior, signing,
notarization, controlled-hardware coverage or elapsed-time evidence. Existing
release gates remain unchanged.

The results below are historical source-validation records, not evidence for
a newer commit or a public package.

## Implementation result

| Item | Resulting status | Evidence now owned by source | Remaining evidence |
|---|---|---|---|
| S1.1 prompt/resize/input | **Partially done** | Existing deterministic/native storms plus a 512-case independent resize queue model | Linux X11/Wayland and macOS native storms |
| S1.2 resources/hardware | **Partially done** | Existing native report now has a strict allowlisted S2 memory normalizer | Elevated/named-hardware/long-soak runs |
| S1.3 visuals | **Partially done** | Bounded `visual-diff` command, reviewed tolerance policy, atomic heatmap/report, focused tests | Approved golden matrices, Linux/macOS captures, human review |
| S1.4 accessibility | **Partially done** | Existing automated contracts preserved | Controlled Narrator/NVDA/VoiceOver/Orca evidence |
| S1.5 external-tool recovery | **Partially done** | Existing bounded/cancellable/freshness contracts preserved | Real CLI latency and recovery runs |
| S1.6 QA orchestration | **Source tooling fully done** | S2 policy/mutation checks join bounded QA; benchmark targets are unique per run | Retained bundles on every release host |
| S1.7 models/strength | **Partially done** | Resize property model plus bounded generation-aware atomic publisher | Long corpora, branch/region baseline, mutation/vet governance |
| S1.8 measurement | **Partially done / collecting** | Classified Criterion/native-memory composition, exact runner identity, path-free reports, 90-day retention | Thirty complete controlled Windows runs |
| S2 enforcement | **Source and automation fully done; release evidence collecting** | Digest-frozen 5%/10% policy, bounded sample quality, clean exact-source/operator binding, independent baseline/waiver review, protected activation validation, and fail-closed tagged-release job | Provisioned controlled runner plus reviewed activation of the externally collected 30-day baseline |

The `collecting` fixture is intentional. Source implementation is present, but
no generated or local evidence was substituted for elapsed controlled time,
native platforms, assistive technology, or human visual approval.

## External completion checklist

- [ ] Linux X11 native GPU/PTY/resize/prompt and frame bundle.
- [ ] Linux Wayland native GPU/PTY/resize/prompt and frame bundle.
- [ ] macOS Intel and Apple-Silicon native bundle.
- [ ] Elevated Windows AppVerifier and reviewed WPR summary.
- [ ] Named Intel/AMD/NVIDIA/RDP/software-fallback resource evidence.
- [ ] Narrator/NVDA, VoiceOver, and Orca smoke records.
- [ ] Human-approved dark/light/font/scale/layout screenshot matrix.
- [ ] Thirty consecutive same-runner UTC days covering every required metric.
- [ ] Reviewed activation change that binds the accepted baseline digest.
