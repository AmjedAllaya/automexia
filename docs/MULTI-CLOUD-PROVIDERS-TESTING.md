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
fixture ran. D3 activation/attestation, M11 private kubeconfig ownership, real
process-tree cleanup, and controlled native/provider/release evidence remain.
