
# Terminal Runtime and Compatibility

**Current repository direction:** continue the existing Rio-derived parser/renderer unless measurements justify replacement.

The contract is standards/application correctness, not Ghostty product parity.

Qualify:
- VT/xterm semantics;
- Unicode/graphemes;
- wide/combining cells;
- alternate screen;
- cursor/scroll regions;
- bracketed paste;
- focus/mouse;
- OSC behavior;
- selected extended keyboard/graphics protocols.

Ghostty may be:
- a pinned differential reference;
- an optional migration-profile source;
- a possible future `libghostty-vt` implementation candidate behind an Automexia adapter.

Do not make Ghostty runtime presence or Zig a requirement for normal Automexia.

If `libghostty-vt` is evaluated, its API instability and FFI/maintenance cost must be measured against the current engine.
