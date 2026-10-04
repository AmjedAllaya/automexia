# Quick Actions and reviewed workflows

Open **Quick Actions** from `Ctrl/Cmd+K`. Use the arrows or Tab to choose a row,
Enter to open it, and Escape to go back. Typing searches actions and management
commands. All rows also support pointer activation.

## Save a command

Choose **New Quick Action** (`Ctrl/Cmd+N` inside the action list). Enter a name,
the command text and its shell. Description is optional. The shell selector
keeps PowerShell, CMD, Bash, Zsh and Fish commands separate; the same text can
have different meanings in different shells.

Open a field with Enter. `Ctrl/Cmd+A` selects its value, typing or paste replaces
it, Enter accepts it and Escape cancels that field edit. `Ctrl/Cmd+S` saves the
draft. Saving never runs the command. Escape from the editor asks before
discarding changes.

Choose the saved action to review it. **Insert without Enter** puts its text in
the current terminal's input buffer; edit it or press Enter yourself. **Copy**
copies the reviewed command. Destructive and privileged actions require another
confirmation. Existing typed actions retain their exact arguments and shell
quoting; their public parameters are requested before review. Secret parameters
remain unavailable in this interface.

**Manage saved actions** provides editing, duplication, enable/disable and
confirmed deletion. Built-in actions are copied before editing. Changes are
saved to the existing `actions/actions.toml` store under the configuration
directory. The terminal does not write a separate action preference database.
If another window changes that store, the save fails without overwriting it.
Your draft remains open: choose **Duplicate** to preserve it as a separate
action, or discard it and reopen the saved version.

## Build a workflow

Choose **New workflow** (`Ctrl/Cmd+Shift+N` inside Quick Actions). Name it and
open **Step 1**. Each step has:

- **Command:** one command line, at most 4,096 bytes. The selected shell parses
  it normally. Automexia does not add quoting or substitute hidden values.
- **Shell:** the shell that will receive this step.
- **After this command:** successful completion, a new integrated shell, or
  pause after successful completion.
- **Timeout:** 1–3,600 seconds, measured while waiting for the completion condition.
- **Move earlier / Move later / Remove step:** edits the draft's order.

Use **Add step** for up to 32 steps. Give the workflow a risk classification
that describes its most consequential command. Do not put passwords, tokens,
private keys or other credentials in saved commands. Use your shell's normal
interactive authentication or a configured external credential reference.

Saving and opening a workflow never execute it. In its review, open any numbered
step to read the complete command in scrollable display fragments, including
text that does not fit the summary row. These fragments do not change the
stored command. Escape returns to review. **Run workflow** is the explicit
approval to submit the displayed steps automatically in the current terminal.
Destructive and privileged workflows require a second Run confirmation.

The first step needs an empty integrated prompt. After each command, Automexia
waits for the matching command's successful exit status and the next empty
prompt. A failed command, missing status, timeout, terminal reset, unexpected
shell change, keyboard input or pasted input pauses further steps. No command
is retried automatically. Stock CMD does not supply the verified command
identity/status required by automatic workflows; use ordinary insert-only
Quick Actions there. Missing or disabled integration also prevents automatic
submission.

For example, a local PowerShell workflow can run `git status --short`, then
`git diff --stat`, with **successful completion** for both. A shell-appropriate
workflow may change directory in one step and run a tool in the next; the
existing shell owns that directory change. Actions with a separate fixed-CWD
policy are shown as unavailable rather than silently ignoring that policy.

## SSH and interactive transitions

An SSH step uses **Wait for a new integrated shell (SSH)**. Set the following
step's shell to the remote shell. Automexia's SSH integration must publish the
remote scope and an empty prompt; an ordinary unintegrated SSH session cannot
authorize subsequent steps. Passwords, host-key checks and MFA stay interactive
in the terminal. Entering authentication input pauses subsequent automation.
Once the new shell is ready, open **Workflow progress** and explicitly **Resume**.
Unknown shells and missing metadata remain paused; screen text is never used to
guess that login succeeded.

The same rule applies to switching users or entering another shell: a new shell
must be integrated before a later step can run there. Workflow metadata is a
sequencing aid, not proof of a server's identity. SSH owns host verification and
authentication. Automexia does not upload credentials or install a remote helper
as a side effect of running a workflow.

## Pause, resume and cancel

Open **Quick Actions → Workflow progress** to see the current step and status.
**Pause** prevents further submissions; a command already delivered to the shell
continues. **Resume** requires the original terminal and a safe integrated
prompt. A submitted command is not repeated. A failed command must be corrected
by cancelling the run and reviewing an edited workflow; Resume does not skip it.
**Cancel remaining steps** leaves any active command under ordinary shell
control. Use the shell's normal interrupt mechanism if it must also stop.

Changing tabs or panes pauses the run. Returning does not resume automatically.
Runs are tied to their original terminal identity and are never transferred to
a replacement pane. Closing the terminal or restarting Automexia does not resume
an old run, even when the workspace itself is recovered.

## Portable files and compatibility

The existing `automexia actions import` and `automexia actions export` commands
handle portable files; use their `--help` for preview/apply and revision options.
Import does not run an action. Review any imported command before Run.

The action document remains schema 1. Workflows add a `workflow` template with
its own `version = 1`, explicit ordered steps, and `mode = "run-workflow"`.
Existing insert/copy documents keep their behavior. Unknown workflow versions
are rejected. Older builds cannot read workflow variants: export/back up the
actions first and remove workflow entries before downgrading. Run progress and
prompt receipts are temporary, content-free state; they are never persisted.
