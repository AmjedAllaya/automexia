# Automexia roadmap

## Product direction

Automexia is a free, open-source, keyboard-first terminal. The public roadmap is
limited to ordinary terminal quality, safe interoperability, accessibility,
recovery, packaging, and release evidence. It is not a disclosure of advanced
product plans, commercial features, pricing, or internal implementation phases.

## Current public focus

1. Complete native correctness and lifecycle evidence on Windows, Linux, and
   macOS.
2. Improve startup, input, PTY, resize, rendering, shutdown, and long-session
   resource behavior.
3. Finish accessibility evidence for keyboard navigation, focus, high contrast,
   reduced motion, scaling, and supported screen readers.
4. Make configuration, migration, recovery, cleanup, and uninstall predictable.
5. Preserve terminal compatibility, Unicode behavior, image protocols, shell
   integration, search, selection, and clipboard behavior.
6. Complete signed packaging and public release evidence before claiming stable
   installers.
7. Improve getting-started, reference, troubleshooting, contributor, and manual
   testing documentation.
8. Keep optional source foundations disabled or release-gated until their exact
   security, native, performance, resource, and accessibility gates pass.

### Public Linux distribution status

Repository support exists for x64 and Arm64 Linux package formats, manifests,
checksums, release policy, and verification. A source-complete release path is
not evidence that a public package exists.

The exact current status and remaining external gates are documented in
[Public Linux Early Access distribution](PUBLIC-RELEASE-DISTRIBUTION.md).

### Build and CI performance status

| Status | Feature | Remaining requirement |
|---|---|---|
| **Fully done (source and policy)** | Readiness avoids the duplicate pre-Clippy Cargo check; quality jobs use a reviewed, versioned compiler cache; Cargo source caches share a lockfile-bound identity; native x64/Arm64 package jobs run beside quality while remaining cold; nFPM uses checksum-pinned native archives; downstream jobs join all required evidence; semantic mutation tests guard every boundary | Preserve the cold package boundary and rerun all mutations for every workflow or toolchain change. |
| **Fully done (exact hosted evidence)** | Exact-commit PR CI and credential-free cold/warm release rehearsals passed; both native package lifecycles remained cold; cache counters and job timestamps are retained in runs `33593759776` and `33593793029` | Preserve the evidence links and repeat the measurement after material workflow, dependency, toolchain, or cache-generation changes. |
| **Fully done (bounded measured claim)** | For the linked rehearsal only, the whole path improved from 37m11s to 25m56s cold and 15m01s warm (30.3% and 59.6%); the warm release quality job was 52.9% faster and the independent ordinary PR quality/test job improved from 38m20s to 13m44s (64.2%) | Do not generalize these figures: GitHub runner load, eviction, quota, rate limits, and future dependency graphs remain external service conditions. |

The complete design and evidence ledger is
[CI and build performance plan](CI-BUILD-PERFORMANCE-PLAN.md).

## Direction after the first stable release

Only high-level category labels are public: diagnostic work, production
operations, Automation Studio, optional LLM orchestration, video editing, and
other specialized extensions. Their requirements, architecture, sequencing,
business model, pricing, and validation plans remain private. Listing a category
here is not a feature promise, implementation claim, schedule, or commercial
offer.

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

Unreleased advanced capabilities, specialist extensions, commercial products,
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
