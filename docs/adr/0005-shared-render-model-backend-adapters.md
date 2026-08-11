# ADR 0005: Shared render semantics, backend-specific GPU adapters

Status: Accepted as architectural policy; incremental implementation

## Decision

Automexia features enter rendering as backend-neutral models/snapshots. GPU/backend-specific modules own only platform/device submission details. New product features must not be independently reimplemented in each renderer backend unless a documented backend capability requires it.

## Why

Duplicating feature semantics across GPU backends causes platform drift and makes fixes expensive. A shared render-model layer keeps ordering/damage/feature semantics testable once while preserving native backend optimizations.

## Consequences

The 0.x Rio-derived renderer is migrated incrementally; v0.3 does not rewrite working GPU code solely for architectural aesthetics. New Automexia functionality follows this rule immediately, and renderer refactors should move toward it when backed by tests/measurements.
