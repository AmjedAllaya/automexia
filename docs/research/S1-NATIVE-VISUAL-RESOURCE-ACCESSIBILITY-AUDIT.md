# S1 native, visual, resource, and accessibility assurance audit

Status: **source implementation fully done; phase partially done pending external evidence**

Audited: 2026-08-24

## Outcome and authority

This pass closes every repository-owned implementation gap in the S1 native,
visual, resource, and accessibility boundary. The authoritative requirements
are the S1 sections in `docs/STABILIZATION-ROADMAP.md`, ADR 0013,
`docs/ACCESSIBILITY.md`, `docs/TESTING.md`, and `RELEASING.md`.

S1 does not add product network, credential, provider, or process authority.
It adds a test-only visual fixture, safer native harnesses, a strict evidence
policy/validator, CI and release enforcement, and documentation. Product builds
do not enable `visual-test-hooks`.

The phase cannot honestly be marked fully done until controlled Linux,
macOS, hardware, elevated-tool, assistive-technology, and human-review evidence
is collected. Those are external prerequisites, not missing repository code.

## Evidence ledger

| Requirement | Before | Source result | Remaining external proof |
|---|---|---|---|
| Native prompt/PTY/resize storms | Partially implemented | **Fully implemented locally.** The bounded Windows WGPU/CPU harness covers PowerShell, CMD, independent panes/tabs/clones, Unicode, history, output bursts, tiny/large layouts, fullscreen, search, image UX, cleanup, and renderer snapshots. The 2026-08-24 local run passed both renderers. | Linux X11, Linux Wayland, macOS Intel, and macOS Apple Silicon controlled runs. |
| Native resource lifetime | Partially implemented | **Fully implemented locally.** Reports enforce process/handle/thread/private-byte/working-set/descendant ceilings. Preview soak uses 16 deterministic production-resource lifecycles after one real pointer workflow. | Intel/AMD/NVIDIA/RDP named-hardware matrix and longer controlled soaks. |
| Application Verifier | Partially implemented | **Fully implemented as source.** Exact-target refusal, elevation, Basics, separately bounded `/faults` low-resource phase, 64 MiB log limit, failure detection, redaction, distinct reports, and `finally` cleanup are enforced. | An elevated clean run with private reviewed XML on a controlled Windows host. |
| WPR | Fully implemented as a wrapper | Preserved: exact binary, elevation, trace ceiling, private ETL, redacted manifest, cancellation, and optional ETL deletion. | Elevated threshold/manual trace and review. |
| Deterministic visual hooks | Not implemented | **Fully implemented.** Exact fixture `s1-standard-v1` freezes the clock at `12:34`, disables animation, injects fixed public DevOps facts, uses existing readiness/capture hooks, and is absent without an opt-in feature. | None for source; matrix captures remain external. |
| Visual matrices and review | Partially implemented | **Fully specified and enforced.** Four platform/display suites each require the exact 560-capture theme/scale/viewport/surface cross product and independent HTTPS-linked review. | Actual approved captures for Windows, Linux X11/Wayland, and macOS. |
| Accessibility baseline | Partially implemented | **Fully implemented at the v0.4 source boundary.** Keyboard/focus/contrast/scale/reduced-motion contracts and limitations remain; the S1 policy requires Narrator, NVDA, VoiceOver, and Orca on both X11 and Wayland with independent review. | Controlled assistive-technology sessions. The full native semantic tree remains deliberately deferred to v0.5 ADR 0013. |
| One release evidence authority | Not implemented | **Fully implemented.** A bounded schema validates exact environments, suites, coverage, tools, artifacts, privacy, freshness, clean commit binding, redaction canaries, and review independence. Missing evidence is `external` in local QA and fatal under `--require-complete`. | Populate the private controlled-runner manifest. |
| Release enforcement | Not implemented | **Fully implemented.** Manual controlled assurance and stable-tag workflows validate policy/mutations and require a complete current-commit manifest before preflight. | Configure the controlled runner and private evidence path. |

## Design and trust boundaries

The repository adopts existing platform tools and wraps their evidence; it does
not rebuild a native accessibility API, GPU test framework, or ETW collector.
Microsoft documents Application Verifier Basics as the minimum layer and low
resource simulation as a separate fault-injection scenario. Microsoft WPR
remains the ETW recording authority. Automexia owns only exact invocation,
bounds, cleanup, redaction, and the release decision.

S1 evidence is untrusted. `tools/ci/s1_assurance.py` rejects:

- symlinks, non-regular files, identity changes during read, duplicate JSON
  keys, unknown fields, and manifests above 1 MiB;
- unknown platform/architecture/display/backend/GPU/power/privacy values,
  duplicate environments or suites, and coverage drift;
