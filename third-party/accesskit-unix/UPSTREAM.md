# AccessKit Unix source provenance

This directory adapts AccessKit Unix 0.24.0 (MIT OR Apache-2.0).
Registry source: <https://crates.io/crates/accesskit_unix/0.24.0>.
Archive SHA-256: `202f24df034a7476d07b7f74284de84f6d62aabd858dbe7ee9cad3b7ad6f8f9d`.
Upstream source revision: `ce8164ba92995cfa86005b6259115e08c8244253`.

Production files were copied from the verified archive. Its referenced license
files are included from that exact upstream revision.

Local changes:

- Preserve the single structured argument in AT-SPI cache signals. Native
  Ubuntu 24.04 rejected flattened AddAccessible and RemoveAccessible bodies;
  a raw D-Bus wire-signature regression guards both messages.
- Bound queued work to 8192 events and 4 MiB of event payload, in batches of 64.
  A separate registry retains window-removal ownership under pressure and
  supports at most 64 concurrent adapters. Overflow requests a fresh provider
  connection and tree. Producers never wait for D-Bus or a worker join.
- Register initial trees in sequence, handle out-of-order window identifiers,
  terminate on closed streams, and deactivate trees on worker failure without
  panicking or logging provider-controlled content. A failed session-bus worker
  leaves its adapters inactive for the remainder of that process.
- Validate the registry Embed reply with typed zbus fields before constructing
  its object reference. Malformed peer bus names return an error instead of
  entering the upstream ObjectRef deserializer assertion. Encoded-wire tests
  cover empty names, non-unique names and a valid registry reply.
- Reject native text replacement over 8192 UTF-8 bytes before copying the wire
  value, and reject disabled or read-only targets. Real adapter-node tests cover
  all three cases; ordinary keyboard input remains owned by the application.
- Retain the existing async-io runtime and cancel-on-drop task ownership. The
  unused optional Tokio runtime is excluded from this unpublished adaptation.
- Scope dependencies/source to non-macOS Unix. The upstream 0.21.0 translation
  dependency fixes disabled controls and supplies Image interface semantics.

The source inventory is pinned by `tools/ci/check_accessibility_contract.py`.
Native Linux scenarios live in `tests/integration/accessibility-linux.py`;
queue/wire regressions run with `cargo test -p accesskit_unix --lib --locked`.

The workspace package is unpublished and platform scoped. Modified local source
must not receive a registry audit; transitive dependencies retain their review
requirements. Native X11 API tests do not certify Orca, Wayland or OS IME input.
