# Documentation anchor evidence

Documentation checks no longer accept headings inside fenced examples as
reference evidence. Repeated headings and naturally suffixed titles receive
distinct anchors. Independent fixtures cover both collision orders and actual
missing-reference rejection by the production consumers.

Repeated references parse a target only once per repository validation. A new
validation reads current bytes, so this cache cannot conceal stale references.