- stale or future timestamps, oversized artifacts, failed suites, synthetic
  release evidence, dirty or mismatched commits, and incomplete matrices;
- terminal text, command history, clipboard, credentials, environment dumps,
  user paths, hostnames, raw ETL, or screen-reader transcripts in publishable
  evidence; and
- self-review, missing review, non-HTTPS review records, or review performed
  before the corresponding visual/accessibility suite.

The validator reads and writes bounded files atomically, performs no network
access, launches no product process, installs no tool, and persists only
allowlisted aggregate metadata and artifact digests. Raw frames, ETL, verifier
logs, and assistive-technology notes remain private controlled-host artifacts.

## Exact controlled matrix

The versioned policy requires 24 suites over eight environments:

- native: Windows WGPU, Windows CPU/RDP fallback, Linux X11, Linux Wayland,
  macOS Intel, and macOS Apple Silicon;
- resources: Windows Intel/AMD/NVIDIA/RDP, Linux X11/Wayland, Application
  Verifier Basics, separate low-resource injection, and redacted WPR summary;
- visuals: Windows, Linux X11, Linux Wayland, and macOS, each with dark/light,
  100/125/150/200/300% scale, eight viewport classes, and seven surfaces;
- accessibility: Windows Narrator, Windows NVDA, macOS VoiceOver, Linux X11
  Orca, and Linux Wayland Orca.

Every suite is commit-bound and at most 14 days old. Visual and accessibility
suites require independent review. The stable-tag workflow uses
`--require-complete`; there is no skip-to-pass fallback.

## Tests-first evidence

`tools/ci/test_s1_assurance.py` owns ten mutation groups covering:

- policy/commit/review binding and missing-suite behavior;
- failed, duplicate, unexpected, stale, future, oversized, and synthetic data;
- exact environment capabilities and coverage;
- the 560-capture visual cross product and independent review;
- Narrator/NVDA/VoiceOver/Orca requirements;
- privacy, redaction, artifact, symlink, duplicate-key, and clean-worktree rules;
- deterministic visual fixture and separate AppVerifier low-resource source
  contracts.

Workflow mutation tests reject release bypass, incomplete evidence, missing
source binding, hosted instead of controlled execution, or writable workflow
permissions. The architecture verifier also rejects loss of the fixture,
policy, validator, or release requirements.

## Local native result

On 2026-08-24, Windows x86_64 ran:

```text
cargo xtask test resize-stress --native-gui
```

The final WGPU and CPU runs passed. Both exercised four panes, 16 preview
open/dismiss cycles, exact prompt/route isolation, native PowerShell/CMD,
search, fullscreen restoration, painted-frame checks, and teardown ceilings.
The preview lifecycle ended with zero thread/private-byte growth on both
renderers; CPU also had zero handle/working-set growth. The sampled WGPU/CPU
preview comparison reported mean delta `0.00`, spread delta `0.00`, and bright
ratio delta `0.000`.

This is one local Windows host. It does not substitute for the policy's named
GPU, RDP, Linux, macOS, elevated, screen-reader, or human-review suites.

## Operation and rollback

Local source checks:

```text
python tools/ci/s1_assurance.py check-policy
python tools/ci/test_s1_assurance.py
python tools/ci/check_platform_coverage.py
```

Controlled validation:

```text
python tools/ci/s1_assurance.py validate \
  --manifest <private-redacted-manifest.json> \
  --expected-commit <40-hex-commit> \
  --require-complete \
  --output target/s1-assurance/summary.json
```

Set repository variable `AUTOMEXIA_S1_ASSURANCE_RUNNER=1` only after a runner
with label `automexia-assurance` is online, and set
`AUTOMEXIA_S1_ASSURANCE_EVIDENCE` to its private manifest path. A stable tag is
intentionally blocked when either prerequisite is absent or any suite is
missing, stale, failed, unreviewed, synthetic, or commit-mismatched.

Rollback removes the opt-in feature, policy/validator, controlled workflow,
and release dependency together. It does not migrate product data. Silently
disabling the stable-tag dependency is not an acceptable rollback because that
would weaken the documented release boundary.

## External completion checklist

- [ ] Windows Intel, AMD, NVIDIA, and RDP/software resource suites retained.
- [ ] Elevated AppVerifier Basics and low-resource XML accepted with no stop.
- [ ] Elevated WPR redacted summary accepted; raw ETL remains private.
- [ ] Linux X11 and Wayland native/resource/visual suites retained.
- [ ] macOS Intel and Apple Silicon native suites retained.
- [ ] Windows, Linux X11/Wayland, and macOS exact visual matrices approved.
- [ ] Narrator, NVDA, VoiceOver, and Orca X11/Wayland tasks approved.
- [ ] One complete clean-commit manifest passes `--require-complete`.
- [ ] The controlled workflow result is linked from the release review.
