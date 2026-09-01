# Situation-aware Production Operations experience

Status: planned public experience summary.

## Surface map

The public experience is intentionally small: the existing prompt area shows
environment context, completion shows a short set of relevant choices, a
progressive detail view explains evidence, and the application-owned review
surface handles any action that needs authority. These are roles, not a frozen
layout or internal state-machine specification.

## The simple workflow

1. The prompt area makes the current account and environment visible.
2. A user starts with an ordinary command or opens an incident-oriented action.
3. Automexia gathers only the permitted, bounded evidence needed for that
   question.
4. A compact list shows relevant options with a short reason, risk, freshness,
   and uncertainty indicator.
5. The user can inspect evidence or continue typing without losing focus.
6. A mutating choice opens a review that shows the exact target and effect.
7. Production policy requires an explicit confirmation after context is
   revalidated.
8. Automexia observes the result and offers verification, rollback guidance, or
   a clean incident handoff.

## Interaction requirements

- Keyboard use is complete; pointer use is optional.
- The interface never relies on color alone to communicate environment or risk.
- Production is visually distinct but not noisy.
- Explanations use direct language and fit narrow panes as well as wide screens.
- Loading never blocks typing or terminal output.
- Empty, stale, partial, offline, denied, cancelled, and failed states say what
  happened and how to continue with the native tool.
- Screen readers receive useful roles, names, state changes, and focus recovery.
- Users can disable recommendations while retaining the normal DevOps/SRE
  terminal features.

The exact layouts, key map, ranking presentation, and incident-state flow remain
subject to implementation testing and publication review.
