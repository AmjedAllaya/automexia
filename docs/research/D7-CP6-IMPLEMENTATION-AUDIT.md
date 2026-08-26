# D7/CP6 ecosystem and selected-input model suggestion implementation audit

Status: Partially done overall; fully done locally at the accepted source boundary. Component execution, public downloads and SDK publication, provider calls, and stable-release claims remain disabled or externally gated.

Accepted contract SHA-256:
fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4

Acceptance: ADR 0029 and the exact contract digest were accepted on 2026-08-25 for source implementation. The immutable acceptance receipt is tests/fixtures/ecosystem/d7-cp6-acceptance-v1.json. It does not grant process, network, credential, PTY, provider, public-distribution, or component-execution authority.

## Outcome, scope, and non-goals

The local result is a private, capability-free policy crate plus an explicit local-package/runtime adapter. A person can inspect a signed local package, review its exact identity and access, install it disabled into private atomic storage, review a signed action pack, and remove or disable owned package state. The optional CP6 path can build a consent snapshot from explicitly selected text and validate a typed explanation or suggestion. It cannot call a provider, use tools, press Enter, start a process, write a file, access credentials, read a terminal grid, or execute a component.

The work does not publish a public SDK or registry, activate a downloader, add startup or typing network activity, enable broad WASI, replace the PTY/session/editor/renderer/process owners, or authorize the separate LO0-LO5 orchestration proposal. Built-in first-party extensions and CP1-CP3 remain the fallback.

The accepted logical world identifier is automexia:ecosystem/suggestion@1. The valid WIT source world is named extension because WIT package interfaces and worlds share one item namespace and the contract also requires the imported suggestion interface. The acceptance receipt freezes this syntax mapping without modifying the accepted contract.

## Evidence ledger

| Item | Before implementation | Resulting local status | Owner and proof | Still required |
|---|---|---|---|---|
| Accepted boundary | Partial/proposed | Fully done locally | Accepted ADR 0029, immutable proposal contract, separate acceptance receipt, duplicate-key and digest checker | None for source; release authority remains separate |
| Private domain model | Not implemented | Fully done locally | automexia-ecosystem: strict JSON, manifest, paths, capabilities, lifecycle, distribution policy, CP6 consent/response, renderer-neutral UI | Native product rendering remains external |
| Signed local bundles | Not implemented | Fully done locally | automexia-ecosystem-runtime/package.rs; exact Ed25519, provenance, SPDX/license, digest, timestamp, trust and revocation verification before extraction | Production trust-root governance and compromised-key drills |
| Atomic private store | Not implemented | Fully done locally on Windows x86_64 | no-follow staging, bounded expansion/cache, fsync/write-through publication, protected current-user DACL, crash recovery, two retained generations, exact uninstall | Native Unix/macOS permission/runtime evidence, disk-full/read-only campaigns |
| Component host | Not implemented | Fully done locally as disabled conformance source | Wasmtime 48.0.1 behind component-host; exact generated WIT host traits; no WASI; import inventory; fuel, epoch, memory, table, instance and joined worker tests | Protected release permit, native malicious-component/resource/soak evidence |
| Capability grants | Partial first-party only | Fully done locally for the D7 model | exact publisher/ID/version/digest/capability/scope/profile/expiry/generation binding; revocation and one-shot selected input | Persisted grant UX and native accessibility only when activation is authorized |
| Distribution/update | Not implemented | Partially done by design | disabled gate plus offline monotonic root/targets/snapshot/timestamp, expiry, consistent-snapshot and role-separation validation | Separate network ADR, adopted TUF client, transport/privacy, mirrors, root rotation and repository drills |
| WIT/SDK conformance | Not implemented | Partially done by design | versioned private WIT, real wit-parser test, private compatibility fixture and unpublished conformance README | Generated language bindings, public signing guide/support policy and publication are not authorized |
| Signed action packs | Not implemented | Fully done locally as non-executing source | verified immutable package maps into existing disabled typed QuickActionDocument; collisions, capability requests, raw execution and revocation fail closed | Native install/update/remove workflow and release UX |
| Selected-input model suggestions | Not implemented | Fully done locally at the no-provider boundary | exact selection, deterministic redaction, disclosure, review digest, single-use route/generation consent, strict response, independent risk and cancel-first surface | Provider/legal/privacy/native lifecycle evidence; provider calls remain disabled |
| Product adapter | Not implemented | Fully done locally at nonactivation boundary | explicit inspect/install-disabled/review/disable/kill/uninstall methods; marketplace truth status; activation/download/grant methods hard-deny | User-facing native file picker and rendered workflow only after release authorization |
| Assurance | Proposal-only | Partial overall | Windows tests, real ZIP fixtures, property tests, dependency policy, WIT parse, component tests, benchmarks and checker source | Linux/macOS native, screen readers, visual matrix, nightly fuzz campaign, 1,000 native cycles, 30-day soak, signed packages |

