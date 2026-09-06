# Public manual feature-testing guide

This guide covers the free terminal capabilities listed in
[the public feature catalog](FEATURES.md). It intentionally excludes unreleased
advanced features and commercial plans. Their source material is maintained in
the ignored private documentation area.

A passing source test is not a native-platform release claim. Record the exact
commit, operating system, package, display environment, shell, and result for
every manual run.

## Safety rules

- Use an isolated temporary configuration directory and synthetic data.
- Never copy real credentials, hostnames, account IDs, shell history, or
  customer data into evidence.
- Use loopback or disposable test services only.
- Do not test against production systems.
- Stop if cleanup ownership, process identity, or the target path is uncertain.
- Record a blocked or unavailable environment as not run, never as passing.

## Prepare a clean environment

1. Identify the exact source revision or package checksum.
2. Install the prerequisites listed in [Installation](INSTALLATION.md).
3. Create a fresh test profile outside normal user configuration.
4. Run the repository validation commands from [Testing](TESTING.md).
5. Launch Automexia with a supported local shell.
6. Prepare synthetic files containing ASCII, Unicode, combining marks,
   bidirectional text, long lines, and invalid byte sequences where supported.
7. Keep a second terminal available to observe child processes and cleanup.

For visual checks, cover a small window, a normal window, an ultrawide layout,
and a high-resolution display when available. Exercise 100%, 200%, and 300%
scale, light/dark/high-contrast themes, and reduced motion.

## Terminal rendering

### TERM-01 — text, Unicode, and reflow

Print ASCII, emoji, CJK, combining marks, wide glyphs, bidirectional samples,
tabs, and long wrapped lines. Resize repeatedly across narrow and wide layouts.

Expected result: cell widths, cursor positions, selection, and reflow remain
coherent. No stale pixels, panics, or unbounded growth appear.

### TERM-02 — control sequences and alternate screen

Exercise colors, styles, cursor movement, erase operations, hyperlinks, title
updates, and a full-screen program that enters and leaves the alternate screen.

Expected result: supported sequences render correctly; unsupported or malformed
input fails safely; returning from the alternate screen restores the ordinary
terminal without corrupting scrollback.

### TERM-03 — output pressure

Generate sustained bounded output while typing, selecting, searching, resizing,
and switching panes.

Expected result: input remains responsive, queues stay bounded, and closing the
pane stops and joins owned work.

## Sessions, tabs, and panes

### SESSION-01 — local shell lifecycle

Open a local shell, run a command, resize, send interrupt, request EOF, and exit.

Expected result: input order is preserved, exit status is reported, and no
owned child, PTY, handle, or worker survives closure.

### SESSION-02 — windows, tabs, splits, and focus

Create multiple windows, top-level tabs, pane-local tabs, horizontal and
vertical splits. Move focus with the keyboard and pointer and resize dividers.

Expected result: focus and input go only to the selected route. Closing one
surface does not affect unrelated sessions.

### SESSION-03 — fresh and cloned sessions

Create both a fresh split and a session clone using the documented commands.

Expected result: a fresh split uses default launch context; a clone copies only
the documented launch context and still owns an independent PTY.

### SESSION-04 — shutdown

Close panes, windows, and finally the application while shells are idle and
while they are producing output. On Windows, repeat with at least four live
panes or pane-local tabs whose shells are blocked in a long-running command;
time application close from the close request until every shell disappears.

Expected result: shutdown is bounded and all owned descendants and temporary
resources are cleaned up. The four-session Windows case completes within six
seconds on the controlled native fixture; no shell remains after the Automexia
window closes. Other hardware is compared against its recorded same-host
ceiling rather than this development observation.

## Search, selection, and clipboard

### INPUT-01 — search scopes

Search the selected pane and the visible workspace for empty, missing, common,
Unicode, and very long patterns. Continue output while search is active.

Expected result: result counts and navigation match the chosen scope, stale
results are rejected, and dismissing search restores focus.

### INPUT-02 — keyboard and pointer selection

Select forward and backward across wrapped lines, wide glyphs, and scrollback.
Repeat with keyboard-only and pointer-only input.

Expected result: selected text matches terminal cells without splitting a
grapheme. Selection never becomes terminal input automatically.

### INPUT-03 — copy, paste, interrupt, and mouse reporting

Test the documented copy and paste bindings, interrupt behavior, bracketed
paste, and an application that enables mouse reporting.

Expected result: copy does not send interrupt, paste does not add an implicit
Enter, and mouse events go to the correct owner.

### INPUT-04 — command navigation and palette isolation

Run at least six output-producing commands with visibly different text, resize
the window to a narrow layout and back to a normal layout, then use
Ctrl+Shift+Up and Ctrl+Shift+Down repeatedly in both directions. Stop on each
command and inspect every visible completion datetime/duration badge. Repeat in
a split pane with enough workspace/provider context to fill most of the prompt
row, then open the command palette over active output.

Expected result: each visible result has one badge, no badge text is duplicated,
merged, clipped into another badge, or moved onto the wrong command, and the
result label never overlaps a prompt-context chip. The same command keeps the
same datetime and duration through reflow. Wait briefly on each position and
confirm the frame does not fill in progressively, flicker between two metadata
owners, or become obscured by an error dialog. Navigation remains pane-scoped
and does not edit or execute the live command; palette
keystrokes do not leak to the PTY; closing the palette restores prior focus.

## Configuration and appearance

### CONFIG-01 — first start

Launch with no Automexia configuration.

