"""Context wire admission guards; compiled streaming tests prove behavior."""
from pathlib import Path
import re

SOURCES = {
    "contract": "automexia-extension-api/src/lib.rs",
    "decoder": "automexia-extension-api/src/context_decode.rs",
    "tests": "automexia-extension-api/tests/context_contributions.rs",
    "benchmark": "automexia-extension-api/benches/context_contributions.rs",
    "manifest": "automexia-extension-api/Cargo.toml",
}


def validate_sources(sources: dict[str, str]) -> None:
    def code(owner: str) -> str:
        return re.sub(r"\s+", "", re.sub(r"//[^\n]*", "", sources[owner]))

    contract = code("contract")
    wire = contract.split("structContextContributionWire{", 1)[-1].split("implTryFrom<ContextContributionWire>", 1)[0]
    if '#[serde(deserialize_with="context_decode::segments")]segments:Vec<StatusSegment>' not in wire:
        raise ValueError("context wire lost early-admission dispatch")
    required = {
        "contract": ("MAX_STATUS_SEGMENTS:usize=64;", "modcontext_decode;"),
        "decoder": ("whileitems.len()<MAX_STATUS_SEGMENTS", "sequence.next_element_seed(RejectExcess)?", "ContractError::TooMany", "maximum:MAX_STATUS_SEGMENTS", "letmutitems=Vec::new()"),
        "tests": ("fncontext_wire_rejects_before_decoding_an_excess_element()", "reader.consumed<=prefix.len()+1", "fncontext_wire_preserves_exact_count_boundaries_and_values()", "fncontext_typed_constructor_retains_the_same_limit()", "fictional-tail-"),
        "benchmark": ("BenchmarkId::new(\"decode-drop\",count)", "assert_eq!(decoded.segments.len(),count)", "reject-excess-1mib-tail", "drop(value)", "criterion_main!(benches)"),
        "manifest": ('name="context_contributions"harness=false',),
    }
    for owner, tokens in required.items():
        if any(token not in code(owner) for token in tokens):
            raise ValueError(f"context {owner} lost admission or evidence boundary")
    decoder = code("decoder")
    if any(token in decoder for token in ("size_hint(", "Vec::deserialize", "std::fs", "std::net", "std::process", "Command::new", "rio_vt", "automexia_devops")):
        raise ValueError("context admission acquired unbounded allocation or unrelated authority")


def validate_repository(root: Path) -> None:
    try:
        sources = {owner: (root / path).read_text(encoding="utf-8") for owner, path in SOURCES.items()}
    except (OSError, UnicodeError) as error:
        raise ValueError("context admission source unavailable") from error
    validate_sources(sources)
