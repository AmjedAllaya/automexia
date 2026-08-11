# Terminal conformance fixtures

Fixtures use JSON-escaped control bytes so reviews can see complete sequences.
Tests consuming these files must feed every sequence whole, fragmented at every
byte boundary, and with malformed/truncated variants. Coverage includes CSI,
OSC, DCS, OSC 7 working directories, OSC 133 prompt lifecycle, OSC 1337 user
variables, titles, Unicode graphemes, combining marks, emoji widths, and cursor
positions.

`rio-vt/tests/conformance_fixtures.rs` executes the OSC and Unicode corpora.
Run it with `cargo xtask test conformance` or directly with
`cargo test -p rio-vt --test conformance_fixtures`.
