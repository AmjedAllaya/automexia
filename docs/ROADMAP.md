# Automexia roadmap

## Product direction

Automexia is a free, open-source, keyboard-first terminal. This page records
current implementation and release evidence only.

## Current public focus

The existing source covers terminal sessions, tabs, panes, search, selection,
clipboard, images, shell integration, keyboard navigation, configuration, and
local inventory. [Features](FEATURES.md) identifies each current user path and
its release limitations. Native accessibility, hardware, packaging, and
long-duration evidence remain external where they have not been collected.

### Configuration and appearance status

Current source provides declarative terminal configuration for fonts, themes,
cursor, window appearance, navigation and keybindings. The application-owned
runtime-preference overlay separately stores font size and forced light/dark
appearance; it does not store every terminal preference or rewrite `config.toml`.
These are implementation boundaries, not a claim of complete native verification
for every theme, scale, keyboard profile or platform.

Use the [customization guide](user-guide/customization.md) for supported user
workflows, [configuration reference](CONFIGURATION.md) for exact keys and
[ADR 0036](adr/0036-application-owned-runtime-user-preferences.md) for persistence
ownership. Configuration recovery, keyboard/focus, responsive rendering and
platform-specific appearance evidence remain subject to the existing
[testing](TESTING.md) and [release principles](#release-principles).

### Public Linux distribution status

Repository support exists for x64 and Arm64 Linux package formats, manifests,
checksums, release policy, and verification. A source-complete release path is
not evidence that a public package exists.

| Status | Feature | Remaining requirement |
|---|---|---|
| **Fully done (source and policy)** | Six-package x64/Arm64 DEB, RPM, and portable archive allowlist; source-owned nFPM revision; bounded manifest; checksums; SBOMs; Minisign; create-once immutable-release and attestation verification; fail-closed website handoff | Preserve the exact package, permission, immutability, and no-activation-before-verification contracts. |
| **Fully done (unsigned native rehearsal)** | Exact commit `09ae5d2723542a447947b8cf1d2ddd405d7a7a9e` passed quality without retry, both native package/install lifecycles, aggregation, and independent byte-for-byte manifest reconstruction at SHA-256 `4edcd25c4bf44a97bf774acbbeb679f645052b89f40c3dff546133fbd0914b97` in hosted run `33994146068`; PR CI run `33994118326` passed and correctly skipped stable-only jobs | Repeat after any source, dependency, toolchain, package, or workflow change. The rehearsal is intentionally non-distributable. |
| **Partially done** | Official signed Linux Early Access publication | Run `34032755370` passed quality, both native package lifecycles, signing and live governance, then failed at unpublished-draft lookup. Numeric release identity and temporary draft URL handling pass against the actual draft and in regression tests; the exact failed draft was removed after signature/identity checks. A new guarded source run must publish and verify every asset. Offline backup is a separate owner responsibility. ADR 0040 accepts owner-only authorization. |
| **Not done** | Public download activation | Consume only the workflow-generated activation handoff, verify every live asset from a signed-out client, then enable the landing-page routes. |

The exact current status and remaining external gates are documented in
[Public Linux Early Access distribution](PUBLIC-RELEASE-DISTRIBUTION.md).

### Build and CI performance status

| Status | Feature | Remaining requirement |
|---|---|---|
| **Fully done (source and policy)** | Readiness avoids the duplicate pre-Clippy Cargo check; quality jobs use a reviewed, versioned compiler cache; Cargo source caches share a lockfile-bound identity; native x64/Arm64 package jobs run beside quality while remaining cold; nFPM uses checksum-pinned native archives; downstream jobs join all required evidence; semantic mutation tests guard every boundary | Preserve the cold package boundary and rerun all mutations for every workflow or toolchain change. |
| **Fully done (exact hosted evidence)** | Exact-commit PR CI and credential-free cold/warm release rehearsals passed; both native package lifecycles remained cold; cache counters and job timestamps are retained in runs `33593759776` and `33593793029` | Preserve the evidence links and repeat the measurement after material workflow, dependency, toolchain, or cache-generation changes. |
| **Fully done (bounded measured claim)** | For the linked rehearsal only, the whole path improved from 37m11s to 25m56s cold and 15m01s warm (30.3% and 59.6%); the warm release quality job was 52.9% faster and the independent ordinary PR quality/test job improved from 38m20s to 13m44s (64.2%) | Do not generalize these figures: GitHub runner load, eviction, quota, rate limits, and future dependency graphs remain external service conditions. |
| **Fully done (source and local contract)** | Cargo intermediates are separated from final artifacts; assurance tools are shared through an integrity-checked content-addressed cache; disposable verification and QA benchmark targets have explicit cleanup owners; cache inventory and dry-run-first GC protect current, dirty, leased, live, required, recent, linked, and unexpected paths | Obtain current native evidence on every claimed host and keep any destructive legacy/worktree cleanup an explicit reviewed action. |
| **Fully done (CI trigger policy)** | GitHub's free hosted CI remains automatic for pushes and pull requests; the local pre-push hook is a tested non-blocking placeholder with the manual command preserved as a comment | A future automatic-local policy change must update the generator, mutation tests, contributor docs, and cache/resource budget together. |

The complete design and evidence ledger is
[CI and build performance plan](CI-BUILD-PERFORMANCE-PLAN.md).
The cache lifecycle and cleanup guide is
[Development cache and build storage](DEVELOPMENT-CACHE.md).

## Release principles

A public free feature moves forward only when:

- one source owner and user contract are clear;
- hostile and boundary inputs are bounded;
- cancellation, cleanup, recovery, disable, and uninstall work;
- Windows, Linux, and macOS claims match actual native evidence;
- visible behavior has keyboard and accessibility evidence;
- performance and resource baselines remain acceptable;
- documentation describes the resulting truth; and
- packaging and release evidence covers the exact artifact.

Source code, a unit test, a cross-compile, or a roadmap entry alone does not make
a feature available.

## Publication boundary

All future capabilities, including free and open-source ideas, commercial products,
market analysis, pricing, detailed schemas, algorithms, UX state machines, and
phase-by-phase execution plans are maintained only in the ignored private
workspace. Public documents should not link to or summarize that material.

See the [public/private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md),
[Features](FEATURES.md), [Stabilization](STABILIZATION-ROADMAP.md), and
[Readiness](READINESS-AUDIT.md).

## Command-productivity release evidence

Complete release evidence for command productivity across supported native
shells, platforms, accessibility environments, packages, lifecycle operations,
and resource limits. Some capabilities may exist in source but remain disabled or release-gated
until those exact-artifact gates pass. See [DEVOPS-ALIASES.md](DEVOPS-ALIASES.md).
