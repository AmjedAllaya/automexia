Added

- Windows WSL profiles offer **Check WSL setup** to report shell integration
  and file icon prerequisites. Supported Ubuntu and Debian setups can open APT
  to install `eza` after the user confirms the selected distribution.
- **Install Bash integration** sets up the bundled prompt and command-color
  hooks for the selected WSL distro's default non-root Bash user. Open a new
  Bash session when the popup reports that setup succeeded.
- Opening an ordinary WSL profile in the current window or typing `wsl` at an
  integrated Windows shell can surface the prerequisite warning as a themed
  Automexia popup. Cancel keeps the WSL session open. The `eza` installation
  uses a separate tab.
- The PowerShell popup follows the accepted command, including fast input
  where Enter arrives before the terminal paints the draft.
- Background WSL checks keep their console hidden so Windows Terminal does
  not open a separate tab for the distribution inventory or setup probe.
- Interactive Bash and Zsh sessions in WSL warn once when enhanced listings
  are enabled and `eza` is missing. The installed-hook regression covers normal
  WSL startup, prompt tags, command colors, and file icons.
