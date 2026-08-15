# Support

Use GitHub issue forms for reproducible defects, feature proposals, performance
regressions, platform regressions, and upstream-port proposals. Search existing
issues first and include `automexia --version`, OS/build details, relevant
configuration with secrets removed, and minimal reproduction steps.

Security issues belong in private vulnerability reporting; conduct reports use
the private address that must be configured before the public v0.4.0 launch.
General usage questions may use GitHub Discussions once enabled.

The latest v0.4 patch is supported. Once v0.5 ships, v0.4 receives only critical
security fixes for 90 days.

## Windows fullscreen brightness

Automexia uses borderless fullscreen on Windows; it never switches the monitor's
resolution or refresh rate. While a window is fullscreen (F11 or Alt+Enter), it
owns a route-scoped Windows `DisplayRequired` request so idle power management
cannot dim or turn off the display. Leaving fullscreen or closing that exact
window releases the request immediately. Automexia does not request system-wide
wakefulness and does not change the active power plan or brightness percentage.
The Windows GPU surface is also pinned to SDR sRGB so a fullscreen resize cannot
negotiate an HDR output path.

If the physical panel still changes brightness immediately while entering a
mostly dark fullscreen terminal, Windows or the display firmware is applying
content-adaptive brightness/contrast rather than Automexia changing its pixels.
On a supported Windows 11 laptop, open **Settings > System > Display >
Brightness** and set **Change brightness based on content** to **Off** (or **On
battery only**). For an external display, disable Dynamic Contrast, Eco, Local
Dimming, or an equivalent feature in the monitor's on-screen menu. On an HDR
display, also verify Windows' SDR content brightness setting. These controls are
system/display-wide, so Automexia deliberately does not modify them.
