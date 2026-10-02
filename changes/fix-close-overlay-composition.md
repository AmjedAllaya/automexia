Close confirmation now replaces covered overlay graphics and labels, keeping its
card and buttons opaque over Customizations, Settings, search, and other panels.
Cancel preserves the covered editor. The Welcome screen uses the same dialog,
and covered tab-name editors and hints no longer intercept confirmation input.

Regression coverage includes modal text replacement, repeated open/cancel,
keyboard priority, and native Windows card/button pixel checks with GPU and CPU
renderers. Native macOS and Linux confirmation rendering remains unverified.
