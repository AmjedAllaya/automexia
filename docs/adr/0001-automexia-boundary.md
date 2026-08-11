# ADR 0001: Automexia application boundary

Status: Accepted

## Decision

Automexia-owned product and extension behavior lives under `frontends/rioterm/src/automexia` and communicates with the Rio-derived terminal engine through narrow snapshots/adapters. VT/PTY/font/render-backend internals do not depend on Automexia marketplace or extension business logic.

## Why

This reduces upstream conflict surface, makes the eventual standalone fork manageable, and prevents product features from destabilizing terminal correctness/performance.
