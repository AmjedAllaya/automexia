# Sandboxed ecosystem and optional AI testing

Status: proposal evidence only. No package, sandbox, third-party component, AI
provider, product UI, or native lifecycle exists to test yet.

## Current offline gates

Run:

```text
python tools/ci/check_ecosystem_d7_cp6.py
python tools/ci/test_ecosystem_d7_cp6.py
```

These checks passed locally on Windows x86_64 with nine frozen threats, 28
exact resource ceilings, 12 verification domains, ten external gates, and 15
mutation tests. The checker rejects duplicate JSON keys, contract digest drift,
authority activation, ecosystem runtime/dependency adoption, broad WASI/imports,
ambient capabilities, stale provenance/revocation, distribution network,
weakened cleanup/fallback, AI ambient data/tool/automatic execution, unbounded
limits, and missing documents. It is registered in repository validation and
full QA.

No Rust ecosystem crate, package/archive/signature/update parser, WIT, Wasmtime
runtime, provider transport, product UI, native package behavior, benchmark,
fuzz target, accessibility surface, or stored user data was created. These
checks are proposal evidence, not a sandbox, package, privacy, or release pass.

## Future evidence matrix

| Future layer | Required pull-request evidence | Required native/nightly/release evidence |
|---|---|---|
| Manifest/package domain | Strict schemas, duplicate/unknown/version/Unicode/depth/size tests, capability diff, redacted receipts, property/fuzz, no-IO architecture check | Hostile archives, link/path containment, disk-full/read-only/interrupted atomic install and recovery on each claimed OS |
| Signature/update | Exact digest/publisher/root/time/provenance/SBOM/license/revocation fixtures, compromised-signer and metadata mutation tests | Root rotation, rollback/freeze/mix-and-match, expired/offline/emergency-revoke drills with production trust owners |
| Component host | Import/export/feature allowlist, fuel/deadline/memory/table/instance/host-transfer/output/log/concurrency limits, trap/panic/cancel/shutdown models | Malicious components, native process memory/handle/thread cleanup, binary/startup cost, 1,000 cycles and 30-day soak |
| Capability/review UX | Digest/version/scope/profile/expiry binding, upgrade diff, stale/cross-session denial, renderer-neutral states and goldens | Keyboard/IME/focus/modal, 100–300% scale, tiny-to-8K, localization/themes and controlled screen readers |
| CP6 action packs | Existing typed-action collision/risk/preview/stale/final-review rules plus revoked/updated/malicious pack fixtures | Signed install/update/remove/rollback and native shell fallback |
| Optional AI | Exact selected-input consent, redaction canaries, provider/locality/model/destination/retention disclosure, malformed/oversized/stale/prompt-injection tests, no tools/execute | Provider offline/timeout/cancel/delete, legal/privacy review, native accessible consent, kill/uninstall/fallback |

Runtime work may begin only after explicit acceptance of
[ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md) and the exact
[contract](../tests/fixtures/ecosystem/d7-cp6-ecosystem-contract-v1.json)
digest. Every authority-bearing slice remains subject to ADR 0003 protected
approvals and server enforcement. The ordered implementation and full external
gate inventory are in the
[D7/CP6 implementation audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md).

## Evidence reporting

Every future result must identify the exact commit, package digest, runtime and
SDK versions/features, operating system and architecture, hardware, cold/warm
state, fixtures, sample count/duration, limits, peak process and guest memory,
handles/threads/processes/files/storage before and after cleanup, and failures.
A cross-compile, synthetic component, or retry does not become native or stable
release evidence. Secret, selected-input, provider, credential, path, or package
content must not enter logs or reports.
