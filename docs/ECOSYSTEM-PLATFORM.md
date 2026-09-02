# Private planning boundary

This retained compatibility path intentionally contains no public feature,
architecture, testing, roadmap, distribution, or commercial details. Historical
material is preserved only in ignored private documentation.

Do not infer a product from this filename, source package, test fixture, or
checker. Public behavior is limited to [the feature catalog](FEATURES.md).

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Current Behavior

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

# Extension ecosystem platform boundary

Status: accepted source implementation; release activation disabled

This page documents only the free public safety contract for optional
extensions. It does not publish marketplace, commercial, or unreleased product
plans.

## Current behavior

The repository contains a versioned, deny-by-default source contract and a
bounded component-host boundary. Source presence does not activate extensions,
grant capabilities, publish a package, or create a stable-release claim.

## Fixed safety boundary

Every extension has an explicit manifest, version, identity, provenance,
capability set, resource limits, lifecycle state, and application-owned broker.
Unknown fields, capabilities, versions, or stale identities fail closed.
Untrusted labels and payloads are bounded and sanitized. Credentials remain in
platform or external authorities and are referenced, never copied by default.

Optional work remains outside renderer, input, PTY, resize, and startup hot
paths. An extension cannot become a second owner for terminal state, sessions,
processes, focus, routes, configuration, or persisted records.

## Recovery and fallback

Timeout, crash, malformed output, signature failure, policy denial, disable,
uninstall, or host shutdown must leave the terminal usable. The application
cancels outstanding work, rejects stale publication, joins owned processes,
removes only Automexia-owned artifacts, and restores last-known-good or disabled
state. Activation requires separate exact-artifact release evidence.

See [architecture](ARCHITECTURE.md), [extensions](EXTENSIONS.md),
[testing](ECOSYSTEM-PLATFORM-TESTING.md), and [ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md).
