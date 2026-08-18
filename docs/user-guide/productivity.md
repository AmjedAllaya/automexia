# Command productivity

Automexia's command-productivity model is designed to improve command discovery and repetition **without replacing the shell or turning terminal text into implicit execution**.

> **Status:** session-only shell integration is **Available now**. Native completion adapters, typed Quick Actions, persistent aliases, built-in DevOps packs, and trusted workspace-task bridges are **implemented locally / release-gated**. Do not assume every stable package exposes them until release evidence is complete.

## Pick the least complicated approach that solves the problem

| Need | Recommended approach |
|---|---|
| One command, used once | Type it normally in the shell |
| Shell already provides good completion | Use native shell `Tab` completion |
| Command is repeatable but should be reviewed each time | Quick Action → Insert/Copy |
| Stable reviewed action used many times per day | Optional generated shell alias |
| Command belongs to one repository | Trusted workspace task bridge rather than a global alias |
| You forgot an Automexia UI operation | Command palette, not a shell alias |
| You need a background provider/cloud lookup on every keystroke | Do **not** do this; Automexia keeps provider work off the typing path |

## Native completion: let the shell own editing

The completion design keeps PowerShell/PSReadLine, Bash/Readline, Zsh/ZLE, Fish, and CMD semantics authoritative. Automexia can provide bounded generated adapters, but it does not reconstruct the command line from rendered terminal cells.

For a user, the normal rule remains:

1. Type in the shell.
2. Press the shell's normal completion key (typically `Tab`).
3. Let the shell own quoting, cursor placement, menu behavior, and accepted text.

If managed completion state is missing or invalid, the native shell fallback remains usable.

### Source-build completion administration

In repository builds that expose the local CP1 implementation, the management surface is under `cargo xtask completion`:

```text
cargo xtask completion doctor
```

Read-only health check.

```text
cargo xtask completion refresh --provider ID --shell SHELL
```

Explicitly generates one reviewed provider artifact within bounded process/output limits.

```text
cargo xtask completion enable
cargo xtask completion disable
```

Toggles managed artifacts while leaving native shell behavior available.

```text
cargo xtask completion remove --provider ID --shell SHELL
```

Removes the fixed managed artifact for that provider/shell.

These are source/repository administration commands, not a reason to replace your shell's ordinary completion setup.

## Quick Actions: named commands that remain reviewable

A Quick Action stores a structured command template rather than an opaque shell snippet. The intended interaction is review-first:

1. Search the action catalog from the appropriate palette/management surface.
2. Read the title/description and exact command tokens.
3. Inspect required placeholders and context.
4. Check risk/collision/tool-health information.
5. Fill placeholders.
6. Choose **Insert** or **Copy**.
7. Review/edit in the shell, then run it yourself.

This is the right approach for commands that are complicated enough to forget but important enough that auto-execution would be risky.

### Inspect actions from the CLI

```text
automexia actions list
```

Lists action metadata and the current revision without exposing every command template.

```text
automexia actions show ACTION_ID
```

Explicitly reveals one reviewed template.

```text
automexia actions doctor
```

Reports redacted store health, revision, and action count.

### Add/update an action safely

The write flow is intentionally two-stage.

First validate and preview exactly one action file:

```text
automexia actions put ONE_ACTION.toml
```

Then apply only after review, using the expected revision returned by the dry run:

```text
automexia actions put ONE_ACTION.toml --apply --expected-revision N
```

Use `--replace` only when the reviewed operation truly intends to replace an existing action.

### Transfer actions

Preview an import first:

```text
automexia actions import EXPORT.toml
```

Then apply with the reviewed revision and any separately required conflict/path consent:

```text
automexia actions import EXPORT.toml --apply --expected-revision N
```

Export to an explicit destination:

```text
automexia actions export DESTINATION
```

Machine-specific paths are excluded by default; including them requires explicit consent. Treat exported action collections as configuration that should still be reviewed on another machine.

### Remove or recover

Preview removal:

```text
automexia actions remove ACTION_ID
```

Apply only with the expected revision:

```text
automexia actions remove ACTION_ID --apply --expected-revision N
```

Recovery is also preview-first:

```text
automexia actions recover PREVIOUS_REVISION
```

