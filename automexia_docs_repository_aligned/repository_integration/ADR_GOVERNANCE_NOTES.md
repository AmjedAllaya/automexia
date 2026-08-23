
# ADR Governance Notes

Proposal files from earlier research used ADR numbers `0001–0015`. Those numbers conflict with the real project namespace and must not be imported.

## Rules

1. The real repository assigns ADR numbers.
2. Research notes use `RFD`/descriptive names only.
3. An implementation agent may draft a proposed decision, but must not create its own governance authority by marking a new security/lifecycle decision Accepted.
4. Existing real ADRs win over proposal duplicates.

Known real owners from the supplied repository audit:

```text
ADR 0025 → proposed CP5 editor bridge
ADR 0028 → accepted bounded top-level tab parked-PTY undo/redo
ADR 0029 → proposed non-activating public ecosystem/sandbox/AI boundary
```

When a research note contains useful material:
- extract the delta;
- apply it to the existing owner;
- preserve repository history;
- do not replace the owner wholesale.
