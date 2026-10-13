Fixed

- Keep Windows native-test cleanup bound to process creation times and retained
  handles, so recycled process IDs cannot select unrelated processes. Add stale
  ancestry, PID replacement, native cleanup, and session-recovery coverage.
