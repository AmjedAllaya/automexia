
# Proposal Index

This index catalogs research/proposal material. It does **not** establish authority over real project ADRs or source.

## Adopt/integrate now as deltas

1. `repository_integration/ACTIVATION_HARDENING_DELTA.md`
2. `repository_integration/GHOSTTY_ROADMAP_INTEGRATION.md`
3. `repository_integration/CP5_ADR_0025_RESEARCH_DELTAS.md`
4. `repository_integration/SUPPLY_CHAIN_INTEGRATION.md`
Useful supporting research, to mine selectively rather than adopt wholesale:

- `research_proposals/docs/09_TESTING_RELEASE.md`
- `research_proposals/docs/17_SUPPLY_CHAIN_AND_DEPENDENCY_POLICY.md`
- `research_proposals/docs/18_MODERN_TERMINAL_COMPATIBILITY.md`
- `research_proposals/docs/19_GHOSTTY_MIGRATION_COMPATIBILITY.md`

## Keep as target principles / candidate architecture

- `research_proposals/ARCHITECTURE_RESEARCH_AND_TARGET_PRINCIPLES.md`
- `research_proposals/docs/01_RUST_CORE_ARCHITECTURE.md`
- `research_proposals/docs/02_PROCESS_SESSION_IPC.md`
- `research_proposals/docs/04_EXTENSION_PLATFORM.md`
- `research_proposals/docs/06_RESOURCE_ACCESS_MODEL.md`
- `research_proposals/docs/07_CREDENTIAL_BROKER.md`
- `research_proposals/docs/11_PLATFORM_CAPABILITIES.md`
- `research_proposals/docs/12_AUTOMEXIA_LAB.md`

These documents must distinguish current architecture from candidate/future architecture.

## Future/product-discovery only

- video architecture and VideoLab;
- public Wasmtime/WIT extension ecosystem;
- broad universal Resource Graph/Context Guardian/Policy-service refactor;
- large process-topology redesign.

## Canonical resolutions after the pack snapshot

- The terminal and domain extensions remain model-independent.
- Accepted ADR 0029 authorizes the nonactivating D7/CP6 ecosystem and
  selected-input source boundary. Activation and public distribution remain
  gated; selected-input model suggestions receive no tools or workflows.
- Optional cross-extension workflow planning is a separate LO0-LO5 proposal;
  canonical details are in the project
  [LLM Orchestration specification](../docs/LLM-ORCHESTRATION-EXTENSION.md),
  [testing contract](../docs/LLM-ORCHESTRATION-TESTING.md), and proposed
  [ADR 0033](../docs/adr/0033-optional-llm-orchestration-extension.md).

## Real project ADR references

- CP5 authority: **accepted project ADR 0025; activation remains gated**
- bounded top-level tab parked-PTY history: **accepted project ADR 0028**
- public ecosystem/sandbox/selected-input model boundary: **accepted project ADR 0029; activation and public distribution remain gated**
- optional LLM Orchestration boundary: **proposed project ADR 0033**

Proposal/RFD files in this pack deliberately do not reuse the project's ADR number namespace.

## Historical only

Everything in `historical/`.
