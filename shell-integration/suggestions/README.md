# Optional native-editor suggestion adapters

These files are the session-only CP5.5 adapters governed by ADR 0025. They are
packaged as inert resources: normal shell integration does not source them and
the preview remains disabled. A reviewed preview launcher may source exactly
one matching adapter only after it has created the private route endpoint and
one session-resident, package-signed helper.

The helper owns schema-1 JSON framing and the route capability. Shell code
never receives, prints, logs, persists, or places that capability in argv or an
environment variable. Adapters receive only inherited request/response handles
or file descriptors and exchange the bounded records described in
`helper-protocol.md`.

Every adapter:

- checks its native editor version before opening both inherited channels;
- starts unbound and changes no profile, native completer, prediction, history,
  or existing key;
- binds only an explicitly supplied collision-free chord;
- captures bounded buffer/cursor/span state and bounded native candidates where
  a reviewed mutation-free editor API is available;
- waits only on the persistent helper, re-reads the current editor state, rejects
  stale generation/span/content, and performs one native replacement;
- never emits Enter, executes the candidate, or sends suggestion text to the
  PTY;
- removes only its own chord, functions, and streams during disable;
- fails back to the shell-native CP1 experience when unsupported, disconnected,
  killed, stale, or malformed.

PowerShell uses `TabExpansion2` plus PSReadLine `GetBufferState`, `Replace`, and
`SetCursorPosition`. Fish uses `commandline` token/cursor state, streams
`complete -C` through a 512-item/4-KiB-line bound, applies UTF-8 byte accounting
through URL escaping, and uses fixed launcher-owned request fd 3 plus response
fd 4 opened through `/dev/fd/4` with a 2,176-byte `fish_read_limit`; Unicode is
not silently disabled. Bash and Zsh use their native line-editor
buffers and conservative shell-token spans; they currently send no native
candidate batch, so CP1 remains their complete native completion fallback.

The source bridge is complete enough for inert verification: the packaged
`automexia-suggestion-helper` binary target translates bounded shell records to
an authenticated application submission; the application owns one bounded
read/publication/reply exchange; and every adapter has a bounded response reader,
current-buffer revalidation, and one-shot native replacement. The source still
is not an activated product bridge. The reviewed launcher, restricted inherited-
handle process composition, signed/attested package artifact, WSL host relay,
live screen composition, public controls, and stable release evidence remain
disabled.

Native local evidence now covers PowerShell preview/anonymous-handle/status/
cleanup behavior on Windows and real Bash, Zsh, and Fish enable, Unicode byte-
span, replacement, stale, status, invalid UTF-8/C0/C1/bidi rejection, oversized
input, and callback-return paths in WSL. Hosted Linux/macOS/Windows jobs own the
same OS-appropriate harnesses, but
a workflow definition is not a passing remote run. Interactive PowerShell
`PSConsoleReadLine::Replace`, WSL host relay, native endpoint churn, profile
preservation, signing, accessibility, resource, rollback, and soak gates remain
external CP5.6 evidence.

Windows PowerShell 5.1 and CMD intentionally have no adapter. WSL activation is
also absent until the signed host-relay native gate is completed. The scripts
expose truthful health rather than silently falling back to another transport.
