
# Ghostty Migration Compatibility

**Audited state:** substantial implementation exists at committed baseline `20ff7928ea2d1eac5d26f13c62d0cda9b86bc078`; this is not fresh native/release certification.

Use:
- `automexia` as default;
- versioned Ghostty compatibility profiles;
- safe bounded config migration;
- generated fixtures;
- action adaptation;
- inspector/tooling.

Keep Ghostty authority out of core.

## Profile revisions

Do not silently mutate a released pinned profile to correct a behavioral mistake.

Use an explicit revision:

```text
ghostty-1.3-r1
ghostty-1.3-r2
```

and provide migration/release notes.

`ghostty` may remain a moving alias to the newest supported profile.

## Evidence

Report separately:
- fixture provenance;
- binding/action source completion;
- Windows/Linux/macOS native evidence;
- accessibility;
- long-run/resource;
- migration fuzz campaigns.

Do not claim blanket "full Ghostty product compatibility."
