# ADR 0082: pin the Windows console runtime

Status: accepted; native x64 evidence is separate from ARM64 and release certification.

## Failure and decision

Fixed, fictional long table records lose soft-wrap provenance with the Windows
inbox ConPTY when they overflow a short viewport. Three of ten native fixtures
failed with the inbox transport; the identical test executable passed all ten
with Microsoft's newer runtime. The fixtures check exact source and table
geometry during output, without resizing, including direct and nested WSL.
This is a transport limitation, not evidence that arbitrary hard lines should
be joined by the table parser.

Adopt `Microsoft.Windows.Console.ConPTY` version `1.24.260710001` from the
[official NuGet package](https://www.nuget.org/packages/Microsoft.Windows.Console.ConPTY/1.24.260710001).
Keep the existing platform adapter and owned sibling loader from
[ADR 0075](0075-owned-bundled-conpty-loader.md). No parser, extension, worker,
global library cache, or second PTY owner is introduced.

The root DLL matches the application architecture. Both `x64/OpenConsole.exe`
and `arm64/OpenConsole.exe` are present so the vendor loader can select the
kernel architecture, including an x64 application on ARM64. A root
`OpenConsole.exe` is rejected because it overrides that selection. Windows x86
is not a new supported application target.

## Acquisition and trust

`packaging/windows/conpty-runtime.json` is the one pinned recipe. It records the
official package URL, SHA-256, member hashes, sizes and PE architectures. The
package hash was independently compared with NuGet's catalog hash. The MIT
license is included in third-party notices; the package declares no NuGet
dependencies. These are native vendor binaries, with their own unsafe-code and
Windows process capabilities, not new Rust dependencies or feature flags.

The maintained upstream is [Microsoft Terminal](https://github.com/microsoft/terminal).
Its [public advisory page](https://github.com/microsoft/terminal/security/advisories)
listed no published advisories when checked on 2026-10-01. This is not a claim
that no vulnerabilities exist; version updates require renewed provenance,
advisory and native regression review.

Preparation runs only in build/package tooling, using the existing development
cache lease. Download, member count, expanded bytes, PE offsets and paths are
bounded. The existing QA process owner enforces the 120-second download deadline
and separate five-second cleanup ceiling, including stalled headers and reads;
native process creation retains that owner's documented platform limits. If
cleanup cannot be confirmed, one staging directory and its cache lease remain
quarantined and further preparation fails. Offline mode uses a verified cached
archive or fails. Existing
conflicting files are preserved and rejected. Hosts are staged before the DLL;
identical installed bytes are left untouched. Terminal startup, input, output,
resize and rendering never download or scan for this package.

Release checks preserve Microsoft signatures separately from Automexia
signatures, verify exact hashes and architectures in archives and installers,
and bind the NuGet identity to both SBOM formats. See
[release trust](../RELEASE-TRUST.md). Nothing in this decision authorizes signing
or publishing a release.

## Cost, compatibility and rollback

Each installation adds about 2.3 MB of native files. The bounded archive and
four extracted members share the existing development cache. Preparation adds
one cold download and hash verification to Windows builds; it adds no Rust
compilation. Existing ConPTY process/lifetime ownership remains unchanged.
No startup or input-latency improvement is claimed without measurement.

`cargo automexia` and package preparation include the runtime. A bare Cargo
build does not provision native companions; run the preparation helper first
when testing that executable directly. Missing/unusable companions retain the
existing inbox fallback, including its known wrap-provenance limitations.
No user settings or stored terminal data migrate. Rollback replaces the matched
vendor set through the package owner; removing the bundle restores inbox
behavior. Do not mix DLL and host versions.

The standard library cannot repair information already lost by the inbox
serializer. Parser guesses, output polling and command-specific workarounds
were rejected. The existing loader can use the vendor implementation without
changing its restricted search path, export contract or module lifetime.

## Evidence limits

The native table fixtures cover Windows x64, PowerShell and direct/nested WSL
with repository shell integration and isolated settings. Source/model tests
also cover chunking, tall panes, bounded history, styles and structural table
formats. The existing PTY lifecycle, handle-lifetime and launch-allocation
fixtures also passed eleven tests beside the verified runtime, covering
sustained output, resize, process exit, shutdown and repeated handle cleanup.
These checks do not establish native compositor appearance, all shell
versions, ARM64 execution, Linux/macOS desktop behavior, signed installer
operation or release readiness. Those remain separate platform gates.
