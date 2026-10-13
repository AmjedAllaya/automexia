---
name: automexia-isolated-validation
description: Select, verify, and manage disposable execution environments for Automexia builds, tests, native probes, fuzzing, benchmarks, and package validation. Use before runtime validation or when preparing a clean OS test setup; keep execution off the personal workstation and do not require a push for local candidate testing.
---

# Automexia Isolated Validation

Bind every executable check to a verified disposable environment and exact inputs. This skill defines the workflow; it does not imply that a VM, image, runner, or automation harness already exists.

## Choose the boundary before executing

1. Identify the claim and its existing test owner. Search applicable sections of `../../../docs/TESTING.md`, `../../../docs/CI-ASSURANCE.md`, current workflows, and package policy rather than loading every matrix.
2. Inspect available environments read-only: OS/architecture, virtualization support, approved hypervisor/runner, trusted baseline, free resources, and current guest state. Do not install services, reboot, or alter unrelated VMs merely to inspect them.
3. Use [references/environment-contract.md](references/environment-contract.md) to select a builder, clean customer, native desktop, or controlled performance environment. Record what it can and cannot prove.
4. Reuse a repository-owned driver when it meets the contract. If one is missing, implementation belongs in tracked tooling with focused tests, not `.automexia-private/`. Do not disguise a sequence of manual commands as an existing automated harness.
5. Use existing user authorization for routine setup and cleanup within scope. If an unavailable ISO, license, hardware, permission, or interactive setup is required, give the exact prerequisite and continue safe source work. Do not fall back to execution on the personal workstation.

Host-side file inspection, source packaging, result inspection, and hypervisor orchestration are allowed. Run application code, project builds/tests, fuzzers, and benchmarks inside the selected guest/ephemeral runner. A new host user, changed HOME, cleared cache, container label, or CI environment variable alone does not establish a clean OS boundary.

## Freeze the inputs without a push

- Identify the checkout revision and task-owned local changes, including required untracked files. Do not silently omit dirty inputs or mix an older binary with newer source.
- Stage an explicit source allowlist or reviewed patch with a digest. Exclude private documentation, personal profiles, credentials, signing material, `.git` credentials/configuration, build caches, and unrelated work. Reject unexpected links and path escapes.
- Transfer through a bounded, dedicated staging location or authenticated guest channel. Prefer a read-only input mapping and a separate owned output directory; never map the user's home or whole checkout read-write.
- Verify the manifest/digests inside the guest. Record the exact source identity used to build and the artifact identity used for package probes. No commit or push is required to test a local candidate.

## Execute with owned state

1. Prefer an existing authorized VM with verified resettable snapshots when available. A single VM can alternate between customer and builder snapshots; additional clones are optional and require authorization. Record the exact instance, baseline, owned paths, reset method, and cleanup plan before launch. Preserve the baseline and any user-owned state.
2. Verify reset conditions in the guest: expected OS, architecture, prerequisites, fresh application state, shell/profile policy, and absence of unintended host mounts or credentials. A snapshot name is not sufficient proof.
3. Keep builder and customer states separate, using snapshots of one VM or separately authorized instances. Restore and verify the customer state before installation and first-run checks; builder tools must not supply missing product dependencies. Record intentional dependencies and network use.
4. Invoke existing repository entrypoints with typed argument arrays, pinned toolchain/lockfiles, explicit feature sets, bounded runtime/resources, and per-run mutable storage. Use the focused-to-owning-to-affected-gates ladder; `tools/ci/qa.py --full --bundle` is a broader owner, not a reason to run full QA for every edit.
5. Before GUI automation, verify a usable interactive desktop, readiness, renderer, focus, scale, and input ownership. A running VM without an interactive session is not GUI evidence.
6. Preserve existing host/runner guards. Never set `GITHUB_ACTIONS` or similar flags to impersonate a hosted runner. A new guest execution mode needs an explicit reviewed boundary, proof of the actual guest identity, and negative tests that reject host execution.
7. Collect exit status, executed/skipped cases, bounded redacted diagnostics, artifact digests, and cleanup observations. Preserve failed attempts; do not publish raw guest profiles or environment values.

## Close the lifecycle

In success, failure, timeout, or interruption paths, retain ownership of guest processes and instance cleanup. Export permitted evidence before teardown. For a reused user-owned VM, follow the agreed stop/restore/retain plan; never delete it as disposable infrastructure. Remove only an explicitly authorized task-created instance and its verified disposable disks after confirming it has stopped.

Resolve canonical paths and check containment, links/reparse points, ownership markers, and instance IDs before deletion. Never delete by a broad VM name prefix, guessed PID, or a path taken from untrusted output. Preserve baseline images and unrelated VMs. If teardown fails, report the retained instance and recovery action; do not report a clean environment or abandon a running workload.

Do not reuse failed-run state for a new-user claim. Reset from the baseline for the next independent case; supported upgrade tests deliberately introduce only their declared prior-version state.

## Evidence and completion

Report the source/package identity, guest baseline and reset method, executed commands and nonzero case counts, observed behavior, environment limitations, and final lifecycle state. Distinguish package installation, model tests, native desktop behavior, and controlled performance evidence.

Unavailable native OS/hardware, a missing baseline, or an unimplemented driver is **not run**, not a pass. A hosted runner's fresh job is not proof that it lacks developer tooling. No network push, private-file transfer, credential change, or unrelated host cleanup is implied by this skill.
