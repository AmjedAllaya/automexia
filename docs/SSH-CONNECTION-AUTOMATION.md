# SSH connections and automation

Status: the read-only connection foundations exist in source; managed live
launch and broader automation remain release-gated or planned.

## User journeys

Users should be able to find a known host, understand the selected identity and
route, review host trust, and open a session with minimal ceremony. Reusable
workflows may add preparation, connection, remote setup, verification, and
cleanup, but every stage remains visible and cancellable.

The Connection Hub is the visual home for inventory and connection review.
Command-first users can reach the same records and actions from the palette and
terminal workflow. Neither path stores private keys or bypasses the user's
native SSH configuration.

## M6 review-only implementation

The current source includes review and editing foundations for typed recipes and
multi-environment workspaces. Execution remains disabled behind the independent
session-launch and native-release gates. A review surface is not proof that the
corresponding remote action is supported.

## Safe automation rules

- Profiles contain references and typed settings, not secret material.
- Recipes are structured actions, not opaque shell snippets.
- A plan shows the exact target, route, identity reference, executable,
  arguments, working directory, and environment changes before launch.
- Production or higher-risk steps require explicit review and policy checks.
- Retries are bounded and never repeat a mutation unless its contract declares
  safe behavior.
- Cancellation and shutdown clean up tunnels, processes, temporary files, and
  partially created state.
- A failed recipe leaves a clear manual recovery path using the native tool.

## Placement in the existing architecture

SSH and automation behavior belongs in first-party extensions behind the
application capability broker. Cloud and Kubernetes adapters can contribute
context and typed actions without becoming separate connection owners. Future
Automation Studio integration may edit scripts and recipes, but connection
state remains owned by the connection extension.

Detailed recipe schemas, provider mappings, risk catalogs, resource budgets,
and future phase instructions remain private until the corresponding behavior
is implemented and ready for public compatibility review.

See [Connection Hub](CONNECTION-HUB.md), [Session Launch Broker](SESSION-LAUNCH-BROKER.md),
and the [user guide](user-guide/connection-hub-and-ssh.md).
