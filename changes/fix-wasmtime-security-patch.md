Fixed

- Raise the optional component host's Wasmtime minimum to 48.0.6 for the
  security fixes published on 2026-10-14. Preserve the existing feature set,
  capability boundaries and resource limits. Reject the affected 48.0.4 and
  48.0.5 versions in the dependency regression check.
