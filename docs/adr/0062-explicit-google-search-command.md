# ADR 0062: Explicit Google search command

Status: Accepted

## Decision and ownership

Add `automexia google <terms>` to the existing application CLI and a session-local
`amx` wrapper in shipped shell integration. This is a core application command:
launch dispatch and OS default-handler activation already belong there. It is
not a VT command parser, a provider integration, or a second executable/service.
An extension would duplicate shell/CLI and native launch ownership without a
distinct provider capability. Persistent aliases would require avoidable profile
mutation and an installed PATH name; retain the existing session-only authority.

Use the existing `url` dependency to append a single `q` pair to the fixed HTTPS
Google search endpoint. Reject empty/control-bearing or over-limit input before
activation. Limits are 256 arguments and 4096 joined UTF-8 bytes. No API key,
network client, telemetry, startup query, config write or additional dependency
is introduced. OS argument storage remains subject to native command-line limits.

Dispatch before GUI/config/log startup. The existing Windows `ShellExecuteW`
mechanism has one shared application owner used by both screen links and this
CLI. Its UTF-16 buffers remain live through the call and reject interior NUL.
The one-shot path explicitly initializes COM before invoking shell associations;
successful and already-initialized calls receive exactly one matching cleanup.
An existing incompatible apartment retains its original ownership. A fresh-thread
native test verifies success/error cleanup and existing STA/MTA behavior without
opening a browser. This enables the existing Windows binding's COM API feature,
not a new dependency, thread, persistent service or unsafe pointer owner.
Native Unix CLI activation spawns `xdg-open` or `open` with one exact URL argument
and null streams; existing GUI daemon/context behavior remains unchanged.
No shell evaluation or implicit Enter is used. Browser lifetime is externally
owned and handoff success does not certify loading or network success.

## Interaction, lifecycle and compatibility

The shell owns editing, quoting, history, keyboard accessibility and feedback.
No new overlay, focus owner, renderer work, motion or polling is added. A missing
handler reports a static error where the native launch API can detect it;
`--print-url` offers an explicit offline alternative. Debug/errors do not echo
the query. The browser necessarily receives the search, and ordinary shell
history policy still applies.

Wrappers forward native argument arrays, preserve name collisions, and resolve
PATH only when invoked (CMD checks once before installing its DOSKEY macro).
The application supplies a current-executable path, translated one-way to WSL
with `WSLENV` path flags. No path is written to repository or user configuration.
CMD rejects expansion-bearing executable paths rather than evaluating them.
`AUTOMEXIA_AMX=0` disables installation in new sessions. Existing packaging
already includes these shell sources; no additional install or uninstall owner
is required. Removing the command and wrappers reverses this increment.

## Evidence and remaining gates

Rust tests cover exact encoding, origin, boundaries, controls, preview, redacted
errors, debug output, dispatch and WSLENV policy. The correctness-checked encoding
microbenchmark is distinct from native browser or network latency. Post-build
native checks execute the real CLI in an isolated home and source actual shell
adapters twice, covering quoting, missing executable, disable and collisions.
No test searches Google. Readiness runs these checks after building the executable;
source/mutation guards prevent dropping that dispatch or privacy/resource guards.

Native desktop browser association and interactive CMD DOSKEY activation require
their owning native runners; macOS runtime behavior is not established by a
Windows/WSL test. Keep these external rather than inferring universal support.

## Primary references

- [URL query-pair encoding](https://docs.rs/url/latest/url/struct.Url.html#method.query_pairs_mut)
- [Microsoft WSLENV translation](https://learn.microsoft.com/en-us/windows/wsl/filesystems#share-environment-variables-between-windows-and-wsl-with-wslenv)
- [Microsoft ShellExecute initialization guidance](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew)
- [COM initialization and balancing](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-coinitializeex)
- [Google search operators](https://support.google.com/websearch/answer/2466433?hl=en)

The implementation adopts existing encoding and platform handlers rather than
rebuilding a browser, authentication flow or search protocol.
