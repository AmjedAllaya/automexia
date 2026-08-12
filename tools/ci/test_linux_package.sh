#!/usr/bin/env bash
set -euo pipefail

package_dir=${1:?package directory is required}
version=${2:?version is required}
deb=$(find "$package_dir" -maxdepth 1 -type f -name '*.deb' -print -quit)
rpm=$(find "$package_dir" -maxdepth 1 -type f -name '*.rpm' -print -quit)
archive=$(find "$package_dir" -maxdepth 1 -type f -name '*.tar.gz' -print -quit)
test -n "$deb" -a -n "$rpm" -a -n "$archive"

portable_root=$(mktemp -d)
cleanup() {
    sudo apt-get remove -y automexia-terminal >/dev/null 2>&1 || true
    sudo rpm -e automexia-terminal >/dev/null 2>&1 || true
    rm -rf -- "$portable_root"
}
trap cleanup EXIT

tar -xzf "$archive" -C "$portable_root"
portable=$(find "$portable_root" -type f -name automexia -print -quit)
test -n "$portable"
chmod +x "$portable"
"$portable" --version | grep -F "$version"

dpkg-deb --info "$deb" >/dev/null
rpm -qip "$rpm" >/dev/null
rpm2cpio "$rpm" | cpio -t 2>/dev/null | grep -q '/usr/bin/automexia'

sudo apt-get install -y "$deb"
automexia --version | grep -F "$version"
desktop-file-validate /usr/share/applications/automexia-terminal.desktop
appstreamcli validate --no-net /usr/share/metainfo/io.github.AmjedAllaya.AutomexiaTerminal.metainfo.xml
grep -qF 'x-scheme-handler/automexia;' /usr/share/applications/automexia-terminal.desktop
test -f /usr/share/icons/hicolor/scalable/apps/automexia-terminal.svg
infocmp automexia >/dev/null
infocmp xterm-automexia >/dev/null
test -f /usr/share/doc/automexia-terminal/NOTICE.md
test -f /usr/share/doc/automexia-terminal/THIRD_PARTY_NOTICES.md

sudo apt-get remove -y automexia-terminal
if command -v automexia >/dev/null 2>&1; then
    echo 'uninstall left automexia on PATH' >&2
    exit 1
fi

sudo rpm -i --nodeps "$rpm"
/usr/bin/automexia --version | grep -F "$version"
test -f /usr/share/applications/automexia-terminal.desktop
test -f /usr/share/metainfo/io.github.AmjedAllaya.AutomexiaTerminal.metainfo.xml
sudo rpm -e automexia-terminal
test ! -e /usr/bin/automexia

trap - EXIT
rm -rf -- "$portable_root"
echo 'PASS: Linux DEB/RPM install/uninstall, portable smoke, metadata, URL/icon lookup, notices, and terminfo'