## Build, wrap, or adopt decision

Automexia builds the product-specific policy, typed contracts, capability and lifecycle state, consent, UI snapshots, cache ownership and rollback rules.

Automexia adopts:

- Wasmtime 48.0.1 with default features disabled and only Component Model, Cranelift, runtime, standard-library and pinned WAT support.
- ed25519-dalek 3.0.0 with alloc, fast and zeroize for strict Ed25519 verification.
- zip 8.6.0 with only the Rust deflate implementation. High-level extraction is intentionally not used; every path and entry is prevalidated and copied manually.
- unicode-normalization 0.1.25 for NFC boundary validation.
- wit-parser 0.254.0 and wat 1.254.0 only for exact conformance/test ownership.

Automexia does not adopt Sigstore or a TUF client in this slice. Public distribution remains false, so adding network/update authority would expand the accepted scope. A later network ADR must select the maintained repository implementation and re-run license, advisory, provenance, unsafe, binary/startup, platform, privacy, cancellation and rollback review.

The dependency gate passes with exact reviewed compatibility exceptions for Wasmtime 48's digest 0.10 and itertools 0.14 families. WAT is pinned to 1.254.0, removing the avoidable wasm-encoder/wasmparser 0.258 duplicates.

## Threat and resource model

The immutable contract retains nine threat owners and 28 ceilings. Implementation enforces:

- 16 MiB bundles, 32 MiB expanded content, 32 files, 512-byte paths, 64 KiB manifests, depth 16 and 4 KiB JSON strings;
- 8 MiB components, 64 imports, 32 capability requests, 64 MiB guest memory, 100,000 table elements and two instances;
- 10,000,000 fuel, 250 ms interactive and 2,000 ms explicit deadlines, 1 MiB host transfer/output and 64 KiB diagnostics;
- 16 queued calls per extension, two concurrent calls per extension, eight global calls and quarantine at three crashes in five minutes;
- 16 KiB selected model input, 64 KiB model responses, 256 MiB cache, two retained generations and 128 installed extensions.

All package, terminal, provider, model and guest values are untrusted. Paths reject traversal, links/reparse points, platform devices, case collisions, bidi/control characters, non-NFC text and option-shaped roots. Errors remain actionable but do not log selected text, keys, paths, credentials or provider data.

## Implementation checklist

### D7.0/CP6.0 — accepted fail-closed contract

- [x] Fully done: reconcile ADR 0003, existing first-party owners and roadmap authority.
- [x] Fully done: preserve the exact proposal JSON and canonical digest.
- [x] Fully done: record acceptance separately so the immutable proposal contract is not rewritten.
- [x] Fully done: mutation-check acceptance digest, dependency allowlist and every false release authority.
- [x] Fully done: register D7/CP6 in repository and architecture validation.
- [x] Fully done: keep execution, downloads, provider/tool calls, network, process, credential and PTY authority false.

### D7.1 — private manifest and package-domain crate

- [x] Fully done: add private publish=false automexia-ecosystem with no filesystem, process, network, runtime or ZIP ownership.
- [x] Fully done: strict duplicate/unknown/trailing JSON, exact schema/compatibility/world, unique imports/capabilities and safe identifiers/paths.
- [x] Fully done: reject oversized/deep/non-NFC/control/bidi/platform-hostile inputs with redacted errors.
- [x] Fully done: add 512-case properties and same-host policy benchmark.
- [x] Fully done: freeze exact dependencies and pure-domain architecture markers.
- [ ] External prerequisite: nightly sanitizer fuzz campaign beyond the compiled harness.

