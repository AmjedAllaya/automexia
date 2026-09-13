# Product vision

Status: public purpose for the open-source terminal. The
[feature catalog](FEATURES.md) is authoritative for current availability.

## Purpose

Automexia is a flexible open-source terminal that helps people organize and
control command-line work without hiding the shell.

It combines familiar terminal capabilities—shell sessions, tabs, panes, search,
images, configuration, and keyboard navigation—in one focused workspace.

## Intended users

Automexia is useful to anyone who works through a shell, including developers,
system administrators, operators, researchers, analysts, and technical
contributors. Public documentation does not assume that one profession or one
external tool defines every user.

## Public product values

1. **Direct commands.** The shell and installed tools retain their normal
   execution behavior.
2. **Visible state.** The active pane, shell, path, selection, search scope, and
   session state remain understandable.
3. **Keyboard completeness.** Primary terminal workflows work without requiring
   a pointer.
4. **Optional structure.** Tabs, panes, local tabs, search, command discovery,
   and local actions help without being mandatory.
5. **Local control.** Local files, previews, commands, clipboard contents, and
   credentials are not silently uploaded.
6. **Safe recovery.** Invalid configuration does not replace the last working
   state; disable, cleanup, and uninstall remain explicit.
7. **Honest status.** Source-complete, release-gated, externally unverified, and
   publicly released behavior are described separately.
8. **Replaceable tools.** Automexia does not claim ownership of shells, SSH,
   cloud CLIs, editors, or other specialist programs.
9. **Open-source usefulness.** The published terminal remains useful without an
   account, hosted service, or paid dependency.

## Current product

The current v0.4 source focuses on well-known terminal capabilities:

- Windows, tabs, split panes, and pane-local tabs;
- PowerShell, Command Prompt, WSL, Bash, Zsh, and Fish integration;
- selection, scrollback, reflow, clipboard, and Unicode terminal behavior;
- selected-pane and visible-workspace search;
- command palette and marked-command navigation;
- local and terminal-protocol images;
- themes, fonts, shortcuts, shell selection, and transactional configuration;
- optional keyboard-compatibility profiles;
- local command-productivity foundations; and
- bounded read-only OpenSSH inventory foundations.

Some source behavior still requires native, packaging, security, accessibility,
or release evidence. See [Features](FEATURES.md), [Platforms](PLATFORMS.md), and
[Readiness](READINESS-AUDIT.md).

## Publication boundary

Public vision and roadmap pages describe current implementation, observable
limitations and release evidence only. Future features remain private,
including free and open-source ideas. Business plans, commercial strategy,
pricing, market analysis and future architectures are not public documentation.

Private planning is not evidence that a feature exists. Public descriptions
must match actual source behavior and distinguish it from released artifacts.

## Documentation language

Public pages should:

- lead with current user value;
- use “available,” “release-gated,” “disabled,” and “not implemented”
  precisely;
- avoid implying coverage of every shell, provider, platform, or workflow;
- never present source tests as a public binary release;
- avoid announcing private future plans; and
- link to the exact guide, reference, architecture, and evidence owner for
  current behavior.

The [brand guide](BRANDING.md) owns public wording.
