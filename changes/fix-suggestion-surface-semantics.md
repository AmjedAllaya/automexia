# Bounded full suggestion semantics

Suggestion labels reuse the shared whitespace-preserving grapheme helper.
Visually shortened labels retain full bounded accessibility and insertion text;
generated ellipses cannot inherit candidate match highlighting. Projection rejects
over-limit presentation input before copying it. Keyboard and pointer acceptance
retain exact editor replacement bytes and do not execute a command or activate
the disabled preview.
