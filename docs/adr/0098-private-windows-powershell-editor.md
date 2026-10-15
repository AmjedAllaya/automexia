# ADR 0098: Package the maintained Windows PowerShell editor

Status: Accepted

## Context

A fresh Windows installation can include PSReadLine 2.0.0. Its native editor
writes input at stale coordinates after a pane shrinks and grows with existing
output. The same native test fails with both inbox and packaged ConPTY, and an
explicit repaint barrier does not fix it. Replacing only the editor with
Microsoft PSReadLine 2.4.5 makes the unchanged test pass.

Microsoft tracks the defect in [issue 1456](https://github.com/PowerShell/PSReadLine/issues/1456)
and fixed it in [PR 3074](https://github.com/PowerShell/PSReadLine/pull/3074).

## Decision

Adopt the maintained Microsoft editor instead of modifying native cursor
coordinates, injecting redraw keys, patching private editor internals, or
requiring a global module update.

Windows MSI and portable ZIP packages include eight original PSReadLine 2.4.5
files under `powershell-modules/PSReadLine/2.4.5`. The packaging recipe pins the
official Gallery archive, each file's size and hash, Microsoft publisher, and
MIT license. Build tooling owns bounded download, extraction, cache and staging.
No binary or generated module belongs in source control or private documentation.
Offline builds use the verified cache or an explicitly supplied pinned archive.

Windows PowerShell loads its editor before profiles or command bootstrap.
Integrated interactive launches therefore prepend the private directory only
in their child module environment. Other module locations and native default
locations remain available; profile customizations run against the selected
editor. PowerShell 7, explicit commands/scripts, other shells and integration
opt-outs keep their launch behavior.

Resource setup reads a bounded snapshot of the Windows PowerShell policy
configuration through the Windows registry API. A pure model combines those
advisory values with each launch's process policy and host arguments. Restricted,
AllSigned, unreadable and unknown settings keep the system editor, avoiding a
new publisher-approval prompt. Windows PowerShell remains the sole policy
enforcer; Automexia neither changes policy nor grants script permission.
Restart after a machine/user policy change to refresh the advisory snapshot.
No provider query, module scan or subprocess is added to input/tab creation.
Profiles and global modules remain unchanged; runtime never downloads packages.

The existing package/resource trust boundary owns the installed directory.
Release verification checks all eight hashes, seven original Microsoft code
signatures and timestamps, MSI/ZIP layout, both SBOM identities and the MIT notice.
The native resize driver stages the same private editor and ConPTY, verifies the
selected module and standard-module availability, and exercises real native
selection, input, resize, scrollback and cleanup. Inbox-editor failures remain
recorded as upstream compatibility evidence rather than passing product tests.

## Compatibility and failure behavior

Windows trust reports advance to schema 5, adding the editor package identity
and its vendor signatures. Earlier evidence cannot certify the new payload;
generate a new report from the final packages. Public package filenames and
application configuration remain unchanged.

Builds reject changed, linked, incomplete or unknown module payloads and preserve
conflicting files. A missing development module permits the existing shell
fallback; release gates require the complete module. Download timeouts retain
cleanup ownership before another preparation can start. Rollback uses an earlier
complete Automexia package rather than editing global PowerShell state.
