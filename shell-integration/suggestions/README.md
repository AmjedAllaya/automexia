# Optional native-editor suggestion adapters

These files are the session-only CP5.5 adapters governed by ADR 0025. They are
packaged as inert resources: normal shell integration does not source them and
the preview remains disabled. A reviewed preview launcher may source exactly
one matching adapter only after it has created the private route endpoint and
one session-resident, package-signed helper.

The helper owns the schema-1 JSON framing and the route capability. Shell code
never receives, prints, logs, persists, or places that capability in argv or an
environment variable. Adapters receive only inherited helper handles/file
descriptors and exchange the bounded `automexia-suggestion-helper-v1` records
described in `helper-protocol.md`.

Every adapter:

- checks its native editor version before opening inherited handles;
- starts unbound and changes no profile, native completer, prediction, history,
  or existing key;
- binds only an explicitly supplied collision-free chord;
- captures bounded buffer/cursor state and, where the native API supports a
  mutation-free query, bounded native candidates through the shell editor API;
- reserves native-editor replacement for the unshipped acceptance adapter and
  never emits Enter or PTY text;
- removes only its own chord, functions, and streams during disable;
- fails back to the shell-native CP1 experience when unsupported, disconnected,
  killed, or stale.

These resources are activation scaffolds, not a complete shell bridge: the
signed helper, response reader, current-buffer revalidation, native replacement,
and application launcher are intentionally absent until their native gates are
implemented and reviewed. PowerShell is the only current request adapter that
queries native candidates; Bash, Zsh, and Fish send zero candidates and preserve
their native UI. Fish also fails closed to its native UI for non-ASCII buffers.

Windows PowerShell 5.1 and CMD intentionally have no adapter. WSL activation is
also absent until the signed host-relay native gate is completed. The source
scripts expose truthful health rather than silently falling back to another
transport.
