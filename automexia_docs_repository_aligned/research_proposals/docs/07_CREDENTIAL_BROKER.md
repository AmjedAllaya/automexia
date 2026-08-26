
# Credential Custody Principles

Apply the security objective now:

- do not persist raw secrets unnecessarily;
- prefer platform/external custody;
- persist opaque references/handles;
- use short-lived materialization;
- redact by construction;
- make agent/signing operations explicit.

Do **not** create a new generalized credential service solely to match this proposal.

First activate and qualify current:
- OpenSSH agent integration;
- platform stores;
- cloud CLI/session flows;
- Teleport;
- existing provider owners.

Only extract a shared credential-broker subsystem when repeated lifecycle/ownership needs are proven.

Provider nuance:
- 1Password / Bitwarden / OpenSSH can act as agent servers;
- KeePassXC is better modeled as loading keys into an existing compatible agent, not as an independent agent socket owner.
