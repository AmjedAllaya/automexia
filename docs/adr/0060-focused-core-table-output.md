# ADR 0060: Focused core table output

Status: implemented in source; native desktop and assistive-technology evidence
remains a separate validation requirement.

## Decision

The user selected a focused core table view, not inline scrollback rewriting.
Core owns this capability because terminal text presentation, input and session
lifecycle are fundamental terminal mechanisms. The existing extension semantic
table contract remains provider/capability-bound and is not a second source of
terminal text. A new extension would add an unnecessary lifecycle boundary.

`automexia-ui-model::tables` owns bounded whitespace-gutter recognition,
grapheme clipping and cell-based scrolling. The application table-output adapter
reads the authoritative VT only after explicit activation. VT display copying
shares normal copying's wrap/blank handling but expands actual tab cells for
presentation without changing normal clipboard semantics. `table_view` owns a
bounded, immutable session snapshot, local input and graphics. Its private draw
adapter supports the same text shaper in controlled CPU tests and live graphics.

There are no added dependencies, background tasks, filesystem/provider/network
operations, persistent records or terminal mutations. Close and route replacement
release the snapshot. Bounds and unsupported input fail to a local explanatory
view; Escape always returns to normal terminal operation. Public shortcut
identity uses the existing customization owner, not a second preference store.

## Alternatives and consequences

Inline horizontal scrolling would change hit testing, selection, search and
scrollback projection. It was rejected in favour of the user's focused choice.
Reexecuting commands is unsafe and cannot restore historical output. Guessing
CSV or application schemas would introduce another parser and false authority;
recognition therefore requires repeated shared whitespace columns. Text colours
are neutral rather than presenting inferred status as fact.

The common frontend is platform-independent. Native PTY adapters and application
output remain unchanged. This cannot repair text truncated or hard-wrapped by
the producer. It also is not a claim of native accessibility: keyboard focus,
screen-reader semantics, compositor pixels and physical input require distinct
evidence on each platform being claimed.

GNU [output formatting](https://www.gnu.org/software/coreutils/manual/html_node/General-output-formatting.html)
documents native listing width/tab behaviour. The WAI-ARIA
[grid pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) informs keyboard
navigation conventions, not a claim that this native surface is an ARIA widget.

The presentation rejects hidden directional controls described by
[Unicode UAX 9](https://www.unicode.org/reports/tr9/) rather than silently
rewriting them. The original output remains available in the terminal. Error
guidance reuses the existing measured-label wrapper; table cells do not wrap.

Rollback removes the focused action/model and its adapters; terminal bytes,
configuration and persisted history need no migration. Tests retain independent
literal columns, original source text, parser-created tab/reflow data, extreme
geometry, controlled pixels and one-pixel mutation evidence. See
[the guide](../user-guide/table-output.md) and [testing](../TESTING.md).
