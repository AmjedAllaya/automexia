# OpenSSH inventory and connection review

Implementation owner: `automexia-connectivity`. This package name records
current source ownership and does not announce a connection product.

This public page describes only ordinary free system-OpenSSH interoperability
and the explicit local inventory behavior that is safe to document publicly.
It is not a roadmap for managed connectivity, cloud services, organization
features, or commercial products.

## What Automexia owns

Automexia owns the terminal session, focused route, PTY lifecycle, review UI,
and bounded public inventory records.

System OpenSSH owns configuration semantics, host-key policy, agents,
credential prompts, network behavior, and remote authentication.

Automexia is not a credential vault and does not silently scan accounts,
networks, provider tools, or user directories.

## Explicit inventory

The public inventory workflow reads only configuration files the user explicitly
selects and reviews. It applies fixed limits to file count, bytes, include
depth, records, labels, and refresh work.

Inventory may show bounded public connection metadata such as an alias and the
source type. It must not persist passwords, keys, tokens, command history,
terminal output, or secret environment values.

Opening, searching, sorting, or filtering the inventory performs no connection,
login, network request, credential-cache read, or process launch.

## File safety

Inventory input is untrusted. The reader rejects or safely handles:

- missing, malformed, oversized, and partial files;
- include cycles and excessive depth;
- links and identity replacement;
- permission changes and read failures;
- duplicate and conflicting records;
- control characters and hostile Unicode labels; and
- stale results from an obsolete refresh generation.

A failed refresh preserves the last-known-good view and returns a redacted,
actionable error.

## Ordinary SSH use

Users can run the system `ssh` executable manually in Automexia like in any
other terminal. Paste never adds Enter. Resize, selection, search, tabs, panes,
exit reporting, and cleanup remain terminal responsibilities.

Use only authorized destinations. For tests and documentation, prefer a
disposable loopback fixture and fictional values.

## Keyboard and accessibility

The inventory and review surfaces are keyboard reachable, identify focus
clearly, restore focus when closed, distinguish empty/loading/error/stale
states without color alone, and remain usable at high scale and narrow layouts.

## Disable and recovery

Revoking a selected file removes its public records from the next accepted
snapshot. Disabling the inventory leaves ordinary local and manual OpenSSH
terminal use available. Removing Automexia does not modify user-owned OpenSSH
configuration.

Unreleased connection products and internal plans are private and must not be
inferred from types, tests, or filenames.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### F2/D5.0 capability-free inventory and planning

F2/D5.0 owns the bounded, non-executing connection models, explicit local
inventory, review state, and renderer-neutral Hub contract described here.
Network, authentication, process, PTY, and credential authority remain outside
this phase and require their separately reviewed activation boundaries.

### Aws

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Azure

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### D51 Read Only Hub And First Run Detection

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### D60 Provider Neutral Capsule And Authentication Framework

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Google Cloud

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Kubernetes

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Product Outcome

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Renderer Neutral Interaction Goldens

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Search Filters And Refresh

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.
