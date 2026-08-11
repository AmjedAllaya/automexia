# ADR 0007: session-aware passive DevOps discovery

Status: accepted in v0.3.2

## Context

The Automexia desktop process owns the host environment. Entering a nested shell such as WSL cannot mutate that parent environment, so host-only discovery misses Docker/Kubernetes/cloud configuration that exists in the active Linux session. The original worker also used a global completion generation, allowing one pane's completed request to acknowledge another pane's pending request.

## Decision

- expose generic `SessionFacts` from the application snapshot rather than DevOps-specific data from terminal core;
- snapshot the raw terminal/OSC title alongside cwd under the existing terminal snapshot lock;
- key DevOps cached snapshots by terminal route/session id;
- assign a per-session completion revision and retain a global atomic generation only as a cheap wake signal;
- on Windows, passively recognize conventional WSL terminal titles and use `\\wsl.localhost`/`\\wsl$` filesystem bridges for bounded config reads;
- never spawn `wsl.exe`, Docker/Kubernetes/cloud CLIs, or network calls for HUD discovery;
- schedule short redraw polling only while a queued async refresh is pending.

## Consequences

A quiet terminal displays the HUD when discovery completes, panes cannot overwrite/acknowledge one another, and nested WSL sessions can surface local configured context without custom CLI integration. Discovery remains best-effort: a shell that publishes neither cwd nor recognizable title may expose less context.


## v0.3.8 amendment

The `\\wsl.localhost` / `\\wsl$` probing part of this ADR is superseded by ADR 0010. Runtime discovery now uses shell-published distro/version metadata plus `/mnt/<drive>` mapping and never enumerates WSL provider roots.
