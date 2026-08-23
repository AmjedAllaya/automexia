# Multi-cloud providers implementation audit

Status: active M8-M12 implementation ledger and execution plan.

Last reconciled: 2026-08-23.

This page turns the provider portion of the
[SSH, connectivity, multi-environment, and multi-cloud plan](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md)
into evidence-led implementation slices. The focused roadmap remains the phase
order authority. Shipped behavior, locally complete source contracts, native
evidence, and protected activation are intentionally reported separately.

## Outcome and authority

The requested outcome is independently enabled AWS, Azure, Google Cloud,
Kubernetes/OpenShift, and Teleport adapters which read only explicitly granted
public configuration, build exact reviewed requests for official tools, bind
all work to one immutable session capsule, and never take custody of provider
credentials. OpenBao remains blocked until its separately required security ADR
is accepted.

The user authorized source, tests, documentation, coherent DCO-signed commits,
and a non-force push to the current feature branch. The user did not authorize
cloud-account changes, real logins, remote connections, provider configuration
mutation, credential changes, release publication, or merging. No installed
provider account, hardware token, controlled cloud fixture, or non-Windows
native host is available as assumed evidence.

### Measurable acceptance criteria

- Each provider is a separate disabled-by-default first-party extension with
  only exact process and network capabilities.
- Parser inputs are exact user-granted bytes with byte, record, field, nesting,
  and hostile-text limits. Tokens, private keys, credential caches, certificate
  contents, and secret-bearing fields are rejected or omitted.
- Login and observation use the M7 exact-operation review. Connection and
  cluster intents remain typed data until the existing application runner owns
  their reviewed process, PTY, cancellation, and descendant cleanup.
- Every provider request pins the capsule ID, session, revision, provider,
  configuration reference, account/scope, executable, ordered arguments,
  browser policy, destination, timeout, and risk. Global context mutation is
  never used as isolation.
- Disabling or uninstalling one extension removes only that provider's catalog
  entry and leaves the official CLI and user-owned files untouched.
- Deterministic unit, hostile-input, redaction, exact-argv, isolation,
  uninstall, architecture, and full contributor gates pass. Native official
  tool, MFA, browser, cloud network, accessibility, and release evidence is
  reported as external until it actually runs.

## Current evidence ledger

| Slice | Current status | Existing evidence | Missing exit evidence |
|---|---|---|---|
| M8 / D6.1 AWS | Partially done overall; source-complete and nonactivated | Independent bounded extension; granted public parser; exact capsule-bound SSO/STS/SSM/EKS-dry-run contracts; ten focused tests and app registration | D3 activation, M11 private kubeconfig ingestion, product UI, and controlled real AWS/native/resource/accessibility/release evidence |
| M9 / D6.2 Azure | Not done | Provider-neutral `Azure` kind, Bastion transport descriptor, M7 framework | Independent extension, public JSON parser, exact login/account/Bastion/AKS-isolated builders, tests, docs, native fixtures |
| M10 / D6.3 Google Cloud | Not done | Provider-neutral `Gcp` kind, IAP transport descriptor, M7 framework | Independent extension, granted configuration parser, scoped login/status/IAP/GKE-isolated builders, tests, docs, native fixtures |
| M11 / D6.4 Kubernetes/OpenShift | Not done | Provider-neutral kinds and exec/rsh transport descriptors | Code-capable kubeconfig parser, merge/collision/source model, default-denied exec plugins, exact isolated kubectl/oc intents, tests, docs, native fixtures |
| M12.1 / D6.5 Teleport | Not done | Provider-neutral `Teleport` kind and SSH transport descriptor | Independent extension, bounded public status decoder, exact version/login/status/ssh builders, tests, docs, native fixtures |
| M12.2 / D6.5 OpenBao | External prerequisite | Provider-neutral kind only | Accepted security ADR for token-helper/certificate-file custody, then a separate implementation and native evidence |

M7 is fully implemented at its authority-free model boundary and is preserved.
The application launch broker remains nonactivated; adapters therefore produce
reviewable operations and connection intents but cannot silently execute them.

## Build, wrap, or adopt decision

Official `aws`, `session-manager-plugin`, `az`, `gcloud`, `kubectl`, `oc`, and
`tsh` tools remain protocol, authentication, MFA/browser, credential-cache, and
remote-session authorities. Automexia wraps exact invocations; it does not add
cloud SDKs, Kubernetes clients, browser components, vaults, or independent
process launchers.

