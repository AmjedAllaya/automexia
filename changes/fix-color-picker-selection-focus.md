# Keep color selection ready for keyboard actions

Choosing a suggested or favorite color now focuses Apply instead of selecting the
hex input. A or Enter applies the draft, Escape cancels, and R requests Reset.
Typing, paste and delayed IME events cannot overwrite a chosen color until the
user explicitly focuses the input. The shared editor preserves this behavior
across customization, theme, profile, extension and tab-color hosts. Regression
coverage includes keyboard and pointer selection, alpha colors, resize, favorites
refresh, repeat protection and native Linux/Windows/macOS input scenarios.
