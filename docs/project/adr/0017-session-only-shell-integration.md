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
   session and only when validated resources are available. Explicit
   `-Command`, `-File`, `cmd /c`, and other noninteractive calls remain
   unchanged.
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
