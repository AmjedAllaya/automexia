# Semantic Diagnostic Navigator

Status: planned. No general diagnostic detector is available yet.

## User value

Long command output and incident logs should not force users to search manually
for the point where a failure began. The planned navigator will let a user jump
to the latest relevant error section and repeat the action to visit earlier
sections.

The experience should feel like normal terminal navigation:

- one configurable keyboard action moves to the latest diagnostic section;
- repeating it moves backward through earlier sections;
- the reverse action moves forward again;
- the terminal explains what it found without changing the command line or
  sending input to the running process;
- when nothing is found, the terminal stays usable and gives a brief message.

## Product boundary

The terminal core may own bounded scrollback navigation because it already owns
terminal state and viewport movement. Domain-specific detectors belong to
reviewed extensions. The DevOps/SRE extension may later contribute Kubernetes,
cloud, and infrastructure patterns without giving provider authority to the
renderer or terminal hot path.

The first implementation should be deterministic and on demand. It must not
require an LLM, network access, a background index of all output, or automatic
command execution.

## Safety and resource principles

- Scan only a bounded portion of retained scrollback when the user asks.
- Reuse terminal-owned text and identities instead of copying full logs.
- Cancel obsolete work and reject results from another pane, session, or
  scrollback generation.
- Treat output and extension-supplied patterns as untrusted input.
- Bound pattern count, pattern complexity, bytes, matches, time, concurrency,
  cache size, and retained metadata.
- Never persist terminal content by default and never include matched text in
  telemetry or ordinary logs.
- Degrade to ordinary scrolling if detection is disabled, unavailable, or over
  its limits.

## Public delivery direction

Delivery begins with exact navigation to failures already known from terminal
command boundaries, then adds a small generic detector set, complete keyboard
and accessibility behavior, and finally reviewed extension contributions.
Specialized formats remain optional.

Detailed detector algorithms, internal ceilings, data structures, and delivery
recipes are maintained outside the public repository until implementation and
publication review justify exposing them.

See also [Roadmap](ROADMAP.md), [Command Productivity](COMMAND-PRODUCTIVITY.md),
and [public/private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).
