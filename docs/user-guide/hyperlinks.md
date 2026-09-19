# Keyboard hyperlinks

Status: implemented in source. Native desktop, browser-handler latency and
screen-reader verification remain separate platform gates.

Press **Ctrl+Alt+O** to review links in the selected pane's visible output,
including scrolled history. This uses the existing configured hint rule.

| Key | Action while reviewing links |
|---|---|
| Tab / Shift+Tab or Down / Up | Select the next / previous link, wrapping at the ends |
| Displayed letter or number label | Select that link without opening it |
| Enter | Perform the selected rule's action; the default opens the link |
| Left / Right | Inspect successive portions of a long destination |
| Ctrl+Shift+C; Command+C on macOS | Copy the exact destination without closing the review |
| Backspace | Remove a partial label |
| Escape or Ctrl+C | Return to the shell without sending input |

The destination card shows the actual URI, not only its displayed label. The
focused link is underlined; short keyboard labels stay within the pane without
overlapping. Where a complete label cannot fit, Tab still reaches its link and
the destination card shows the selected label. The
card moves between the top and bottom to avoid the selected rows where space
permits. Narrow windows shorten labels visually; Left/Right inspect the target
and copying retains its full value. A very short window cannot display the full
card; Escape remains available. No animation is needed.

Output changes, pane changes, scrolling, resizing or losing focus dismiss the
captured review rather than reassigning a label to a new target. Reopen it with
Ctrl+Alt+O. A mouse click/wheel dismisses review and is consumed, so it cannot
accidentally paste or activate a covered pane. Normal modifier-click opening is
unchanged outside keyboard review. Table/image/search surfaces retain their
own input; this feature does not rewrite terminal cells or selection.

## Safety and limits

Explicit OSC 8 destinations retain their punctuation exactly. The default opener
accepts recognized URL schemes and local-looking paths, rejects controls, bidi
formatting, credential-bearing URLs, unrecognized schemes, option-like targets
and nonlocal `file://` authorities. Rejected destinations are marked copy-only.
Remote file links are not interpreted as local paths; use the remote tool itself.
Configured custom handlers retain their explicit authority: review them before
using untrusted output. No key in review adds Enter to the shell.

Opening uses the existing host OS handler: Windows ShellExecuteW, macOS `open`,
or Linux/BSD `xdg-open`. WSL runs inside the Windows host and uses its handler;
there is no automatic guest-file download or path translation. Launch failures
remain content-free diagnostics, not a guaranteed success notification. Copy
does not invent a success badge when the platform clipboard returns no status.

Capture is limited to 256 links, 4 KiB per destination, 256 KiB destination
payload and 256 Ki visible cells. An exact cell snapshot retains at most 2 MiB
of packed cells plus 256 KiB of extras payload and bounded container overhead.
Logical regex buffers are limited to 16 KiB; matches exceeding limits are
omitted. Regex matching has retry and stack ceilings. There is no idle scan,
background service, persistence, network prefetch or added dependency.

## Customize or disable

Edit the existing `[hints]` rule's `binding` to change Ctrl+Alt+O; set its
`binding` to none by omitting the field from an explicit rule to keep only mouse
interaction. `rules = []` under `[hints]` disables hints. See
[configuration](../CONFIGURATION.md#hints) for exact syntax and custom actions.
Keyboard label selection now requires Enter before action; this intentional
change prevents a typed label from silently opening a misleading destination.

See [ADR 0061](../adr/0061-keyboard-hyperlink-review.md) for ownership and tests.
