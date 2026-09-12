# ADR 0058: bounded, allocation-free palette conversion

Status: accepted for current source.

## Problem and placement

Core colour conversion compiled two regular expressions for every colour and
allocated replacement strings and channel vectors. Default palette construction,
configuration deserialization and application palette setup repeated this work.
The Unicode-aware digit class could accept six scalars occupying eight UTF-8
bytes, then panic in an ASCII alpha decoder.

The existing `rio-vt::config::colors` owner remains authoritative, forwarded by
`rio-backend`. This is common terminal configuration, not an optional DevOps
feature or a new extension. App branding, renderer state, persistence and user
configuration recovery retain their existing owners and dependency direction.

## Decision

Use one private borrowed parser, with the existing public owned method forwarding
to it. Inspect at most nine prefix bytes, accept exactly six/eight ASCII digits
with one optional leading marker, and decode into four stack bytes. Invalid
inputs produce bounded static-text errors without echoing input. An invalid
character beyond the token ceiling can report size instead of character error;
valid colour values, alpha and existing float conversion remain exact.

Literal defaults and borrowed colour helpers use the same decoder without input
copies. There is no new dependency, worker, I/O, global cache or unsafe production
code. Retaining compiled regexes was rejected: it keeps unnecessary regex state
for a fixed eight-digit grammar and does not itself fix the Unicode mismatch.
The existing regex dependency remains available to unrelated consumers.

Official references: [regex performance and Unicode](https://docs.rs/regex/latest/regex/)
and [Rust ASCII hexadecimal validation](https://doc.rust-lang.org/std/primitive.u8.html#method.is_ascii_hexdigit).

## Evidence and limits

`rio-vt/tests/color_conversion.rs` exercises the public conversion/serde/default
paths. Independent numerical and grammar oracles preserve values and rejection.
A test-only allocator delegates each original pointer/layout unchanged to System;
constant-initialized thread-local counters observe first-use and repeated
allocations without counting concurrent tests. No allocation is performed inside
the counting callback. This instrumentation does not enter product builds.

Existing VT and application Criterion targets measure default, serde, helper and
full effective palette setup with correctness assertions. Same-host development
results are not release-profile or native compositor timing. The complete
configuration recovery and platform release gates remain required.

No migration or user action is necessary. Rollback may restore implementation,
but must retain malformed-input rejection and exact-value regressions; restoring
the original panic is not an acceptable rollback.
