# ADR 0030: Automation Studio and domain-extension boundary

Status: proposed public summary; editor dependency and implementation are not
accepted yet.

## Context

Automexia needs a comfortable script workspace without turning the core terminal
into a mandatory IDE or duplicating mature editor infrastructure.

## Proposed direction

- Deliver Automation Studio as an independently enabled extension after the
  first stable release and before the video-editing extension.
- Host its primary interface inside an Automexia pane.
- Adopt or wrap a maintained editor component after a dedicated dependency and
  accessibility review.
- Let domain extensions contribute language support and workflows through
  versioned contracts, while Automation Studio remains the sole editor owner.
- Broker file, process, language-service, and network capabilities through the
  application boundary.

## Consequences

The core stays small and responsive, while users gain an integrated workflow.
The extension can be disabled or replaced independently, but the first release
must deliberately support a smaller language and platform set than a mature
desktop IDE.

The detailed dependency comparison and internal topology remain local until the
decision is accepted with implementation evidence.
