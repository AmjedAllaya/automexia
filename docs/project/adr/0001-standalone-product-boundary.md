# ADR 0001: Standalone product boundary

Status: accepted for v0.4.

Automexia preserves Rio history and private engine crate names while moving the
desktop frontend to `apps/automexia-terminal`. Product IDs and paths are
centralized in one compatibility adapter. A clean clone builds without patchers
or another source checkout. This creates a reviewable stable baseline without
mixing the mechanical move with engine refactoring.
