# ADR 0008 — Dedicated extension-status lane and font-independent default UI

Status: partially superseded by ADR 0009 in v0.3.7

## Context

Real Windows/WSL dogfooding showed that the DevOps HUD was drawn at a hard-coded top coordinate while the Rio-derived navigation renderer independently drew a centered single-tab title in the same strip. Discovery was working, but cyan extension text and the white title overlapped. Long Git branches amplified the collision.

The DevOps icon vocabulary also used Nerd Font private-use codepoints. Those are not portable Unicode UI contracts: whether they render depends on the user's selected font/fallback set.

## Decision

Automexia owns a dedicated extension-status lane in application chrome. The lane is positioned after active navigation/title chrome and its height is reserved before terminal rows/columns are calculated. Extension renderers receive/use that layout origin rather than inventing absolute top-window coordinates.

Default first-party extension UI must not rely on private-use glyphs. Standard Unicode symbols may be used only with explicit human-readable labels. User/repository/context-derived status strings must be bounded or truncated before drawing, and lower-priority segments are omitted when the lane is full.

## Consequences

- tab/title, extension status and terminal content have separate layout regions;
- toggling/rendering status cannot obscure shell output;
- the DevOps context remains understandable without a Nerd Font;
- the application owns chrome geometry while extensions own renderer-neutral context models;
- future extension APIs should generalize this lane into generic status contributions rather than add more extension-specific top-level coordinates.


The font-independent icon decision remains active. The fixed-lane placement decision is superseded by ADR 0009.
