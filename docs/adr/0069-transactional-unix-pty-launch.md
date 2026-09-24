# ADR 0069: transactional Unix PTY launch and owned native records

Status: accepted for current source; native macOS, BSD and Flatpak evidence remains external.

## Ownership and decision

The existing `teletypewriter` Unix adapter retains both public launch entry
points and owns PTY descriptors, child identity, signals and waiting. The fork
entry point now selects an explicit compatibility policy on the existing
`openpty` plus `Command` launcher. No extension, dependency, capability,
configuration key or persisted format is added.

Both PTY descriptors become `OwnedFd` values immediately after successful
creation, before fallible lookup, cloning, signals or process setup. Standard
streams receive distinct owned descriptors. Nonblocking setup and signal
registration occur before child creation. Failure closes the same owners; it
cannot publish a partially launched PTY.

Account lookup uses the native status/result contract, bounded retry storage
(up to 1 MiB), and owned field copies validated against the returned buffer.
Native pointers are not dereferenced before validation. Terminal names use
caller-owned buffers: `ptsname_r` on Linux/FreeBSD and the documented XNU ioctl
on macOS. Invalid or unavailable names return errors at that boundary.

## Compatibility and child safety

Fork compatibility preserves inherited environment and working directory,
individual argument words, executable search and executable-script fallback.
It bypasses the ordinary spawn policy's macOS login and Linux Flatpak wrappers.
A bare macOS shell retains its dash-prefixed login argv[0]. Exact managed
launches retain their reviewed executable descriptor, explicit environment and
process-group ownership. Standard `Child` retention does not introduce a second
reaper; the existing PTY wait owner remains authoritative.

Arguments, environment and stream owners are prepared in the parent. The small
pre-exec callback performs native session/controlling-terminal setup, closes
non-standard descriptors, resets signal dispositions and masks, and retains the
existing exact-executable boundary. It does not allocate strings or vectors,
log, acquire a Rust lock, or return into application frames. Standard `Command`
owns the failed-exec handshake and child cleanup.

A failed fork-mode executable now returns an error to the parent before a PTY
is published. Restoring child-side allocation or application fallthrough is
not an acceptable rollback. Successful public argument and environment behavior
requires no migration. This is core process infrastructure; an optional extension
or a separate custom PATH/error-pipe implementation would duplicate authority.

Primary references: [getpwuid_r](https://man7.org/linux/man-pages/man3/getpwnam.3.html),
[ptsname and ptsname_r](https://man7.org/linux/man-pages/man3/ptsname.3.html),
[CommandExt pre_exec](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html),
and [XNU terminal ioctl definitions](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/ttycom.h).

## Evidence and limits

The Unix boundary tests exercise error/no-record/retry ceilings, malformed
account fields, concurrent terminal names, failed launch cleanup, exact-launch
early errors, argument/environment/cwd compatibility, executable-script fallback,
closed standard streams, and actual child controlling-terminal/session/foreground
group identity. Native Linux library and lifecycle tests and strict Clippy ran
against matching source hashes. Source-level macOS argv checks are not native
macOS execution. A skipped child fixture is only evidence when its parent test
explicitly launches and verifies that fixture.

Native macOS, BSD, Flatpak, exact-executable low-standard-descriptor identity,
ordinary-session descendant cleanup and controlled performance remain separate
evidence boundaries. Passing Linux tests do not certify a whole-platform release.
