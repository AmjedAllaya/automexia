# Session-helper record protocol

This protocol exists only between a native editor adapter and the one
session-resident signed helper. It is not the authenticated application
endpoint; the helper translates it to ADR 0025's length-prefixed schema-1 JSON.

## Request records

Each binary request starts with ASCII `AXSH`, little-endian `u16` version `1`,
one `u8` kind, and a little-endian `u32` payload size. Payload integers are
little-endian. Strings are a `u32` byte length followed by strict UTF-8.

Kind `1 request` contains, in order:

1. buffer string;
2. cursor byte (`u32`);
3. nonzero adapter generation (`u64`);
4. exact replacement start/end bytes (two `u32` values);
5. selection-present byte, followed by start/end when present;
6. quote-context byte;
7. candidate count (`u32`) followed by zero to 512 candidate strings.

PowerShell supplies native candidates from `TabExpansion2`; Fish streams native
`complete -C` lines through a 4-KiB per-line read limit and stops after 512
accepted candidates. Bash and Zsh currently send zero. The
helper's inherited bootstrap supplies shell/editor and authenticated route
identity; the shell record never contains an endpoint or capability.

Unknown kinds, invalid UTF-8/grapheme boundaries, trailing bytes, zero/replayed
generations, invalid spans, controls/bidi in candidates, more than 512
candidates, candidates over 1 KiB, or payloads above 512 KiB fail closed.

## Response records

The persistent helper emits a NUL-free ASCII line because POSIX shell variables
cannot preserve a binary header containing NUL. A replacement is:

```text
AXSR1<TAB>R<TAB>generation<TAB>request-id<TAB>start<TAB>end<TAB>UPPERCASE-HEX-UTF8<LF>
```

A redacted status is `AXSR1<TAB>S<TAB>code<LF>`, where `code` is exactly 1
through 6. The encoded line, including LF, is at most 2,176 bytes; adapters bound
the pre-LF content to 2,175 bytes before field parsing. Lowercase/odd/non-hex
insertion data, invalid strict UTF-8, controls, bidi, CR, extra/unknown fields,
unknown status codes, zero identities, or an unterminated/oversized line fail
closed. The adapter compares generation and exact byte span with the request,
re-reads the unchanged native buffer/cursor, then replaces once using native
character indices. It never accepts an execute bit and never sends Enter.

Internal binary kinds `2 accept`, `3 dismiss`, `4 replace`, and `5 status` remain
available to the signed helper/application state machine; shell-facing response
serialization is deliberately the restricted ASCII envelope above.

Request and response streams are inherited handles on Windows and inherited
file descriptors on Unix. Fish uses launcher-owned fd 3 for requests and fd 4
for responses; `/dev/fd/4` makes the builtin response read non-interactive on
Fish 3.7 while preserving the same inherited pipe, and a function-local
`fish_read_limit` caps the read at 2,176 bytes. Bash and Zsh validate RFC 3629
byte structure directly; Fish validates the same byte-state model; PowerShell
uses strict exception-fallback UTF-8 decoding. All four reject C0/C1 controls
and the protocol's bidi controls before calling a native editor API. These descriptors are not
secrets and carry no route capability. They close on route teardown. The
adapter never opens TCP/UDP, an endpoint by name, a profile, a history file, or
a second helper.

Between helper and application, schema-1 JSON replies are either a replacement
or an authenticated status. Both bind request, route, capability, prompt/buffer
generation, and cancellation. The application reconstructs replacements from a
published request-local candidate; the helper revalidates the reply before
translating it to this restricted ASCII envelope. Source ranking is bounded to
250 ms and an unanswered UI publication to 30 seconds; kill/route closure wakes
the wait early.
