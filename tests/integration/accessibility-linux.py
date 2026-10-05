#!/usr/bin/env python3
"""Native AT-SPI probe. Run inside an isolated dbus-run-session and Xvfb.

Requires a visual-test-hooks binary, python3-pyatspi and xdotool. This exercises
the real application/provider and CPU renderer; it does not certify Orca, a
Wayland compositor or delivery from a real OS IME service. Never log native text.
"""
from __future__ import annotations

import argparse
from collections import deque
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import time

import pyatspi
from gi.repository import Atspi, GLib


FIXTURE = "AX_FIXTURE \u4e2d\u6587 e\u0301 \u0645\u0631\u062d\u0628\u0627 \u05e9\u05dc\u05d5\u05dd \U0001f469\u200d\U0001f4bb"
PREEDIT = "\u65e5\u672c e\u0301 \U0001f469\u200d\U0001f4bb"
HIDDEN = "AX_HIDDEN_CANARY"
LIMIT = 1024


class ProbeFailure(Exception):
    pass


class ReplacedTree(Exception):
    """A native object disappeared while a frame replaced its modal subtree."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ProbeFailure(message)


def command(args: list[str]) -> str:
    result = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                            timeout=5, check=False)
    require(result.returncode == 0 and len(result.stdout) <= 65536,
            "Native fixture command failed")
    return result.stdout.decode("utf-8", errors="strict")


def owned_application(pid: int):
    desktop = pyatspi.Registry.getDesktop(0)
    require(desktop.childCount <= LIMIT, "AT-SPI desktop exceeded the fixture bound")
    for index in range(desktop.childCount):
        app = desktop.getChildAtIndex(index)
        if app is not None and app.get_process_id() == pid:
            return app
    return None


def read_owned(app) -> list[dict]:
    pending = deque([app])
    nodes = []
    text_bytes = 0
    while pending:
        require(len(nodes) + len(pending) <= LIMIT, "AT-SPI tree exceeded the node bound")
        node = pending.popleft()
        states = node.getState()
        if states.contains(pyatspi.STATE_DEFUNCT):
            raise ReplacedTree()
        entry = {"name": node.name, "role": node.getRole(), "text": "", "bounds": None,
                 "enabled": states.contains(pyatspi.STATE_ENABLED),
                 "sensitive": states.contains(pyatspi.STATE_SENSITIVE)}
        interfaces = node.get_interfaces()
        if "Text" in interfaces:
            text = node.queryText()
            require(0 <= text.characterCount <= 65536, "AT-SPI text exceeded the bound")
            entry["text"] = text.getText(0, text.characterCount)
        if "Component" in interfaces:
            entry["bounds"] = node.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
        text_bytes += len(entry["name"].encode()) + len(entry["text"].encode())
        require(text_bytes <= 262144, "AT-SPI aggregate text exceeded the fixture bound")
        require(HIDDEN not in entry["text"] and HIDDEN not in entry["name"],
                "Concealed terminal text reached the native accessibility tree")
        nodes.append(entry)
        count = node.childCount
        if count == -1 and node.getState().contains(pyatspi.STATE_DEFUNCT):
            raise ReplacedTree()
        require(0 <= count <= LIMIT - len(nodes) - len(pending),
                f"AT-SPI child count outside bound (children={count}, visited={len(nodes)}, pending={len(pending)})")
        for index in range(count):
            child = node.getChildAtIndex(index)
            if child is not None:
                pending.append(child)
    return nodes


def wait_for(process, predicate, message: str, repaint=None):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        require(process.poll() is None, "Fixture application exited before verification")
        app = owned_application(process.pid)
        if app is not None:
            try:
                nodes = read_owned(app)
                if predicate(nodes):
                    return nodes
            except ReplacedTree:
                # Native object queries are not an atomic frame snapshot. Retry
                # only explicitly defunct nodes within the original deadline;
                # malformed data, warnings and oversize trees still fail.
                pass
            except GLib.Error as error:
                # libatspi reports an object retired during an asynchronous
                # subtree replacement with this specific error. All other
                # native errors still fail and retain private diagnostics.
                if not (error.domain == "atspi_error" and error.code == 0 and
                        error.message == "The application no longer exists"):
                    raise
        if repaint is not None:
            repaint()
        time.sleep(0.05)
    raise ProbeFailure(message)


def named(nodes, name):
    return [node for node in nodes if node["name"] == name]


def run(binary: Path, diagnostics: Path | None) -> None:
    require(binary.is_file() and os.access(binary, os.X_OK), "Test binary is unavailable")
    require(bool(os.environ.get("DBUS_SESSION_BUS_ADDRESS")) and bool(os.environ.get("DISPLAY")),
            "Run this fixture inside a private session bus and Xvfb")
    require(os.environ.get("GSETTINGS_BACKEND") == "memory",
            "Start the private session bus with GSETTINGS_BACKEND=memory to isolate accessibility settings")
    native_warnings = [0]

    def provider_warning(_domain, _level, _message, _data):
        native_warnings[0] = min(LIMIT, native_warnings[0] + 1)

    GLib.log_set_handler("dbind", GLib.LogLevelFlags.LEVEL_WARNING | GLib.LogLevelFlags.LEVEL_CRITICAL,
                         provider_warning, None)
    Atspi.set_timeout(2000, 5000)
    command(["gdbus", "call", "--session", "--dest", "org.a11y.Bus",
             "--object-path", "/org/a11y/bus", "--method", "org.freedesktop.DBus.Properties.Set",
             "org.a11y.Status", "IsEnabled", "<true>"])
    with tempfile.TemporaryDirectory(prefix="automexia-atspi-") as temporary:
        root = Path(temporary)
        control = root / "control"
        control.write_text("", encoding="utf-8")
        child = root / "fixture.py"
        # No interactive shell or command echo can satisfy the output oracle.
        child.write_text("import sys\nprint(" + repr(FIXTURE) + ", flush=True)\n"
                         "print('\\x1b[8m" + HIDDEN + "\\x1b[0m', flush=True)\n"
                         "for line in sys.stdin: pass\n", encoding="utf-8")
        (root / "config.toml").write_text(
            "confirm-before-quit = false\nworking-dir = " + json.dumps(str(root)) +
            "\n[shell]\nprogram = '/usr/bin/python3'\nargs = [" + json.dumps(str(child)) +
            "]\n[renderer]\nuse-cpu = true\n[session-recovery]\nenabled = false\n",
            encoding="utf-8")
        env = dict(os.environ, AUTOMEXIA_CONFIG_HOME=str(root),
                   AUTOMEXIA_NATIVE_TEST_CONTROL=str(control),
                   AUTOMEXIA_VISUAL_TEST_FIXTURE="s1-standard-v1",
                   WINIT_UNIX_BACKEND="x11", RUST_LOG="off")
        # Explicitly select the isolated X server over a host Wayland socket.
        env.pop("WAYLAND_DISPLAY", None)
        process = subprocess.Popen([str(binary)], cwd=root, env=env,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                                   start_new_session=True)
        diagnostic_bytes = bytearray()

        def drain_stderr():
            with process.stderr as stream:
                while chunk := stream.read(4096):
                    diagnostic_bytes.extend(chunk[:max(0, 65536 - len(diagnostic_bytes))])

        reader = threading.Thread(target=drain_stderr, name="atspi-fixture-stderr")
        reader.start()
        try:
            wait_for(process, lambda nodes: len(named(nodes, "Terminal output")) == 1,
                     "Native AT-SPI terminal did not activate")
            handles = command(["xdotool", "search", "--pid", str(process.pid)]).splitlines()
            require(len(handles) == 1 and handles[0].isdigit(), "Fixture window ownership is ambiguous")
            handle = handles[0]
            command(["xdotool", "windowfocus", "--sync", handle])
            frame = 0

            def repaint():
                nonlocal frame
                frame += 1
                command(["xdotool", "windowsize", handle, str(960 + frame % 2), "700"])

            def send(action):
                next_control = root / "control.next"
                next_control.write_text(action, encoding="utf-8")
                next_control.replace(control)
                repaint()

            wait_for(process, lambda nodes: any(FIXTURE in node["text"] for node in nodes),
                     "Native AT-SPI text ranges lost Unicode output", repaint)
            send("ime-preedit-hex:composition:" + PREEDIT.encode().hex())
            wait_for(process, lambda nodes: any(node["text"] == PREEDIT for node in
                                               named(nodes, "Input method composition")),
                     "Native AT-SPI composition lost Unicode text", repaint)
            send("ime-cancel:composition")
            wait_for(process, lambda nodes: not named(nodes, "Input method composition"),
                     "Cancelled composition remained accessible", repaint)
            send("open-palette:palette")
            palette = wait_for(process, lambda nodes: bool(named(nodes, "Search commands")) and
                               not named(nodes, "Terminal output"),
                               "Native AT-SPI modal isolation failed", repaint)
            require(sum(node["role"] == pyatspi.ROLE_LIST_BOX for node in palette) == 1,
                    "Native AT-SPI palette lacks a unique selection container")
            options = [node for node in palette if node["role"] == pyatspi.ROLE_LIST_ITEM]
            require(bool(options), "Native AT-SPI palette lacks individual options")
            require(all(node["bounds"] is not None and node["bounds"].width > 0 and
                        node["bounds"].height > 0 for node in options),
                    "Native AT-SPI options lack painted bounds")
            send("dismiss-modal:palette")
            wait_for(process, lambda nodes: bool(named(nodes, "Terminal output")) and
                     not named(nodes, "Search commands"), "Terminal did not return after modal close", repaint)
            send("open-customizations:settings")
            settings = wait_for(process, lambda nodes: bool(named(nodes, "Search settings")) and
                                not named(nodes, "Terminal output"),
                                "Native AT-SPI settings projection failed", repaint)
            require(sum(node["role"] == pyatspi.ROLE_PUSH_BUTTON for node in settings) >= 3,
                    "Native AT-SPI settings were flattened into a summary")
            restore = named(settings, "Restore saved")
            require(len(restore) == 1 and not restore[0]["enabled"] and not restore[0]["sensitive"],
                    "Native AT-SPI reports an unavailable action as enabled")
        finally:
            # Only this test's new process group can be signalled. Reap it before
            # removing its private config; never enumerate or kill user sessions.
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
            reader.join(timeout=5)
            require(not reader.is_alive(), "Fixture diagnostic reader did not close")
            if diagnostics is not None:
                # Explicit private output only; success/failure console reports
                # never contain native diagnostics or identifying shell data.
                with diagnostics.open("xb") as output:
                    failure = sys.exc_info()[1]
                    detail = ("\nFixture exception: " + repr(failure)).encode("utf-8")[:8192] if failure else b""
                    output.write(diagnostic_bytes[:65536 - len(detail)] + detail)
        deadline = time.monotonic() + 5
        while owned_application(process.pid) is not None and time.monotonic() < deadline:
            time.sleep(0.05)
        require(owned_application(process.pid) is None, "Closed fixture remained on the accessibility bus")
        require(native_warnings[0] == 0, "Native AT-SPI client rejected provider data")
    print("PASS: native Linux AT-SPI Unicode, concealed text, IME projection, modal controls, bounds and teardown")
    print("Orca, real OS IME, Wayland and physical DPI usability remain separate evidence requirements.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--diagnostics", type=Path, help="New private log path (at most 64 KiB)")
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(ProbeFailure("Fixture timed out")))
    try:
        run(args.binary.resolve(strict=True), args.diagnostics)
    except (ProbeFailure, OSError, subprocess.SubprocessError, RuntimeError):
        # Native errors may include private paths/text. Preserve the stage-only
        # assertion message, never arbitrary provider exception details.
        error = sys.exc_info()[1]
        print(str(error) if isinstance(error, ProbeFailure) else "Native AT-SPI fixture failed", file=sys.stderr)
        sys.exit(1)
