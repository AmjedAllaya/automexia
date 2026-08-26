
# Semantic Overlays

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Substance useful; real renderer/accessibility decisions own adoption.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Decision
Automexia is keyboard-first and keyboard-complete, not GUI-free.

- Every essential action has a keyboard path.
- Pointer interaction is optional convenience.
- Keyboard, pointer, palette, and extension-triggered actions converge on the same `CommandId`.
- The core owns overlay rendering, focus, accessibility, keyboard/pointer semantics, themes, and layout.
- Extensions contribute semantic overlay models only.
- Extensions receive no arbitrary GPU/native-window/general GUI authority.
- Automexia does not adopt a Termius-style dashboard/sidebar/card workflow as its required interaction model.
