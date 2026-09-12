# ADR 0061: Keyboard hyperlink review

Status: implemented in source; native desktop and assistive-technology evidence
remain external.

## Decision

Extend the core application's existing `hints` owner. OSC 8 interpretation, VT
cell ownership, platform launch, clipboard and configurable hint actions retain
their existing authorities. An extension would add an unnecessary lifecycle and
permission boundary to fundamental terminal navigation; a new crate would not
create a demonstrated cross-crate reuse boundary.

`hints/scan` captures bounded visible cells and exact extras, maps UTF-8 regex
offsets back to native cells, preserves combining marks and soft wraps, and
gives OSC 8 anchors precedence over their displayed text. Adjacent fragments of
the same anchor remain one target. Fixed-length labels over a unique alphabet
are prefix-free, including when matches outnumber alphabet symbols. The first
256 matching targets are retained; oversized logical runs are omitted.
Grid labels are shifted inward at the right edge and omitted rather than
clipped or overlapped in a dense/tiny view. Tab reaches every captured link;
the preview also names the selected full label. This layout remains bounded
by the captured match count and does not alter terminal contents.

The existing Screen owner routes keyboard review before shell/IME dispatch and
retains consumed key releases after dismissal. Label entry selects; Enter
activates. Tab/arrows and destination panning share a pure key-intent contract.
Output/geometry changes invalidate exact captured cells and extras. Route changes
clear the original route's highlights, never a sibling's search state.

The preview uses existing theme, font and bounded grapheme-fitting owners. No
new dependency, process, worker, persistence or provider authority is introduced.
OS opening remains a deliberate call to the existing platform adapter, not shell
evaluation. Default opening validates target text and scheme; custom handlers
retain explicit configuration authority. Target strings and arguments are not
included in launcher diagnostics or HintMatch debug formatting.

## Evidence and tradeoffs

Real-parser tests cover negative history, wrapped URLs, CJK/combining columns,
OSC destinations differing from labels, punctuation, anchor fragments, controls,
limits, malformed regex, snapshot replacement, navigation, actual platform
bindings and custom copy actions. Controlled font/pixel tests compare exact
restored preview pixels and reject a one-channel pixel mutation. The explicit
correctness-checked microbenchmark measures capture plus navigation/snapshot
validation, not native PTY, browser launch or compositor latency.

Rebuilding matches every frame would change label identities and repeat regex
work; immutable bounded capture plus exact stale checks was chosen instead.
Automatic opening on the final label character was rejected in favor of visible
destination review. Modal input does not alter shell shortcuts outside review.
Custom handler behavior and filesystem resolution remain inherited platform
integration boundaries; these tests do not certify arbitrary external handlers.
No automatic migration or new preference store is needed; reverting this
increment restores earlier keyboard behavior without changing saved data.

## Primary references

- [OSC 8 specification](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda): explicit URI and anchor identity, file authority semantics.
- [Alacritty hints](https://alacritty.org/config-alacritty.html#hints): terminal-owned visible hint labels and actions.
- [kitty hints](https://sw.kovidgoyal.net/kitty/kittens/hints/): keyboard-driven selection of terminal links.
- [W3C link interaction](https://www.w3.org/WAI/ARIA/apg/patterns/link/): Enter activation and visible focus guidance; not native accessibility certification.
