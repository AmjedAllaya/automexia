# Preserve Connection Hub text clusters

Connection Hub label truncation and review-text wrapping now use the shared
grapheme boundary helper, keeping accents, joined emoji and flags intact.
Existing ASCII, whitespace and extra-ellipsis behavior is retained. Wrapping
preserves the exact source bytes, and underlying connection/review data remains
unchanged. Font measurement and pixel fitting remain separate renderer contracts.
