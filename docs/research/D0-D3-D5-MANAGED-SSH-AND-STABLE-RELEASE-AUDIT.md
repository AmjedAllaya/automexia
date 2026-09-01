# D0/D3/D5 managed SSH and stable-release completion audit

Date: 2026-08-27

Status: **local source and assurance gaps fixed; production activation and
stable-release evidence remain partial/external.**

This audit is the evidence ledger for the 2026-08-27 re-audit. It does not
authorize a release, enable managed SSH, accept an unverified package, or treat
synthetic/cross-compiled evidence as a native result. ADR 0012 is accepted, but
ADR 0003's two independent exact-head protected approvals, enforcement, and the
controlled evidence below remain mandatory.

## Placement decision

| Concern | Owner | Decision |
|---|---|---|
| Provider-neutral direct/routed SSH, trust, and typed-tunnel policy | `automexia-connectivity::connections` | Keep capability-free models reusable and off terminal hot paths. |
| Package/capability review, exact executable identity, request generation, receipt creation, and launch lease | Application-owned broker/runner under `apps/automexia-terminal/src/context` | Process/PTy/route authority is fundamental application authority and must not be duplicated by an extension. |
| Static public OpenSSH inventory | Existing `extensions/devops-ssh` extension | It remains an optional bounded filesystem-reading domain with no launch/network/credential authority. |
| Native/release evidence | `tools/ci`, protected workflows, and versioned fixtures | Evidence validation is release infrastructure, not runtime authority. |

Putting provider/network policy directly in terminal input, PTY, renderer, or
startup paths was rejected. A new extension was also rejected: it would split
one cohesive SSH inventory domain or duplicate the application launch broker.
No dependency or capability boundary was added.

## Evidence ledger

| Item | Classification after this pass | Local evidence | Remaining exit |
|---|---|---|---|
| Stable-release source policy | Fully implemented in source | Existing exact tag/main/fork/DCO, hosted workflow, repository protection, S1, S2, packaging, signing, artifact, and mutation contracts remain intact. | Execute protected hosted/native/signing/notarization/accessibility/human-review gates and activate the reviewed 30-day S2 baseline. |
| D0 decision/threat/compatibility contract | Fully implemented locally; phase partial overall | Active schema 7 preserves schemas 1-6 by SHA-256, freezes authority/argv/trust/tunnel/receipt/native evidence, and remains production-disabled. | Two exact-head protected approvals, server enforcement, controlled native results, and attestation. |
| D3 exact-argv launch broker source | Fully implemented locally; nonactivated | One application runner, exact identity/argv/lease, publish-before-use, real child outcome, bounded audits/receipts, process-tree teardown, and permanent request-ID exhaustion before wraparound. | Verified first-party loader identity/revocation plus real platform PTY/process/resource/accessibility evidence. |
| D5.0/D5.1 models and read-only Hub | Fully implemented locally; native evidence partial | Existing review-only records, responsive Hub, read-only inventory/library, and capability-free planning remain unchanged. | Controlled native picker, permissions, pixels, and assistive-technology evidence. |
| F5.1 direct SSH / F5.2 routes and trust / F5.3 tunnels | Fully implemented locally; nonactivated | Exact shell-free argv; current executable and review equality; complete host trust; safe copy; typed configuration-free tunnels; strong per-use review; bounded lifecycle; one opaque operation-scoped receipt reference per reviewed tunnel. | Actual managed status/OpenSSH execution and controlled descendant/listener cleanup. |
| F5.4 native evidence path | Fully implemented locally; real results external | Schema 2 binds clean commit, native tuple, application version/binary/package, four fixed OpenSSH tools, OpenSSH 10.5 controls, advisory review, package provenance, fixture, scenarios, resources, cleanup, and redaction. The workflow is manual, protected, exact, fail-closed, and uploads one path-free 90-day summary. | Real private manifests for Windows, macOS, and Linux x86_64/aarch64 claims; WSL remains separately denied. |

## Defects found and fixed

1. The review request counter used wrapping atomic addition. At exhaustion it
   rejected only ID zero, then could reuse ID one and collide with stale UI
   ownership. It now uses compare-and-update semantics and permanently rejects
   `MAX`, zero, and every later allocation. Tests cover boundary-minus-one,
   exhausted, repeated exhausted, zero, no publication, and no completion.
2. Managed completion receipts always emitted an empty tunnel ownership list.
   The runner now derives the reviewed tunnel count before launch and emits one
   generated `managed-tunnel-<operation>-<ordinal>` opaque reference per tunnel.
   No user tunnel ID, listener, target endpoint, destination, or terminal text
   is copied into the receipt.
3. Native evidence accepted an application version string without executing the
   exact binary and hashed only the OpenSSH client. Schema 2 performs typed,
   bounded `automexia --version`, hashes all four fixed OpenSSH tools, rejects
   links/reparse points, and binds advisory and package-provenance artifacts.
4. The F5 workflow checker accepted substring conditions such as an appended
   `|| true`, loose runner labels, unbounded timeout drift, variable-backed
   private paths, and artifact drift. The checker now validates exact trigger,
   inputs, concurrency, condition, runner, GitHub-Free private-environment
   exclusion, secret-backed
   paths, timeout, checkout, platform command sets, fail-closed behavior, and
   one exact summary upload. Mutations prove each weakening fails.

