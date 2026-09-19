# Evidence ledger

Use this reference when planning any non-trivial Automexia implementation. The ledger prevents duplicate owners, roadmap-driven assumptions, and unsupported completion claims.

## Classification

| Status | Minimum evidence | Planned action |
|---|---|---|
| Fully implemented | Source owner, important callers, contract tests, applicable platform evidence, and current documentation agree | Preserve the owner and record why no implementation is needed |
| Partially implemented | A real owner or behavior exists, but a contract, boundary, failure case, platform, test, or document is missing | Extend the existing owner and close the named evidence gap |
| Not implemented | No authoritative source-and-test owner implements the contract | Design the smallest architecture-aligned owner and its tests |
| External prerequisite | Hardware, account, signing identity, native OS, assistive technology, controlled runner, long campaign, or human assessment is required | Automate local preparation and leave the feature partial until the external gate runs |

Roadmap prose, filenames, test names, mocked returns, line coverage, compilation, and zero-test success are not implementation evidence.

## Ledger fields

Create one row per independently verifiable behavior or contract.

| Field | Required content |
|---|---|
| Requested behavior | Concrete user-observable or contributor-observable contract |
| Status | Full, partial, missing, or external |
| Source owner | Exact crate/module/type/function or explicit absence |
| Consumers | Important callers, routes, features, and platform adapters |
| Existing tests | Exact unit, integration, conformance, native, fuzz, property, benchmark, or policy owners |
| Documentation | Current specification, ADR, roadmap, public docs, and mismatches |
| Reuse candidates | Existing functions, crates, SDKs, libraries, tools, generators, or external alternatives |
| Trust and authority | Input source, capability, privilege, credential, filesystem, process, network, or persistence boundary |
| Resource behavior | Byte, item, queue, task, time, retry, cache, history, storage, process, and cleanup limits |
| Platform behavior | Windows, Linux/BSD, macOS, shell, renderer, architecture, and feature differences |
| Missing proof | Specific behavior or evidence that is absent |
| Exit criteria | Observable tests, artifacts, reviews, and external gates required for completion |

## Evidence rules

- Separate observed behavior, reproducible failures, source-derived risks, and hypotheses.
- Verify callers and feature gates before declaring code live, dead, or duplicate.
- A line-based scan is not semantic review. Repeated-token counts are not duplicate behavior.
- Generated or vendored code keeps its generator or upstream owner unless the plan explicitly changes that boundary.
- Test copies may be independent oracles. Do not consolidate them merely because their text resembles production code.
- A cross-compile proves compilation for a target, not native process, compositor, accessibility, packaging, or credential behavior.
- Retrying a flaky test does not erase the first failure. Record the cause or leave the evidence unresolved.
- Filtered tests must report the executed names and a nonzero count in the correct target.
- Artifacts must be freshly generated and inspected; stale reports, pixels, or packages cannot justify the current change.

## Reuse candidate record

For every material reuse or consolidation candidate, record:

1. source locations and production consumers;
2. common invariant;
3. semantic differences in errors, defaults, permissions, units, limits, lifecycle, and platform support;
4. proposed authoritative owner;
5. rejected alternatives and their costs;
6. characterization and regression tests;
7. migration and removal criteria for any temporary adapter.

Classify apparent duplication as one of:

- duplicate mechanism;
- divergent behavior requiring reconciliation;
- intentional separation;
- compatibility forwarding;
- generated or upstream code;
- independent test evidence.

## Completion check

The ledger is ready only when every row has an owner or an explicitly external gate, and every partial or missing row has a testable exit criterion. Do not convert external or unavailable proof into a passing status.
