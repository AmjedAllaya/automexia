
# Future Extension Capabilities

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Future public ecosystem.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Context

Boolean permissions that grant unrestricted network or process execution are too broad for an extensible terminal that handles credentials and production resources.

## Decision

Extensions receive scoped capabilities, not generic execution/network authority.

Examples:

```text
resources.read(kind/scope)
managed_tool.execute(tool-id)
browser.open
loopback.listen
http.connect(domain-set)
credentials.ssh_sign
credentials.cloud_session_handle
ui.overlay(kind)
```

Raw credential material, arbitrary process execution, raw keyboard observation, editor-buffer access, and unrestricted networking remain denied by default.

All managed external executables are launched through ProcessBroker using an approved executable identity and structured arguments. The same rule applies to DevOps tools, FFmpeg, database CLIs, and future extensions.
