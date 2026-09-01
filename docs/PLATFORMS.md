# Platform support

Automexia v0.4 has release and CI ownership for Windows, Linux, and macOS.
Portable Rust tests are necessary but not sufficient: an OS-specific adapter is
accepted on the native host that owns its window server, PTY, shell, graphics,
packaging, and accessibility APIs.

## Support matrix

| Surface | Windows | Linux | macOS |
|---|---|---|---|
| Native architecture | x86_64; ARM64 compile/package checks | x86_64 and ARM64 artifacts | Universal x86_64 + ARM64 app |
| Shells | PowerShell 5/7, CMD, WSL Bash/Zsh | Bash, Zsh, other shells without enhanced integration | Zsh, Bash; other shells without enhanced integration |
| PTY | ConPTY | Unix PTY | Unix PTY |
| Display/render | WGPU/DX path plus experimental CPU fallback | X11-only, Wayland-only, and combined builds | Metal/WGPU path plus experimental CPU fallback |
| Package | Signed MSI and ZIP | DEB, RPM, tar.gz | Signed/notarized universal app in DMG |
| Native deep evidence | ConPTY/resize/clone/image/AppVerifier/WPR controlled gates | controlled X11/Wayland GPU/PTY and package containers | controlled Metal/GPU/PTY, VoiceOver, Gatekeeper/notarization |

Ordinary pull requests run the locked all-feature Rust quality gate on the
GitHub-Free Ubuntu runner. Windows and macOS native, package, graphics, shell,
and accessibility evidence is collected by controlled release and assurance
jobs; it is never inferred from the Linux result. Workflow coverage is itself
mutation-tested, so removing a required runner, read-only permission, quality
command, package, or release job fails the policy gate.

## Windows and WSL

Windows owns PowerShell formatting, CMD integration, ConPTY, custom chrome,
window ownership, WSL distribution/user/directory cloning, Authenticode, and
MSI lifecycle. WSL is a Windows-launched Linux session, not a substitute for a
native Windows build. Keep Linux Cargo work under the WSL filesystem and MSVC
work on NTFS; see [Windows and WSL development](WSL-DEVELOPMENT.md).

## Linux

The declared Linux gate uses Ubuntu runners, separate X11/Wayland feature
builds, and DEB/RPM clean-container validation. Packages declare their runtime
libraries and desktop/AppStream/URL/terminfo metadata. This proves the supported
interfaces and package families; it does not claim manual certification of
every distribution, compositor, GPU, or driver combination.

## macOS

macOS owns native window behavior, Metal/WGPU integration, universal binary
assembly, Developer ID signing, notarization, Gatekeeper, and VoiceOver checks.
Cross-compiling an Apple target from another host is not accepted as native
evidence because the SDK, window server, GPU stack, and signing policy are
host-provided.

## BSD and other Unix systems

Unix PTY, shell, and X11 code paths may compile on BSD and documentation uses
Linux/BSD where the behavior is shared. BSD is currently best-effort: it has no
required CI runner or v0.4 artifact and is not included in the machine-declared
supported platform set. A future support claim requires a native job, package
ownership, shell tests, and release policy rather than an inferred Linux pass.

## Evidence levels

- **PR:** deterministic GitHub-Free Ubuntu checks.
- **Nightly:** longer fuzz, sanitizer, benchmark, and unsigned package work.
- **Controlled:** real display/hardware, elevated instrumentation, credentials,
  or manual assistive-technology evidence.
- **External:** explicitly required evidence that the current environment did
  not observe. External never means passed.

The exact jobs and outstanding controlled evidence are maintained in
[Testing and verification](TESTING.md#native-platform-ownership) and the
[feature assurance ledger](../tests/assurance/feature-matrix.json).
