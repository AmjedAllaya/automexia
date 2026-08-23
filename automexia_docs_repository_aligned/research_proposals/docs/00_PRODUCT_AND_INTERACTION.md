
# Product and Interaction Principles

**State:** Aligns with the current product direction; integrate selectively into real project docs.

Automexia is **keyboard-first, not pointer-hostile and not GUI-free**.

Renderer-native overlays may provide:
- Connection Hub;
- Quick Actions;
- extension lists;
- confirmations;
- search/settings/status;
- compatibility inspection;
- future review surfaces.

Every pointer/menu/palette/keyboard entry point dispatches the same semantic `CommandId` or typed action. No button receives a separate privileged implementation.

The shell/terminal remains the primary surface. Do not evolve into a Termius-style dashboard/sidebar product.

Accessibility is a release requirement, not a later cosmetic pass.

Ghostty familiarity is optional migration UX; Automexia's native interaction remains the default.
