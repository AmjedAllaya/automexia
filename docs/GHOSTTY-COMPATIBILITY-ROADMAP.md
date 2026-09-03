# Ghostty keyboard compatibility status

Automexia keeps its own keyboard defaults. An explicit compatibility profile may
map supported Ghostty 1.3 keyboard behavior without changing terminal, PTY,
window, tab, pane, or shell ownership.

## Ghostty compatibility G0-G6

## Machine-readable compatibility phase keys

The following identifiers are stable assurance-contract keys. Their wording must
remain unchanged because repository compatibility checks consume them directly.
Human-readable phase headings may contain additional descriptive text.

- `G0 — source lock`
- `G1 — typed registry`
- `G2 — profiles and reload`
- `G3 — dispatch language`
- `G4 — stateless actions`
- `G5 — tooling`
- `G6 — high-lifecycle`

### Public contract

- Compatibility is opt-in; Automexia remains the default profile.
- The profile is versioned and generated from reviewed public fixtures.
- User bindings and explicit unbinds layer after the selected profile.
- Unsupported actions remain unavailable rather than mapping to unrelated
  behavior.
- Invalid configuration keeps the previous complete registry.
- Bare shell control keys keep their documented terminal fallthrough.
- Migration is bounded, dry-run first, explicit to apply, backup-producing, and
  atomic.
- Inspection is redacted and does not launch Ghostty or evaluate configuration
  as code.

## Ownership

Automexia actions keep Automexia semantics. Window-level tabs and independent
PTY splits are not changed into another terminal's internal ownership model.
Platform-global shortcuts remain owned by the operating-system hotkey adapter;
focused bindings remain route-scoped.

## Verification

Tests cover profile identity, parsing, precedence, unbind, collisions,
sequences/tables/chains, physical/logical keys, layouts, AltGr, dead keys, IME,
shell fallthrough, migration, rollback, generated artifacts, and native
platform behavior. Native evidence is recorded separately for each operating
system and keyboard/input environment.

## Final acceptance gate

Accept a compatibility release only when the generated profile matches the
reviewed public fixture, precedence and fallthrough tests pass, migration and
rollback are verified, unsupported actions fail explicitly, and native
keyboard/IME evidence exists for every platform being claimed. Linux/BSD and
macOS remain external until their native evidence is recorded.

See [Keyboard compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md),
[Keyboard reference](KEYBOARD.md), and the generated
[keybindings](generated/ghostty-1.3-keybindings.md) and
[actions](generated/ghostty-1.3-actions.md).

This page contains no future compatibility roadmap. Unreleased expansion plans
remain private.
