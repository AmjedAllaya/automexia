# Remote connections

Automexia supports ordinary command-line remote work through the system
OpenSSH client and other tools run in the user's real shell.

## Use system OpenSSH

Type normal commands such as `ssh`, `scp`, and `sftp` in a pane. The shell
owns parsing and expansion. OpenSSH owns configuration, host keys, credentials,
agents, authentication, proxy behavior, and the network connection.

Automexia owns only the terminal session: input/output, paste without Enter,
resize, selection, search, tabs, panes, exit reporting, and cleanup.

## Explicit local inventory

Where the public build exposes local OpenSSH inventory, it reads only files the
user explicitly selects and reviews. Reads are bounded and link/replacement
safe. Only public metadata is retained, and opening or searching inventory does
not connect, authenticate, or read credentials.

A failed refresh keeps the last-known-good public view. Revoking a file removes
its records from the next accepted snapshot without modifying the source file.

## Safety

Use authorized hosts only. Tests and examples use fictional names and a
disposable loopback fixture. Never publish real hostnames, account IDs, keys,
tokens, configuration, or remote output.

## WSL

WSL shells are ordinary terminal sessions. The distribution and shell own their
configuration and tools. Automexia forwards input, resize, output, and exit
state and keeps route/session ownership isolated.

## Failure and cleanup

Native OpenSSH diagnostics remain authoritative. Closing the pane cleans up the
owned local client process and PTY. Optional inventory or shell-integration
failure leaves manual system OpenSSH available.

Unreleased managed connectivity, provider products, and commercial services are
private and are not described here.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Internal Connection Library Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.
