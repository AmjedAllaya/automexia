---
name: automexia-systematic-debugging
description: Diagnose Automexia defects, crashes, hangs, flaky tests, CI failures, and platform or package discrepancies using reproducible evidence and discriminating experiments. Use before fixing an unexplained failure; performance measurement belongs to automexia-performance-investigation.
---

# Automexia Systematic Debugging

Explain the failure with evidence, fix its owning contract, and retain a regression that detects the original cause. Do not start by changing timeouts, clearing caches, upgrading dependencies, or copying a guessed fix.

## Establish the case

1. Record expected behavior, actual behavior, shortest known trigger, affected users/platforms, frequency, and impact. Separate observations from hypotheses. Ask only for missing facts that materially change the investigation; inspect available evidence first.
2. Identify the exact source or package: revision, dirty source identity, artifact digest, target, features, compiler, OS, shell, and relevant configuration origin. A development build and a downloaded installer are different cases.
3. Preserve the first failure's exit status, bounded redacted logs, seed/input, and owned-process state. Do not print user commands, secrets, home paths, or unrestricted environment dumps.
4. Trace the live path through its entrypoint, dispatch, state owner, platform adapter, and observable result. Inspect feature gates and configuration precedence before assuming the code is reachable.

Read [references/failure-modes.md](references/failure-modes.md) only for the observed failure class. Load `$automexia-isolated-validation` before executing a reproduction; never try the application or test on the personal workstation.

## Form and test hypotheses

Keep a small working record: **hypothesis, supporting evidence, predicted observation, discriminating experiment, result**. Rank by evidence and user impact, not familiarity. Keep the record in private notes without executable helpers.

- Reproduce with the original conditions first. Reduce inputs only while preserving the failure; retain one normal control. If reproduction is unavailable, report source-supported risk separately from a reproduced cause.
- Change one relevant factor at a time: package versus source build, clean versus migrated profile, shell policy, optional dependency, architecture, renderer, or cache state. Reset disposable state between comparisons.
- Choose experiments that can disprove the leading hypothesis. Add bounded instrumentation at the suspected boundary and verify that instrumentation does not change scheduling or behavior enough to hide the defect.
- For regressions, inspect a known-good/bad range only when it helps isolate the cause. Use a separate checkout and consistent environment for a bisect; never reset or stash away the user's working tree.
- For races, use explicit barriers, deterministic scheduling, or controlled fault injection where available. For hangs, observe blocked owners, queues, locks, child state, and timeout cleanup rather than assuming the slowest log line is the cause.
- If an experiment does not discriminate between hypotheses, redesign it. Do not pile unrelated changes into a speculative fix.

## Fix and prove the cause

When implementation is authorized, load `$automexia-feature-planning` for a contract-impacting fix and `$automexia-terminal-assurance` when terminal boundaries are involved.

1. Add or identify a real-path regression that fails for the observed reason. Verify its target and nonzero test count. A setup error or missing dependency is not the intended failing assertion.
2. Implement the smallest correction in the existing owner. Preserve legitimate behavior, security policy, compatibility, and cleanup ownership.
3. Run the regression, legitimate control, relevant negative/boundary cases, and owning target in the verified isolated environment. Reproduce package-only defects against newly built package bytes as well as source tests.
4. Explain why the change removes the cause. Review important callers and adjacent platforms for the same mechanism without broadening into unrelated repairs.
5. Remove task-owned temporary instrumentation or retain it only with bounded, redacted production behavior and a clear owner. Self-review the final non-trivial diff with `$automexia-code-review`.

## Retries and CI

Inspect the exact job, step, source commit, first failing command, exit code, and artifacts. Distinguish product failure, test/harness defect, configuration drift, missing prerequisites, and external service failure. A log summary alone is not enough to assign a cause.

Retry only to test a hypothesis or recover a documented transient failure, with a bounded attempt/time budget. Keep all failed attempts in the evidence. A green retry does not establish that a flake is fixed. Do not add `continue-on-error`, broaden retries/timeouts, remove coverage, or quarantine a test to hide a failure.

When repeated attempts yield no new evidence, change the investigation or name the unavailable prerequisite. Continue independent safe work. Do not claim completion while the original failure is unexplained.

## Handoff

State the trigger, observed cause or remaining hypothesis, changed owner, regression/control results, isolated environment and artifact identity, and unresolved evidence. Do not present an inferred cause as confirmed or claim that another platform ran because its code compiled.
