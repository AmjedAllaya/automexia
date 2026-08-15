# Command Productivity Compatibility Baseline

Status: CP0 accepted baseline, 2026-08-15. This document describes current
behavior and future adapter contracts; it does not claim that CP1 completion or
CP2 Quick Actions are shipped.

## Purpose

This baseline fixes the shell/editor ownership, supported-platform matrix,
provider-completion contracts, alias behavior, precedence, fallback, and
upgrade rules that later command-productivity phases must preserve. The
machine-readable authority is
[`cp0-contract-v1.json`](../tests/fixtures/command-productivity/cp0-contract-v1.json).

## Current implementation audit

| Surface | Current v0.4 state | CP0 classification | Next implementation |
|---|---|---|---|
| PowerShell | Idempotent prompt/listing integration; PSReadLine still owns input/history/completion; no provider completers or generated aliases | Baseline complete | CP1 health and managed completion adapter |
| Bash/WSL | Idempotent prompt/listing integration; Readline and native completion unchanged | Baseline complete | CP1 completion-directory adapter |
| Zsh/WSL/macOS | Idempotent prompt/listing integration; ZLE/compsys unchanged | Baseline complete | CP1 namespaced `fpath` adapter |
| Fish | No Automexia integration; native Fish behavior remains untouched | Deliberately absent, documented | CP1 first-class Fish adapter |
| CMD | Prompt/listing DOSKEY helpers only; no rich programmable-completion claim | Baseline complete | CP1 health plus action insertion; no false parity claim |
| DevOps short aliases | None supplied by Automexia | Correct CP0 nonactivation | CP3 opt-in generation only |
| Persistent Quick Actions | No source store or generated projection | Not implemented | CP2 |
| Provider-aware candidates | Existing status context only, not completion | Not implemented and forbidden on input | CP4 after D6 |

This audit is deliberately strict: prompt path styling, icon-aware `ls`, command
palette commands, OpenSSH host inventory, and shell history are not evidence of
managed autocomplete or persistent aliases.

## Shell and editor ownership

| Shell | Editor/completion owner | Supported hosts | Managed projection planned | Native fallback |
|---|---|---|---|---|
| PowerShell | PowerShell parser and PSReadLine | Windows required; Linux/macOS when PowerShell is installed | Argument completer or reviewed predictor only on supported versions; `Set-Alias` only for a simple name, function otherwise | Existing completion, keybindings, prediction settings, history, and profile remain unchanged |
| Bash | GNU Readline and Bash programmable completion | Linux/macOS/WSL | One user completion-directory file and namespaced functions | Existing compspec or default Readline filename completion |
| Zsh | ZLE and `compsys`/`compinit` | Linux/macOS/WSL | One Automexia-owned directory prepended to `fpath`; namespaced completion functions | Existing functions/options and user `fpath` remain authoritative |
| Fish | Fish editor, completions, and autosuggestions | Linux/macOS/WSL when installed | User completion file; abbreviation for visible insert expansion; function for logic | Existing Fish completions, autosuggestions, history, and universal variables remain authoritative |
| CMD | Console input and DOSKEY | Windows | Generated macro file for compatible aliases plus Automexia action insertion | Native CMD/DOSKEY behavior; no context-aware parity claim |

Automexia never recovers the editable command from terminal-grid cells. A future
custom completion surface requires the CP5 editor bridge and must preserve the
native editor as a complete disable/failure fallback.

## Read-only discovery contract

CP1 health checks may inventory names and configuration only through the active
shell's own read-only introspection. They must not source a profile, execute an
alias/function/completer body, start a provider, authenticate, access the
network, or write any shell state. Each discovery request has a 500 ms deadline,
a 256 KiB output ceiling, and a 4,096-entry ceiling; profile candidates retain
the general 1 MiB source-file limit.

| Shell | Permitted read-only sources | Current-user profile candidates |
|---|---|---|
| PowerShell | Current process version; profile path variables; alias/function names; PSReadLine options and key-handler names | CurrentUserAllHosts and CurrentUserCurrentHost |
| Bash | BASH_VERSION; type, alias, function, completion-specification, and Readline-binding listings | .bash_profile, .bash_login, .profile, and .bashrc |
| Zsh | ZSH_VERSION; command, alias, function, completion-name, binding, and fpath listings | .zshenv, .zprofile, .zshrc, and .zlogin |
| Fish | Version; command, function, abbreviation, completion, binding, and fish-complete-path listings | config.fish plus conf.d, completions, and functions directories |
| CMD | COMSPEC, executable resolution, and current DOSKEY macro names | Current-user Command Processor AutoRun registry value |

Profile paths are candidate metadata, not permission to read without limits or
to evaluate content. Unsupported or unavailable introspection produces an
unknown health field and preserves native behavior; it never enables a fallback
probe with more authority.

## Platform contract

| Environment | Required CP1 evidence | Installation boundary |
|---|---|---|
| Windows | Windows PowerShell 5.1 and current PowerShell 7 where available; CMD; ConPTY; Unicode/space profiles; enable/update/disable/uninstall | Current-user LocalAppData generated files and one marked profile block |
| Linux | Bash, Zsh, Fish where available; PTY; X11 and Wayland application builds; representative distribution packages | XDG user configuration/completion paths; never system directories without an explicit packaging action |
| macOS | Zsh default path plus Bash/Fish where installed; native PTY; signed application environment | User configuration/completion paths; respect platform shell initialization order |
| WSL | Per-distribution Bash/Zsh/Fish contract, Linux paths and permissions, Windows-host isolation | Explicit install inside each selected distribution; never write Linux adapters through translated Windows paths |
| SSH/container | Native remote/container shell remains complete without Automexia files | Separate explicit destination-scoped install; connecting never implies installation |

