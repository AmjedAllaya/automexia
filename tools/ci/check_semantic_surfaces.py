"""Source ownership guards complementing the compiled surface behavior tests."""
from __future__ import annotations

from pathlib import Path
import re

SOURCES = {
    "contract": "automexia-extension-api/src/surface/mod.rs",
    "bounds": "automexia-extension-api/src/surface/bounds.rs",
    "host": "apps/automexia-terminal/src/automexia/semantic_surfaces.rs",
    "benchmark": "automexia-extension-api/benches/semantic_surfaces.rs",
}


def validate_sources(sources: dict[str, str]) -> None:
    def code(owner: str) -> str:
        return re.sub(r"\s+", "", re.sub(r"//[^\n]*", "", sources[owner]))

    contract, bounds, host = (code(owner) for owner in ("contract", "bounds", "host"))
    required = {
        "contract": ["MAX_SURFACE_FRAME_BYTES:usize=8*1024*1024;", "MAX_TABLE_ROWS:usize=20_000;", "MAX_TABLE_COLUMNS:usize=64;", "MAX_TABLE_CELLS:usize=320_000;", "MAX_TABLE_TEXT_BYTES:usize=4*1024*1024;", "List<TableRow,MAX_TABLE_ROWS,MAX_TABLE_CELLS,MAX_TABLE_TEXT_BYTES>", "duplicate row identifier", "duplicate column identifier"],
        "bounds": ["whileitems.len()<N", "seq.next_element_seed(RejectElement)?", "::charge(&mutcost,&item)", "C.saturating_sub(total.cells)", "B.saturating_sub(total.text)"],
        "host": ["frame.len()>MAX_SURFACE_FRAME_BYTES", "grant.capability!=Capability::UiOverlay", "grant.resource!=ResourceScope::Session", "grant.decision==Decision::Deny", "grant.extension_id!=binding.extension", "grant.session_id!=binding.session", "grant.operation_id!=binding.operation", "grant.capsule_revision!=binding.capsule_revision", "now_ms<grant.decided_at_ms", "now_ms>=grant.expires_at_ms", "now_ms<self.last_observed_ms", "now_ms>=self.expires_at_ms", "self.once&&self.consumed", "update.binding()!=&self.binding", "update.surface()!=self.surface", "update.generation()!=self.generation", "update.revision()<=self.revision", "self.active(now_ms).ok()?", "self.snapshot=None", "map_err(|_|AdmissionError::InvalidFrame)"],
    }
    for owner, tokens in required.items():
        for token in tokens:
            if re.sub(r"\s+", "", token) not in code(owner):
                raise ValueError(f"semantic surface {owner} lost a required boundary")
    for token in ("implfmt::DebugforSemanticTable", '.field("rows",&self.rows().len())', '"(<identifier>)"'):
        if token not in contract:
            raise ValueError("semantic surface diagnostics lost bounded redaction")
    for token in ("construct_validate_drop", "BatchSize::PerIteration", "SamplingMode::Flat", "bounded_summary"):
        if token not in code("benchmark"):
            raise ValueError("semantic surface benchmark lost isolated validation or bounded diagnostics")
    sampling_modes = re.findall(r"sampling_mode\(SamplingMode::(\w+)\)", code("benchmark"))
    if not sampling_modes or any(mode != "Flat" for mode in sampling_modes):
        raise ValueError("semantic surface benchmark sampling must remain bounded for every group")
    if host.index("frame.len()>MAX_SURFACE_FRAME_BYTES") > host.index("serde_json::from_slice(frame)"):
        raise ValueError("semantic surface framing must precede decoding")
    if "size_hint()" in bounds or "Vec::deserialize" in contract + bounds:
        raise ValueError("semantic surface decoding must not trust sequence allocation hints")
    for value in (contract, bounds, host):
        if any(token in value for token in ("std::fs", "std::net", "std::process", "Command::new", "automexia_devops", "rio_vt", "teletypewriter")):
            raise ValueError("semantic surface data/admission acquired unrelated authority")


def validate_repository(root: Path) -> None:
    try:
        sources = {owner: (root / path).read_text(encoding="utf-8") for owner, path in SOURCES.items()}
    except (OSError, UnicodeError) as error:
        raise ValueError("semantic surface source unavailable") from error
    validate_sources(sources)
