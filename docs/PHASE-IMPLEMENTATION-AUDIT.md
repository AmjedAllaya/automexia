# Public implementation status summary

This summary records the public free-terminal scope. It is intentionally not an
internal phase plan and does not enumerate private advanced or commercial work.

| Public area | Current documentation status |
|---|---|
| Core terminal and configuration foundations | Publicly documented: product identity, non-destructive configuration, VT/Unicode/scrollback/reflow/search/selection, PTY/ConPTY lifecycle, tabs/panes/focus/clipboard, GPU rendering, appearance, accessibility, protocol images, recovery, and cleanup boundaries. |
| Command productivity foundations | Publicly documented: command navigation, search, palette, native completion, reviewed Quick Actions, local aliases/packs/workspace bridges, no implicit Enter, and native-shell ownership. |
| Connection inventory and read-only Connection Hub | Publicly documented: explicit system OpenSSH inventory and keyboard-first read-only review without credential custody, passive discovery, login, or hidden activation. |
| Managed SSH and provider-neutral launch foundations | Source/test-owned but availability remains release-gated where the public feature catalog says so; exact review, argument, trust, cancellation, and external-credential boundaries apply. |
| Multi-cloud and orchestrator adapters | Source/test-owned cached public metadata boundaries are documented; provider authentication remains external and unshipped or unassured paths are not public availability claims. |
| Public extension ecosystem | Only the deny-by-default extension contract, lifecycle, capability, disable/uninstall, and failure-isolation boundary is public; unpublished product/marketplace plans remain private. |
| Semantic Diagnostic Navigator | High-level direction only; detailed product, architecture, sequencing, and commercialization remain private and no availability is claimed here. |
| Situation-aware Production Operations | High-level direction only; detailed workflows and commercial planning remain private and existing provider/approval systems retain authority. |
| Automation Studio | Separate later direction only; detailed product and commercial plans remain private and it is not part of the current public terminal offer. |
| Optional LLM Orchestration extension | Separate optional later direction only; detailed model/provider/commercial plans remain private and no hidden AI authority is implied. |
| Video-editing and other specialized domains | Preserved as separate later extensions with independent validation; detailed private plans are not public commitments or current terminal scope. |

## Truth rule

Source tests remain authoritative for implementation truth. Public availability
also requires an entry in [Features](FEATURES.md) and an applicable user guide.
Disabled source, fixtures, compatibility paths, internal names, proposals, or
private plans do not create a public feature.

## Remaining evidence

Native operating-system, renderer, shell, display, accessibility, package,
signing, hardware, and long-duration evidence remains external until it runs on
the exact revision and packaged artifact. A missing environment is not a pass.

## Documentation result

The public catalog, architecture, user guides, testing, roadmap, decision index,
and navigation must agree on the baseline scope. Historical mixed documents are
preserved privately so the public rewrite is lossless and reversible.

Feature documentation and source tests remain authoritative for exact behavior
and local implementation status; the public feature catalog remains
authoritative for availability.