Expected result: a starter configuration is created only when appropriate and
existing unrelated files are not overwritten.

### CONFIG-02 — transactional reload

Change one documented appearance option, then introduce invalid TOML.

Expected result: the valid change appears; invalid input reports a redacted,
actionable error and preserves the last-known-good configuration.

### CONFIG-03 — themes, fonts, cursor, and scale

Exercise built-in and custom themes, available fonts, font-size limits, cursor
styles, opacity, line spacing, and high DPI.

Expected result: content remains legible, clipped UI remains operable, and
restart preserves only documented preferences.

### CONFIG-04 — migration and rollback

Use only synthetic legacy configuration. Preview migration, apply it, restart,
and restore the backup.

Expected result: migration is explicit and non-destructive; source files remain
recoverable; rollback returns to the prior behavior.

## Shell integration

### SHELL-01 — supported shells

Test each installed supported shell with session-local integration enabled and
disabled.

Expected result: prompt metadata appears only when supported, ordinary shell
editing remains shell-owned, and disabling integration restores normal shell
behavior.

### SHELL-02 — prompt and directory metadata

Change directories, run successful and failing commands, and enter and leave a
Git repository.

Expected result: visible metadata updates for the correct pane and command
generation without exposing secrets or changing the command.

### SHELL-03 — listings and pipelines

Use the documented enhanced listing in interactive output and in a pipeline.

Expected result: decoration is useful interactively while piped objects and
bytes remain unchanged.

### SHELL-04 — missing or damaged integration

Temporarily make an integration resource unavailable in the isolated profile.

Expected result: the terminal and shell still launch; the error is bounded and
redacted; repair or disable instructions work.

## Images

### IMAGE-01 — local raster preview

Preview small supported local images, then try unsupported, oversized, linked,
replaced, and permission-denied files.

Expected result: valid previews are bounded and dismissible. Unsafe or invalid
files fail without following links, blocking input, or retaining stale data.

### IMAGE-02 — inline protocols

Exercise Sixel, Kitty, and iTerm2 images using public synthetic fixtures.

Expected result: decoded dimensions and memory remain bounded, images clip and
scroll with their owning terminal state, and clearing or closing releases them.

## OpenSSH interoperability

### SSH-01 — ordinary system client

Use the system OpenSSH client manually against an authorized loopback fixture.

Expected result: Automexia behaves like an ordinary terminal, does not take
credential custody, and preserves OpenSSH diagnostics.

### SSH-02 — explicit inventory

Use only synthetic OpenSSH configuration files selected through the documented
public workflow. Include duplicates, includes, invalid syntax, links, oversized
files, replacement, and revocation.

Expected result: only explicitly reviewed bounded files are read; public
metadata is shown; secrets are not persisted; stale or unsafe input keeps the
last-known-good view.

### SSH-03 — clipboard and resize

During a loopback SSH session, test paste without Enter, selection, search, and
resize storms.

Expected result: remote input semantics match local terminal semantics and
closing the session cleans up the owned local client process.

## Accessibility and visual review

### ACCESS-01 — keyboard-only operation

Reach every public surface without a pointer, including menus, dialogs, tabs,
panes, search, and dismissal paths.

Expected result: focus order is logical, focus is visible, and closing an
overlay restores the previous target.

### ACCESS-02 — semantics and announcements

Inspect the accessibility tree and events for names, roles, states,
relationships, selections, errors, and live announcements.

Expected result: meaning is not conveyed by color alone and repeated updates are
coalesced rather than flooding assistive technology.

### ACCESS-03 — native assistive technology

When the environment is available, run the public workflow with Narrator or
NVDA on Windows, VoiceOver on macOS, and Orca on Linux.

Expected result: the core terminal workflow is understandable and operable.
Missing native evidence remains an explicit release gate.

### VISUAL-01 — deterministic and native frames

Run renderer-neutral layout checks and deterministic raster comparisons, then
inspect native frames at the documented themes, scales, and window sizes.

Expected result: hierarchy, clipping, z-order, cursor placement, contrast, and
focus are correct. Any intentional golden update is reviewed separately.

## Packaging, upgrade, and uninstall

### PACKAGE-01 — package identity

Inspect the package, checksum, version, license notices, and provenance.

Expected result: identity matches the tested revision and no private or
machine-local documentation is included.

### PACKAGE-02 — clean install and upgrade

Install into a clean supported environment, launch, upgrade from the supported
predecessor, and verify configuration preservation.

Expected result: install and upgrade do not mutate unrelated shell or user
configuration and failures provide recovery instructions.

### PACKAGE-03 — uninstall

Uninstall after creating only synthetic configuration and sessions.

Expected result: packaged files are removed, user-owned data follows the
documented retention rule, and no child processes or startup hooks remain.

## Contributor verification

For documentation-only changes, run the documentation hygiene, link,
confidentiality, and repository validation checks. For code changes, follow the
full evidence ladder in [Testing](TESTING.md), including formatting, focused
tests, linting, architecture checks, fuzz/property ownership, and platform
evidence proportional to risk.

## Final record

A complete manual record includes:

- exact source revision or package digest;
- operating system, architecture, shell, renderer, display scale, and theme;
- scenarios run, skipped, failed, and blocked;
- redacted evidence locations;
- first observed failure and investigation result;
- cleanup confirmation; and
- reviewer and date where controlled release evidence is required.

Do not publish screenshots, logs, or fixtures until they have been checked for
credentials, account information, local paths, hostnames, and private product
documentation.
