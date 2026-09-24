# ADR 0071: validate Windows launch inputs before allocation

Status: accepted for current source; broader native lifecycle evidence remains external.

## Problem and placement

ConPTY inputs must preserve Windows environment-name identity and argument
quoting while rejecting invalid native dimensions and strings before resources
are created. Invalid or oversized explicit batches must not trigger large wide
copies or partial child launch.

The existing `teletypewriter::windows` adapter owns both launch constructors,
command-line quoting and native environment construction. This is core PTY
behavior. An extension, application-only validation or new shared crate would
leave independent library consumers unprotected or reverse dependencies.

## Decision and compatibility

Require rows and columns from 1 through 32767 before creation or resize. Keep the
public representation conversion for compatibility, but native calls use checked
geometry. Reject embedded NUL and launch strings exceeding 32766 UTF-16 units,
leaving space for the native terminator. The command builder counts existing
quoting expansion, separators and the terminator before allocation; its quoting
rules and both public constructor signatures remain unchanged.

Validate the entire explicit environment batch before taking an inherited
snapshot or making wide copies. Explicit names are nonempty and contain neither
NUL nor equals signs; values reject NUL and preserve empty values, Unicode and
additional equals signs. Native construction permits at most 65536 entries and
16 Mi UTF-16 units, including separators, the final terminator and superseded
entries. One full wide block is at most 32 MiB; this is not a total process-memory
or caller-input bound. Wide reservations are fallible. Combined inherited and
explicit entries are checked again before retention.

Use Windows `CompareStringOrdinal` with case ignored for environment-name order
and equivalence. An input ordinal breaks equal-name ties so the last override
wins. Preserve inherited wide data without lossy conversion. Comparison failure
rejects publication. Empty explicit environments contain two terminating NULs.
Sessions with no overrides retain the existing native inheritance path.

The existing windows-sys dependency enables only `Win32_Globalization` to expose
this native comparison API; no version, capability, persisted schema, executable
lookup policy or DLL loader policy changes in this decision. The small unsafe
call borrows initialized, owned buffers with checked lengths and retains no
pointer. Content-free errors identify invalid input or allocation failure.

## Reuse and alternatives

Reuse the existing quoting and process adapter. A second command serializer or
application environment parser would duplicate different native contracts.
Locale-sensitive lowercase or ASCII-only ordering cannot implement Windows
Unicode environment identity. Replacing ConPTY launch with a general process
builder would lose its established pseudoconsole attributes and process owner.
Native DLL location/reference ownership remains a separate boundary.

See Microsoft's [process creation limits](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw),
[environment contract](https://learn.microsoft.com/en-us/windows/win32/procthread/changing-environment-variables),
[ordinal comparison](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringordinal),
and [pseudoconsole geometry](https://learn.microsoft.com/en-us/windows/console/createpseudoconsole).

## Verification and recovery

Native regressions cover Unicode/equals/last override, ordinal name identity,
empty-block terminators, malformed batches, exact and excessive UTF-16 counts,
entry counts, quoting expansion and signed geometry. Isolated missing-executable
fixtures distinguish input rejection from child lookup or PTY allocation. Resize
validation must reject invalid geometry before reaching a successful native stub,
and an actual native failure remains retryable.

An independent public-constructor allocation observer rejects oversized quoted
commands and combined environment batches before large copies. Its allocating
canary and unwind restoration validate the observer. These checks do not claim
that every accepted huge variable launches on every Windows release or that
native inherited-snapshot allocation is controlled by the library.

No user-data migration is required. Rollback must preserve validation, native
name identity and argument compatibility. Native ConPTY, handle, process-tree,
GUI, ARM64 and full contributor gates retain their own evidence; unit success
cannot substitute for those checks.
