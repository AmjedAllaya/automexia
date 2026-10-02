# ADR 0085: Plain command input accents

Status: Accepted

## Context

PowerShell's PSReadLine supplies syntax colors, while the integrated CMD and
Bash editors normally send plain input. Zsh can also send plain input without a
syntax-highlighting plugin. Output status colors cannot fill this gap: they
have a different owner and must never classify an editable command as output.

## Decision

Keep native shell editors responsible for all input and native syntax styles.
For integrated CMD, Bash and Zsh, add a display-only command/option accent when
the shell uses the default foreground. The existing OSC 133 prompt owner keeps
the input start and dialect on each row. Reflow, snapshots and prompt repair
preserve that metadata; row replacement retires it. Metadata grants no execution
or filesystem authority.

The existing per-pane renderer snapshot caches accents on source or mode damage.
It inspects only bounded visible input, with no history search, executable
lookup, subprocess, provider access, keyboard handler, or input-buffer mutation.
Clipped input with an unknown beginning and complex substitutions remain plain.
This is an appearance aid for simple command words and options, not a shell
parser, command validator, completion engine or replacement editor. Replacing
Readline or CMD's editor solely to supply colors would violate that boundary.

Explicit native colors, dim/inverse/hidden styles, selections and search keep
their existing precedence. Fish and PowerShell retain native highlighting.
Full-screen and mouse-reporting applications remain outside this decoration.
Output and Kubernetes status palettes remain separate.

## Compatibility and verification

There is no new persistent setting, protocol, dependency or public command.
Bash and Zsh stop forcing a plain white foreground after the input marker.
Missing or disabled integration retains the shell's unmodified presentation.
Removing the accent cache restores the previous appearance without migration.

Regressions exercise input/output boundaries, shell and pane isolation,
Unicode/reflow, source-damage invalidation, bounded work and native foreground
precedence. A native Windows fixture checks actual glyph colors with CMD,
PowerShell and optional WSL Bash/Zsh/Fish startup resources. Windows and WSL
results do not establish native macOS or Linux GUI support.
