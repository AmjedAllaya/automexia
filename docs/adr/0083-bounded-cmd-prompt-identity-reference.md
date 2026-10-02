# ADR 0083: Bounded CMD prompt identity reference

Status: Accepted

## Context

Native CMD truncates its PROMPT format after 511 UTF-16 code units. The previous
identity-rich format exceeded that limit, removing the visible input glyph and
OSC 133 input marker. Echoing the environment variable from a batch did not
exercise interactive prompt expansion and missed this failure. Removing identity
republication would instead retain guest metadata when a nested shell returned.

## Decision

Reuse OSC 1337 SetUserVar, the terminal's existing bounded user-variable map and
write chronology, and the application's ShellMetadataState admission owner.
There is no new parser, registry, worker, filesystem access or prompt process.

Applications advertise `AUTOMEXIA_CMD_REFERENCE_V1=1` through the existing child
environment owner. That owner computes a stable 32-character lowercase hexadecimal
correlation key: the first sixteen bytes of SHA-256 over UTF-8 user, NUL, UTF-8
shell executable path. This key is not authentication. The PowerShell CMD launcher
refreshes the identity once per launch and restores its caller's environment on
return, including failure paths.

Startup registers two ordinary base64-encoded user variables, consecutively:

- `automexia_cmd_user_v1_<key>`: user identity;
- `automexia_cmd_path_v1_<key>`: shell executable path.

Each prompt publishes the existing complete `automexia_env_pending` frame with
CMD shell name, activation, and `automexia_cmd_ref_v1` containing the key. The
existing OSC 7, title, A/P/B markers, full directory and input glyph are unchanged.
The resulting format is 466 code units with the default or lambda glyph,
independent of identity lengths. Each registration is a separate CMD write, so
the maximum accepted 4096-byte fields also stay below CMD's 8191-character line
limit. The reference branch never constructs the old oversized identity string.

Admission accepts only exact version-1 key syntax, two valid nonempty values,
consecutive accepted registration stamps, and registration before frame begin.
Partial replacements, missing records, malformed references and late writes are
unavailable, rather than permission to borrow another shell's identity or local
provider state. Subsequent valid frames recover. Old framed and unframed resources
retain their existing admission behavior; absent capability uses the old script
format. Applications need current resources to receive this fix.

## Bounds and lifecycle

Registrations remain in the existing per-terminal map: at most 128 total names,
8192 bytes per generic value and 64 KiB total; CMD admission further limits each
identity value to 4096 bytes without control characters. Repeated launches of the
same identity reuse two entries. Distinct nested identities use distinct keys.
The existing map does not evict records to accommodate a new identity: saturation
rejects new registration, leaves existing identities intact and marks unresolved
references unavailable. No metadata is persisted or added to diagnostics.

Clear-window, ED2 and full terminal reset preserve existing user-variable values
and monotonic write stamps. A new terminal starts with an empty map. Regression
tests exercise real reset parsing, nested return, changed directories, missing
records, saturation and independent terminal state. CMD still owns line editing;
the integration never injects commands or Enter to repaint a prompt.

## Evidence and alternatives

`tools/ci/test_cmd_prompt.py` executes tracked CMD startup and actual interactive
prompt expansion with fictional long/Unicode identity, maximum-size fields,
default/ASCII/lambda glyphs and directory changes. Renderer metadata tests exercise
real OSC dispatch and snapshot admission. Native application GUI coverage remains
a separate required gate; these focused tests do not claim other OS execution.

Keeping identity only at startup fails nested return. Merely trimming prompt
decoration cannot bound arbitrary accepted identity. CMD does not expand `%VAR%`
or `!VAR!` inside PROMPT, so environment indirection cannot remove this limit.
