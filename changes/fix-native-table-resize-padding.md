# Preserve native table output through shrink and restore

- Match ConPTY hard-line padding semantics in the existing core reflow owner,
  preserving forced wraps, cursor distance, text extras and Unix whitespace.
- Add long-table real-shell/worker fixtures, duplicate checks, byte-fragmented
  native repaint and boundary/Unicode/colour regressions.
- Replace CMD's screen-moving acknowledgment with a non-echoing probe, verify
  its cursor/row invariance without resize, and retain separate Enter coverage.
- Include native integration binaries in default resize stress with exact
  dispatch, failure-propagation and mutation tests.
- Release ConPTY caller pipe handles and failed-startup resources; check exact
  native handle recovery across fully joined successful and failed launches.
- Parse pending output before native EOF when resizing holds the terminal lock;
  preserve unrelated I/O errors and independent child-exit verification.
- Label grid-only benchmarks accurately and add per-iteration table-content and
  history-bound oracles. Native desktop validation remains a separate gate.
