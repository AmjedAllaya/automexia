#!/usr/bin/env bash
set -euo pipefail

package_dir=${1:?package directory is required}
version=${2:?version is required}
deb=$(find "$package_dir" -maxdepth 1 -type f -name '*.deb' -print -quit)
rpm=$(find "$package_dir" -maxdepth 1 -type f -name '*.rpm' -print -quit)
test -n "$deb" -a -n "$rpm"

dpkg-deb --info "$deb" >/dev/null
rpm -qip "$rpm" >/dev/null
rpm2cpio "$rpm" | cpio -t 2>/dev/null | grep -q '/usr/bin/automexia'

sudo apt-get install -y "$deb"
trap 'sudo apt-get remove -y automexia-terminal >/dev/null 2>&1 || true' EXIT
automexia --version | grep -F "$version"
desktop-file-validate /usr/share/applications/automexia-terminal.desktop
appstreamcli validate --no-net /usr/share/metainfo/io.github.AmjedAllaya.AutomexiaTerminal.metainfo.xml
infocmp automexia >/dev/null
infocmp xterm-automexia >/dev/null
test -f /usr/share/doc/automexia-terminal/NOTICE.md
test -f /usr/share/doc/automexia-terminal/THIRD_PARTY_NOTICES.md

sudo apt-get remove -y automexia-terminal
trap - EXIT
if command -v automexia >/dev/null 2>&1; then
    echo 'uninstall left automexia on PATH' >&2
    exit 1
fi
echo 'PASS: Linux package structure, install, metadata, terminfo, and uninstall'
