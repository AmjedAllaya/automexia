# Integration suites

`teletypewriter/tests/pty_lifecycle.rs` runs natively on Windows, Linux, and
macOS. It covers ConPTY/Unix PTY creation, resize, sustained output, child-exit
notification, and teardown.

Release jobs validate package structure on every architecture. Native x86_64
release runners additionally perform clean install, version smoke, uninstall,
desktop/URL metadata, terminfo, and package-identity checks. Real-GPU and WSL
smoke testing is an explicit protected release-environment prerequisite; see
`RELEASING.md` for the manual evidence record.
