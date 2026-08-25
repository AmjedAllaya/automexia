# Multi-cloud provider adapter testing

This page owns focused evidence that would exceed the byte budget of the main
[testing guide](TESTING.md). Provider adapters remain disabled and
nonactivated. The cached Providers catalog/review and M11 private transient
lifecycle are product-connected locally, but a source or UI test is not evidence
that a real provider, network, credential cache, connection, cluster, or native
process tree ran.

## M8-M12 cached provider product and M11 private lifecycle

Run the product-boundary evidence with:

```text
cargo test -p automexia-terminal --lib --locked providers
cargo test -p automexia-terminal --lib --locked provider_transients
cargo test -p automexia-terminal --test m8_m12_provider_product --locked
cargo test -p automexia-ui-model --locked provider_catalog
cargo test -p automexia-terminal --lib --locked provider_catalog
cargo clippy -p automexia-terminal -p automexia-ui-model --all-targets --locked -- -D warnings
cargo bench -p automexia-terminal --bench connection_catalog --locked -- provider_catalog_6_cached_projection --sample-size 100
```

The regression set freezes a six-row cached-only catalog, public bounded
projection, responsive 320/1,920/5,120 px and 300% layouts, semantic status and
focus, exact keyboard/pointer ownership, disabled action and PTY input, stale
review invalidation, unsupported OpenBao rejection, strict new-capsule/new-
session/higher-revision replacement, exact revoke, and runtime shutdown.

The M11 manager is the only app-owned private output lifecycle. Its Windows
fixtures cover all six accepted provider relations, valid/invalid/oversized/
expired data, cross-session and stale-generation denial, post-publication tamper,
path/debug/secret canaries, provider/session revoke, shutdown/drop, and 64
repeated full-capacity cycles (1,024 files). Files are capped at 1 MiB, active
handles at 16, stale recovery at 64 roots/files older than 24 hours, and every
candidate is validated by the canonical M11 kubeconfig parser before an opaque
handle is published. The Unix symlink case is compiled but did not run on the
Windows host.

A Windows x86_64 optimized 100-sample run of the six-provider cached projection
measured 4.8751-4.9453 microseconds, with 12 high-side outliers (eight mild, four
severe). It is same-host local evidence, not a controlled release ratchet.

No provider CLI, account, browser/device/MFA flow, cloud or cluster network,
credential/token/certificate cache, client plugin, managed provider child, or
PTY ran. Controlled native pixels/screen readers, Unix no-follow execution,
real descendant cleanup/resources, packaging, signing, and release fixtures
remain external. OpenBao has no implementation because ADR 0024 is unaccepted.

## M10 Google Cloud adapter source contracts

Run the deterministic M10 source gates with:

```text
cargo test -p automexia-devops-gcp --locked
cargo clippy -p automexia-devops-gcp --all-targets --all-features --locked -- -D warnings
cargo test -p automexia-terminal gcp_is_independently_registered_and_disabled --lib --locked
cargo bench -p automexia-devops-gcp --bench provider --locked
```

Eight Windows x86_64 tests cover the disabled least-privilege manifest;
256 KiB/64-section/512-entry/4 KiB-field limits; duplicate, invalid UTF-8,
hostile, external-credential/token-key, and caller-created metadata rejection;
exact named-configuration user browser/remote login and public project/IAM
observation without global activation; opaque Workforce/Workload configuration
references; project/zone-bound IAP with gcloud-owned SSH keys and OS Login;
M11-only private `KUBECONFIG` GKE output; explicit non-ready failures; version
bounds; and redaction. The near-limit Criterion target measured
444.00–460.66 µs over 100 samples and reported 9 high-side outliers. Criterion
compared it with the earlier ~144 KiB fixture and reported a 60.9–71.7%
increase; that comparison changed the workload to ~252 KiB and is not a
same-input code regression.

No gcloud executable, browser/2FA/federation flow, Google network, credential
database, IAP/SSH process, OS Login, GKE cluster, kubeconfig file, active provider action, Linux/macOS native, accessibility,
resource, packaging, signing, or release fixture ran. The cached product review
and private transient lifecycle tests above do not run gcloud. D3 activation/
attestation and real
process-tree cleanup, and controlled native/provider/release evidence remain.

## M11 Kubernetes and OpenShift source contracts

Run the deterministic M11 source gates with:

```text
cargo test -p automexia-devops-kubernetes --locked
cargo test -p automexia-devops-openshift --locked
cargo clippy -p automexia-devops-kubernetes -p automexia-devops-openshift --all-targets --all-features --locked -- -D warnings
cargo test -p automexia-terminal kubernetes_is_independently_registered_and_disabled --lib --locked
cargo test -p automexia-terminal openshift_is_independently_registered_and_disabled --lib --locked
cargo bench -p automexia-devops-kubernetes --bench kubeconfig --locked -- --sample-size 50
cargo deny --locked --color never check --hide-inclusion-graph
```

