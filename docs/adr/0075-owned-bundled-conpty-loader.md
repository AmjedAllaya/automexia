# ADR 0075: restrict and own the optional ConPTY module

Status: accepted for current source; vendor bundle and other Windows architectures require separate native evidence.

## Owner and decision

The existing `teletypewriter::windows::conpty` adapter owns optional library
selection and every pseudoconsole call. Keep this core PTY responsibility in
that module, using the existing windows-sys dependency. No extension, process
owner, library cache, dependency, or global DLL search policy is added.

Resolve `conpty.dll` beside the current application executable. Reject relative,
NUL-containing, or excessive native paths before allocating the final UTF-16
buffer, preserving native Windows code units. The input and result each allow
at most 32766 units before the terminator. Load the absolute result with
`LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32`; filesystem
dependency searches therefore exclude ambient current-directory and PATH
entries. Windows API-set, KnownDLL, loaded-module, and redirection rules still
apply. The application installation directory remains a trust prerequisite;
this is not signature verification or a sandbox for a modified installation.

Wrap each successful loader reference in one private RAII guard. The API owner
retains it through creation, resize, and the final close call. Missing exports,
early creation failure, and completed pseudoconsole teardown release their
owned references. The function pointers stay private and cannot outlive that
owner. Each simultaneous API owner holds its own native reference; releasing
one cannot unload another's live module. Cross-thread pseudoconsole ownership
retains the same guard. Native DLL loading and teardown remain Windows-owned
operations, without a new worker or an added cancellation guarantee.

A missing or unusable optional DLL keeps the existing Windows inbox fallback.
The three compatibility export names and their Windows ABIs remain unchanged.
The optional library itself controls discovery of its OpenConsole host; this
change does not install or certify that host. Diagnostics contain no host paths.

## Alternatives and compatibility

An absolute filename without dependency-search flags would leave a separate
ambient dependency boundary. A general loading crate would still require this
application-specific path policy and would add dependency surface. A global
cache would retain references after otherwise completed sessions and create a
second lifetime owner. Reuse the existing platform adapter instead.

Optional libraries discovered only through PATH or the current directory are no
longer supported. Application-sibling deployments and the default inbox path
remain supported. No public Rust API, persisted schema, user data, or Unix
behavior changes. Rollback requires no data migration and must retain explicit
module ownership and the trusted-location boundary.

## Verification

An isolated native controller compiles inert ABI fixtures with the pinned Rust
toolchain. Separate child processes distinguish an absent DLL, CWD/PATH-only
copies, a valid sibling, partial exports, creation failure, malformed and
incompatible images, and dependent libraries in allowed or excluded locations.
Its lifetime fixture checks two simultaneous owners, resize after the first
close, cross-thread close ordering, and sixteen complete unload cycles. A fresh
completion marker rejects zero-test helper invocations.

Pure path tests cover relative/root-only input, NUL, Unicode, unpaired Windows
code units, separators, and exact/excessive UTF-16 lengths. Existing native PTY
handle-lifetime, launch-allocation, and lifecycle tests remain distinct evidence
for real pseudoconsole operation. The local Windows run passed all fourteen
loader scenarios, the owning unit and integration targets, and warning-denied
Clippy. Inert fixtures cannot certify a vendor bundle, its OpenConsole process,
Windows ARM64/x86 behavior, or release packaging. No such claim is made.

See Microsoft's [loader flags](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw),
[module reference lifetime](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary),
[search-order rules](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order),
and [ConPTY compatibility exports](https://github.com/microsoft/terminal/blob/main/src/winconpty/dll/winconpty.def).
