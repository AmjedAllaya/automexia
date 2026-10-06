# ADR 0095: Separate application performance observation

Status: Implemented; controlled native baselines are independently required.

## Decision

Keep the private Criterion package's benchmark-only compilation boundary. Put the
Python application campaign alongside it under `tools/renderer-benchmarks/application`,
without adding a Cargo runtime target or dependency. Reuse bounded performance
JSON IO and the QA process supervisor. Existing S2 policy and its reviewed baseline
activation remain the sole owners of S2 release evidence.

The application owns a separate, default-off `application-benchmarks` feature.
Explicitly activated runs record bounded numeric observations on existing event,
search and successful frame-submission paths. One session owner finalizes at
application shutdown; a returning event loop uses the same one-shot finalizer.
Overflow, poisoned observation, incomplete worker shutdown and missing frame
endpoints cannot produce complete evidence. No terminal text, user commands or
native process/window identifiers enter public evidence.

Native process counters and input drivers remain contributor adapters. Collection
uses isolated configuration, exact executable/argument vectors, fixed synthetic
workloads and existing process-tree retirement. Native keyboard probes require
ownership and focus. Platform-specific adapters and missing sensors are explicit;
a successful Windows run does not verify Linux, macOS or another GPU.

The application evidence schema is independently versioned. Cohort equality,
independent repetitions, raw samples, noise handling and observed-range regression
comparison precede any CI claim. There is no automatic baseline promotion, FPS
gate or implication that submitted frames equal physical scanout.

## Consequences

Normal builds have no observer overhead. Measurement builds include bounded
observer overhead, so the observer and driver versions are part of the cohort.
Microbenchmarks remain useful for attribution; application results establish the
measured user-facing boundary. Native clock, driver, display and hardware evidence
must be evaluated per platform. Removing the tooling does not migrate user data,
preferences, public configuration or terminal state.

See the [performance methodology](../../tools/renderer-benchmarks/README.md) for
commands, metric endpoints, privacy scope, CI behavior and current limitations.
