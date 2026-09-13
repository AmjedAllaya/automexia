#!/usr/bin/env python3
"""Read-only, offline gate for the public v0.4.0 archive's user documentation.

Run with --root pointing to a separate public metadata checkout. Live asset
availability, GitHub rendering and repository permissions are separate checks.
No source files, scripts or private evidence are copied into that archive.
"""

from __future__ import annotations

import argparse
import hashlib
from html.parser import HTMLParser
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit


RELEASE_ROOT = "https://github.com/AmjedAllaya/automexia-releases/releases/download/v0.4.0"
PACKAGES = (
    "automexia-terminal_0.4.0-1_amd64.deb",
    "automexia-terminal_0.4.0-1_arm64.deb",
    "automexia-terminal-0.4.0-1.x86_64.rpm",
    "automexia-terminal-0.4.0-1.aarch64.rpm",
    "automexia-terminal-0.4.0-x86_64-unknown-linux-gnu.tar.gz",
    "automexia-terminal-0.4.0-aarch64-unknown-linux-gnu.tar.gz",
)
GUIDES = {"README.md", "INSTALL.md", "VERIFY.md", "GETTING-STARTED.md",
          "UNINSTALL.md", "SUPPORT.md", "SECURITY.md", "DISTRIBUTION.md"}
FILES = GUIDES | {"LICENSE", ".gitignore", ".github/CODEOWNERS",
                 "assets/README.md", "assets/automexia-logo.png"}
LOGO_SHA = "196c25ca8208671b9a19a581de4fae2e849b707b9ee2c4371ae6dc6ac6be015e"
PUBLIC_KEY = "RWTO3NFbh6cxrzSTATcR6SBkp/bHhwCdR48B+G7IS83pkW8XPqVDrNkN"
MAX_TEXT = 64 * 1024


def safe_external_link(target: str) -> bool:
    parsed = urlsplit(target)
    return bool(parsed.scheme == "https" and parsed.netloc and not parsed.username and not parsed.password)


class Markup(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []

    def handle_starttag(self, tag, attrs):
        fields = dict(attrs)
        if tag == "img" and not fields.get("alt", "").strip():
            raise ValueError("image alternative text is required")
        for field in ("href", "src"):
            if fields.get(field):
                self.links.append(fields[field])


def links(text: str) -> list[str]:
    parser = Markup()
    parser.feed(text)
    return parser.links + re.findall(r"\[[^\]]*\]\(([^\s)]+)\)", text)


def read_text(path: Path) -> str:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_TEXT:
        raise ValueError("documentation must be a bounded regular file")
    return path.read_text(encoding="utf-8")


def check_text(name: str, text: str) -> None:
    if re.search(r"BEGIN [A-Z ]*PRIVATE KEY|\b[A-Za-z]:[\\/]|/(?:Users|home)/", text):
        raise ValueError(f"private material or machine path in {name}")
    if "/releases/latest" in text and re.search(r"https?://\S*/releases/latest", text):
        raise ValueError("prerelease download links must be version pinned")
    for target in links(text) + re.findall(r"https?://[^\s<>]+", text):
        parsed = urlsplit(target)
        if parsed.scheme and not safe_external_link(target):
            raise ValueError("external link is not credential-free HTTPS")


def check_links(root: Path, owner: str, text: str) -> None:
    for link in links(text):
        parsed = urlsplit(link)
        if parsed.scheme:
            if not safe_external_link(link):
                raise ValueError("unsafe external link")
            continue
        if parsed.netloc or parsed.path.startswith("/"):
            raise ValueError("absolute local link is forbidden")
        path = root / Path(owner).parent / unquote(parsed.path or Path(owner).name)
        if not path.resolve().is_relative_to(root.resolve()):
            raise ValueError("link escapes metadata root")
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"missing or linked target in {owner}")
        if parsed.fragment:
            headings = re.findall(r"^#{1,6}\s+(.+)$", read_text(path), re.M)
            anchors = {re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-") for heading in headings}
            if unquote(parsed.fragment) not in anchors:
                raise ValueError(f"missing heading target in {owner}")


def check_asset(path: Path, expected: str) -> None:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 1024 * 1024:
        raise ValueError("brand asset must be a bounded regular file")
    if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
        raise ValueError("supplied brand pixels changed")


def check_downloads(text: str) -> None:
    targets = set(links(text))
    if not all(f"{RELEASE_ROOT}/{name}" in targets for name in PACKAGES):
        raise ValueError("all six exact version-pinned package links are required")


def check(root: Path) -> None:
    listing = subprocess.run(
        ["git", "-C", str(root), "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        capture_output=True, check=True, timeout=15,
    )
    if len(listing.stdout) > MAX_TEXT:
        raise ValueError("metadata inventory exceeds limit")
    names = set(listing.stdout.decode("utf-8").strip("\0").split("\0"))
    if names != FILES:
        raise ValueError("public file inventory differs from reviewed metadata allowlist")
    for name in sorted(names - {"assets/automexia-logo.png"}):
        text = read_text(root / name)
        check_text(name, text)
        check_links(root, name, text)
        if name in GUIDES and not re.search(r'<img\s+src="assets/automexia-logo.png"', text):
            raise ValueError(f"missing shared branding in {name}")
    check_asset(root / "assets/automexia-logo.png", LOGO_SHA)
    check_downloads(read_text(root / "README.md"))
    verify = read_text(root / "VERIFY.md")
    if f"-P '{PUBLIC_KEY}'" not in verify or "sha256sum --check" not in verify:
        raise ValueError("verification must bind the pinned key and package checksum")
    # Git checkout line endings can differ; license wording must not.
    if read_text(root / "LICENSE") != read_text(Path(__file__).resolve().parents[2] / "LICENSE"):
        raise ValueError("license differs from shipped attribution")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    args = parser.parse_args()
    try:
        check(args.root)
    except (ValueError, OSError, UnicodeError, subprocess.SubprocessError):
        # Errors must not echo a caller's checkout path or forbidden contents.
        print("FAIL: public metadata contract; inspect the reviewed files locally")
        return 1
    print("PASS: public metadata inventory, branding, links, pinned downloads and key")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
