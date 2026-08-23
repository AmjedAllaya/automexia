
# CP5 Research Deltas for Real Project ADR 0025

> **Reference copy:** The maintained integration note is
> [`repository_integration/CP5_ADR_0025_RESEARCH_DELTAS.md`](../../repository_integration/CP5_ADR_0025_RESEARCH_DELTAS.md).
> Do not treat this retained RFD copy as a second authority. Project ADR 0025
> remains proposed, and the same-user limitation is now recorded there.

**Owner:** proposed project ADR 0025; no CP5 runtime is authorized.
This file is not an ADR and must not compete with it.

Useful research safeguards to consider merging into ADR 0025 where not already covered:

## Same-user threat

UID/SID peer identity alone does not establish the exact Automexia shell/pane/session.

Bind the bridge to:

```text
AutomexiaInstanceId
SessionId
PaneId
ShellInstanceId
generation
short-lived capability
authenticated handshake
nonce/replay protection
```

Clarify threat boundary: this protects against wrong/stale/cross-user/accidental same-user clients, not a fully compromised same-user OS account with arbitrary process-memory inspection.

## Transport

Platform-native local IPC with:

```text
Windows restricted named pipe
Linux private pathname AF_UNIX + SO_PEERCRED
macOS private socket + getpeereid
```

Use strict endpoint ownership/permissions.

## Data boundary

Editor buffer is sensitive ephemeral state.

Do not automatically write it to:

```text
logs
telemetry
SQLite
history
recordings
crash reports
extension storage
network services
```

## Keystroke-path invariant

Normal local suggestion typing:

```text
network calls             0
provider launches         0
credential materialize    0
persistent writes         0
```

## Authority staging

Early transport phases do not automatically authorize editor-buffer access, shell hooks, ranking, UI, extensions, or other later capabilities.

## UX

Suggestion acceptance inserts/edits text but does not implicitly press Enter/execute.

## Testing

Integrate through existing test owners/xtask:

```text
wrong peer
same-user attacker
replay
frame/state fuzz
slowloris
generation races
privacy canaries
real shell matrix
accessibility
idle/performance
```
