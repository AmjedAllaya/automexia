# ADR 0006: DevOps integration is native, passive and shell-independent

Status: accepted in v0.3.1; session-awareness hardened in v0.3.2

## Decision

Automexia DevOps is implemented as a first-party extension that contributes a cached native HUD model and display-only semantic row styles. It does not install shell wrappers, aliases, custom CLI commands, prompt scripts, or command interceptors.

The extension is enabled by default on fresh installs and can be explicitly disabled through `/market`. The explicit disable decision is persisted separately from the default.

## Rationale

The previous Automexia environment proved useful context/status semantics, but shell-level helpers such as context commands and output wrappers couple behavior to a particular shell/tool invocation. Automexia Terminal needs the same value across PowerShell, cmd, WSL and Unix shells without rewriting user commands.

Native passive integration also preserves the terminal trust boundary: terminal output cannot cause process execution and rendering does not depend on remote connectivity.

## Consequences

- context is local/configured state, not guaranteed live connectivity;
- semantic coloring may enhance plain output but must preserve explicit ANSI colors;
- context discovery stays on the bounded extension worker;
- `/market` is application UI, not a PTY/CLI command;
- future active actions (context switching, process execution, network checks) require separate capabilities and must not be smuggled into this passive built-in.
