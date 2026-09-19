### Improved

- Keyboard hyperlinks now support Tab/arrow navigation, exact destination review,
  explicit Enter activation, copying and Escape. Existing Ctrl+Alt+O hint
  configuration remains authoritative.
- Fixed history/Unicode/wrapped-link detection, ambiguous labels, duplicate OSC
  anchor fragments and altered explicit URI punctuation. Bound captured state
  and reject stale/unsafe default-open targets without logging private targets.
