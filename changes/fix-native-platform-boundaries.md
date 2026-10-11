Fixed

- Open Linux/macOS PTYs with atomic close-on-exec ownership so unrelated child processes cannot inherit other panes' descriptors.
- Deny OSC 52 clipboard reads and writes from terminal output; explicit user copy and paste remain available.
- Bound unfinished Kitty image uploads to eight transfers and 64 MiB of retained payload capacity per terminal. Expire idle uploads after ten seconds on the existing worker and clear them on terminal reset.
- Keep macOS process paths and Foundation locale strings owned and bounded across native calls, preserving non-UTF-8 Unix paths.
- Include and discover session integration in Linux DEB/RPM installations.
- Add native boundary, idle cleanup, packaging and adversarial-input performance regressions. Native macOS runtime evidence remains a separate CI requirement.

- Optional prompt discovery now sleeps when no ready local prompt is visible. The footer clock schedules its own minute-boundary update, so disabled discovery or hidden clocks do not require background repaint polling.
