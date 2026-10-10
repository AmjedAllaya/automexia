# Consistent terminal caption controls

Automexia now uses its customizable caption controls by default on Linux and
macOS as well as Windows. macOS retains its native resizable frame. Explicit
native-decoration choices remain supported, and reloads retain the existing
window’s control ownership while applying appearance edits live.

Tab layout no longer reserves space for absent caption controls or hidden macOS
traffic lights. Invalid pointer positions cannot trigger window actions, and
unsupported manual resizing no longer swallows macOS input. Regression coverage
includes default/explicit frames, live reload, scale-invariant caption hit targets,
and isolated Wayland session/UI scenarios alongside X11 and AppKit.

Drawing and input now share header visibility when a single tab is hidden.
Native screenshot fixtures use synthetic prompts and account labels, with a
capture check that rejects identifying shell output.

Custom caption buttons now expose bounded native accessibility actions with current-frame validation and ordinary close confirmation.

Coalesced continuation markers retain the current prompt start without claiming
stale or foreign generations. Native regression probes now wait for complete
theme/tag presentation, exercise keyboard theme preview, and retain only bounded
numeric diagnostics. The prompt benchmark asserts that fragmented native-console
markers produce exactly one start row.

The optional terminal-core window bridge now forwards selected Linux display
backends without activating windowing for headless embedders. CI checks the core
in isolation so workspace feature unification cannot hide a broken backend.

Fish 4 native prompt markers now coexist with identified context rows without
replacing their owner or counting command completion twice. Native caption
probes handle initially maximized desktops and require a fresh transition whose
presented bounds match the owned native window. Repeated CPU and Metal scenarios
cover resize event ordering before subsequent pointer or accessibility actions.
