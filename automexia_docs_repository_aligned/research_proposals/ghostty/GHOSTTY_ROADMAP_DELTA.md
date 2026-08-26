
# Ghostty Roadmap Integration

> **Reference copy:** The maintained integration note is
> [`repository_integration/GHOSTTY_ROADMAP_INTEGRATION.md`](../../repository_integration/GHOSTTY_ROADMAP_INTEGRATION.md).
> Do not treat this retained research copy as a second authority. The canonical
> project roadmap now keeps G0-G6 and overlays the TC/GM/PS classification.

The TC/GM/PS distinction should be integrated into the **existing real roadmap**, not maintained as a parallel authoritative roadmap.

## Recommended terminology

### TC — Modern Terminal Compatibility

Core terminal contract:

```text
VT/xterm
Unicode/graphemes
keyboard protocols
mouse/paste/focus
OSC
selected graphics protocols
real application compatibility
```

Ghostty is one pinned differential reference, not the oracle.

### GM — Ghostty Migration Compatibility

Optional migration UX:

```text
versioned Ghostty profile
generated reference fixtures
safe config import
action adaptation
compatibility inspector
maintainer tooling
```

`automexia` stays default.

### PS — Persistent Session Lifecycle

Owned by the actual project session-lifecycle architecture.

Current repository truth from the supplied audit:

```text
top-level tab parked-PTY undo/redo
    accepted + implemented under ADR 0028

split/pane/window history
    future / not yet qualified
```

## Reclassify G0–G6

| Old | Recommended real-roadmap treatment |
|---|---|
| G0 | GM fixture provenance/evidence |
| G1 | Core binding infrastructure |
| G2 | Core profile infrastructure + GM migration |
| G3 | Core dispatch + GM adaptation |
| G4 | Core action semantics + GM adaptation |
| G5 | GM maintainer tooling/evidence |
| G6 | PS lifecycle; top-level implemented, broader topology future |

## Profile immutability

Use profile revisions when correcting compatibility defects.

Example:

```text
ghostty-1.3-r1
ghostty-1.3-r2
```

Do not silently rewrite an already released pinned profile.

The moving alias:

```text
ghostty
```

may resolve to the newest supported profile.

## Authority rule

Compatibility mapping does not create new PTY/process/credential/filesystem/network authority.

Mapped actions continue through the real Automexia owners.
