# Give responsive fitting one renderer-local owner

The existing end/start width helpers now live in one private renderer module.
Active-font adapters and editable values remain with their current owners.
Characterization preserves the existing whitespace, marker and scalar-width
behavior. A directly included helper benchmark supports same-host comparisons
without exposing a new public runtime API.
