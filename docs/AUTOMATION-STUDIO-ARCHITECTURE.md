# Automation Studio architecture

Status: planned after the first stable Automexia release and before the first
video-editing extension.

## Product value

Automation Studio will make it easier to create, review, run, and maintain
scripts without leaving the terminal workflow. Its first audience is DevOps and
SRE teams working with shell scripts, infrastructure configuration, deployment
files, and repeatable operational tasks.

The editor should open inside an Automexia pane, similar to a terminal-native
editor, rather than forcing every edit into a separate application window. A
separate window may remain an optional platform action, not the primary design.

## Placement

The editing workspace belongs in a separate Automation Studio extension. It is
not part of the terminal core and it is not embedded inside the DevOps/SRE
extension. Domain extensions may contribute language support, templates,
validation, and safe run profiles through stable capability contracts.

This keeps editor dependencies, language servers, file access, subprocesses,
and future collaboration features out of terminal startup and rendering paths.
Users who only need a terminal do not pay their resource or authority cost.

## Initial experience

- Open or create a script from a command, palette action, or reviewed file.
- Edit with familiar keyboard navigation, search, syntax highlighting, and
  accessible diagnostics.
- See validation and a clear diff before saving generated or transformed text.
- Run through typed profiles that show the interpreter, arguments, working
  directory, environment changes, and target context.
- Keep editing, execution, output, and terminal sessions in separate panes with
  obvious ownership.
- Recover unsaved work after a crash without silently overwriting the source.

## Architectural principles

- Reuse a maintained editing component after license, security, accessibility,
  platform, binary-size, and lifecycle review.
- Use a thin Automexia host around an editor model; do not build a full editor
  engine from scratch.
- Keep file, process, network, and language-service access capability-gated.
- Launch interpreters and tools with typed executable and argument arrays.
- Bound files, decoded size, diagnostics, language servers, queues, caches,
  history, temporary data, and background work.
- Make cancellation, crash recovery, disable, uninstall, and fallback to an
  external editor explicit.
- Preserve native terminal responsiveness when the extension is disabled,
  loading, busy, or failed.

The specific editor component, internal data model, language-server topology,
resource ceilings, and implementation phases remain unpublished until the
dependency and implementation decisions are ready for public review.

See [Automation Studio testing](AUTOMATION-STUDIO-TESTING.md),
[Extensions](EXTENSIONS.md), and the [Roadmap](ROADMAP.md).
