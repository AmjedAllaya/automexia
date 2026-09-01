---
category: Fixed
summary: Restored the canonical cold-build reproducibility contract to the stable-release workflow.
---

The Linux x64 and ARM64 release checks now run the maintained isolated-source
reproducibility script and retain a per-architecture evidence receipt. Runtime
trust mutation coverage rejects a future replacement of that canonical contract.
