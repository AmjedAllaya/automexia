
# Automexia Lab — Thin Scenario/Evidence Layer

Keep the idea, but build it over existing test owners.

Desired form:

```text
existing xtask/tests/fixtures
        +
small scenario schema
        +
deterministic seed/clock helpers
        +
evidence receipts
        +
replay command
```

Avoid creating a monolithic parallel test framework.

High-value reusable fixtures:
- Process Spy;
- PTY/ConPTY fixture;
- environment poisoning;
- filesystem swap/permission failure;
- network fault harness where needed;
- deterministic clock/IDs;
- privacy canaries.

A scenario run should record:
- scenario hash;
- seed;
- commit;
- platform;
- tool versions;
- artifact/evidence identity.
