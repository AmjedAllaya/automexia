#!/usr/bin/env bash
set -euo pipefail

package_dir=${1:?package directory is required}
version=${2:?version is required}
package_dir=$(realpath "$package_dir")
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
portable_dir=$(dirname "$portable")
for name in automexia amx automexia-suggestion-helper; do
    test -x "$portable_dir/$name"
done
"$portable" --version | grep -F "$version"
"$portable_dir/amx" --version | grep -F "$version"

dpkg-deb --info "$deb" >/dev/null
rpm -qip "$rpm" >/dev/null
# Query the RPM manifest directly. Extracting it through rpm2cpio/cpio under
# pipefail is needlessly fragile and has produced intermittent false failures.
rpm -qpl "$rpm" | grep -Fx '/usr/bin/automexia' >/dev/null
for name in amx automexia-suggestion-helper; do
    rpm -qpl "$rpm" | grep -Fx "/usr/bin/$name" >/dev/null
done

check_installed_runtime() {
    for name in automexia amx automexia-suggestion-helper; do
        test -x "/usr/bin/$name"
    done
    /usr/bin/automexia --version | grep -F "$version"
    /usr/bin/amx --version | grep -F "$version"
}

check_uninstalled_runtime() {
    for name in automexia amx automexia-suggestion-helper; do
        if test -e "/usr/bin/$name"; then
            echo "Uninstall left /usr/bin/$name behind" >&2
            exit 1
        fi
    done
}

sudo apt-get install -y "$deb"
check_installed_runtime
desktop-file-validate /usr/share/applications/automexia-terminal.desktop
appstreamcli validate --no-net /usr/share/metainfo/io.github.AmjedAllaya.AutomexiaTerminal.metainfo.xml
grep -qF 'x-scheme-handler/automexia;' /usr/share/applications/automexia-terminal.desktop
test -f /usr/share/icons/hicolor/512x512/apps/automexia-terminal.png
infocmp automexia >/dev/null
infocmp xterm-automexia >/dev/null
test -f /usr/share/doc/automexia-terminal/NOTICE.md
test -f /usr/share/doc/automexia-terminal/THIRD_PARTY_NOTICES.md

sudo apt-get remove -y automexia-terminal
check_uninstalled_runtime

sudo rpm -i --nodeps "$rpm"
check_installed_runtime
test -f /usr/share/applications/automexia-terminal.desktop
test -f /usr/share/metainfo/io.github.AmjedAllaya.AutomexiaTerminal.metainfo.xml
sudo rpm -e automexia-terminal
check_uninstalled_runtime

trap - EXIT
rm -rf -- "$portable_root"
echo 'PASS: Linux DEB/RPM install/uninstall, portable smoke, metadata, URL/icon lookup, notices, and terminfo'
