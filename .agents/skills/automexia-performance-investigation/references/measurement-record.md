# Performance measurement record

Use the applicable fields; a focused microbenchmark does not need a complete application campaign.

## Question and identity

- User-visible problem and measured layer; units and timing boundaries.
- Baseline and candidate revisions, dirty input digests, package/build identity.
- Exact harness/command, compiler, target, profile, features, dependency lockfile.
- Guest baseline, hardware/resource allocation, renderer, display, fonts and scale.
- Workload/corpus/seed, sizes, concurrency, operation count and correctness oracle.
- Cold/warm state, setup exclusion, warmup, samples, repetitions and run ordering.
- Required effect threshold, regression ceiling and statistical method.

## Results

| Metric and unit | Baseline | Candidate | Variation/uncertainty | Interpretation |
| --- | --- | --- | --- | --- |
| Requested user-path metric | | | | |
| Relevant resource/tradeoff metric | | | | |

Use absolute values beside relative changes. Record a zero or near-zero denominator explicitly rather than presenting a misleading percentage. Report unsupported percentiles as unavailable.

Separate profiler observations from benchmark samples. Preserve failed runs and explain any excluded setup/outlier sample without silently replacing it. If the harness does not provide a confidence interval, describe observed variation instead of inventing one.

## Relevant workload boundaries

- Empty, normal, large and limit inputs when algorithmic scaling changes.
- Idle versus sustained output, repeated resize/input and multiple sessions when contention changes.
- Cold launch versus warmed operation when initialization or caches change.
- Cancellation, timeout, restart and cleanup when ownership or retention changes.
- Repeated cycles or a bounded soak for memory, handles, child processes, queues and disk growth.

## Decision

Record correctness/control results, the measured cause, change in mechanism, resource tradeoffs, fresh artifacts, and unresolved noise or platform gaps. Define which inputs would invalidate this baseline or require another run.

Keep the detailed operational record private and redacted. Shared benchmark code, fixtures, and reproducible invocation belong in tracked owners; never place executable helpers in a private documentation folder.
