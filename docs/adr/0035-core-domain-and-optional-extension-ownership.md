# ADR 0035: Core terminal and optional extension ownership

Status: Accepted for the public free-terminal boundary

## Context

The terminal must keep optional dependencies and failures away from input, PTY,
resize, renderer, and startup hot paths.

## Decision

Core owns VT state, PTY/process lifecycle, input routing, panes/tabs,
configuration, renderer snapshots, clipboard, and stable capability-broker
contracts.

An existing extension owns cohesive optional behavior within its established
capabilities. A new extension requires a distinct optional authority or
dependency footprint, bounded lifecycle, explicit disable/uninstall behavior,
failure isolation, tests, and packaging ownership.

Optional code never becomes a second owner for terminal state, PTYs, processes,
routes, focus, or persisted core settings. Filesystem, network, authentication,
provider, credential, and optional process work stays outside core hot paths.

## Consequences

The basic terminal remains usable when optional code is disabled, missing, or
failing. Unreleased extension products and commercial placement are private and
outside this ADR.
