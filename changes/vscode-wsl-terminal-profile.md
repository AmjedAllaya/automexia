Added

- Add an **Automexia** profile to the VS Code Terminal panel on Windows,
  macOS, Linux, and WSL. The profile uses the window's default shell and its
  startup arguments.
- Use normal shell command lookup for `amx` and `automexia`. Configure a
  machine-specific binary directory for a CLI installation outside `PATH`.
- Add a docked Automexia preview with native shell sessions through a matching
  `automexia-vscode-host` companion. Open it with
  **Automexia: Open Docked View**.
- Add an information bar and dynamic tags to each docked terminal pane with
  the updated companion. The bar shows the shell's working directory when
  reported, its launch directory otherwise, and tags for the shell context
  and Git state from repositories known to VS Code.
- Show the docked shell's exit status when available and preserve its final
  output so startup failures can be diagnosed in the pane.
- Fix Windows PowerShell 5.1 startup in the docked view when its executable
  has an equivalent standard Windows path.
- Include the shared desktop information-bar presets in the docked panel.
- Browse saved Quick Actions and Workspaces through Automexia's existing CLI
  catalogs from the VS Code Command Palette or docked panel.
- Open the existing Automexia desktop app from VS Code in a separate window.
- Retry context updates when the webview temporarily rejects delivery.
- Assemble platform VSIXs with the native companion and desktop runtime.
  Select the bundled tools automatically in the absence of user overrides.
- Simplify the docked tabs, toolbar, and pane headers. Keep the working
  directory beside the compact tags when a pane has room.
