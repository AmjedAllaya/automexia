# Situation-aware Production Operations contracts

Status: planned public contract summary; no production-operation contract is
active.

## Stable principles

Future contracts must preserve these externally reviewable rules:

- facts identify their source, freshness, scope, and collection outcome;
- suggestions identify their target, reason, confidence, prerequisites, risk,
  and missing evidence;
- read-only investigation and mutating action authority are separate;
- provider credentials are referenced through approved external custody and are
  never placed in completion items, persistence, or ordinary logs;
- a suggestion cannot execute itself, type into a PTY, or imply approval;
- every mutating action is rendered from a typed request into an exact
  executable-and-argument review;
- stale context, changed targets, insufficient authority, or expired evidence
  invalidates the action before execution;
- every pane, tab, environment, session, and evidence generation stays isolated;
- limits, cancellation, disable, uninstall, and recovery behavior are explicit.

## Compatibility

The public interface should remain provider-neutral and versioned. Provider
adapters may add typed evidence and actions only through reviewed capability
boundaries. Missing adapters must degrade to ordinary terminal and native-tool
workflows.

Detailed schemas, scoring fields, provider mappings, and wire formats remain
internal until the implementation is ready for public compatibility review.

## Exact neutral records

Any future public record contract must be provider-neutral, versioned, bounded,
strictly validated, freshness-aware, and safe to ignore when unsupported. This
heading preserves compatibility for assurance references; the exact unpublished
record catalog remains outside the public repository.
