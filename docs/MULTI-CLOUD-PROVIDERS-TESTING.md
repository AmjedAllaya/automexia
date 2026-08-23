# Multi-cloud provider adapter testing

This page owns focused evidence that would exceed the byte budget of the main
[testing guide](TESTING.md). Provider adapters remain disabled and
nonactivated; a source test is not evidence that a real provider, network,
credential cache, connection, cluster, or native process tree ran.

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
database, IAP/SSH process, OS Login, GKE cluster, kubeconfig file, product UI,
Linux/macOS native, accessibility, resource, packaging, signing, or release
fixture ran. D3 activation/attestation, M11 private transient-file lifecycle activation, real
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
credential, EKS/AKS/GKE/OpenShift cluster, product UI, PTY, or user kubeconfig
ran. Windows exercised pure/parser/native file checks; the Unix symlink test was
compiled but did not run on Windows. D3 activation, private transient-file
allocation/permissions/deletion, real clients and controlled clusters, plugin
execution, forced descendant cleanup, sustained resource/storage evidence,
Linux/macOS native runs, accessibility, packaging, signing, and release evidence
remain external.