Two format parsers are adopted behind provider-owned byte and complexity
limits:

- `configparser` with no optional async/runtime features for AWS/GCP INI-style
  public configuration. It is dependency-free and available under MIT or
  LGPL-3.0-or-later; Automexia consumes it under MIT. It performs no I/O when
  parsing supplied strings.
- `serde-saphyr` with only deserialization for typed Kubernetes YAML. It is
  pure Rust, declares no unsafe code, rejects malformed typed input, and has
  parser budgets for input bytes, depth, events, nodes, anchors, aliases, and
  scalar bytes. Automexia tightens those defaults for kubeconfig.

Both are removable by deleting only their provider extensions. Neither is on
startup, renderer, input, PTY, resize, or keystroke paths. Lockfile, license,
advisory, duplicate, MSRV, platform, and binary impact remain contributor-gate
evidence; native provider tools are never bundled.

## Ownership and implementation sequence

1. `extensions/devops-aws` owns bounded public AWS config and exact AWS CLI
   request construction.
2. `extensions/devops-azure` owns bounded public Azure metadata and exact Azure
   CLI request construction.
3. `extensions/devops-gcp` owns bounded public gcloud configuration and exact
   Google CLI request construction.
4. `extensions/devops-kubernetes` owns Kubernetes/OpenShift source manifests,
   typed YAML/JSON parsing, kubectl-compatible first-file-wins merge, source
   fingerprints, exec-plugin denial, and exact isolated kubectl/oc requests.
5. `extensions/devops-teleport` owns exact `tsh` operations and bounded public
   `tsh status --format=json` decoding.
6. `apps/automexia-terminal` only registers independent manifests. It does not
   parse provider files or create a provider-specific runner.
7. The accepted M7 model continues to own capsules, exact capability review,
   browser policy, authorization, generation isolation, receipts, and audits.

Each extension exposes pure functions over supplied bytes and typed parameters.
Filesystem selection, no-follow reads, stable source identity, execution,
environment injection, output capture, PTY ownership, cancellation, deadlines,
and descendant cleanup remain application responsibilities.

## Trust, resource, and failure contracts

- Maximum provider configuration input is 1 MiB, 128 profiles/contexts, 128
  records per collection, 4 KiB per public field, 64 YAML levels, 100,000 YAML
  events, 25,000 nodes, 64 anchors/aliases, one document, and 1 MiB total scalar
  data. Smaller provider-specific ceilings may apply.
- Controls, bidirectional overrides/isolates, zero-width formatting characters,
  duplicate identifiers, ambiguous merge collisions, relative credential or
  exec paths, inline tokens/keys/certificates, unsupported auth providers, and
  secret-bearing command arguments fail closed with public redacted errors.
- Kubernetes merge follows documented `KUBECONFIG` first-file-wins semantics.
  A duplicate name with different public material is a visible collision, not a
  silent override. An exact source-set digest binds review to source order and
  revisions; source drift invalidates the request.
- Kubernetes `user.exec` is represented only as denied public metadata. No exec
  command, arguments, environment values, `ExecCredential`, token, client key,
  or certificate value is retained. Future execution needs a separate exact
  executable digest and per-session grant.
- EKS, AKS, and GKE generation always targets a private transient kubeconfig
  reference. Defaults that merge into the user's kubeconfig or change
  `current-context` are forbidden.
- Errors identify provider, public field, and recovery action but never echo
  input values. Debug output reports counts and stable public identifiers only.
- Adapters have no background thread, cache, retry loop, filesystem handle,
  process, socket, PTY, or persisted state. Revoke/disable/uninstall therefore
  drops Automexia authority immediately and does not delete official CLI state.

## UX and accessibility contract

Provider cards remain progressive and compact: icon plus provider name, one
primary pinned context, explicit freshness/risk text, and one review action.
Details reveal exact executable/arguments, configuration reference, account and
scope, destination, browser/device behavior, requested capabilities, and safe
recovery. Loading, MFA/browser/device pending, denied, offline, stale, expired,
unsupported, and error states use text and icon in addition to color. Keyboard
focus returns to the invoking control after review. No provider work runs when
the Hub opens or while the user types. Product rendering and controlled screen
reader evidence remain a later application integration gate; this slice must
not claim them from pure model tests.

