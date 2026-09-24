---
kind: Fixed
component: shell-integration
---

Keep PowerShell integration in the interactive session after application startup,
preventing repeated fallback prompts and preserving its input, prompt metadata,
completion, and nested CMD helpers. Development and executable launches share
the fix; explicit commands and execution-policy fallback retain native behavior.
