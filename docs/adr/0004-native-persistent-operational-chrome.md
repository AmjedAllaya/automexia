# ADR 0004: Native persistent operational chrome

Status: Accepted

Automexia reserves renderer-owned space above the VT grid for profile tabs and
live operational context. Every shell prompt also reserves an empty semantic
row for a per-command context snapshot, followed by a complete-path row and a
short editable lambda/command row. Prompt and command state is communicated
through OSC 7, OSC 133 and
narrowly scoped OSC 1337 user variables; the renderer never inserts decorative
characters into PTY output.

This prevents context from disappearing when a command writes over a prompt or
when terminal columns reflow. Command completion metadata is stored on the
logical semantic prompt row and follows its stable `aid` through scrollback and
resize. Shell/context discovery remains asynchronous and local-only. Explicit
ANSI application styling, selection and search retain precedence over semantic
decoration.

Consequences: the application reserves 148 logical pixels when tab navigation
is enabled and three semantic rows for every prompt. Readline, ZLE, and
PSReadLine own only the short editable row; the complete path is terminal grid
history and reflows without shell-editor duplication. Stable `aid` values join
each context row to both continuations and let the renderer restore historical
snapshots after reflow. Windows supplies its own move/resize/control
hit targets when using disabled native decorations; chrome or prompt-row
changes require renderer, input, reflow and shell-integration regression
coverage.
