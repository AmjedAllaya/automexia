# Multi-cloud provider implementation summary

Status: provider foundations are partial and release-gated. This is not the
private execution ledger.

## Public status

The internal assurance contract tracks the independently gated M8, M9, M10,
M11, and M12 provider increments. These identifiers describe verification
boundaries; they do not promise availability or expose the private execution
ledger.

Automexia is building independently enabled adapters for AWS, Azure, Google
Cloud, Kubernetes, OpenShift, and organization access. Provider-neutral types,
cached review surfaces, and some lifecycle foundations exist in source. Live
authentication, discovery, native accounts, production mutations, and complete
platform evidence must be treated as unavailable unless their specific release
documentation says otherwise.

## Required provider boundary

- Native provider tools or approved SDKs keep authentication authority.
- Automexia stores opaque credential references, not secret material.
- Discovery is explicit, bounded, cancellable, cached with freshness, and kept
  off input and rendering paths.
- Pane, account, region, cluster, namespace, and generation state remain
  isolated.
- Provider results are untrusted and must pass limits, normalization, redaction,
  and stale-result checks.
- Any mutating action shows the target, identity, risk, executable, arguments,
  and policy result before approval.
- Failure or uninstall restores a usable native-tool workflow.

Detailed provider implementation order, compatibility findings, internal
budgets, and evidence ownership remain local until each adapter is ready for
public release review.

See [Multi-cloud provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md),
[SSH and multi-cloud architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md), and
[Extensions](EXTENSIONS.md).
