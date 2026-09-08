# Preserve native resize failures during branch integration

- Exercise real intermediate worker resize commits as well as coalesced bursts,
  retaining exact output order, prompt adjacency and successful child cleanup.
- Add phase/dimension and bounded later-state diagnostics only on assertion
  failure. A later redraw or successful retry never changes that failure to pass.
- Record the intermittent native CMD prompt-adjacency failure as unresolved.
  Isolated and concurrent repetitions supplement, but do not erase, the failed
  full-suite evidence. No production resize fix or merge readiness is claimed.
