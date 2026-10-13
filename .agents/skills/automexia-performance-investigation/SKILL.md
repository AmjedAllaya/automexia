---
name: automexia-performance-investigation
description: Investigate and measure Automexia latency, throughput, allocation, CPU/GPU, memory, startup, build-time, and resource-growth changes. Use for performance regressions, benchmarks, profiling, and optimization; do not infer speed from code shape or run load on the personal workstation.
---

# Automexia Performance Investigation

Measure the user-relevant path, identify its limiting work, make one justified change, and compare equivalent workloads. A faster microbenchmark is not automatically a faster terminal.

## Define the measurement

1. Name the user outcome and measured layer: input-to-state publication, renderer enqueue, presented frame, PTY throughput, search, startup, build, or shutdown. Define units and observable start/end points.
2. Find the existing owner in applicable sections of `../../../docs/TESTING.md`, `../../../tools/renderer-benchmarks/README.md`, owning crate benches, and `../../../tools/ci/qa.py`. Read the actual harness before choosing flags or interpreting output.
3. State the workload, sizes/concurrency, correctness oracle, baseline revision, build profile/features, and relevant resource ceiling. Choose a practical effect threshold before seeing candidate results; use repository policy when it exists.
4. Use `$automexia-isolated-validation` to select the controlled environment. For startup/package complaints, distinguish customer-image performance from developer builds and cold from warm caches.

Use [references/measurement-record.md](references/measurement-record.md) for a campaign involving baseline/candidate comparison, multiple metrics, or long-run resources.

## Baseline and diagnosis

- Establish correctness first. Confirm actual operations, output, test/iteration counts, and completion; prevent dead-code elimination and accidental measurement of setup when using a microbenchmark.
- Record equivalent compiler, dependencies, features, input corpus, renderer, fonts/scale, guest resources, hardware and background load. Keep source changes separate from environment changes.
- Measure an unmodified baseline with sufficient independent runs to estimate variability. Use the owning harness's statistical method and required sample count; do not invent confidence from a handful of timings.
- Profile the baseline to locate allocations, blocking I/O, contention, rendering work, queue buildup, or hot functions. Use an existing platform profiler in the guest. Profiling overhead makes those timings unsuitable for direct comparison with unprofiled runs.
- Trace the suspected cost to actual callers and input sizes. Inspect throughput/latency tradeoffs, bounds, cancellation and cleanup before changing data structures or parallelism.

For an unexplained crash, hang, or flake during measurement, use `$automexia-systematic-debugging` and retain the failure. A failed correctness run is not a fast performance result.

## Change and compare

1. Use `$automexia-feature-planning` for a non-trivial optimization. Preserve the existing semantic oracle and public behavior. Avoid speculative caching, pooling, parallelism, unsafe code, or dependencies without measured justification.
2. Change the owning mechanism, then run focused correctness and boundary tests. A cache must retain explicit identity, bounds, invalidation, cancellation and cleanup ownership.
3. Compare baseline and candidate on equivalent isolated conditions. Interleave or randomize runs when feasible to reduce drift; do not compare different runner types, debug/release profiles, workloads or cache states as one regression result.
4. Report distributions and uncertainty for supported metrics, not just the best sample or a mean. Use p50/p95/p99 only when the harness and sample count support them. Keep failed and outlier runs with a documented diagnosis.
5. Check relevant tradeoffs: memory/allocations, CPU/GPU, handles/threads/children, queue depth, storage growth, startup, cancellation and post-run cleanup. Include repeated or long-lived operation when retention can change.
6. Rerun only checks invalidated by the final relevant edit and self-review the patch. If the effect is below noise or hardware differs, report inconclusive evidence rather than a speedup.

## Guardrails and completion

Do not loosen thresholds, change a baseline, remove slow cases, reduce representative inputs, or suppress failed runs just to pass. A reviewed baseline change must explain the semantic/environment change and preserve evidence of the prior result.

Keep hot-path diagnostics bounded and disabled unless requested by the owning harness. Never benchmark real customer sessions or publish captured terminal contents, process arguments, environment values, or identifying profiler paths.

Report the measured layer, source/build/environment identity, workload, baseline/candidate results, variability, correctness checks, resource tradeoffs, and limitations. Distinguish measured improvement, no detected regression, inconclusive evidence, and not run. Controlled guest results do not certify physical hardware behavior outside that environment.