### D7.2 — signed local bundles and atomic lifecycle

- [x] Fully done: verify package/content/provenance/SBOM/license digests, publisher/key, Ed25519 signature, source revision/builder, trust validity, timestamps and current revocation before review/extraction.
- [x] Fully done: reject encrypted, linked, special, directory, duplicate/case-colliding, traversal and oversized archive entries.
- [x] Fully done: manually extract into a private no-follow staging tree; never call ZIP high-level extraction.
- [x] Fully done: disk/cache/extension preflight, fsync/write-through atomic publication, exact receipts, protected Windows DACL, last-known-good and two-generation retention.
- [x] Fully done: recover interrupted staging and a single interrupted third publish; reject unowned staging, cross-identity placement, linked/special state and invalid receipts.
- [x] Fully done: disable, global kill and exact owned-subtree uninstall preserve native shell/provider/credential/user state.
- [ ] External prerequisite: production trusted-root/publisher governance, key compromise and native Linux/macOS/disk-failure drills.

### D7.3 — sandboxed Component Model host

- [x] Fully done locally at conformance boundary: Wasmtime is optional behind component-host and absent from the ordinary terminal feature set.
- [x] Fully done: parse real WIT and generate typed host bindings for public-context, selected-input, suggestion and diagnostic.
- [x] Fully done: default host state returns no context/selection and denies suggestion, diagnostic and structured-operation publication.
- [x] Fully done: exact actual-versus-reviewed imports, no WASI, no process/network/filesystem/credential/PTY interfaces.
- [x] Fully done: fuel, epoch deadline, memory/table/instance limits, bounded output/log/transfer endpoints, joined worker/watcher, cancellation and public activation denial.
- [x] Fully done: typed grant revalidation and single-use selected input reject expiry, revocation, stale generations and cross-scope replay.
- [ ] External prerequisite: protected release permit and controlled native malicious-component, host-panic, large canonical ABI, concurrency, crash/resource and soak evidence.

### D7.4 — distribution, SDK, compatibility and revocation

- [x] Fully done locally: distribution gate is hard-disabled for startup, typing and explicit refresh.
- [x] Fully done locally: offline metadata rejects rollback, expiry, unsafe roles and inconsistent snapshots.
- [x] Fully done locally: versioned valid WIT and private compatibility/conformance source.
- [ ] Not done by accepted design: public downloader, mirrors, online metadata, retries and transport privacy require a separate network ADR.
- [ ] Not done by accepted design: generated public SDK packages, publishing, support/deprecation process and public signing/onboarding.
- [ ] External prerequisite: named root/repository/incident/support owners and controlled root-rotation/freeze/mix-and-match/emergency-revocation evidence.

### CP6.1 — signed ecosystem action packs

- [x] Fully done: only a verified action-pack package with no imports/capabilities maps into the existing typed Quick Action model.
- [x] Fully done: actions remain disabled, insert-only, no Enter, no alias/fixed cwd/raw execution, namespaced and provenance-bound.
- [x] Fully done: collisions, duplicate IDs, unsafe documents, revoked packages and capability expansion fail closed.
- [x] Fully done: final revalidation remains mandatory and the pack has no execution authority.
- [ ] External prerequisite: native install/update/remove/customization and fallback UX evidence.

### CP6.2 — optional isolated selected-input model suggestions

- [x] Fully done locally: explicit NFC selection is bounded to 16 KiB and Debug output is redacted.
- [x] Fully done locally: deterministic secret canaries are redacted before the consent digest and transfer envelope.
- [x] Fully done locally: review discloses redacted preview, provider/locality/model/destination/purpose/retention/environment, original/transferred bytes, redaction count and independent risk.
- [x] Fully done locally: consent is exact-digest, route/generation-bound and single-use.
- [x] Fully done locally: response is strict typed JSON, bounded, sanitized, route/generation checked and independently classified.
- [x] Fully done locally: provider calls, tools, MCP, background/typing requests and automatic execution are hard-disabled; output is review/copy/insert only.
- [ ] Not done by accepted design: a real local or remote provider adapter and account/transport lifecycle.
- [ ] External prerequisite: legal/privacy/retention approval and native offline/timeout/cancel/delete/accessibility/kill/uninstall evidence.

