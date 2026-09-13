# Platforms and accessibility

## Platform behavior

Automexia uses native PTY/process, window, clipboard, input, packaging, and
accessibility adapters while sharing terminal state and public UI models.

A cross-compile proves portability only. Native claims require the exact
operating system, architecture, package, shell, renderer, display, and workflow
to run.

See [Platforms](../PLATFORMS.md).

## Keyboard and focus

Every public surface is reachable without a pointer, has visible focus, and
restores focus after dismissal. Overlay input does not reach the PTY.

## Semantics

Public UI state defines roles, names, values, states, relationships, focus order,
selection, errors, and live announcements independently of pixels. Native
adapters translate that state for the platform accessibility API.

## Visual access

Test light, dark, high-contrast, reduced-motion, long/localized text, Unicode,
IME, and 100–300% scale across small through high-resolution viewports. Meaning
must not rely on color or animation alone.

## Native evidence

Automated tree/event checks are necessary but do not replace current
Narrator/NVDA, VoiceOver, and Orca review for releases claiming those
environments.

Record exact versions, scenarios, first failure, redacted evidence, and cleanup.
Missing native evidence remains external.

See [Accessibility](../ACCESSIBILITY.md) and
[Manual testing](../MANUAL-FEATURE-TESTING.md).
