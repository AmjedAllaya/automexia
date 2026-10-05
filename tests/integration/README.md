# Integration suites

`teletypewriter/tests/pty_lifecycle.rs` runs natively on Windows, Linux, and
macOS. It covers ConPTY/Unix PTY creation, resize, sustained output, child-exit
notification, and teardown.

The isolated Windows menu handoff regression uses
`resize-stress-windows.ps1 -MenuNavigationOnly -UseCpuRenderer` with an Automexia
binary built with `--features visual-test-hooks`. It drives physical Enter,
Backspace, Alt+Left and Escape through Customizations, Theme Gallery and Profiles,
including held Enter across asynchronous page opening. It checks first-press
re-entry, restored parent queries, and unchanged terminal input and preferences.
Run it on an unlocked desktop without other keyboard or mouse use.

Release jobs validate package structure on every architecture. Native x86_64
release runners additionally perform clean install, version smoke, uninstall,
desktop/URL metadata, terminfo, and package-identity checks. Real-GPU and WSL
smoke testing is an explicit protected release-environment prerequisite; see
`RELEASING.md` for the manual evidence record.
