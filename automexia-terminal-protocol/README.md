# Automexia terminal protocol

Private, dependency-free wire values shared by the terminal parser and SSH
integration library. This crate is `no_std`, forbids unsafe code, and performs no
allocation, I/O, process execution, discovery, or session ownership.

`ScopeRevision::decode` is the canonical decoder for the existing
`AMXSSHREV1|pane|generation|revision` value. Empty input revokes the value. All
three numbers must be nonzero canonical ASCII decimal integers; pane and
generation fit `u64`, revision fits `u32`. The complete value is at most 96 bytes.
Unknown versions, extra fields, whitespace, control characters, leading zeros,
and numeric overflow are rejected. Errors contain no input text.

The parser owns admission, ordering and nested-scope state. The SSH library owns
its existing generation-aware API and production of the wire value. Neither
consumer acquires the other's engine, product, platform, or process dependencies.
