
# Scenario Evidence Layer

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Thin layer over existing xtask/tests.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Decision
Create Automexia Lab for deterministic and native scenario execution. The Lab owns reusable fixtures for time, processes, PTYs, provider state, network faults, filesystem faults, event replay, and evidence collection.

Release evidence maps:

```text
Feature → Requirement → Test → Evidence → Commit → Platform → Artifact
```

The Lab complements rather than replaces real native/provider testing.
