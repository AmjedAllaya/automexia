
# Credential Custody

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Custody principles now; generalized service deferred.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Decision
Credentials are mediated by a trusted broker. External agents/vaults/SSO are preferred; extensions receive opaque handles/signing/session capabilities rather than raw secrets whenever possible.
