# Bundle Validation

**Scope:** generated documentation pack only; not source/native/release certification.

**Last verified:** 2026-08-26 against the frozen source baseline below and the
later canonical-resolution links.

- Manifest entries: 61
- Non-historical entries: 57
- Historical entries: 4
- Broken local Markdown links: 0
- Blocking documentation issues: 0
- Exact duplicate non-historical groups: 0
- Research master copies: 1
- Proposal ADR namespace collisions: none
- Audited committed baseline pinned to `20ff7928ea2d1eac5d26f13c62d0cda9b86bc078`: yes
- G6 top-level accepted/implemented state reflected: yes
- Broader topology history marked future: yes
- Frozen-snapshot ADR 0025/0028/0029 state reflected: yes
- Current accepted-but-nonactivated ADR 0025/0029 resolutions reflected: yes
- Later canonical ADR 0033 optional-orchestration resolution linked: yes
- Video marked future/deferred: yes
- Large LLM / paid API required by core or a domain extension: no

Validation commands:

```text
python tools/ci/test_documentation_hygiene.py
python tools/ci/test_repository_aligned_docs.py
python tools/ci/check_repository_aligned_docs.py
python tools/ci/validate_repository.py
```
