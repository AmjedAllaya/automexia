# AccessKit macOS source provenance

Local adaptation of AccessKit macOS 0.27.1 (MIT OR Apache-2.0).
Registry source: <https://crates.io/crates/accesskit_macos/0.27.1>.
Archive SHA-256: `5701624bf6a11efb6d4c8e922bf44126ed96250e5faa9a7a0468c979635839a4`.
Upstream revision: `ce8164ba92995cfa86005b6259115e08c8244253`.

Source bytes were verified against the archive. The license files come from
that exact upstream revision, including the Chromium BSD notice.

Local changes check native UTF-16 range arithmetic, return empty geometry for
detached views or invalid coordinates, and require declared, enabled, writable
value actions. Native text values are bounded to 8192 UTF-16 units before
allocation and reject malformed surrogates; numeric values must be finite.
Selection changes require their own declared capability.

Portable boundary tests run on all platforms. A native NSRange regression and
the `native-accessibility-smoke` example exercise the actual macOS adapter;
the example uses AppKit objects on the main thread and checks stale teardown,
Unicode text, value limits and detached views. It never launches a terminal or
changes system accessibility permissions. Source and dependencies are platform
scoped and pinned by `tools/ci/check_accessibility_contract.py`.

This unpublished local source must not receive a registry audit. Native macOS
runtime results must be recorded from `.github/workflows/accessibility-native.yml`
independently for its Intel and Apple Silicon jobs. Cross-compilation, portable
boundary tests and native API smoke do not certify VoiceOver or OS IME usability.
