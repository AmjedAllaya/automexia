# ADR 0017: Session-only shell integration

- Status: Accepted
- Date: 2026-08-16
- Supersedes: ADR 0009

## Context

Launch-time profile provisioning made a normal GUI launch persist scripts,
rewrite PowerShell profiles, enumerate/start WSL, and invoke PowerShell with an
execution-policy override. Those operations were bounded and idempotent, but
their combination resembles persistence and encoded-script behaviors used by
malware. Endpoint behavioral engines can classify the interpreter process
rather than the signed application, especially for frequently changing,
unsigned debug builds.

Prompt metadata and same-pane presentation need integration in the shell
session. They do not require a persistent profile edit. Nested shells outside
Automexia are useful, but that is a separate user decision.

## Decision

1. Normal launch performs no persistent shell operation. `cargo dev` and
   `cargo automexia` pass the checked-out resource root to the child process;
   release builds accept only a complete adjacent package resource tree.
2. The application replaces any config-supplied resource-root value after
   configuration loading. Arbitrary inherited roots are accepted only in debug
   builds.
3. PowerShell and CMD integration is injected only into a new interactive child
   session and only when validated resources are available. PowerShell hosts use
   exact executable basenames and recognized interactive host options. Explicit
   commands, scripts and encoded forms, positional scripts, noninteractive/server
   modes, unknown options and missing option values never gain a bootstrap
   command. Argument values and order remain intact; existing PowerShell banner
   suppression may prepend `-NoLogo`. Linux/macOS ordinary interactive Bash, Zsh
   and Fish launches use the session adapters below; other Unix launches retain
   their native arguments. The shell remains responsible for parsing arguments.
4. Persistent integration is exposed only as
   `automexia shell-integration install|uninstall`. The Windows command honors
   the effective execution policy and never requests `Bypass`.
5. Release packaging signs/timestamps every PowerShell script and format file
   before creating MSI/ZIP artifacts. Portable and ARM64 MSI builders include
   the complete resource tree under fixed file-count and byte ceilings.
6. WSL compatibility install/uninstall streams bounded raw UTF-8 to a fixed
   `wsl.exe ... --exec sh -s` child. Encoded commands, decoder pipelines, and
   payload-derived command lines are forbidden.
7. Static policy and hostile mutation tests pin these boundaries. Installer
   behavior remains covered in isolated homes, but it is not a launch phase.

## Linux and macOS launch delivery

The Linux default uses ordinary interactive startup, inheriting the login
environment and loading the native rc file. This replaces the generic Unix
`--login` default on Linux only: Bash ignores an rc-file bootstrap in login
mode. Explicit login-shell configuration and the macOS default are retained.

The existing application shell owner prepares ordinary Linux/macOS launches before
constructing `SessionLaunchDescriptor`. The PTY receives that descriptor's
executable, arguments, directory and environment. The legacy fork compatibility
entry remains available; its enriched adapter accepts directory/environment
requirements through the same transactional owner while preserving the selected
launch policy. On macOS the spawn default retains login(1), while fork mode
retains its historical bare-shell login argv[0] behavior.

Bash uses a package-owned rc file that first sources the user's normal `.bashrc`.
Fish uses a fixed initialization expression with a quoted resource environment
value. Zsh temporarily points `ZDOTDIR` at a package-owned `.zshenv`, restores the
original location before user startup, and loads integration through a one-shot
pre-prompt hook after normal startup. It creates no files or global environment
changes. Repeated preparation preserves the original directory.

Linux admits empty arguments and the native interactive switch. macOS also
admits native login switches and preserves implicit login intent before adding
adapter arguments. Explicit commands/scripts, custom rc files and startup
opt-outs are left alone; explicit Linux login switches remain unchanged. Missing/incomplete resources or an explicit integration opt-out preserve
the native shell. macOS keeps its native login lifecycle and startup files.
Bash login uses a one-shot `PROMPT_COMMAND` entry that sources the shipped
adapter after startup and removes itself; a user replacement of that variable
remains authoritative. Prompt status is captured before user hooks and emitted
after them, so a successful hook cannot erase a failed command result. Capturing
status returns the same exit code to the user's first hook. Zsh restores both
unset and explicitly empty `ZDOTDIR`, and shadows stale bootstrap markers.
Linux running inside WSL
keeps a native Linux launch when cloning; the distro label cannot select
Windows' `wsl.exe`.


System Bash on macOS predates PS0. Bash before 4.4 uses one bounded in-process
DEBUG hook for the same OSC command boundary; newer Bash retains PS0. The
legacy hook is installed after startup returns, with a traced installer so Bash
does not restore an earlier DEBUG trap over it. Login bootstrap finishes this
installation outside the sourced adapter before the first input prompt. PS1
arms the command latch after user prompt hooks; blank Enter never starts a
command. Existing DEBUG actions stay in their native trap context and preserve
exit status, tracing options, pipelines/functions and repeated sourcing.

The installer decodes only the shell-quoted declaration returned by builtin
trap -p DEBUG, once per installation. A dedicated capture descriptor excludes
user DEBUG diagnostics; a canonical single-quote round trip rejects malformed
declarations without shell evaluation. No command history, BASH_COMMAND content,
terminal metadata or external file is evaluated. There are no external
processes per command and no Readline bindings or history settings are changed.
Native PTY tests cover ordinary and login startup, user DEBUG hooks, functrace,
extdebug, blank input and repeated sourcing on both system and current Bash.

## Consequences

- Ordinary launch cannot create persistence or trigger WSL provisioning.
- Missing integration resources produce a plain shell rather than a hidden
  repair operation.
- Users who want integration in shells opened outside Automexia must opt in
  once and can inspect/remove the exact managed state.
- Signed release resources work with signed-script policies when the publisher
  is trusted; local debug resources remain intentionally unsigned.
- No architecture can guarantee acceptance by every antivirus. The smaller,
  explicit behavior surface improves explainability and makes vendor review
  evidence correspond to the signed package.
