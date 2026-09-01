# SSH, connectivity, and multi-cloud delivery direction

Status: public summary; not an implementation ledger.

## Target experience

Users can choose a host or environment, see the identity and route that will be
used, review trust and production risk, and open an isolated session. Cloud and
orchestrator adapters add context and resources through the same consistent
workflow while their native tools retain credential ownership.

## Broad order

Delivery is verified through the separately gated M8, M9, M10, M11, and M12
provider increments. The identifiers are assurance boundaries, not public
release dates or availability commitments.

- release-quality read-only inventory and Connection Hub behavior;
- exact-argument, user-reviewed system OpenSSH sessions and tunnels;
- reusable typed connection and workspace automation;
- provider-neutral context and authentication references;
- independently enabled AWS, Azure, Google Cloud, Kubernetes, OpenShift, and
  organization-access adapters;
- provider-aware completion and reviewed operational actions;
- situation-aware investigation only after freshness, policy, impact, and
  authority contracts are proven.

## Required gates

Each increment needs bounded resource use, hostile-input handling, credential
redaction, cancellation, cleanup, disable and uninstall behavior, deterministic
tests, native platform evidence, accessible UX, and honest fallback to the
underlying tool.

Detailed milestones, evidence ledgers, provider recipes, and implementation
ownership maps are local-only planning material. Current behavior remains
documented in [Connection Hub](CONNECTION-HUB.md),
[SSH connection automation](SSH-CONNECTION-AUTOMATION.md), and
[Multi-cloud provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md).
