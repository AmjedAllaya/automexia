# ADR 0081: Kubernetes status capability boundary

Status: implemented local-only boundary; non-activating. ADR 0003 remains
authoritative for any future capability change.

## Context

The information bar's shell identity tags are renderer-owned and remain visible
when optional DevOps discovery is disabled or unavailable. Additional local
DevOps tags use the built-in local-read `automexia.devops` provider. Its Git
branch tag has a separate default-on preference, so hiding broader DevOps
context does not hide Git identity. Both features still require the installed
local-read package. Reading kubeconfig identifies the configured context and namespace;
it cannot prove that the namespace still exists in the cluster. A separately
registered `devops.kubernetes` package declares process and network
capabilities but is disabled by default and is not the live prompt provider.
An installed marker or a terminal-controlled value is not a capability grant.

## Current decision

The live runtime admits only the reviewed local-read DevOps manifest for
passive discovery. It rejects process, network, session-launch and clipboard
capabilities on that path. Generic terminal output coloring belongs to core
presentation under ADR 0035 and is independent of DevOps installation.
Discovery continues on the existing bounded worker,
with session/capsule/source revision checks; input, PTY, resize and rendering
perform no provider I/O or wait.
On Windows, the pre-existing app-owned WSL local-file helper may launch the
current Automexia executable with fixed arguments on that worker. It reads
only configured local context files and does not grant the extension a process
capability or verify a remote namespace.
Passive file discovery rejects linked or reparse-point path components and,
on Windows, mapped network drives. Relative Git worktree pointers are
normalized before the same check. A configuration reached only through such a
link is unavailable to the status bar. The entry check and file open are not
atomic across every parent component; concurrent path replacement and Unix
network mounts remain limitations of this boundary.

The Kubernetes tag shows the configured namespace without a question-mark suffix
or corner status dot. Its accessible explanation and freshness metadata retain
the distinction between a configured selection and verified cluster state.
A completed local refresh replaces the
previous snapshot, so removal of the kubeconfig selection removes its tag.
Visible routes reconcile local sources on a three-second background cycle;
worker saturation may delay completion, and hidden routes refresh when shown.
This is bounded reconciliation, not an immediate filesystem event watcher.
An unchanged kubeconfig does not imply cluster validity.

An application-owned namespace-probe broker has bounded, revocable,
session-scoped leases and rejects obsolete generations, expired results and
unmatched consent. Its production capability gate is closed and it has no
process or network adapter. No consent or settings value can activate a live
request in this state. Positive or absent cluster status is not published.

## Capability boundary

This ADR does not supersede [ADR 0003](0003-extension-capability-and-threading.md)
or authorize a new capability. Live cluster verification requires a security
review, a replacement ADR accepted for the exact adapter, two independent
protected-path approvals on the exact reviewed head, enforced grant binding,
and applicable native lifecycle and performance evidence before activation.
Until then, the local-read-only boundary and unverified metadata remain.

## Consequences

Users are not told that a configured namespace exists in the cluster unless
there is actual authorized evidence. Deleting only the remote namespace leaves
the configured value and its unverified metadata intact; local configuration removal clears it
on refresh. Disabling the provider or retiring a session cancels its optional
state without blocking terminal work. Its tags are removed while core OS/user
tags and an independently enabled Git branch remain attached to each prompt
and pane. No new persistence schema format,
dependency, process, network request, credential access or provider execution
is added by this ADR.
