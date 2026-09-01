# Terminal-first remote operations

Status: public product and interaction summary.

Automexia is designed for people who want the speed of a terminal without
manually carrying every context detail across commands and tools. Important
operations remain reachable from the prompt, keyboard, command palette, and
compact overlays. Large permanent dashboards are optional, not the default.

## Shared interaction pattern

1. Start with an ordinary command or a short action name.
2. See the current user, environment, and remote context before the command.
3. Search a bounded list of relevant profiles, resources, or actions.
4. Review identity, target, risk, and exact effect when the action needs
   authority.
5. Insert editable text or explicitly approve a typed action.
6. Observe the result in a normal terminal session and recover with native
   tools when an extension is unavailable.

## Capability areas

- SSH inventory, routes, host trust, sessions, tunnels, and certificates;
- environment capsules for isolated account, region, cluster, namespace, and
  tool context;
- command completion, aliases, Quick Actions, and reusable recipes;
- cloud, Kubernetes, OpenShift, and infrastructure adapters;
- workspaces, multi-pane sessions, transfer and remote-file actions;
- bounded session memory and diagnostic navigation;
- optional situation-aware production guidance;
- optional Automation Studio and LLM Orchestration extensions.

## Invariants

Terminal input and output never wait on provider, filesystem, authentication,
or network work. Actions use exact executables and argument arrays. Credentials
remain in approved custody. Context is pane-scoped. Expensive work is explicit,
bounded, cancellable, and safe to disable. Color is never the only production
warning, and all primary flows must support keyboard and accessible navigation.

Detailed internal delivery sequences and unreleased workflow recipes are kept
outside the public repository. See the [Roadmap](ROADMAP.md) for direction and
the [user guides](user-guide/index.md) for behavior users can rely on today.
