# Focused debugging probes

Select only rows that can explain the observed symptom. Each probe must run through the isolated-validation boundary.

| Failure class | Useful discriminating evidence | Common false conclusion |
| --- | --- | --- |
| Works on developer machine, fails for a new user | Exact package in a clean customer image; shell defaults, policy, fonts, runtime dependencies, configuration origin, first-run state | A new profile in the same host proves a new OS install |
| Works in source build, fails in package | Compare package inventory and startup environment with the build output; exercise actual installation and entrypoint | Version output or extraction proves the application works |
| Missing information tags or shell behavior | Observe bootstrap selection, shell execution policy, module paths, integration load and metadata arrival at the parser | Disabling shell policy is an acceptable product fix |
| UI action appears inert | Trace enabled state, focus, pointer/keyboard routing, command dispatch, state publication and renderer wakeup | Updating a screenshot baseline fixes the interaction |
| Flaky timeout or hang | Monotonic timeline, readiness conditions, wait ownership, queue pressure, process identity and cleanup completion | A larger delay explains the failure |
| Crash or corrupted state | First failing stack/input, native error, feature combination, ownership/lifetime, persistence version | The final logged function necessarily caused the crash |
| Resource leak or shutdown stall | Baseline and post-cleanup processes, handles, threads, queues, memory and retained owners across repeated cycles | The parent process exiting proves all children were cleaned up |
| CI-only failure | Exact runner image, target, compiler, feature set, shell, dependency versions, job isolation and artifact download identity | Local success invalidates CI evidence |
| Cache-sensitive failure | Same source and prerequisites with separate cold/warm disposable caches; generated-input identity | Clearing the user's caches is a diagnosis |
| Upgrade-only failure | Supported prior package and fictional persisted state, migration result, restart and rollback behavior | A clean install covers upgrades |
| Network/service error | Status, request identity without secrets, service availability, timeout stage, retry semantics | Retrying a state-changing operation is always safe |

For an external service, inspect whether a retry can duplicate a side effect. Reuse existing idempotency or reconciliation mechanisms; do not invent a second owner during diagnosis.

For a platform discrepancy, compare the same logical contract while recording intentional OS differences. An OS-specific workaround must not silently weaken another platform's behavior.

## Minimal failure record

- Expected and observed behavior; user impact.
- Source/package and environment identities.
- Reproduction steps, seed or minimized fictional input, control.
- First failure and subsequent attempts, including changed factors.
- Hypotheses accepted/rejected and the evidence that distinguishes them.
- Owning correction and permanent regression, or exact unavailable evidence.

Keep raw/private diagnostics out of public reports. Record durations and stable labels instead of machine names or full environment values.
