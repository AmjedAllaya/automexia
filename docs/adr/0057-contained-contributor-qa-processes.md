# ADR 0057: Contained contributor QA processes

Status: Accepted

## Context and placement

A QA command can exit while a descendant retains its output pipe. Waiting only
for the parent, then closing a buffered stream held by its reader, does not bound
cleanup. PID-based tree discovery after parent exit also loses ownership.

`tools/ci/qa_process.py` owns this contributor-only mechanism. Its consumers
are QA step execution, bounded tool-version capture, raw source-status capture
and the compiler-identity probe in `rust_toolchain.py`.
Formatting, redaction, output budgets and pass/fail policy remain in `qa.py`.
Neither terminal core nor an extension is appropriate: this code is not packaged
with the application and grants no user-facing process capability. Existing Rust
PTY and readiness adapters retain their distinct interactive/native contracts.
There was no existing Python job-object owner to reuse.

## Decision

Use standard-library exact-argument process launch and a small gated Python
supervisor. On Windows, assign its original owned process handle to an unnamed
kill-on-close Job Object before admitting the command. Descendants cannot request
job breakaway. On Unix, retain the unreaped supervisor in a new process group
until group termination; never rediscover it by a recycled PID. The supervisor
waits for the actual command and reports its exit through a separate bounded
control pipe. Command stderr is data, not control. Commands are noninteractive
and receive closed standard input.

On completion, deadline, output-consumer failure or interruption, terminate the
owned native group, reap the supervisor and join both readers before releasing
streams or the serial admission slot. Windows additionally waits for zero active
job processes. The command exit code and final buffered output survive normal
completion even if descendants retain data handles. No unbounded buffered
`close()` or PID-based post-exit `taskkill` is used.

Enforce one owner per interpreter, 512 arguments, 64 KiB encoded arguments,
8 KiB data chunks, a 32-byte control receipt, finite positive command deadlines
and a separate five-second cleanup ceiling. Consumers retain their existing byte
budgets. Incomplete cleanup retains its owner and blocks subsequent launches; it
cannot become a successful test. Native process creation and filesystem I/O can
be uninterruptible, so these are not hard real-time guarantees. Unix tools that
deliberately escape the process group are outside this trusted-tool lifecycle
boundary; it is not a hostile-code sandbox. No service, persistence, dependency,
environment dump, network access or terminal hot-path work is introduced.

## Evidence and consequences

Real process tests cover parent-exit/retained-pipe races, final tails, exact argv,
stderr separation, nonzero exit, timeout, consumer overflow, interruption,
admission failure, concurrent launch rejection and repeated reader retirement.
Windows handles and Linux pidfds independently pin a live descendant before its
parent may exit and prove that exact process terminates. Missing native oracles
are explicit skips, not evidence for another platform. QA consumer tests retain
raw source bytes, privacy, source identity and log limits. Checker mutations
protect admission/cleanup ordering; source checks supplement runtime tests.

The extra supervisor has measurable per-command startup overhead. Benchmark
short commands separately from full QA duration and repeat cleanup checks; do not
infer application performance from these contributor-only measurements. Rollback
is a normal reviewed revert of the helper and its callers together, never
a gate bypass. Native macOS execution remains an external validation requirement.

## Primary references

- [Python subprocess lifecycle and timeout behavior](https://docs.python.org/3/library/subprocess.html)
- [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [Job assignment and nested jobs](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject)
- [Extended job limits](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information)
