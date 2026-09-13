# Private planning boundary

This path is retained only because current machine contracts still reference
it. It intentionally contains no public feature, architecture, testing,
roadmap, provider, workflow, or commercial details.

Historical content is preserved in ignored private documentation. Do not infer
a product from the filename, a source package, a fixture, or a checker.

Public behavior is limited to [the feature catalog](../FEATURES.md).

# D7/CP6 ecosystem implementation audit

Status: Partially done overall; fully done locally at the accepted source boundary

This is a public source-and-assurance summary, not a marketplace or commercial
roadmap. Runtime and release activation remain disabled.

## Evidence ledger

| Area | Current status | Exit evidence |
|---|---|---|
| Contract and manifest | Accepted, versioned, deny-by-default source boundary | Mutation checks and exact contract digest |
| Host isolation | Local component-host source boundary | Native sandbox, process, handle, cancellation, and crash evidence |
| Lifecycle | Bounded install-state, disable, uninstall, and rollback models | Packaged-artifact recovery and cleanup evidence |
| Distribution | Not activated | Signing, provenance, revocation, policy, and support evidence |

### D7.1

The public contract defines versioning, capabilities, identity, size/count
ceilings, explicit activation state, and fail-closed parsing. Installation alone
grants no authority.

### D7.5/CP6.3

The application-owned broker validates each request and keeps optional work off
input, renderer, PTY, resize, and startup hot paths. Host failure, timeout,
disable, or removal preserves ordinary terminal operation.

## External prerequisites

Required before activation: native Windows, Linux, and macOS sandbox and
process-lifecycle evidence; signature and revocation drills; resource and
long-duration tests; accessibility review; signed package identity; upgrade,
disable, uninstall, rollback, and offline recovery; and independent security
review of the final contract and host.