## Executed local evidence

- Focused source suites passed: 79 `automexia-connectivity` tests, 32 broker
  lifecycle tests, 4 OpenSSH review-worker tests, 7 application direct-SSH
  composition tests, and 4 UI-model direct-SSH tests.
- Contract assurance passed: schema-7 checker plus 10 mutations; schema-2
  native-evidence validator with 12 passed and one Windows symlink-privilege
  skip; 35 F5 workflow, 23 repository-protection, 12 PR-policy, 10 feature-
  reinforcement, 14 S1, 17 S2/performance, and 17 stable-release mutations.
- Criterion measured `direct_openssh_prepare_selected` at
  7.5300-7.5767 microseconds on this uncontrolled Windows host. This is useful
  local latency evidence, not an active release ratchet or named-hardware
  baseline.
- Rustfmt and warning-denied all-target workspace Clippy passed. CI-profile
  Nextest passed 2,270 tests with 7 declared skips. Documentation tests passed
  64 with 3 ignored. Repository validation passed all structured, documentation,
  policy, roadmap, and assurance inventories.
- Full QA passed its policy, architecture, repository, shell, research,
  warning-denied lint, Nextest, documentation, resize-stress, session-clone,
  Loom, and dependency stages. The first run failed after the drive reached
  20 KiB free; only the isolated worktree's 14.6-GiB regenerable Cargo target
  was purged, and the complete QA run was repeated successfully.
- `cargo ready` passed cold all-target checks, Clippy, unit/integration/
  documentation tests, dependency policy, fresh application build, and the
  `automexia 0.4.0` version smoke using a short native C: target. A first
  deep-worktree attempt failed with Windows linker `LNK1104`; the short target
  removed the path-depth condition. Readiness removed its 9.44-GiB isolated
  tree, and the remaining 4.5-GiB temporary target was removed with Cargo.
- The read-only prerequisite probe reports OpenSSH for Windows 9.5p2 client
  tools and no fixed `sshd`; it returned the expected external-prerequisite
  status without installing, connecting, or changing SSH configuration.

## OpenSSH security decision

The upstream baseline is [OpenSSH 10.5, released 2026-08-11](https://www.openbsd.org/openssh/releasenotes.html).
It includes fixes relevant to locked `ssh-agent` session binding/restricted-key
behavior and pending remote-forward cleanup. The existing contract continues to
observe the hybrid post-quantum default and non-post-quantum warning without
overriding system crypto policy. A numeric version comparison was rejected
because supported OS vendors may backport fixes; the validator instead binds
the exact binaries, a reviewed advisory artifact, package provenance, and
explicit required security outcomes.

## Stable-release and activation gates still open

- Keep `MANAGED_SESSION_LAUNCH_ENABLED` false and the linked package
  `Unverified` until the accepted protected process is complete.
- Obtain two independent exact-head approvals and verify protected-path server
  enforcement without self-review or bot substitution.
- Supply real loader attestation and revocation for the exact first-party
  package/contract identity.
- Run the 23-case loopback OpenSSH matrix on each claimed native OS and
  architecture, including host keys, encrypted keys, agent/certificate,
  ProxyJump, all tunnel kinds, collision, cancellation, hostile output,
  offline, exit status, and shutdown.
- Record 1/10/50 latency, CPU, memory, handles/descriptors, children, PTYs,
  listeners, tunnels, tasks, routes, cache/log/storage, and zero final cleanup.
- Complete native visual and Narrator/NVDA, VoiceOver, and Orca evidence plus
  independent review.
- Complete the ordinary stable-release blockers: GitHub-Free manual-governance
  integration
  on `main`, S1 controlled evidence, 30 reviewed S2 days and activation, final
  signing/notarization/package evidence, and authenticated repository audit.

The generic stable release must not claim managed SSH. A later release that
advertises D5.2 must run the separate F5 protected workflow for the exact same
reviewed commit and final artifacts before tag publication.

## Verification ladder

```text
cargo test -p automexia-connectivity --locked
cargo test -p automexia-terminal --locked context::launch_broker::tests
cargo test -p automexia-terminal --locked context::external_tool_runner::openssh_review_worker_tests
cargo test -p automexia-terminal --locked automexia::connections::direct_openssh::tests
cargo test -p automexia-ui-model --test direct_openssh_review --locked
python3 tools/ci/check_session_launch_d0.py
python3 tools/ci/test_session_launch_d0.py
python3 tools/ci/test_native_openssh_evidence.py
python3 tools/ci/test_platform_coverage.py
python3 tools/ci/test_repository_protection.py
```

The full contributor gate remains `cargo ready`; CI-equivalent and manual
native F5 gates are additive. A filtered command that executes zero tests is not
accepted as evidence; explicit test targets are required.

## Rollback

Rollback is a normal revert of schema 7, its validator/workflow hardening, and
the broker lifecycle fix. Do not edit schemas 1-6 or the schema-1 synthetic
fixture: their exact digests are historical evidence. Rollback must leave
production activation false and rerun the full D0/F5 mutation stack.
