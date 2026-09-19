# UX review report

Use this structure for a design audit, implementation review, or native acceptance record. Omit sections that are genuinely irrelevant and explain material exclusions.

## Review identity

- Revision or immutable artifact:
- Scope and non-goals:
- Platforms, renderers, themes, scales, and viewports exercised:
- Input methods and accessibility technologies exercised:
- Fixtures and privacy controls:
- External or unavailable evidence:

## User tasks

For each task record:

1. starting state and entry point;
2. user goal;
3. exact actions;
4. expected semantic, visual, focus, input, and recovery behavior;
5. observed result;
6. artifact or test evidence;
7. pass, fail, partial, or external classification.

## Findings

Give each finding a stable ID and include:

- severity based on task failure, safety, accessibility, data loss, input isolation, frequency, and recoverability;
- observed scenario and evidence;
- affected users, platforms, states, and interactions;
- authoritative owner and likely boundary;
- acceptance criterion;
- recommended correction without prescribing an unsupported implementation;
- tests and native evidence required to close it.

Do not report personal preference as a defect. Tie visual guidance to hierarchy, readability, consistency, accessibility, product identity, or task performance.

## Accessibility record

Separate:

- semantic model tests;
- native accessibility API inspection;
- screen-reader task result;
- keyboard-only result;
- contrast, zoom/reflow, and reduced-motion result;
- remaining human assessment.

## Visual record

For each image or comparison record:

- artifact path or private identifier;
- revision, fixture, theme, scale, viewport, renderer, font, and color space;
- expected source and independent oracle;
- comparison method and tolerance;
- human inspection result;
- privacy/redaction confirmation.

## Performance and cleanup

Record the measured layer, controlled environment, sample method, correctness assertions, baseline identity, result, resource ceilings, and cleanup observation. Do not compare numbers from different workloads as a regression result.

## Decision

Conclude with accepted behavior, blocking findings, non-blocking findings, external gates, rollback or disable path, and the exact evidence needed for the next status change.
