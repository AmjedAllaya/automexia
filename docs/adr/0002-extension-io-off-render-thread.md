# ADR 0002: Extension discovery is asynchronous

Status: Accepted

## Decision

DevOps/context filesystem discovery runs on a bounded Automexia extension worker. The renderer submits non-blocking refresh requests and consumes cached snapshots by generation.

## Consequences

The HUD may be briefly stale after a directory change, but rendering never blocks on config files. This is an intentional latency/correctness tradeoff.
