# Review by affected boundary

Use only applicable checks. Each finding still needs a reachable trigger and evidence; this is not a requirement to report something from every row.

| Boundary | Inspect |
| --- | --- |
| Public API, CLI, configuration | Defaults, precedence, validation, units, errors, versioning, deprecated values, migrations and documentation |
| Input/parser | Byte and recursion limits, Unicode boundaries, fragmentation, malformed inputs, overflow and allocation before validation |
| Async/worker | Cancellation, stale generation rejection, publication before wakeup, queue saturation, backpressure, lock ordering, shutdown and retained cleanup ownership |
| PTY/process | Exact argv, environment construction, start failure, process identity, reaping, timeout, descendant ownership and surviving sibling sessions |
| Renderer/UI | Input routing, focus, accessibility state, hit-test versus painted geometry, clipping, scale, theme, resize and hot-path work |
| Persistence | Schema compatibility, atomicity, interrupted writes, corruption, migrations, restart, rollback and permission failures |
| Filesystem/archive | Canonical containment, links/reparse points, traversal, quotas, ownership markers, overwrite rules and bounded cleanup |
| Provider/security | Trust transitions, capability checks, authentication boundaries, opaque secret references, redacted diagnostics and least privilege |
| Dependencies/generated code | Existing owner, license/provenance, exact toolchain, reproducible inputs, generator changes and transitive impact |
| Build/package | Actual executable and companions, feature defaults, install layout, shell integration, runtime libraries, signatures and source identity |
| Tests/CI | Meaningful failing oracle, controls, isolated state, skip handling, environment/commit identity, gate preservation and bounded retries |
| Performance | Measured layer, hot-path I/O, allocations, contention, limits, long-run growth and cleanup; use the performance skill for quantitative claims |

## Finding quality check

Before keeping a finding, answer:

1. Can the triggering state occur through an actual caller and enabled feature?
2. What precisely happens, and why does the existing guard not prevent it?
3. Is this introduced by the reviewed patch, or explicitly in scope for a subsystem review?
4. Which documented behavior, invariant, or user outcome is violated?
5. Is the confidence based on execution, direct source reasoning, or an unresolved assumption?
6. Is the proposed next step within the user's requested scope?

For a test gap, identify the important unproven behavior and how a defect could pass the current suite. "More tests would be good" is not an actionable finding.
