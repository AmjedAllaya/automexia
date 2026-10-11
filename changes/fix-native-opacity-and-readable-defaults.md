Fixed

- Keep the terminal body opaque by default from its first clear through restart,
independently of theme background alpha. Reject malformed opacity values safely,
reserve Linux GPU transparency support for live edits, and preserve intentional
saved opacity. Default light and dark tab surfaces remain readable when the body
is translucent. Correct double alpha multiplication in Vulkan and Metal terminal
cell backgrounds and cursors.

- Add native default/theme/opacity/reset/restart scenarios and independent
black/white-backdrop pixel checks for settings cards and Linux compositing.
Isolate native fixtures from inherited preferences and verify cleanup after both
successful and failed runs. Windows and macOS native CI retain their independent
renderer checks.

- Keep dim prompt text in every built-in dark theme above the existing text-contrast requirement.

- Use the shared non-sRGB WGPU surface selection on macOS so already-encoded
  theme colors are not brightened by a second transfer curve. Native appearance
  scenarios continue to check actual default, reset and restart pixels.
