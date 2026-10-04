Fixed

- Keep Connection Hub responsive when scans and file reviews supersede each
  other. Queue saturation preserves the current operation and reviewed files;
  accepted metadata saves publish before another request can supersede them.
- Bind tag drafts to their connection and revision, prevent section changes
  during editing, and discard unconfirmed drafts when reopening the Hub.
- Support platform paste shortcuts in Hub text fields without submitting or
  sending terminal input. Report clipboard failures and successful command
  copies accurately.
- Show managed connection activation limits before approval, preserve Cancel,
  and document current inventory behavior and external SSH-agent setup.
  Managed launch and direct vault/cloud automation remain unavailable.
