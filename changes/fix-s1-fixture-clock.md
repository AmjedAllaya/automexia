# Deterministic S1 assurance fixtures

S1 mutation tests no longer compare fixed synthetic evidence with the live
calendar. The shared fixture helper supplies an explicit UTC clock and verifies
its valid baseline before mutation. New regressions cover clock independence,
exact freshness/skew limits, review ordering and live-clock stale rejection.
Production evidence freshness and native release requirements are unchanged.
