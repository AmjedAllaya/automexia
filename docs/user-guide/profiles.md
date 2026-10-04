# Named Profiles

Open the command palette with Ctrl/Cmd+Shift+P and choose **New Terminal with
Profile**, or open
**Settings → Profiles**. Choose a profile, then **Open in new tab** or **Open in
new window**. Default, Work and Personal start from your configuration; Windows
also offers WSL. The SSH starter uses your system OpenSSH client: edit its
arguments to name your SSH configuration alias before connecting.

Choose **New profile** or **Edit profile** to customize the executable, individual
arguments, operating system, working directory, theme, tab icon and tab color.
Each argument is passed exactly as entered. Do not add shell quoting around
spaces. An empty executable follows the configured shell and arguments.

Use Tab/Shift+Tab to move focus, arrows to select, and Enter to edit or activate.
Ctrl/Cmd+S saves the profile. Escape returns to the previous page; unsaved edits
require a discard confirmation. A saved profile affects future launches.
Deleting a saved override also asks for confirmation.

The directory can follow configuration, inherit the active terminal directory,
use your home directory, or use a fixed absolute local path. For a WSL guest
folder, pass `--cd` and the Linux directory as separate WSL arguments. A missing
local directory prevents launch rather than silently opening elsewhere.

The optional theme is the theme filename without `.toml`, such as `solar-dusk`.
Built-in themes and valid files in the existing themes directory use the same
parser. The focused profile colors the window's terminal, header, footer and
menus. Temporary Theme Gallery previews take priority until you leave the gallery;
Escape restores the profile colors. Selecting an ordinary tab restores its window palette. On macOS, native
tab mode uses the existing system tab group; custom tab icons and accents are
shown by Automexia's own tab bar.

Environment overrides map a destination variable name to an existing environment
variable name. Values are read only for a launch and are never saved in profiles.
Keep passwords, tokens and keys in your agent, vault or system SSH configuration;
do not put them in profile names or arguments. Missing references prevent launch.

**Export TOML** writes one profile to a new file. **Import TOML** validates one
profile and opens an unsaved copy for review. Neither operation launches anything.
The visual editor stores its private document under `profiles/profiles-v1.toml`
inside the Automexia configuration directory. It can also be edited as TOML while
the editor is closed. Refresh reloads it; conflicting saves preserve disk contents.

```toml
version = 1

[[profiles]]
id = "work"
name = "Work"
program = "bash"
args = ["--login"]
platform = "linux"
theme = "aurora-night"
icon = "W"
color = "#64BEFF"

[profiles.directory]
mode = "home"

[[profiles.environment]]
name = "EDITOR"
from = "WORK_EDITOR"
```

To define profiles in `config.toml`, put `version = 1` inside `[profiles]` and
use `[[profiles.profiles]]`, `[profiles.profiles.directory]` and
`[[profiles.profiles.environment]]`. Saved visual records override matching IDs.
Ordinary New Tab continues to use your configured shell; choosing a named profile
is explicit. Recovery continues to restore supported shells and topology without
replaying custom launch arguments or automatically reconnecting remote systems.