and explicit when applied:

```text
automexia actions recover PREVIOUS_REVISION --apply
```

## Persistent aliases: promote only mature actions

Persistent aliases are compiled projections of canonical actions for PowerShell, Bash, Zsh, Fish, and CMD. They are useful when typing a long reviewed action repeatedly has become unnecessary friction.

### Inspect before changing anything

```text
automexia aliases list
```

```text
automexia aliases preview --shell powershell
```

Use `--show-source` only when you intentionally want to disclose the generated shell source.

```text
automexia aliases test --shell powershell
```

`test` verifies compiler invariants and the installed native parser in isolation; it does not run the action.

```text
automexia aliases doctor
```

Read-only health/integrity/transaction information.

### Enable an alias using preview → apply

A preview looks like:

```text
automexia aliases enable ACTION_ID --name NAME --shell powershell
```

It reports the source revision, generation digest, collisions, tool/completion health, and policy decisions. Nothing is published yet.

After reviewing those values, use the exact revision and generation for the apply step:

```text
automexia aliases enable ACTION_ID --name NAME --shell powershell --apply --expected-revision N --expected-generation DIGEST
```

This compare-and-swap pattern matters: if the source or generated state changed after your preview, the stale apply fails instead of changing something you did not review.

### Other alias operations

The management surface also provides:

```text
automexia aliases disable ACTION_ID [--shell SHELL]
automexia aliases rename ACTION_ID NAME
automexia aliases regenerate
automexia aliases disable-all
automexia aliases rollback CURRENT_GENERATION
automexia aliases reload --shell SHELL
```

Mutating operations are previews unless the required `--apply` and compare-and-swap values are supplied. `reload` does not mutate state; it prints the appropriate shell-native reload instruction (CMD requires a new session).

## Quick Action or alias?

| Question | Quick Action | Alias |
|---|:---:|:---:|
| Easy to discover by description/tags | **Yes** | Limited |
| Shows placeholders/risk before use | **Yes** | Less visible at invocation time |
| Keeps command visible before Enter | **Yes** | The alias name hides detail unless inspected |
| Good for occasional/complex commands | **Yes** | No |
| Good for very frequent mature command | Maybe | **Yes** |
| Should be global when task is project-specific | No | Usually no |
| Can be removed without losing canonical action | N/A | **Yes** — alias is derived |

A good lifecycle is: **normal command → Quick Action → optional alias**. Do not jump directly to global aliases for commands you have not stabilized.

## Workspace tasks: keep project commands project-local

**Implemented locally / release-gated.** Trusted workspace-task bridges can expose explicit tasks from supported runners such as `just`, Task, or `mise` only after the workspace has been reviewed/trusted.

Use this approach when the command belongs to one repository and the repository already has a task runner. It avoids polluting the global action/alias namespace and keeps project-specific behavior near the project.

Trust is bounded and revocable. A changed workspace cannot silently inherit stale approval. The bridge remains insertion-oriented rather than executing arbitrary project files in the background.

## Built-in DevOps packs

The local implementation contains reviewed static DevOps packs. They are catalogs, not hidden automation:

- actions are disabled by default;
- each action has a stable identity and documentation;
- tool/version and risk requirements are declared;
- collision and tool-health checks apply before alias eligibility;
- provider processes are not started because you typed a key or opened action search.

Treat built-in packs as starting points. Review the exact command and scope in the same way you would review a user-created action.

## Safety rules worth remembering

- Never convert terminal output directly into an executable command without review.
- Do not store credentials/tokens in canonical actions or generated aliases.
- Keep provider/network discovery off every-keystroke paths.
- Prefer exact arguments and explicit placeholders over shell-string interpolation.
- Let native shell definitions win unless you deliberately accept an Automexia-owned alias and the platform can safely prove ownership.
- Keep separate trust boundaries for Windows, WSL distributions, remote hosts, containers, and other machines; aliases/actions are not automatically synchronized across them.
- Use `doctor`, `preview`, and `test` before `--apply` when the management surface provides them.

For every exact flag and the release-gated CP3 import/workspace commands, use [CLI and automation reference](../reference/cli.md). For the design/status details, see [Shell integration and command productivity](../guide/shell-productivity.md).