Eight Kubernetes and five OpenShift Windows x86_64 tests cover independent
disabled manifests; exact absolute/private-transient grants; stable regular-file
identity and source-revision checks; 1 MiB input, 16-source, 256-item, and YAML
structural budgets; JSON compatibility; controls/bidi; external credential path,
auth-provider, proxy, non-loopback HTTP, duplicate, reference, and merge
rejection; deterministic first-current-context precedence; public-only metadata;
AWS EKS, Azure AKS, and Google GKE transient isolation; default-denied exec plus
exact digest/argv/environment-name/interactivity/session/deadline/output/tree-
cancellation review; secret-argument and token/key/value redaction; production
TLS policy; capsule pinning; exact `kubectl auth whoami`, config view, exec, `oc`
private-output web login, project view, and rsh argv; cross-session denial;
version compatibility; explicit missing/expired/MFA/cancelled/offline/denied/
plugin-failed/unsupported/error states; and disable registration. Doc tests and
warning-denied all-target Clippy pass.

The 50-sample optimized 900 KiB parser benchmark measured 1.8280–1.8788 ms and
reported eight high-side outliers (three mild, five severe). The first benchmark
fixture intentionally fragmented padding across tens of thousands of comments;
the parser rejected it before timing because it exceeded the 20,000-event
budget. The corrected same-size fixture uses one comment and measures the
maximum-valid byte workload without weakening the structural denial-of-service
limit. This is local evidence, not a controlled release ratchet.

`serde-saphyr` 1.1.0 is pinned with deserialization only and no filesystem
include feature. Official metadata reports MIT OR Apache-2.0; `cargo deny`
passed advisories, bans, licenses, and sources. The dependency provides typed
Serde parsing, duplicate-key errors, merge-key denial, and configurable budgets;
the archived `serde_yaml` and a handwritten Kubernetes parser were rejected.

No `kubectl` or `oc` executable, exec plugin, browser, cloud/cluster network,
credential, EKS/AKS/GKE/OpenShift cluster, active provider action, PTY, or user
kubeconfig ran. Windows exercised pure/parser/native file checks plus the
app-owned private transient lifecycle; the Unix symlink test was compiled but
did not run on Windows. D3 activation, real clients and controlled clusters,
plugin execution, forced descendant cleanup, sustained resource/storage evidence,
Linux/macOS native runs, accessibility, packaging, signing, and release evidence
remain external.
## M12 Teleport source contracts

Run the deterministic M12 source gates with:

```text
cargo test -p automexia-devops-teleport --locked
cargo clippy -p automexia-devops-teleport --all-targets --all-features --locked -- -D warnings
cargo test -p automexia-terminal --locked automexia::builtins::tests::teleport_is_independently_registered_and_disabled
cargo bench -p automexia-devops-teleport --bench status --locked -- --sample-size 100 --warm-up-time 5 --measurement-time 10
cargo deny check advisories bans licenses sources
```

Eleven Windows x86_64 integration contracts cover the independent disabled
least-privilege manifest; 256 KiB output, 4,096-node, depth-16, combined
32-profile, 64-item, and 4 KiB public-field limits; malformed/hostile/
unknown-sensitive/ambient status denial and benign unretained-field discard; duplicate profiles; normalized HTTPS proxies;
strict current proxy/cluster/user selection; RFC 3339 current/expiring/expired
state; exact reviewed Teleport 18.10+ version, local status, browser/no-browser
login, logout, and SSH argv; `--add-keys-to-agent=no`; cleared SSH-agent and
Teleport environment overrides; `--relogin=false`; `--request-mode=off`;
process/network scope; output/deadline/tree-cancellation ceilings; explicit
missing/expired/revoked/MFA/cancelled/offline/denied/unsupported/error states;
capsule/session/revision/proxy drift; disable/uninstall; and debug/error
redaction. Unretained traits, certificates, tokens, identity/cache/agent
material, and inherited environment never enter the public model.

The optimized benchmark parses and classifies a representative current profile.
A 50-sample pre-limit run measured 17.500–17.775 µs with two high-side outliers.
The first post-change 50-sample comparison measured 18.138–19.040 µs and
Criterion flagged +3.9185% to +9.0589% (`p = 0`) with eleven high-side outliers.
That signal was investigated with a longer 100-sample, 5-second warm-up,
10-second measurement run: 18.359–18.945 µs, comparison interval -3.2458% to
+2.8032%, `p = 0.82`, and no statistically significant change, with nine
high-side outliers (three mild, six severe). The noisy first signal remains
recorded; this same-host microbenchmark is not a controlled release ratchet.

