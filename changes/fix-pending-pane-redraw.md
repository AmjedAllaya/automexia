# Preserve pending pane redraws

- Retain pane damage when rendering is suppressed for an unfocused or hidden
  window, so returning to it refreshes every affected pane without a resize or
  additional shell output.
- Cover suppression, restored panes, visible updates and obsolete routes in the
  shared redraw decision tests.
