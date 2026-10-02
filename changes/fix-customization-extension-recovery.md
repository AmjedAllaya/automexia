Preserve saved extension choices when package inventory is uninitialized or
unavailable. Confirmed uninstall also removes choices from an active temporary
restore snapshot, while the existing preview guard keeps saved files unchanged.

Extension updates can reclaim retired options when a new edit reaches the saved
choice limit while preserving current controls and other extensions. Feature and
package resets clear only their own choices, including retired controls.

Customizations now reports asynchronous package loading and failures, with a
reopen-to-retry hint that preserves save errors, field drafts and reset feedback.
Regression coverage includes repeated Reset, closing/reopening the sheet,
inventory failure/recovery and Restore saved without changing saved files.
