Fixed

- Preserve the information bar and one editable prompt after Ctrl+L in
  integrated Bash, Zsh and Fish, including WSL and fragmented redraws.
- Clear nested CMD through the same shortcut without inserting `^L`; place
  its prompt at the top without changing native input coordinates.
- Forward native key records to PowerShell and foreground programs, and keep
  nested shells from importing older same-numbered prompt context.
- Test actual keyboard clearing on both Windows renderers with isolated shell
  settings; keep native Linux/macOS validation distinct from WSL evidence.