The adapter pins `time` 0.3.55 with only `std` and `parsing` to decode Teleport
RFC 3339 expiry. The feature set performs no I/O, clock lookup, formatting,
locale, or credential work; callers supply comparison time. The crate is MIT OR
Apache-2.0, the reviewed release includes the upstream RFC 2822
stack-exhaustion fix, and locked `cargo deny` advisories, bans, licenses, and
sources checks pass. A handwritten timestamp parser was rejected in favor of
this bounded maintained primitive.

No `tsh` executable, proxy/network, browser/MFA/hardware-key flow, `~/.tsh`
profile/cache/certificate, SSH agent, access request, real SSH process/PTY,
active provider action, Linux/macOS native host, accessibility, forced descendant
cleanup, sustained resource/storage, packaging, signing, or release fixture ran.
D3 activation/attestation and controlled Teleport/native/release evidence remain
external. OpenBao has no implementation or test owner because proposed ADR 0024
has not been accepted.
### Full M12 repository gate (2026-08-23)

The required post-change commands passed on Windows x86_64:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked --profile ci
cargo test --workspace --doc --locked
python3 tools/ci/qa.py --full
cargo ready
```

The successful clean Nextest run executed 1,915 tests across 64 binaries: all
1,915 passed and seven were profile-skipped. Workspace documentation tests
included 46 passing Corcovado examples plus 18 passing and three intentionally
ignored rio-window examples. Full QA passed repository contracts, shell tests,
formats, CP5 research checks/benchmark, workspace Clippy/Nextest/doctests,
resize stress, session clone, Loom readiness, dependency policy, and JUnit
artifact generation. Its local report is under
`target/qa/20260823T051537Z-24740/report.html`.

The first Nextest attempt failed during linking because the workspace drive had
about 112 KiB free, not because a test failed. Inspection found about 32 GiB of
rebuildable Cargo artifacts and one generated Automexia process holding a
runtime executable. Only that validated generated process was stopped; `cargo
clean` removed 31,656 files/32.0 GiB. The same exact Nextest command then built
from a clean target and passed. This initial infrastructure failure remains part
of the evidence and is not counted as a passing test attempt.

`cargo ready` also passed its cold isolated all-target check, warning-denied
Clippy, unit/integration/documentation tests, dependency policy, persistent app
build, and `automexia 0.4.0` smoke. The isolated verification artifacts were
removed after the run. Controlled native OpenSSH, interactive Windows GPU,
Application Verifier, WPR, named-hardware benchmarks, coverage, 30-day baseline,
Linux/macOS GPU, and screen-reader evidence remain explicitly external.

### M8-M12 cached provider product and private lifecycle gate (2026-08-25)

The completed local M8-M12 change passed the required contributor commands on
Windows x86_64:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked --profile ci
cargo test --workspace --doc --locked
python3 tools/ci/qa.py --full
cargo ready
```

The clean Nextest run executed 2,110 tests: all 2,110 passed and seven were
profile-skipped. Documentation tests included 46 passing Corcovado examples plus
18 passing and three intentionally ignored rio-window examples. Full QA passed
all locally executable stages; its report is
`target/qa/20260825T052759Z-39376/report.html`. The optimized six-provider cached
catalog projection measured 4.8751-4.9453 microseconds over 100 samples, with
eight mild and four severe high-side outliers. This same-host measurement is not
a controlled release ratchet.

The persistent workspace-drive `cargo ready` preflight correctly refused to
start with 1.17 GiB free against its 12 GiB minimum. The identical readiness
workflow then passed from a verified disposable system-temporary target with
22.27 GiB available: clean all-target checks, warning-denied Clippy, the complete
unit/integration/documentation suite, dependency policy, application build, and
the `automexia 0.4.0` executable smoke all passed. The generated temporary target
was verified and removed after the run.

This is source, renderer-neutral, Windows file-lifecycle, and local build/test
evidence. Protected D3 activation and attestation, controlled live AWS/Azure/GCP/
Kubernetes/OpenShift/Teleport accounts and clients, network and browser/MFA
flows, native Linux/macOS lifecycle and symlink execution, forced descendant
cleanup, sustained named-hardware resource/storage campaigns, interactive GPU
and screen-reader checks, packaging, signing, and release evidence remain
external. OpenBao remains **Not done** until proposed ADR 0024 is accepted; no
OpenBao execution or credential owner was introduced.
