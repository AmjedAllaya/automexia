<p><img src="https://raw.githubusercontent.com/AmjedAllaya/automexia-releases/main/assets/automexia-logo.png" width="64" height="64" alt="Automexia logo"></p>

# Install Automexia Terminal on Linux

Verify `SHA256SUMS.minisig` and `SHA256SUMS` before installing. Obtain the
trusted `RW...` minisign key from the [public verification guide](https://github.com/AmjedAllaya/automexia-releases/blob/main/VERIFY.md) rather than
trusting only the key copied beside the package:

```sh
minisign -Vm SHA256SUMS -P '<trusted Automexia RW... public key>'
sha256sum --check --ignore-missing SHA256SUMS
```

The checksum command must report the downloaded package as `OK`. Stop if the
signature, package checksum, version, architecture, or filename differs.

## Debian and Ubuntu

```sh
# x64
sudo apt install ./automexia-terminal_<version>-1_amd64.deb

# Arm64
sudo apt install ./automexia-terminal_<version>-1_arm64.deb
```

## Fedora and RPM-based distributions

```sh
# x64
sudo dnf install ./automexia-terminal-<version>-1.x86_64.rpm

# Arm64
sudo dnf install ./automexia-terminal-<version>-1.aarch64.rpm
```

## Portable archive

The archive contains files at its root. Create a new, empty directory before
extracting; if this directory exists, stop and choose a different unused name.
Select only the extraction command matching your architecture.

```sh
mkdir automexia-portable
```

```sh
# x64
tar -xzf automexia-terminal-<version>-x86_64-unknown-linux-gnu.tar.gz -C automexia-portable

# Arm64
tar -xzf automexia-terminal-<version>-aarch64-unknown-linux-gnu.tar.gz -C automexia-portable
```

Move the extracted directory only to a location that you own, keep the
accompanying notices, and run its `automexia --version` before launching the
desktop application. The portable archive does not register a system package or
automatic updater.

Use the [public install guide](https://github.com/AmjedAllaya/automexia-releases/blob/main/INSTALL.md)
and [support guide](https://github.com/AmjedAllaya/automexia-releases/blob/main/SUPPORT.md)
for current requirements and limitations. Do not force package dependencies,
bypass local signature policy or run the GUI as root. Website activation is
separate from availability of the version-pinned GitHub assets.
