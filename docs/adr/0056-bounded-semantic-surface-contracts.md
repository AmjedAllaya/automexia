# ADR 0056: Bounded semantic surface contracts

Status: Accepted (contract, host admission and bounded presentation; no UI activation)

## Ownership

Capability-free semantic tables belong to the existing extension API. The
application owns admission, authorization, route identity and snapshot lifetime.
The contracts contain no provider, filesystem, network, renderer or PTY calls.
No new crate, dependency, process or capability is introduced. Reusing terminal
embedding surface IDs would confuse distinct lifecycles; these handles have
separate types. Existing connection row models retain their domain ownership.

Serde visitors enforce per-sequence count and aggregate cell/text budgets while
decoding. They ignore size hints and reject the first over-count element without
decoding it. At an aggregate boundary, one bounded row can be transient before
rejection. Exact boxed strings/slices prevent excess retained capacity. Existing
unbounded intermediate JSON maps and generic bounded-text wrappers do not satisfy
these aggregate requirements, so the surface module owns a small private visitor.
See the [Serde visitor guidance](https://serde.rs/impl-deserialize.html) and
[sequence contract](https://docs.rs/serde/latest/serde/de/trait.SeqAccess.html).

Externally tagged cell/event enums avoid buffering an arbitrary map before the
variant is known. Host framing bounds the encoded document and escaped-string
scratch space separately. Version, duplicate members, unknown fields, row shape,
duplicate identities and incompatible typed cells are rejected. Fixed host errors
do not quote decoder or provider data. Table debug formatting emits counts only,
with redacted identifiers and constant-size output; it does not traverse rows.
No fallback silently coerces a value. Local display data can be confidential;
serialization does not authorize persistence, logging or publication.

## Trust and lifetime

The host slot compares claims with independently supplied host binding and grant.
The caller must authenticate the source, allocate exactly one slot per authorized
operation, use a fresh generation on reopen and revoke on route, capsule or
extension lifecycle changes. Constructing a wire binding is not authorization.
Only session-scoped UI-overlay grants are accepted. AllowOnce accepts one valid
update, not a refresh stream. Expiry and host clock rollback fail closed.

One slot retains at most one immutable table. Loading/failure keep the last good
snapshot; invalid updates do not replace it; close/revocation permanently retire
the slot. This model is not registered with a renderer, provider transport or
global runtime. There is no active semantic-table UI or multi-slot registry.
An active registry requires its own total-slot and aggregate-byte budget.

The host now owns a capability-free `TablePresentation` in the existing UI model
crate. It contains the single table, bounded viewport geometry and selected index.
Refresh reconciles stable row/resource handles; deletion clears selection.
Navigation is revision-bound and requires ready state, while expired/revoked data
is removed from both snapshot and presentation reads. No mutable presentation
reference escapes the host slot. The raw terminal table detector and its smaller
capture limits are intentionally separate contracts, not duplicate authorities.

The model follows non-wrapping directional navigation from the
[W3C grid interaction guidance](https://www.w3.org/WAI/ARIA/apg/patterns/grid/),
without treating web ARIA attributes as native accessibility implementation.
The cross-crate API exposes immutable borrows and typed navigation, following
[Rust visibility rules](https://doc.rust-lang.org/reference/visibility-and-privacy.html).
No runtime dependency, provider, persistent state or new capability is added. Removing
the host consumer and model is a migration-free rollback. Priority-based hiding,
details, copy, native input, rendering and assistive-technology delivery are not
established by presentation-model tests.

The optimized presentation benchmark belongs to `automexia-ui-model`, using
the workspace's existing Criterion dev dependency. It does not build the full
application/provider graph merely to measure row navigation. Release optimization
and correctness assertions remain enabled; this is test ownership, not a relaxed
performance profile or skipped application integration tests.
The architecture gate reuses its existing Criterion check for both model owners:
only Cargo's development dependency kind is admitted, including renamed and
target-specific declarations. Runtime, build, missing and malformed kinds remain
rejected; mutation tests guard both helper behavior and dispatch to the UI model.

## Evidence and limitations

Literal protocol tests, hostile mutations, typed-value boundaries, allocation
accounting, fake-provider bytes through host admission and a coverage-guided fuzz
target cover this boundary. Benchmarks measure decode, validation and drop for
0/1/100/2,000/20,000 rows, with separate typed construction/validation and bounded
diagnostic measurements, not renderer or provider throughput. The test-only
allocator forwards exact pointer/layout operations to System, with non-allocating,
non-unwinding thread-local accounting and scoped measurement cleanup. Independently
tested counter errors are reported after measurement. Production gains no unsafe
code or custom allocator.

Native desktop pixels, screen readers, interactive selection/copy and active
provider lifecycle are not established by these tests. The contract is additive;
rollback removes its callers and declarations without persisted-data migration.
The public-data selection obligation remains with the producer: no parser can
prove that arbitrary ordinary text is free of confidential information.
