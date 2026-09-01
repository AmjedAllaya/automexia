# Install Automexia Terminal on Linux

Verify `SHA256SUMS.minisig` and `SHA256SUMS` before installing. Obtain the
trusted `RW...` minisign key from the Automexia release-trust page rather than
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
sudo apt install ./automexia-terminal_<version>_amd64.deb

# Arm64
sudo apt install ./automexia-terminal_<version>_arm64.deb
```

## Fedora and RPM-based distributions

```sh
# x64
sudo dnf install ./automexia-terminal-<version>-1.x86_64.rpm

# Arm64
sudo dnf install ./automexia-terminal-<version>-1.aarch64.rpm
```

## Portable archive

```sh
# x64
tar -xzf automexia-terminal-<version>-x86_64-unknown-linux-gnu.tar.gz

# Arm64
tar -xzf automexia-terminal-<version>-aarch64-unknown-linux-gnu.tar.gz
```

Move the extracted directory only to a location that you own, keep the
accompanying notices, and run its `automexia --version` before launching the
desktop application. The portable archive does not register a system package or
automatic updater.

See the project website's release-trust guide for signature verification,
supported environments, recovery, and known Early Access limitations.
