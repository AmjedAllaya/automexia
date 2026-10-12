# Support

Start with the [FAQ](docs/FAQ.md) and [Troubleshooting](docs/TROUBLESHOOTING.md).
Search existing GitHub issues before filing a report. Include:

- `automexia --version` and the affected source revision or package version;
- OS/build details;
- minimal reproduction steps;
- relevant configuration with secrets removed.

Use issue forms for bugs, feature proposals, performance or platform regressions,
and upstream-port proposals. General questions can use the issue tracker;
GitHub Discussions is not currently enabled.

Report vulnerabilities privately under [Security](SECURITY.md). GitHub's
**Report a vulnerability** form is not currently enabled. Use an established
private maintainer contact or request a private route without disclosing details.
Never post vulnerability details in public issues.

The v0.4.0 Linux Early Access package and current cross-platform source have
different evidence. See [Platforms](docs/PLATFORMS.md). No fixed maintenance
window is promised for older versions.

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
