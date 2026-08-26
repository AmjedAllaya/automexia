
# Activation Hardening Delta Against the Audited Committed Baseline

**Recommended next execution phase**

Audited committed baseline:
`20ff7928ea2d1eac5d26f13c62d0cda9b86bc078`.

Use two independent lanes:

- **v0.4 release closure:** qualify the existing terminal product's declared
  native, security, accessibility, visual, packaging/signing, and sustained
  resource gates;
- **v0.5 activation hardening:** convert existing managed-SSH/provider source
  into controlled, native-tested, observable, reversible behavior.

## v0.5 P0 — process/provider activation

Close the gap between source and activation for:

```text
managed OpenSSH
direct SSH
routes/trust
tunnels
workspaces
auth capsules
AWS
Azure
GCP
Kubernetes/OpenShift
Teleport
provider-aware Quick Actions
```

For each owner, require:

```text
exact executable identity
constructed/sanitized environment
bounded output/input
explicit cancellation/process-tree cleanup
typed outcome
native success/failure evidence
credential/context isolation
release receipt bound to exact commit
```

## P0 — environment isolation

Provider execution must not inherit ambient authority blindly.

Review/remove/bind examples such as:

```text
AWS_PROFILE / AWS_* credential variables
AZURE_CONFIG_DIR / subscription state
CLOUDSDK_CONFIG / gcloud login config
KUBECONFIG / kuberc
SSH_AUTH_SOCK / SSH_ASKPASS
Teleport state/config
OpenBao token-helper environment if/when implemented
```

## P0 — native SSH lifecycle

Qualify:

```text
password
key
agent
MFA
host-key first use
host-key change
jump host
disconnect
network reset
resize
signal
exit status
cancel
reconnect policy
```

Do not replace OpenSSH with a custom protocol implementation.

## v0.4 P0 — release infrastructure

Finish:

```text
artifact signing
installer/package signing
update provenance
SBOM/dependency provenance
native installer tests
rollback/recovery
release evidence graph
```

## P0 — accessibility

Qualify existing renderer-native overlays and compatibility inspector using the real project accessibility owner.

Keyboard completeness remains mandatory.

Pointer controls are allowed and should dispatch the same semantic commands.

## P0 — sustained resource evidence

Run controlled:

```text
idle hours
long output
many tabs/sessions
provider churn
close/reopen cycles
parked top-level tab history
memory/handle/fd growth
GPU/resource stability
```

Do not equate one Criterion run with sustained-release evidence.

## Dependency adoption rule

Do not introduce new major runtime dependencies merely because a proposal recommends them.

Every adoption requires:

```text
current need
measured benefit
license
security/advisories
startup/binary cost
platform compatibility
rollback plan
```
