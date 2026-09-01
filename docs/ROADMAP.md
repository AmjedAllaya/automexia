# Automexia roadmap

This roadmap explains product direction. It is not a promise of dates, exact
scope, or release order. Current behavior is documented in the user guides and
verified by source and tests; planned work remains unavailable until its release
criteria are met.

## Product direction

Automexia is a flexible terminal that simplifies and accelerates many kinds of
workflows. It starts with a strong terminal experience and grows through
installable extensions. Each domain can add its own tools without forcing every
user to carry the same dependencies, permissions, or resource cost.

The first major domain is DevOps and SRE. Future domains can include scripting,
content work, and video editing while preserving the same lightweight core.

## Current public focus

- Stabilize terminal behavior, native platform support, accessibility,
  packaging, recovery, and long-session resource use.
- Complete release evidence for command productivity, context visibility,
  Connection Hub, SSH foundations, and provider-neutral extension boundaries.
- Keep all live provider, authentication, and mutating operations behind clear
  capability, review, and approval boundaries.
- Improve installation and user documentation for the first public release.

Some capabilities may exist in source but remain disabled or release-gated.
That status does not make them safe or supported for production use.

### First public Linux distribution status

| Status | Feature | Remaining requirement |
|---|---|---|
| **Fully done (source and repository setup)** | Exact x64/Arm64 DEB, RPM, and tar allowlist; bundle/manifest/checksum policy; credential-free native rehearsal; release/asset attestation gates; mutation tests; passive public archive; immutable releases; active default-branch and `v*` tag rulesets | Preserve these contracts and rerun them for every release change. |
| **Partially done (external release evidence required)** | Native Linux Early Access execution, real minisign bundle, scoped GitHub App publication, immutable post-upload and GitHub attestation evidence, and website activation handoff | Add an independent reviewer, configure the real signing key and App, integrate the reviewed source/website/archive branches, then pass the exact hosted x64/Arm64 jobs. |
| **Not done (no public package claimed)** | Publish the first real immutable prerelease and activate/download-smoke the landing-page routes | Publish only real verified packages, run the landing site's independent live verifier and preview review, deploy, and smoke every route from a signed-out client. |

The detailed source of truth is
[Public Linux Early Access distribution](PUBLIC-RELEASE-DISTRIBUTION.md). No
source-complete row is evidence that a package is publicly available.

## Direction after the first stable release

### DevOps/SRE workflows

Expand independently enabled SSH, cloud, Kubernetes, OpenShift,
infrastructure, and organization-access adapters. Make environment context,
connections, completion, safe actions, and recovery consistent across tools.

### Diagnostic navigation and production awareness

Add bounded navigation between error sections, then help users investigate
current evidence, understand impact, review relevant choices, and verify
recovery. Recommendations remain explainable and user-approved; Automexia does
not become an autonomous production operator.

### Automation Studio

Deliver an optional in-terminal editing workspace for scripts and operational
files after the first stable release and before the video-editing extension.
Domain extensions provide language and workflow knowledge while one editor
extension owns the editing experience.

### Extension ecosystem

Strengthen versioned capability contracts, package provenance, sandboxing,
permission review, disable and uninstall behavior, and replacement paths before
opening broader third-party distribution.

### Optional model capabilities

Keep small task-specific models narrow and local where they provide clear value.
Offer general LLM workflow automation only through a separate optional LLM
Orchestration extension. The core and first-party domain extensions do not
require an LLM or paid API.

### Future specialized workflow domains

After the terminal, DevOps/SRE, production guidance, and Automation Studio
foundations are mature, evaluate extensions for video editing and other fields.
These extensions reuse stable terminal and capability boundaries instead of
adding domain dependencies to the core.

## Release principles

Every capability moves from planned to supported only when its architecture,
security, resource limits, tests, native behavior, accessibility, documentation,
rollback, and release evidence agree. External platform or account testing is
reported as incomplete until it actually runs.

## Publication boundary

Public roadmaps describe user value, broad order, and safety commitments.
Exact unreleased algorithms, provider playbooks, internal UX state machines,
implementation maps, and phase-by-phase execution ledgers are maintained in a
local ignored workspace under the
[public/private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).

For current capabilities, start with [Features](FEATURES.md),
[Getting Started](GETTING-STARTED.md), the
[Quick Actions and aliases reference](DEVOPS-ALIASES.md), and the
[documentation index](index.md).
