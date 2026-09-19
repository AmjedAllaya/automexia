# Evidence ladder

Use the lightest rung that proves the current step, then broaden only when the change or result justifies it.

## Rungs

1. **Static inspection**
   - Re-read the edited owner, important callers, feature gates, tests, and diff.
   - Check typed units, error paths, cancellation, cleanup ownership, redaction, bounds, and stale-result rejection.
   - Verify generated sources were changed through their generator.

2. **Regression or characterization test**
   - Run the exact new real-path regression and confirm a nonzero test count.
   - When feasible, demonstrate that it failed before the fix for the expected reason.
   - Run the legitimate control and at least one alternate hostile or failure class.

3. **Owning target**
   - Run the owning crate/library/binary/integration target with relevant features.
   - Include disabled/default/combined feature variants when the dependency or authority surface changes.
   - Run relevant property, fuzz, model, or benchmark correctness modes.

4. **Boundary integration**
   - Exercise real PTY, shell, renderer, process, persistence, provider, capability, or platform adapters affected by the change.
   - Verify exact output, exit status, cleanup, and resource recovery.

5. **Policy and architecture**
   - Run feature reinforcement, architecture, dependency, unsafe, licensing, provenance, documentation, workflow, privacy, and release-policy checks that own the changed contract.
   - Mutation-test policy/checker changes so the gate fails when its protected invariant is removed.

6. **Repository profile**
   - Run the repository-prescribed focused, CI, QA, readiness, or pre-push profile appropriate to the authorized task.
   - Do not rerun an expensive full profile without new edits or unresolved evidence.

7. **Native and external evidence**
   - Run each claimed OS, shell, compositor/GPU, architecture, input method, accessibility technology, credential backend, packaging, signing, account, hardware, and long-duration campaign.
   - Record unavailable combinations as external gates.

## Failure handling

- Preserve the first failure and investigate its path before retrying.
- Attribute failures only after inspecting the failing owner and starting state.
- Concurrent edits or shared build targets invalidate a run only when evidence shows they changed its inputs or outputs.
- A later pass does not explain an earlier timeout, crash, corruption, leak, or flake.
- Do not weaken tests, bounds, security, or policy to obtain a pass.
- If a tool or environment is unavailable, run safe independent gates and report the exact remaining evidence.

## Artifact rules

- Generate artifacts from the current revision and inputs.
- Verify identity, timestamp or digest where the repository contract requires it.
- Parse reports after redaction or transformation, not only before.
- Inspect raster content, semantic snapshots, package contents, signatures, provenance, or reports as applicable.
- Keep private artifacts outside public repository paths and remove temporary evidence when its owner requires cleanup.

## Supported completion statement

The final evidence record must name:

- changed contract and owner;
- regression and controls that ran;
- owning and repository gates that passed;
- native platforms and configurations that actually ran;
- fresh artifacts inspected;
- unresolved failures, external gates, and practical limits.
