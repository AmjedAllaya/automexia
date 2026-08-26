
# Editor Suggestion Bridge Research

**Real owner:** accepted project ADR 0025; disabled source implementation is
authorized, while preview and stable activation remain gated.
This document is research input only.

Keep:
- same-user threat nuance;
- exact session/pane/shell/generation binding;
- short-lived capabilities;
- authenticated handshake/replay protection;
- bounded framing;
- no implicit Enter/execute;
- sensitive-buffer non-persistence;
- zero-network/provider/credential/persistent-write keystroke path;
- kill switch;
- native shell matrix;
- privacy/fuzz/race tests.

Do not create a competing bridge ADR or new crate topology unless the real ADR/implementation needs it.

Resolve shell-adapter ownership within the actual repository before activating later CP5 phases.
