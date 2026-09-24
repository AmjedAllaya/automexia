# Native output presentation settings

The command palette now offers Settings with searchable controls for inline
header tables, detected-status highlighting and completion timestamps. Changes
apply to live panes and windows, show their origin and save status, and Reset
inherits the current configuration. The existing external configuration editor
and shortcut customization keep their action identities.

Installed built-in extensions contribute supported controls; disabled features
remain listed and removed extensions disappear. DevOps prompt context can be
controlled separately from output coloring. Installation discovery uses the
existing background worker and cached state.

Preferences use version-2 storage with read-only version-1 import, bounded
validation and the existing writer. The sheet currently covers presentation
controls; broader configuration fields and arbitrary signed-package descriptors
are not implemented by this change. Native platform and assistive-technology
claims require their corresponding runtime checks.
