# Color picker keyboard navigation

The shared color picker visits Suggested, Favorites and Save/Remove favorite
before its bottom action buttons. Visible F1/F2/F3 shortcuts open the palettes
and manage the draft favorite. Entering a palette keeps Tab, Shift+Tab and arrows
inside its colors until Escape returns to its button or a color is chosen.
Selection remains a draft until Apply. Empty favorites, list refreshes, repeated
keys and compact-window focus recovery are covered by regression tests.
