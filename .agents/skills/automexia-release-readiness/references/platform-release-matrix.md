# Platform release matrix

Use the current supported matrix from repository policy. This reference defines evidence categories, not a permanent list of supported versions.

## Every claimed platform

For each OS and architecture record:

- native build environment and toolchain identity;
- default and relevant optional feature sets;
- application version output and package metadata;
- install, first launch, ordinary session, multiple panes/tabs/windows, resize, clipboard, keyboard, IME where applicable, close, restart, upgrade, and uninstall;
- supported shells and process adapters;
- CPU and native GPU renderer paths where claimed;
- accessibility API projection and the applicable screen reader;
- long-session and repeated lifecycle resource evidence;
- package format, signature, provenance, checksum, SBOM, and final artifact identity.

## Windows

Verify applicable combinations of:

- supported x86_64/ARM64 targets;
- ConPTY, process tree, close/shutdown, resize, clipboard, IME, and Windows path behavior;
- installer/package formats and upgrade/uninstall cleanup;
- Start menu, icons, file/protocol associations, terminal integration, and version metadata;
- approved Windows signing backend, timestamp, certificate chain, reputation limitations, and signature verification;
- Narrator and NVDA scenarios at relevant scale and contrast settings.

A Windows cross-build on another OS is not native evidence.

## macOS

Verify applicable combinations of:

- supported Intel/Apple Silicon or universal artifact;
- native PTY, windowing, rendering, clipboard, IME, process cleanup, and shell behavior;
- app bundle structure, identifiers, icons, entitlements, hardened runtime, package/DMG, install, upgrade, and uninstall;
- Developer ID signing, nested code signatures, notarization submission/result, stapling, Gatekeeper assessment, and final artifact verification;
- VoiceOver, keyboard, scale, reduced motion, and appearance variants.

Do not use Swift/Xcode-specific assumptions when the Rust packaging owner differs; follow repository scripts and native platform tools.

## Linux and BSD claims

Verify applicable combinations of:

- supported distributions or declared compatibility baseline and architectures;
- Unix PTY, shells, environment/session integration, Wayland/X11 or supported window systems, clipboard, IME, and process trees;
- DEB/RPM/AppImage or other declared package owners, dependencies, desktop/AppStream metadata, icons, terminfo, install, upgrade, and uninstall;
- package signatures and repository metadata where the release channel provides them;
- Orca and keyboard scenarios in the claimed desktop environment.

Do not generalize one distribution, compositor, package, or libc result to all Linux/BSD environments.

## Cross-platform consistency

Compare public configuration, CLI, shortcuts, migration, persistence, error semantics, security policy, docs, and feature availability. Platform differences must be intentional, documented, and tested rather than silent fallbacks.
