# Session-helper record protocol

This protocol exists only between a native editor adapter and the one
session-resident signed helper. It is not the authenticated application
endpoint; the helper translates it to ADR 0025's length-prefixed schema-1 JSON.

Each record starts with ASCII `AXSH`, one little-endian `u16` version (`1`), one
`u8` kind, and one little-endian `u32` payload size. Integer fields are
little-endian `u32`; strings are a `u32` byte length followed by strict UTF-8.
Unknown kinds, invalid UTF-8, trailing bytes, more than 512 candidates, fields
above their machine limits, or payloads above 512 KiB close the route.

Kinds:

- `1 request`: buffer string, cursor byte, adapter generation, then a candidate
  count followed by zero to 512 candidate strings. PowerShell currently supplies
  candidates from `TabExpansion2`; Bash, Zsh, and Fish send zero. The helper's
  inherited launch context supplies the shell/editor and authenticated route;
  the helper assigns request/candidate IDs and never trusts shell-supplied route
  identity.
- `2 accept`: current prompt/buffer generation and selected request-local
  candidate ID. The helper returns kind `4 replace` only after the application
  revalidates the authenticated route.
- `3 dismiss`: current generation and a bounded reason code.
- `4 replace`: exact start/end byte offsets and insertion text. The adapter
  revalidates current editor state and replaces once without Enter.
- `5 status`: redacted reason code only; no buffer, cwd, candidate, endpoint,
  handle, or capability material.

Request and response streams are inherited handles on Windows and inherited
file descriptors on Unix. They are closed on route teardown; the adapter never
opens TCP/UDP, an endpoint by name, a profile, a history file, or a second
helper. Only request records are implemented by the current inert scaffolds.
Accept, dismiss, replace, and status remain reserved protocol kinds until the
signed helper and native response/replacement adapters pass CP5.5 gates.
