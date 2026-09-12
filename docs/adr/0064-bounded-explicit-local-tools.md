# ADR 0064: Bounded explicit local tools

Status: accepted for the implemented `amx find` and `amx explain` commands.

## Ownership and alternatives

The application CLI owns explicit local-tool activation. Terminal parsing,
rendering, input, startup and extension loading do not launch these tools.
The existing Quick Action insert-only boundary remains unchanged. No new
extension, service, executable package or background index is introduced.

Reuse installed ripgrep for file/content search and the tealdeer `tldr` client
for offline examples. Implementing another ignore engine or example database
would duplicate maintained tools. Missing clients produce an explanation;
Automexia never installs them or requests a cache download automatically.

Reuse the already locked process-wrap 10.0.0 adapters (MIT/Apache-2.0, MSRV
1.87) with only std, process-group, job-object and creation-flags features.
Reuse signal-hook 0.4.4 on Unix without default features. No async runtime or
reader thread is added. The CI runner's capture policy is intentionally not
shared: product diagnostics and untrusted output require different treatment.

## Process and resource contract

The app resolves installed executables from absolute PATH entries, supplies
literal argument arrays and never evaluates shell text. Capture has separate
stdout/stderr ceilings of 4 MiB/64 KiB, a 15-second command deadline, bounded
nonblocking pipe service and scoped cancellation. Native cleanup has a separate
two-second deadline. Failed or unconfirmed cleanup is an error, not success.

Unix process groups and Windows jobs retain one child owner. Exit is observed
without reaping the leader before its group is retired, avoiding a later signal
to a reused PID. Windows flags go through process-wrap's CreationFlags wrapper:
JobObject replaces raw Command creation flags. Each tool gets its own console
group. Ctrl+C/Break handling sets a flag; cleanup runs outside the signal handler.
These lifecycle controls are not a sandbox for hostile installed executables.
Forced OS termination and uninterruptible kernel work have no universal cleanup
guarantee.

Windows completion additionally uses a private accounting adapter within that
same child owner. The process-wrap post-spawn hook assigns an unnamed outer job
while the leader is suspended; process-wrap then assigns its termination job
and resumes the leader. It remains the termination authority. Its private job
handle and leader-only `try_wait` do not expose whole-tree completion; vendoring
the dependency or replacing process creation would broaden the unsafe boundary.
The observer adds one native handle and shared handle allocation per capture,
without a thread, service, new dependency version or PTY-owner coupling.

Before termination, at most 256 live job-member handles are pinned and checked
for job membership. Over-limit lists, inaccessible identities and live membership
mismatches fail closed after requesting termination. Completion requires both a
zero active-job count and signals from retained member handles within the existing
two-second cleanup budget. Neither leader exit, pipe EOF nor accounting alone is
native handle completion. This is not a guarantee against hostile processes that
delegate work outside the job or every kernel teardown race; no PID-name kill or
system-wide process scan is used. Lease EOF still precedes forced WSL cleanup.

Windows-backed WSL wrappers pass transient distro, directory, PATH and HOME
hints only to Automexia's fallback, never to an existing `amx` executable.
An embedded, fixed Python-stdlib supervisor launches exact guest argv and owns
the Linux process group. Python runs with `-I`: current-project imports and
PYTHONPATH cannot replace its standard library. A stdin lease, SIGTERM handler
and 12-second guest deadline retire guest children; a Windows job alone is not
evidence of Linux cleanup. Native Linux/macOS do not use this bridge. The hints
are bounded, redacted from Debug/errors and never saved. Missing Python is an
explicit WSL prerequisite, not an installation request.

## Search and example policy

Search is explicit within the current-directory subtree. Ripgrep keeps its
ignore behavior; additional negative globs exclude common credential files and
private directories. File matching filters the default NUL-separated file list
literally; positive globs could override ignores. Text matching uses fixed
strings and JSON, no preprocessing or symlink following. Results are bounded,
relative, and escaped against terminal control/bidi injection. Invalid,
truncated, byte-oriented or excessive results fail with an actionable message.
This is a privacy filter, not a guarantee that every secret filename is known.

Examples require a tealdeer version probe and `--no-auto-update --raw --color
never`. The user owns and explicitly prepares its offline cache. Automexia
does not execute examples. Client-owned cache maintenance is not represented
as a read-only filesystem or network sandbox.

## Evidence and rollback

Unit tests cover literal arguments, bounds, malformed results, redaction and
control injection. Real child tests cover cancellation, deadlines, stream
saturation, descendant pipes, lease EOF and native console signaling. Native
Windows descendant tests pin acknowledged live handles before releasing the
leader; four workers run 25 inherited/closed-pipe cycles each. An isolated native
fixture checks exact handle recovery after each of 20 captures and reports
correctness-checked capture timing, not interactive terminal latency. These
tests reproduced delayed process signaling after both leader exit and a zero
active-job count; a later diagnostic wait remains a failure, never a retry pass.
Native WSL tests use real ripgrep, exact process handles and a project-module canary;
tealdeer argv is tested with an independent client fixture. A real installed
tealdeer cache, native Unix Rust process execution and native desktop behavior
remain separate validation requirements when unavailable.

The parser benchmark verifies result count, ordering and final contents before
reporting timing; it does not measure filesystem, native process or compositor
latency. Removing these CLI routes and optional helper code reverts the feature
without a persistence migration. `AUTOMEXIA_AMX=0` disables the shell helper;
existing aliases/functions/executables retain ownership.

## Primary references

- [ripgrep guide](https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md)
- [tealdeer source and CLI](https://github.com/tealdeer-rs/tealdeer)
- [process-wrap source](https://github.com/watchexec/process-wrap)
- [signal-hook source](https://github.com/vorner/signal-hook)
- [Python isolated mode](https://docs.python.org/3/using/cmdline.html#cmdoption-I)
- [Windows console handlers](https://learn.microsoft.com/en-us/windows/console/setconsolectrlhandler)
- [Windows nested-job accounting](https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs)
- [QueryInformationJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-queryinformationjobobject)
- [Linux waitid and WNOWAIT](https://man7.org/linux/man-pages/man2/waitpid.2.html)
