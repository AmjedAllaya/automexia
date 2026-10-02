Release cached renderer resources when a pane or tab closes. Background tabs,
inactive pane-local tabs, and parked undo sessions retain their own renderers
until the existing session owner releases them.
