# Acknowledge Quick Action saves observed by the file monitor

Treat publication of the same committed revision and digest as a successful save
when the monitor has already loaded it. Preserve later diagnostics and continue
to reject conflicting or older snapshots. Add deterministic real-store race and
negative tests for the failure exposed by the repository gate.
