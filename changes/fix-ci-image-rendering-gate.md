---
category: Fixed
summary: Restored required PR coverage for QA, concurrent-model, and headless image-rendering assurance.
---

The policy and Rust quality jobs now run the feature-test-reinforcement and
performance-assurance mutations, the QA runner's mutation suite, the
deterministic Loom channel model, and the existing
image-rendering/resource-regression gate. The S2 activation stays manual on the
GitHub-Free/private plan; the free-plan contract explicitly rejects paid-only
protected environments. Controlled native GPU evidence remains a separate manual
assurance path.