BSD remains source-compatible/best-effort under the Unix adapter. It is not a
release-certified native platform unless the main platform policy changes.

## Official provider completion baseline

Provider output is untrusted generated text. Discovery and refresh are explicit,
time/output bounded, version-keyed, atomic, and never performed on startup or
each keystroke.

| Provider | Official contract to prefer | Shell coverage used by Automexia | Important constraint |
|---|---|---|---|
| Git | Platform/package-provided native completion | Package-dependent Bash/Zsh/Fish/PowerShell support | Do not replace an existing user/package definition |
| Docker | `docker completion` | Bash, Zsh, Fish as documented by Docker | Generate only on explicit refresh; dynamic Docker objects remain the CLI/shell boundary |
| Kubernetes | `kubectl completion` | Bash, Zsh, Fish, PowerShell | An optional `kubectl` alias needs the matching official alias completion registration |
| OpenShift | CLI-provided completion when the installed version exposes it | Capability-detected | Missing support disables only this provider adapter |
| Helm | `helm completion` | Bash, Zsh, Fish, PowerShell | Dynamic plugin completion can execute plugin code and is separately opt-in |
| Terraform | `terraform -install-autocomplete` / `-uninstall-autocomplete` | Bash and Zsh | This command edits profiles; Automexia must preview/consent or use a reviewed equivalent, never call it silently |
| OpenTofu | Installed-version official completion contract | Capability-detected | Do not assume Terraform profile syntax without verification |
| AWS CLI | Installed `aws_completer` contract | Officially supported shells for that installation | Never invoke authentication or credential reads merely to diagnose completion |
| Azure CLI | Installed-version official completion contract | Capability-detected, including documented PowerShell integration | Preserve the active PowerShell completion registry and user definitions |
| Google Cloud CLI (GCP) | Completion installed with the official SDK | Officially supported Bash/Zsh paths for that installation | Treat SDK-managed files as externally owned |
| OpenSSH | Native shell host completion plus bounded public Automexia inventory | Shell/package-dependent | Inventory stays non-executable and credential-free |

Provider availability is a health state, not a startup error. Tool absence,
unsupported shell/version, malformed generator output, timeout, or cancellation
leaves native completion intact and produces one actionable diagnostic.

## Precedence and conflict matrix

Quick Action data layers use this deterministic order:

1. session-only;
2. active Environment Capsule;
3. explicitly trusted workspace;
4. shell-specific user;
5. global user;
6. disabled-by-default built-in pack.

Native shell definitions are outside and above this data-layer order: they win
unless the user chooses an explicit reversible override.

| Conflict | Required result |
|---|---|
| Native alias/function/completer uses the requested name | Do not activate; show both owners and offer rename/disable/explicit override |
| Two action IDs in one layer | Reject the candidate source and retain last-known-good state |
| Same action ID in different layers | Select the highest layer and expose the shadowed source in details |
| Built-in pack update meets customized action | Preserve customization and offer a reviewed merge/new version |
| Alias supported in one shell only | Generate only for declared shells; other shells retain the action without the alias |
| Provider/tool disappears | Keep user source; disable generated provider integration with truthful health |
| Generated file is modified externally | Do not import it as authority; report digest mismatch and require explicit regenerate/import decision |
| Workspace becomes untrusted | Remove its actions from the active index and revoke associated exact-launch grants |
| Capsule or session changes | Cancel old-generation work and rebuild the exact scoped view; never leak results across panes |

## Version and fallback policy

- CP fixtures and source schemas start at version 1. Unknown versions fail
  closed while the live last-known-good snapshot remains active.
- Shell and provider versions are detected, never assumed from the executable
  name. Unsupported versions produce a diagnostic and native fallback.
- Generated files carry schema, generator version, provider/tool version, source
  digest, and shell identity. Any mismatch requires bounded regeneration.
- A failed refresh never deletes a valid adapter. Temporary files, processes,
  and watchers are cleaned on success, failure, cancellation, and shutdown.
- Disabling Automexia integration restores native behavior without requiring the
  action source to be deleted.
- Uninstall removes only exact Automexia managed blocks/files and leaves user
  profiles, aliases, functions, completions, history, and provider files intact.

## CP0 acceptance

CP0 is complete when ADR 0015 is accepted, this baseline and the threat model
are published, schema-1 fixtures validate, the versioned hostile corpus proves
the checker rejects weakened nested contracts, architecture verification scans
every runtime crate, existing shell integration still adds no managed provider
completion/short aliases, and the master/stabilization roadmaps record evidence
without claiming CP1 behavior.

## Primary references

- [PSReadLine completion functions](https://learn.microsoft.com/powershell/module/psreadline/about/about_psreadline_functions)
- [PowerShell aliases and persistence](https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_aliases)
- [GNU Bash programmable completion](https://www.gnu.org/software/bash/manual/html_node/Programmable-Completion.html)
- [Zsh completion system](https://zsh.sourceforge.io/Doc/Release/Completion-System.html)
- [Fish interactive abbreviations](https://fishshell.com/docs/current/interactive.html#abbreviations)
- [DOSKEY macros](https://learn.microsoft.com/windows-server/administration/windows-commands/doskey)
- [Docker completion](https://docs.docker.com/engine/cli/completion/)
- [kubectl completion](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_completion/)
- [Helm completion](https://helm.sh/docs/helm/helm_completion/)
- [Terraform shell completion](https://developer.hashicorp.com/terraform/cli/commands#shell-tab-completion)
