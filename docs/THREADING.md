# Threading and latency model

Automexia treats terminal IO, rendering, UI routing and extension services as different latency domains. The goal is not to copy another terminal's exact scheduler; it is to preserve the same professional principle: independent work classes should not stall one another.

## Required separation

```text
PTY/ConPTY read + parser/state ──────┐
PTY/ConPTY write/input ──────────────┼── terminal session scheduling (engine-owned)
render snapshot/damage ─────────────┤
GPU draw/present ────────────────────┘

application event/router ─────────────── application-owned

extension/context discovery ──────────── Automexia worker-owned
future package/network/sandbox work ──── broker/worker-owned
```

Rio-derived terminal scheduling remains authoritative for terminal session IO and rendering in v0.3. Automexia does **not** rewrite that scheduler without measurements and conformance evidence.

## Automexia extension-worker model

The built-in DevOps HUD previously performed its local filesystem/config discovery synchronously when the HUD refresh interval expired. v0.3 removes that work from rendering.

```text
Renderer
  │ session facts changed / refresh deadline
  │
  ├─ try_send(refresh) ──────────────┐    capacity = 1
  │                                 ▼
  │                        automexia-extension-worker
  │                                 │
  │                         bounded local reads
  │                         sanitize/normalize
  │                                 │
  │                                 ▼
  │                         bounded per-session snapshot cache
  │                                 │
  └─ atomic generation check ◄──────┘
        │
        └─ clone/rebuild UI model only when generation changes
```

Properties:
- DevOps context discovery (Kubernetes/Docker/cloud/Terraform/Git) is owned by the worker, while semantic row classification is a pure in-memory function used only for rows already being rebuilt;
- renderer submission never blocks;
- a full queue never grows memory; a busy submission is retried soon so another active window/pane is not starved;
- filesystem work never runs in the render adapter;
- snapshots are keyed by terminal route/session so concurrent windows/panes do not overwrite each other;
- the context cache is bounded;
- each completed request receives a per-session completion revision, so one pane cannot acknowledge another pane's pending refresh;
- the global cache generation is published after the snapshot write is complete;
- failure to start the optional extension worker degrades extension UI instead of crashing terminal startup;
- no terminal-output event is allowed to trigger a shell/process execution path.

## Synchronization policy

Use the smallest synchronization primitive that matches ownership:
- atomics for generations/cheap invalidation signals;
- small locks for cold cached application models;
- bounded channels for asynchronous work submission;
- terminal-engine-native synchronization for terminal state;
- no “one giant application lock” shared by parser, renderer and extension services.

Never hold an Automexia runtime lock while performing filesystem/network/process work.

## Future third-party extension scheduling

A third-party extension must not receive an unrestricted thread inside the terminal process. The planned sandbox host should provide:
- per-extension execution/fuel budgets;
- memory limits;
- bounded event queues;
- cancellable asynchronous capability requests;
- backpressure/drop policy for high-frequency events;
- crash/trap isolation;
- metrics for queue latency and throttling.

The terminal and renderer remain usable even if an extension is slow, broken or denied a capability.
