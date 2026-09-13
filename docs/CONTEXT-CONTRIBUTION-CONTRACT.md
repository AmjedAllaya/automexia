# Context contribution decoding

Automexia's version-1 `ContextContribution` wire format carries extension/session
identity, capsule/source revisions, freshness and up to 64 status segments.
Each segment retains its exact label, accessibility label, role, icon, priority,
observation time and optional details action. This data contract grants no
filesystem, process, network or provider authority.

The decoder checks the collection boundary before decoding element 65. It does
not allocate a complete oversized vector and reject afterward, and does not use
a producer-supplied sequence size hint to reserve storage. Empty and exactly
64-entry collections retain their existing behavior and wire representation.
In-limit elements still undergo normal field/version validation; malformed data
does not become valid merely because its collection is short.

This is a collection-count limit, not a total transport budget. A caller must
separately bound encoded frame bytes, whitespace, individual token processing,
time and queues before decoding untrusted input. Typed construction still checks
the same 64-entry ceiling. Raw shell metadata is not authenticated authority, and
this decoder does not create a live provider connection or a context permission.
Labels can contain private information: do not log, persist or publish payloads.

## Verification

`cargo test -p automexia-extension-api --test context_contributions` exercises
literal v1 fixtures, exact field/order preservation, count boundaries, fragmented
reader input, envelope validation and constructor parity. Its observed-reader
regression requires rejection before consuming an excess entry, including a
large or malformed tail; an eventual error after reading it is insufficient.

`cargo bench -p automexia-extension-api --bench context_contributions` measures
decode/drop for 0, 1, 32 and 64 segments and count rejection before a 1 MiB tail.
Fixtures are verified outside timing. These are local decoder measurements, not
native shell, provider, renderer, compositor, accessibility or whole-process
memory evidence. Source guards and mutation tests complement compiled behavior
tests; neither substitutes for the other.