## Primary-source compatibility findings

- AWS CLI 2.22+ defaults IAM Identity Center login to PKCE and supports an
  explicit device-code flow; STS `get-caller-identity` exposes public caller
  identity. Session Manager requires its official plugin. EKS
  `update-kubeconfig` merges files and changes current context unless redirected
  or used with `--dry-run`.
- Azure CLI 2.61+ uses Windows Web Account Manager by default and supports
  browser/device-code login; Azure now requires MFA for human CLI identities.
  `az account set` changes global selection and is forbidden. Bastion native
  SSH requires Standard-or-higher Bastion and Azure CLI 2.32+.
- A gcloud named configuration can be scoped to one command with
  `--configuration` or `CLOUDSDK_ACTIVE_CONFIG_NAME`; activation is global and
  forbidden. `gcloud compute ssh --tunnel-through-iap` is the official IAP
  path. GKE credential generation changes kubectl current context.
- Kubernetes warns that kubeconfig can execute code. Relative exec commands
  resolve from the config directory, and exec plugins can return tokens or
  client keys/certificates. This implementation rejects exec and relative
  credential paths rather than evaluating them.
- Teleport `tsh status --format=json` is the public status seam. `tsh` retains
  certificate/cache/agent/browser/MFA authority; per-session MFA may occur for
  each SSH session.
- OpenBao token helpers receive `get`, `store`, and `erase` and exchange the
  token over standard streams; signed SSH certificates combine a certificate
  file with the user's private key. Those custody boundaries require the
  separate proposed ADR before code.

Primary sources: [AWS IAM Identity Center](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html),
[AWS STS caller identity](https://docs.aws.amazon.com/cli/latest/reference/sts/get-caller-identity.html),
[AWS Session Manager plugin](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html),
[AWS EKS kubeconfig](https://docs.aws.amazon.com/cli/latest/reference/eks/update-kubeconfig.html),
[Azure interactive authentication](https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli-interactively),
[Azure Bastion native client](https://learn.microsoft.com/en-us/azure/bastion/connect-vm-native-client-windows),
[gcloud configurations](https://cloud.google.com/sdk/docs/configurations),
[Google IAP SSH](https://cloud.google.com/compute/docs/connect/ssh-using-iap),
[GKE kubectl access](https://cloud.google.com/kubernetes-engine/docs/how-to/cluster-access-for-kubectl),
[Kubernetes kubeconfig](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/),
[Kubernetes exec credentials](https://kubernetes.io/docs/reference/access-authn-authz/authentication/),
[Teleport tsh reference](https://goteleport.com/docs/reference/cli/tsh/),
[OpenBao token helpers](https://openbao.org/docs/commands/token-helper/), and
[OpenBao signed SSH certificates](https://openbao.org/docs/secrets/ssh/signed-ssh-certificates/).

## Test-first evidence ladder

Each slice starts with denied/default/hostile-input and exact-argv tests, then
adds the smallest adapter implementation. Focused tests cover parser limits,
preference precedence, capsule/config matching, browser choices, isolation,
production risk, redaction, disabled manifests, and uninstall independence.
Kubernetes adds YAML/JSON corpus, merge ordering/collisions, relative paths,
source drift, exec denial, cloud generated-config isolation, and repeated parse
cleanup. Teleport adds version/status schema drift and cache/agent non-import.

After focused crate tests, run warning-denied Clippy per changed owner,
architecture/identity/repository validation, dependency policy, benchmarks for
maximum valid parsing, full workspace formatting/Clippy/nextest/doc tests,
`python3 tools/ci/qa.py --full`, and `cargo ready`. Real provider login,
network, MFA, device/browser, Bastion/SSM/IAP, Kubernetes/OpenShift clusters,
Teleport, non-Windows clients, screen readers, signing, and packaging remain
explicit external gates until controlled native fixtures run.

## Commit and rollback plan

Use one DCO-signed commit per provider containing its crate, tests, docs, and
status update; a final integration commit owns the manifest registry,
assurance/navigation, phase reconciliation, and full-gate fixes. Reverting one
provider commit removes only that independently disabled adapter. The proposed
OpenBao ADR is a documentation-only protected decision and does not authorize
implementation by being present in the tree.
