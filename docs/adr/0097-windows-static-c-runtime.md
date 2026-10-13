# ADR 0097: include the Windows C/C++ runtime in application binaries

Status: accepted for implementation; native build and clean-OS package evidence pending.

## Failure

The published `0.0.2-alpha` x64 application imports `MSVCP140.dll`,
`VCRUNTIME140.dll`, and `VCRUNTIME140_1.dll`. Its three helpers import
`VCRUNTIME140.dll`. The ARM64 binaries have the same external runtime dependency.
Neither MSI nor ZIP deploys it. A fresh Windows VM therefore cannot launch the
application, although development machines with the redistributable can.

## Decision and ownership

Enable Rust's `-Ctarget-feature=+crt-static` for Windows MSVC targets in Cargo
configuration and in `reproducible_build.py`, which replaces Cargo's flags.
Keep GNU Windows and other operating systems unchanged. The pinned Microsoft
ConPTY binaries remain unchanged; their current imports need no VC runtime.

The existing `xtask package` owner inspects all seven PE files after vendor
staging and before either installer or archive generation, including skipped
builds. `windows_runtime_dependencies.py` wraps Microsoft's DUMPBIN using the
existing bounded `qa_process` owner. It rejects VC runtime imports, missing
inputs/tools, incomplete output, excess output, process failure, and timeout.
It does not execute the binaries or implement a PE parser.

An MSI prerequisite installer would add installation state and leave portable
users needing a separate step. App-local runtime DLLs would introduce another
versioned, licensed deployment inventory. Static linkage fits both existing
formats without a new installer action or global system change.

## Verification and servicing

Native Windows regression fixtures compile one dynamic-CRT and one static-CRT
executable; the import gate must reject the former and accept the latter.
The full release gate also checks actual application and vendor imports.
Mocked policy tests alone cannot establish that the application links correctly.

Before publication, build x64 and ARM64, inspect the final package payloads,
and test installation, application/helper startup, terminal behavior, upgrade,
and uninstall on declared clean customer baselines without the redistributable.
Hosted development images cannot prove that last condition.

Static runtime updates require rebuilding and republishing Automexia. Package
size may increase; no size or performance improvement is claimed. No config or
data migration is needed. Existing immutable releases retain their workaround.
Rolling back this choice requires an explicit runtime deployment decision;
removing the import gate is not a valid rollback by itself.

References: [Rust linkage](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes),
[Cargo flag precedence](https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags),
[Microsoft runtime deployment](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files),
and [the ConPTY dependency owner](0082-pinned-windows-console-runtime.md).
