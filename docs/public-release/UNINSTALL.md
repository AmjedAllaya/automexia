<p><img src="https://raw.githubusercontent.com/AmjedAllaya/automexia-releases/main/assets/automexia-logo.png" width="64" height="64" alt="Automexia logo"></p>

# Remove Automexia Terminal from Linux

Close Automexia and its active work first; use another terminal for removal.
Review the package manager's proposed changes before accepting.

For Debian and Ubuntu packages:

```sh
sudo apt remove automexia-terminal
```

For Fedora and RPM-based packages:

```sh
sudo dnf remove automexia-terminal
```

For the portable archive, stop Automexia and remove only the directory that you
created for that archive. User configuration is intentionally preserved. Remove
it separately only after reviewing and backing up settings that you may want to
reuse.

See the [public removal guide](https://github.com/AmjedAllaya/automexia-releases/blob/main/UNINSTALL.md)
for configuration preservation and separately installed shell integration.
