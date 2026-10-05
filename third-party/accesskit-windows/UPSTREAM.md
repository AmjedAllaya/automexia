# AccessKit Windows source provenance

This directory adapts AccessKit Windows 0.35.1 (MIT OR Apache-2.0).
Registry source: <https://crates.io/crates/accesskit_windows/0.35.1>.
Archive SHA-256: `ce63f35d6bdcf59f26b76b3379063f738e6412cef46999cc772d46aa3de35adb`.
Upstream source revision: `ce8164ba92995cfa86005b6259115e08c8244253`.

Production source and upstream tests were copied from that verified archive.
The archive omits its referenced license files; LICENSE-APACHE, LICENSE-MIT and
LICENSE.chromium are included here from the exact upstream revision above.
The Chromium-derived node implementation retains its BSD notice.

Local changes:

- Check the COM implementation identity of incoming text ranges before reading
  their state; reject foreign providers and preserve cross-tree rejection.
- Handle the full signed movement range without overflow and stop when a text
  position cannot advance.
- Honor native GetText limits in UTF-16 units without splitting surrogate pairs.
- Reject malformed, null, oversized, undeclared and read-only value changes before
  dispatch; reject non-finite numeric values. Text input is bounded to 8192 UTF-16
  units at this adapter boundary.
- Return element-not-available when window coordinate lookup fails during teardown.
- Avoid recursive acquisition of the same range read lock when comparing its
  endpoints.
- Retain upstream tests, with regressions for these boundaries. The focus fixture
  requires foreground ownership before and after its observations; it retries
  only desktop ownership loss, at most three times, and never retries a provider
  disagreement while its window remains foreground. The test client temporarily
  attaches to the foreground input queue only for activation and always detaches
  before UIA observation. Windowing tests use Automexia's existing versioned
  rio-window fork; this does not add a second window owner.
- Disable publication and scope dependencies/source to Windows so the workspace
  remains buildable on other platforms.

This local package is not the unmodified registry package and must not receive
its registry audit. All transitive registry dependencies keep their existing
review requirements. Native UIA tests are separate from Narrator/NVDA usability,
macOS AX and Linux AT-SPI certification. This local crate remains unpublished.

Run `cargo test -p accesskit_windows --lib -- --test-threads=1` on an unlocked
Windows desktop. Update/replace this override only after rerunning the boundary
regressions, reviewing the replacement source, and verifying native behavior.
