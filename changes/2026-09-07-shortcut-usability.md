## Fixed

- Replace both command-palette Back icons with a clear left arrow.
- Use Alt+R/D to clone right/down and Alt+Shift+R/D for fresh panes on
  Windows/Linux/BSD. Ctrl+R/D, explicit bindings and pinned profiles are preserved.
- Derive all palette shortcut labels from effective bindings, respecting platform
  differences, overrides, unbound actions and scoped input ownership.
- Add regressions for default tables, mode/override isolation, retired punctuation,
  label reload, dedicated copy/paste keys and Back geometry. Native desktop
  keyboard, pixel and assistive-technology evidence remains external.
