Fixed

- Review multiline clipboard text in the target pane before pasting. Preserve
  indentation and blank lines, normalize line endings, and filter terminal
  control characters before preview and delivery. Theme-aware syntax colors
  distinguish command names, flags, quoted text, operators, and comments.
- Use **Bash batch** to contain fail-fast exits in a child Bash and keep the
  interactive shell open after a failed batch. The prepared line also works
  with Bash editors that lack bracketed paste.
- Keep local and SSH Bash prompt integration from exiting a strict-mode shell
  while reporting the status of a command that Bash allows to fail.