### D7.5/CP6.3 — product UX and release assurance

- [x] Fully done locally: compact responsive renderer-neutral package, capability, model-consent, disabled, quarantine and revocation surfaces.
- [x] Fully done locally: text plus icon/tone, cancel-first dangerous review, focus restoration, reduced motion, exact accessible labels, safe selected preview and no color-only meaning.
- [x] Fully done locally: app adapter and marketplace status truthfully expose signed review/disabled install and deny execution/download/provider calls.
- [x] Fully done locally: Windows x86_64 native package-store ACL/recovery and product-boundary tests.
- [ ] External prerequisite: rendered tiny-to-8K/100–300%/theme/localization/IME/modal/screen-reader matrix.
- [ ] External prerequisite: native Linux/macOS packages, signing/notarization, binary/startup/resource baselines, 1,000 lifecycle campaign and 30-day soak.

## Verification completed locally

Current focused evidence on Windows x86_64:

- cargo test -p automexia-ecosystem --locked: 12 unit, four 512-case property tests, zero doc tests passed.
- cargo test -p automexia-ecosystem-runtime --locked: package/action/store/WIT suites passed, including a real protected Windows DACL.
- cargo test -p automexia-ecosystem-runtime --features component-host --locked sandbox::tests: seven real Wasmtime tests passed.
- cargo test -p automexia-terminal --lib automexia::ecosystem::tests --locked: two product-boundary tests passed.
- cargo deny check: advisories, bans, licenses and sources passed.
- The hostile-bundle fuzz target compiled. A live campaign was attempted and correctly reported unavailable because the installed stable toolchain cannot use nightly sanitizer coverage.
- Criterion established named Windows x86_64 baselines for capability diff (329.03-341.25 ns), manifest validation (736.70-744.50 ns), grant authorization (341.19-346.91 ns), selection review (2.7259-2.7624 us), consent publication (2.9844-3.0055 us), and a real signed bundle verification (68.771-69.549 us). The earlier superseded policy comparison's noisy +5.46%/14-outlier result remains recorded rather than being treated as a pass.

Full contributor and QA results are recorded in ECOSYSTEM-PLATFORM-TESTING.md after the final gate run.

## Recovery, rollback, disable and uninstall

The source-level rollback is disabling or removing the private adapter/crates; ordinary terminal behavior does not depend on them. Runtime activation is false and the application does not enable the component-host feature.

Disable rotates the store generation and removes active lifecycle authority while retaining verified bytes. The kill switch denies new work, rotates all generations, clears selected-input truth and records closed endpoints/fallback availability. Uninstall validates a portable extension ID and deletes only its exact descendant under packages; it rejects links and special files and preserves files outside the owned root.

Revocation is rechecked before action-pack mapping and typed invocation. A revoked cached version cannot regain authority. Distribution refresh and model provider requests return explicit disabled errors.

## External prerequisites

The following are not source tasks that can be truthfully completed on this Windows workstation:

- ADR 0003 protected-path approvals and server enforcement for any future process, network, file-write, credential, AI-provider, downloader or component-execution authority;
- a separate accepted network/distribution ADR and maintained TUF-style implementation;
- production trusted-root, publisher onboarding, repository, compatibility, incident-response and SDK support owners;
- controlled Windows/Linux/macOS signed-package, sandbox, permissions, cleanup and architecture evidence;
- assistive-technology and rendered visual evidence across scale/layout/theme/localization/IME combinations;
- nightly long malicious-package/component fuzz campaigns;
- production provider privacy/legal/retention, offline/timeout/cancel/delete and account lifecycle evidence;
- measured cold/warm compile/invocation/cancel, startup/binary size, memory/handle/thread/storage cleanup, 1,000 native cycles and 30-day soak.

Until those gates pass, D7/CP6 is partially done overall and fully done only at the accepted non-activating source boundary.
