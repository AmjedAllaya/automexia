# Isolate suggestion label drawing inside the renderer

Move plain and matched label drawing into a private sibling of the overlay.
Preserve existing elision, colors, layout, hit targets and activation boundaries.
Literal text and exact prepared-font CPU characterization plus a directly
included label-and-description benchmark make subsequent changes reviewable.
