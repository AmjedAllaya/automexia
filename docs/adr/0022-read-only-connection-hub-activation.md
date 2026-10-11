# Private planning boundary

Status: Not a public product decision

This compatibility path contains no public feature, workflow, architecture, or
commercial details. The historical record is preserved only in ignored private
documentation.

Do not infer behavior from the filename or source scaffolding. Public behavior
is limited to [the feature catalog](../FEATURES.md).

## SSH-only picker projection

The Connect palette entry reuses the same Hub controller, inventory worker,
private metadata store and review surface. Its temporary search/filter state is
independent from the general Hub; the catalog projection filters SSH and orders
recent entries first. It adds no process, network, credential or file authority.
System OpenSSH ownership is visible in the header/footer and dialog semantics.

A removed review target returns to results instead of switching to another host.
Review navigation cannot move the catalog selection or focus a hidden search.
Opening and closing synchronize painted and keyboard selection. Static discovery
never expands an Include from an unevaluated Match block; conditional state from
an included file remains effective until the next Host declaration.

The existing managed-launch hard denial remains. Launch, split/tab modifiers,
and successful-use updates require the protected launch, loader-attestation and
native evidence gates; picker/model/Windows UI tests do not satisfy those gates.
