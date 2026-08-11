# ADR 0003: Stable releases pin the terminal engine

Status: Accepted

## Decision

A stable Automexia package branches from an exact audited Rio SHA. Moving fork/upstream branches cannot silently change the release source.

## Consequences

Upstream fixes require an explicit compatibility update, but releases are reproducible and merge/source-drift risk is controlled.
