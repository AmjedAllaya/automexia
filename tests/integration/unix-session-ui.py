#!/usr/bin/env python3
"""Native Linux/macOS launch, PTY, features and UI regression scenarios.

Linux runs under isolated Xvfb or isolated Sway (CI pins lavapipe). macOS reuses the owned AX/Quartz
driver and requires preauthorized Accessibility access. macOS pixels are checked
by the separate controlled-raster suite; real display scaling is recorded, not forced.
Only synthetic commands run in temporary homes. Reports exclude terminal text.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager, ExitStack
import errno
import json
import math
import os
from pathlib import Path
import signal
import socket
import shutil
import sys
import subprocess
import tempfile
import time


MACOS = sys.platform == "darwin"
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "tools/renderer-benchmarks/application"))
from macos_driver import MacDriver
from benchmark_model import BenchmarkError


class Failure(RuntimeError):
    pass


class Cancelled(BaseException):
    """Abort the matrix while still unwinding the owned child cleanup."""


def cancel(signum, frame):
    raise Cancelled()


def call(*args: str) -> str:
    return subprocess.run(args, check=True, capture_output=True, text=True,
                          timeout=10).stdout.strip()


def active(state: dict) -> dict:
    return next((panel for panel in state.get("panels", []) if panel.get("active")), {})


def integrated_ready(state: dict) -> bool:
    # OSC boundaries and CWD may arrive in separate PTY reads. An integration
    # marker alone does not mean startup metadata has reached the live grid.
    panel = active(state)
    return bool(panel.get("shell_integration") and state.get("prompt_active")
                and panel.get("current_directory") is not None)


def theme_gallery_ready(state: dict) -> bool:
    # A visible gallery can still contain only its initial configuration row.
    # Require all five bundled themes before testing keyboard preview/scrolling.
    gallery = state.get("settings", {}).get("gallery")
    return bool(gallery and gallery.get("busy") is False and gallery.get("count", 0) >= 6)


def native_geometry_ready(state: dict, scale: float | None, logical_size=None) -> bool:
    actual = state.get("scale_factor")
    if not isinstance(actual, (int, float)) or not math.isfinite(actual) or actual <= 0:
        return False
    if scale is not None and abs(actual - scale) >= .01:
        return False
    if logical_size is None:
        return True
    for key, required in zip(("window_width", "window_height"), logical_size):
        extent = state.get(key)
        if not isinstance(extent, (int, float)) or not math.isfinite(extent) or abs(extent / actual - required) >= 1:
            return False
    return True


def check(condition: bool, message: str) -> None:
    if not condition:
        raise Failure(message)


def pid_exists(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


@contextmanager
def fixture_workspace():
    temporary = tempfile.TemporaryDirectory(prefix="automexia-unix-native-")
    diagnostics = {"retries": 0}
    try:
        yield temporary.name, diagnostics
    finally:
        # An entry can arrive after native directory enumeration, even after
        # the application and every observed shell have exited.
        # Retry only that filesystem race, using the same owned temp directory;
        # process failures and all other I/O errors remain fatal.
        for attempt in range(5):
            try:
                temporary.cleanup()
                break
            except OSError as error:
                if error.errno != errno.ENOTEMPTY or attempt == 4:
                    raise
                diagnostics["retries"] += 1
                time.sleep(.05 * (attempt + 1))


class WaylandDisplay:
    """Private compositor and PID-scoped input; never uses the user's display."""

    def __init__(self, scale: float):
        check(scale in (1.0, 1.5, 2.0), "unsupported controlled Wayland scale")
        self.scale = scale
        self.process = self.temporary = self.log = None
        self.socket = None
        self.tools = {}
        for name in ("sway", "swaymsg", "wtype", "grim", "xvfb-run"):
            executable = shutil.which(name)
            check(executable is not None, "required isolated Wayland tool missing")
            self.tools[name] = executable

    def __enter__(self):
        try:
            self.temporary = tempfile.TemporaryDirectory(prefix="amx-wayland-")
            root = Path(self.temporary.name)
            root.chmod(0o700)
            config = root / "config"
            config.write_text(
                "xwayland disable\n"
                f"output X11-1 mode {int(1600 * self.scale)}x{int(1200 * self.scale)} scale {self.scale}\n"
                "default_border none\ndefault_floating_border none\nfor_window [app_id=\".\"] floating enable\nfocus_follows_mouse no\n", encoding="utf-8")
            self.env = dict(os.environ)
            for key in ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", "SWAYSOCK"):
                self.env.pop(key, None)
            self.log = (root / "compositor.log").open("wb")
            # Xvfb supplies persistent keyboard/pointer devices to Sway's
            # X11 backend. Automexia connects to its private Wayland socket;
            # Xwayland is disabled. xvfb-run owns authentication and cleanup.
            self.env.update(XDG_RUNTIME_DIR=str(root), WLR_BACKENDS="x11",
                            WLR_RENDERER="pixman", WLR_LIBINPUT_NO_DEVICES="1", WLR_X11_OUTPUTS="1")
            self.process = subprocess.Popen(
                [self.tools["xvfb-run"], "-a", "-s",
                 f"-screen 0 {int(1600 * self.scale)}x{int(1200 * self.scale)}x24 -nolisten tcp",
                 self.tools["sway"], "--config", str(config)],
                env=self.env, stdout=self.log, stderr=self.log, start_new_session=True)
            end = time.monotonic() + 12
            while time.monotonic() < end:
                check(self.process.poll() is None, "isolated Wayland compositor exited")
                sockets = list(root.glob("sway-ipc.*.sock"))
                displays = [p for p in root.glob("wayland-*") if p.is_socket()]
                if len(sockets) == len(displays) == 1:
                    self.socket = str(sockets[0])
                    self.env["WAYLAND_DISPLAY"] = displays[0].name
                    self.client_env = {"XDG_RUNTIME_DIR": str(root), "WAYLAND_DISPLAY": displays[0].name,
                                       "WINIT_UNIX_BACKEND": "wayland"}
                    self.ipc("--type", "get_outputs")
                    return self
                time.sleep(.05)
            raise Failure("isolated Wayland display unavailable")
        except BaseException:
            self.__exit__(None, None, None)
            raise

    def __exit__(self, *unused):
        try:
            if self.process is not None and self.process.poll() is None:
                os.killpg(self.process.pid, signal.SIGTERM)
                try:
                    self.process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(self.process.pid, signal.SIGKILL)
                    self.process.wait(timeout=5)
        finally:
            if self.log is not None:
                self.log.close()
            if self.temporary is not None:
                self.temporary.cleanup()

    def ipc(self, *arguments):
        check(self.process is not None and self.process.poll() is None and self.socket is not None,
              "isolated Wayland compositor unavailable")
        result = subprocess.run([self.tools["swaymsg"], "--socket", self.socket, "--raw", *arguments],
                                env=self.env, capture_output=True, text=True, check=True, timeout=5)
        value = json.loads(result.stdout)
        if isinstance(value, list):
            check(all(item.get("success", True) for item in value), "owned Wayland operation failed")
        return value

    @staticmethod
    def owned_window(tree: dict, pid: int, focused: bool = False):
        pending, found, visited = [tree], [], 0
        while pending:
            node = pending.pop()
            visited += 1
            check(visited <= 512, "Wayland tree exceeded fixture bound")
            if node.get("pid") == pid and node.get("type") in ("con", "floating_con"):
                found.append(node)
            pending.extend(node.get("nodes", []))
            pending.extend(node.get("floating_nodes", []))
        check(len(found) == 1, "no unique owned Wayland window")
        check(not focused or found[0].get("focused"), "owned Wayland window lost focus")
        check(isinstance(found[0].get("id"), int) and found[0]["id"] > 0,
              "invalid owned Wayland window identity")
        return found[0]

    def window(self, process, focused=False):
        check(process.poll() is None, "owned Wayland application exited")
        return self.owned_window(self.ipc("--type", "get_tree"), process.pid, focused)

    def prepare(self, process):
        end = time.monotonic() + 10
        while time.monotonic() < end:
            try:
                window = self.window(process)
                break
            except Failure:
                check(process.poll() is None, "owned Wayland application exited")
                time.sleep(.05)
        else:
            raise Failure("no unique owned Wayland window")
        self.ipc(f"[con_id={window['id']}]", "floating enable, border none, focus")
        self.resize(process, 1200, 800)

    def resize(self, process, width, height):
        check(isinstance(width, int) and isinstance(height, int) and
              320 <= width <= 1600 and 240 <= height <= 1200, "invalid Wayland fixture size")
        window = self.window(process, focused=True)
        self.ipc(f"[con_id={window['id']}]", f"resize set width {width} px height {height} px")

    def key(self, process, chord):
        self.window(process, focused=True)
        check(chord in ("ctrl+shift+p", "Escape", "Down"), "unsupported Wayland fixture key")
        modifiers, key = chord.split('+')[:-1], chord.split('+')[-1]
        # Keep the virtual device
        # alive while clients bind wl_keyboard after its capability appears.
        arguments = [self.tools["wtype"], "-s", "150"]
        for modifier in modifiers:
            arguments.extend(["-M", modifier])
        arguments.extend(["-k", key])
        for modifier in reversed(modifiers):
            arguments.extend(["-m", modifier])
        arguments.extend(["-s", "100"])
        subprocess.run(arguments, env=self.env, check=True, capture_output=True, timeout=5)

    def click(self, process, x, y):
        window = self.window(process, focused=True)
        rect = window["rect"]
        check(all(math.isfinite(value) for value in (x, y)) and
              0 <= x < rect["width"] and 0 <= y < rect["height"], "pointer outside owned Wayland window")
        self.ipc("seat", "seat0", "cursor", "set", str(round(rect["x"] + x)), str(round(rect["y"] + y)))
        self.window(process, focused=True)
        self.ipc("seat", "seat0", "cursor", "press", "button1")
        self.ipc("seat", "seat0", "cursor", "release", "button1")

    def capture(self, process, destination):
        window = self.window(process, focused=True)
        rect = window["rect"]
        geometry = f"{rect['x']},{rect['y']} {rect['width']}x{rect['height']}"
        subprocess.run([self.tools["grim"], "-g", geometry, str(destination)], env=self.env,
                       check=True, capture_output=True, timeout=5)


def run_case(binary: Path, captures: Path, backend: str, shell: str, scale: float | None, launch_mode: str,
             display_server: str = "x11") -> dict:
    label = f"{backend}-{shell}-{scale if scale is not None else 'native'}-{launch_mode}"
    if display_server == "wayland":
        label = "wayland-" + label
    output = captures / label
    output.mkdir(parents=True, exist_ok=True)
    with fixture_workspace() as (temporary, cleanup_diagnostics), ExitStack() as stack:
        home = Path(temporary).resolve()
        config = home / "config"
        config.mkdir()
        public_rc = "export AMX_NATIVE_RC=RC_LOADED USER=fixture LOGNAME=fixture USERNAME=fixture\nPS1='fixture> '\n"
        (home / ".bashrc").write_text(public_rc, encoding="utf-8")
        # Isolate fixtures from the host's Debian completion trust prompt. This
        # test-only startup choice never alters the application bootstrap.
        (home / ".zshenv").write_text("skip_global_compinit=1\nexport AMX_NATIVE_ZSHENV=ENV_LOADED\n", encoding="utf-8")
        (home / ".zshrc").write_text(public_rc, encoding="utf-8")
        (home / ".bash_profile").write_text(public_rc, encoding="utf-8")
        fish = home / ".config/fish"
        fish.mkdir(parents=True)
        (fish / "config.fish").write_text(
            "set -gx AMX_NATIVE_RC RC_LOADED\nset -gx USER fixture\nset -gx LOGNAME fixture\n"
            "set -gx USERNAME fixture\nfunction fish_prompt; printf 'fixture> '; end\n", encoding="utf-8")
        renderer = "use-cpu = true" if backend == "cpu" else f"backend = {json.dumps(backend)}"
        program = ("/bin/zsh" if MACOS else "/bin/bash") if shell == "default" else ("/bin/" + shell if MACOS and shell != "fish" else shutil.which(shell))
        check(program is not None, "required shell missing")
        shell_config = "" if shell == "default" else f"[shell]\nprogram = {json.dumps(program)}\nargs = {json.dumps(['--login'] if MACOS else [])}\n"
        # Exercise the platform default and, on macOS, optional fork mode.
        # Profile env and CWD must reach the same validated launch owner.
        fork_config = "use-fork = true\n" if launch_mode == "fork" else ""
        (config / "config.toml").write_text(
            'confirm-before-quit = false\n'
            'env-vars = ["AMX_NATIVE_PROFILE=PROFILE_LOADED"]\n'
            + fork_config + shell_config +
            f'[renderer]\n{renderer}\n[cursor]\nblinking = false\n'
            '[session-recovery]\nenabled = false\n', encoding="utf-8")
        control, snapshot = home / "control", home / "snapshot.json"
        control.write_text("", encoding="utf-8")
        env = dict(os.environ, HOME=str(home), XDG_CONFIG_HOME=str(home / ".config"),
                   XDG_DATA_HOME=str(home / ".local/share"), XDG_CACHE_HOME=str(home / ".cache"),
                   AUTOMEXIA_CONFIG_HOME=str(config), AUTOMEXIA_NATIVE_TEST_CONTROL=str(control),
                   AUTOMEXIA_RESIZE_SNAPSHOT=str(snapshot), WINIT_UNIX_BACKEND="x11",
                   WINIT_X11_SCALE_FACTOR=str(scale or 1), SHELL=program, HISTFILE="/dev/null", RUST_LOG="error",
                   AUTOMEXIA_SHELL_INTEGRATION="1", AUTOMEXIA_CONTEXT_PATH_HINTS="0",
                   PATH="/usr/local/bin:/usr/bin:/bin", USERNAME="fixture", LOGNAME="fixture")
        for key in ("WAYLAND_DISPLAY", "ZDOTDIR", "AUTOMEXIA_ORIGINAL_ZDOTDIR", "AUTOMEXIA_ORIGINAL_ZDOTDIR_SET",
                    "AUTOMEXIA_VISUAL_TEST_FIXTURE"):
            env.pop(key, None)
        if MACOS:
            env.pop("WINIT_UNIX_BACKEND", None)
            env.pop("WINIT_X11_SCALE_FACTOR", None)
            # login(1) owns HOME/USER; isolate Zsh startup through ZDOTDIR.
            env["ZDOTDIR"] = str(home)
        else:
            env["USER"] = "fixture"
        wayland = None
        if display_server == "wayland":
            wayland = stack.enter_context(WaylandDisplay(scale or 1.0))
            env.update(wayland.client_env)
            for key in ("DISPLAY", "WINIT_X11_SCALE_FACTOR", "WAYLAND_SOCKET", "SWAYSOCK"):
                env.pop(key, None)
        # No unredacted native logs or snapshots are published.
        with (home / "stderr.log").open("wb") as log:
            started = time.monotonic()
            command_samples = []
            process = subprocess.Popen([str(binary), "--working-dir", str(home)],
                                       cwd=home, env=env, stdout=subprocess.DEVNULL,
                                       stderr=log, start_new_session=True)
            handle = None
            driver = None
            sequence = 0
            shell_pids: set[int] = set()
            try:
                def state() -> dict:
                    check(process.poll() is None, "application exited")
                    try:
                        value = json.loads(snapshot.read_text(encoding="utf-8"))
                    except (FileNotFoundError, json.JSONDecodeError):
                        return {}
                    for panel in value.get("panels", []):
                        if panel.get("shell_pid"):
                            shell_pids.add(panel["shell_pid"])
                    return value

                def wait(predicate, message: str, seconds: float = 15) -> dict:
                    end = time.monotonic() + seconds
                    value = {}
                    while time.monotonic() < end:
                        value = state()
                        if value and predicate(value):
                            return value
                        time.sleep(.05)
                    error = Failure(message)
                    error.observed = native_failure_state(value)
                    raise error

                wait(lambda s: s.get("sequence", 0) > 0, "no presented frame", 30)
                if MACOS:
                    driver = MacDriver(process)
                    driver.resize(1200, 800)
                elif wayland:
                    wayland.prepare(process)
                else:
                    end = time.monotonic() + 10
                    while time.monotonic() < end and handle is None:
                        found = subprocess.run(["xdotool", "search", "--pid", str(process.pid)],
                                               capture_output=True, text=True, timeout=5).stdout.split()
                        if len(found) == 1:
                            handle = found[0]
                        else:
                            time.sleep(.1)
                    check(handle is not None, "no unique owned X11 window")
                    call("xdotool", "windowsize", handle, "1200", "800")
                    call("xdotool", "windowfocus", handle)

                def key(chord: str) -> None:
                    if driver:
                        driver.key(chord.replace("ctrl+", "meta+"))
                    elif wayland:
                        wayland.key(process, chord)
                    else:
                        check(call("xdotool", "getwindowfocus") == handle, "owned window lost focus")
                        call("xdotool", "key", "--clearmodifiers", chord)

                def click(x: float, y: float) -> None:
                    if driver:
                        driver.click(x, y)
                    elif wayland:
                        wayland.click(process, x, y)
                    else:
                        check(call("xdotool", "getwindowfocus") == handle, "owned window lost focus")
                        current_scale = state()["scale_factor"]
                        call("xdotool", "mousemove", "--window", handle, str(round(x * current_scale)), str(round(y * current_scale)))
                        call("xdotool", "click", "1")

                def send(action: str) -> dict:
                    nonlocal sequence
                    sequence += 1
                    name, _, payload = action.partition(":")
                    command = f"{name}:{sequence}" + (f":{payload}" if payload else "")
                    pending = control.with_suffix(".next")
                    pending.write_text(command, encoding="utf-8")
                    pending.replace(control)
                    return wait(lambda s: s.get("last_control") == command, "control not presented")

                def capture(name: str) -> None:
                    # Reject identifying shell output before publishing pixels.
                    visible = active(state()).get("visible_text", "")
                    for marker in (socket.gethostname(), os.environ.get("USER", ""), os.environ.get("USERNAME", "")):
                        check(not marker or marker == "fixture" or marker not in visible,
                              "fixture exposed host identity before capture")
                    # Native pixels, separate from the semantic assertions.
                    if wayland:
                        wayland.capture(process, output / f"{name}.png")
                    elif not MACOS:
                        call("import", "-window", handle, str(output / f"{name}.png"))

                def command(text: str) -> dict:
                    previous = wait(lambda s: s.get("prompt_active"), "shell not ready")
                    prior_id = previous.get("latest_prompt_id")
                    command_start = time.monotonic()
                    send("write-line:" + text)
                    value = wait(lambda s: s.get("prompt_active") and s.get("latest_prompt_id") != prior_id,
                                "command did not complete at a fresh prompt")
                    command_samples.append(round((time.monotonic() - command_start) * 1000, 3))
                    return value

                initial = wait(lambda s: integrated_ready(s) and
                               s.get("latest_prompt_start_count") == 1, "session integration absent")
                if scale is not None:
                    # Wayland sends preferred fractional scale asynchronously;
                    # the first shell frame can precede that configure event.
                    initial = wait(lambda s: integrated_ready(s) and native_geometry_ready(s, scale),
                                   "native scale did not settle")
                if wayland:
                    initial = wait(lambda s: native_geometry_ready(s, scale, (1200, 800)),
                                   "initial Wayland size did not settle")
                ready_ms = round((time.monotonic() - started) * 1000, 3)
                expected = "wgpu" if backend == "webgpu" else backend
                check(initial.get("renderer_backend") == expected, "unexpected renderer fallback")
                if scale is not None:
                    check(abs(initial.get("scale_factor", 0) - scale) < .01, "scale not applied")
                check(initial.get("scale_factor", 0) > 0, "invalid native scale")
                chrome = initial.get("chrome", {})
                check(chrome.get("custom_controls") is True, "platform default bypasses application caption controls")
                logical_width = initial["window_width"] / initial["scale_factor"]
                check(abs(chrome.get("controls_x", 0) + 3 * chrome.get("button_width", 0) - logical_width) < .01,
                      "caption controls do not occupy their visible window edge")
                check(chrome.get("left_margin", 1000) <= 52, "unused native traffic-light inset retained")
                check(active(initial).get("current_directory") == str(home), "launch CWD discarded")
                result = command("printenv AMX_NATIVE_RC; printenv AMX_NATIVE_PROFILE")
                text = active(result).get("visible_text", "")
                check("RC_LOADED" in text and "PROFILE_LOADED" in text, "startup/profile environment lost")
                result = command("false")
                check(any(row.get("result_exit_code") == 1 for row in result.get("semantic_rows", [])), "failed command status lost")
                result = command("printf 'NAME  STATUS   AGE\\napi   Running  2d\\nweb   Pending  1d\\n'")
                check(result.get("inline_table_count", 0) >= 1, "inline table not rendered")
                wait(lambda s: bool(s.get("prompt_context_paints")), "information tags not painted")
                capture("terminal")

                key("ctrl+shift+p")
                wait(lambda s: s.get("palette_enabled"), "keyboard palette failed")
                capture("palette")
                key("Escape")
                wait(lambda s: not s.get("palette_enabled"), "palette did not close")
                chrome = state()["chrome"]
                check(chrome.get("palette_x") is not None, "header palette affordance absent")
                click(chrome["palette_x"], chrome["header_height"] / 2)
                wait(lambda s: s.get("palette_enabled"), "native header pointer action failed")
                key("Escape")
                wait(lambda s: not s.get("palette_enabled"), "pointer-opened palette did not close")
                if MACOS:
                    for maximized in (True, False):
                        chrome = state()["chrome"]
                        click(chrome["controls_x"] + 1.5 * chrome["button_width"], chrome["header_height"] / 2)
                        wait(lambda s: s.get("chrome", {}).get("maximized") is maximized,
                             "native maximize/restore caption failed")
                if MACOS:
                    # Reading children activates the native adapter lazily. Wait
                    # for its real frame; snapshots cannot satisfy this oracle.
                    def await_caption(label, present=True):
                        end = time.monotonic() + 8
                        while time.monotonic() < end:
                            if driver.caption(label) is present:
                                return
                            time.sleep(.05)
                        raise Failure("native AX caption state did not settle")

                    for label in ("Minimize window", "Close window"):
                        await_caption(label)
                    for label, maximized in (("Maximize window", True), ("Restore window", False)):
                        await_caption(label)
                        check(driver.caption(label, press=True), "native AX caption action missing")
                        wait(lambda s: s.get("chrome", {}).get("maximized") is maximized,
                             "native AX maximize/restore failed")
                send("open-customizations")
                wait(lambda s: s.get("settings", {}).get("ready"), "settings not ready")
                capture("customizations")
                if MACOS:
                    await_caption("Close window", False)
                key("ctrl+shift+p")
                check(not (state().get("palette_enabled") and state().get("settings", {}).get("open")),
                      "two overlapping root menus")
                key("Escape")
                wait(lambda s: not s.get("settings", {}).get("open"), "settings did not close")
                send("open-themes")
                wait(theme_gallery_ready, "theme library did not load")
                key("Down")
                preview = wait(lambda s: bool((s.get("settings", {}).get("gallery") or {}).get("builtin")),
                               "keyboard theme preview failed")
                gallery = preview["settings"]["gallery"]
                check(gallery["selected"] != gallery["current"], "preview overwrote the applied theme")
                capture("themes")
                key("Escape")
                restored = wait(lambda s: not s.get("settings", {}).get("gallery"), "gallery did not close")
                if restored.get("settings", {}).get("open"):
                    key("Escape")
                    wait(lambda s: not s.get("settings", {}).get("open"), "gallery parent did not close")

                send("clone-right")
                split = wait(lambda s: s.get("panel_count") == 2 and integrated_ready(s),
                             "native pane clone failed")
                check(active(split).get("launch_program") == (None if MACOS and shell == "default" else program), "clone changed native shell")
                check(active(split).get("current_directory") == str(home), "clone lost CWD")
                command("printenv AMX_NATIVE_RC")
                send("local-tab")
                tabbed = wait(lambda s: active(s).get("local_tab_count") == 2 and
                              integrated_ready(s), "native local tab failed")
                check(active(tabbed).get("launch_program") == (None if MACOS and shell == "default" else program), "tab changed native shell")
                capture("panes")
                # Shrink and expand after integrated output and UI use. The
                # viewport and grid must remain nonzero and within the window.
                if driver:
                    previous_width = state().get("window_width")
                    driver.resize(720, 520)
                    resized = wait(lambda s: 0 < s.get("window_width", 0) < previous_width,
                                   "native resize not presented")
                elif wayland:
                    wayland.resize(process, 720, 520)
                    resized = wait(lambda s: native_geometry_ready(s, scale, (720, 520)),
                                   "Wayland resize not presented")
                else:
                    call("xdotool", "windowsize", handle, "720", "520")
                    resized = wait(lambda s: s.get("window_width") == 720 and s.get("window_height") == 520,
                                   "resize not presented")
                check(all(p.get("cell_width", 0) > 0 and p.get("cell_height", 0) > 0
                          for p in resized.get("panels", [])), "invalid grid after resize")
                capture("resized")
                return {"scenario": label, "renderer": expected, "scale": initial["scale_factor"],
                        "timing_kind": "diagnostic-polling-50ms-not-performance-baseline",
                        "control_transport": "file-consumed-on-render-may-wait-for-idle-refresh",
                        "startup_shell_ready_ms": ready_ms, "command_roundtrip_ms": command_samples,
                        "native_pixel_capture": not MACOS,
                        "display_server": "AppKit" if MACOS else display_server,
                        "fixture_cleanup": cleanup_diagnostics,
                        "shell_integration": True, "table": True, "tags": True,
                        "palette": True, "settings": True, "themes": True,
                        "split": True, "local_tab": True, "resize": True, "shared_caption_controls": True,
                        "header_pointer": True, "native_maximize_restore": True if MACOS else None,
                        "native_ax_caption": True if MACOS else None}
            finally:
                if driver:
                    driver.dispose()
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait(timeout=5)
                end = time.monotonic() + 5
                while time.monotonic() < end and any(pid_exists(pid) for pid in shell_pids):
                    time.sleep(.05)
                check(not any(pid_exists(pid) for pid in shell_pids),
                      "owned shell survived application shutdown")


def native_failure_state(state: dict) -> dict:
    # Preserve only geometry and fixed booleans, never terminal text, paths or IDs.
    result = {}
    for section, keys in (("pointer", ("x", "y", "raw_y")),
                          ("chrome", ("header_height", "palette_x", "controls_x", "button_width", "maximized"))):
        values = state.get(section) or {}
        result[section] = {key: values[key] for key in keys
                           if isinstance(values.get(key), (int, float)) and math.isfinite(values[key])}
    for key in ("scale_factor", "window_width", "window_height", "palette_enabled", "prompt_active"):
        value = state.get(key)
        if isinstance(value, (int, float)) and math.isfinite(value):
            result[key] = value
    return result


def failure_details(error: BaseException) -> dict:
    # Never publish exception messages, filesystem paths, command text or
    # arbitrary traceback frames. Fixed owners, line numbers and errno identify
    # a failing operation without exposing the isolated session's contents.
    message = str(error)[:240] if isinstance(error, (Failure, BenchmarkError)) else type(error).__name__
    result = {"failure": message}
    if isinstance(error, Failure) and hasattr(error, "observed"):
        result["observed"] = error.observed
    if isinstance(error, OSError):
        result["errno"] = error.errno
    owners = {"unix-session-ui.py": "native-ui", "macos_driver.py": "native-input",
              "tempfile.py": "fixture-cleanup", "shutil.py": "filesystem-cleanup"}
    operations = []
    frame = error.__traceback__
    while frame is not None:
        owner = owners.get(Path(frame.tb_frame.f_code.co_filename).name)
        if owner:
            operations.append(f"{owner}:{frame.tb_lineno}")
        frame = frame.tb_next
    result["operations"] = operations[-8:]
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--captures", type=Path, required=True)
    parser.add_argument("--backend", choices=("cpu", "vulkan", "webgpu"), action="append")
    parser.add_argument("--shell", choices=("default", "bash", "zsh", "fish"), action="append")
    parser.add_argument("--scale", type=float, action="append")
    parser.add_argument("--launch-mode", choices=("default", "fork"), action="append")
    parser.add_argument("--display-server", choices=("x11", "wayland"), default="x11")
    parser.add_argument("--timeout", type=int, default=480)
    args = parser.parse_args()
    if not 1 <= args.timeout <= 1800:
        parser.error("timeout must be between 1 and 1800 seconds")
    signal.signal(signal.SIGTERM, cancel)
    signal.signal(signal.SIGINT, cancel)
    signal.signal(signal.SIGALRM, cancel)
    signal.alarm(args.timeout)
    captures = args.captures.resolve()
    captures.mkdir(parents=True, exist_ok=True)
    results = []
    if MACOS and args.display_server != "x11":
        parser.error("Wayland scenarios require Linux")
    if MACOS and args.scale:
        parser.error("macOS records its real display scale; controlled raster tests cover synthetic scales")
    for backend in args.backend or (["cpu", "webgpu"] if MACOS else ["cpu", "vulkan"]):
        for shell in args.shell or ["default", "bash", "zsh", "fish"]:
            for scale in args.scale or ([None] if MACOS else [1.0, 1.5]):
                for launch_mode in args.launch_mode or (["default", "fork"] if MACOS else ["default"]):
                    try:
                        result = run_case(args.binary.resolve(), captures, backend, shell, scale, launch_mode, args.display_server)
                    except (Failure, subprocess.SubprocessError, OSError, ValueError) as error:
                        result = {"scenario": ("wayland-" if args.display_server == "wayland" else "") + f"{backend}-{shell}-{scale if scale is not None else 'native'}-{launch_mode}",
                                  **failure_details(error)}
                    results.append(result)
                    print(json.dumps(result), flush=True)
                    (captures / "summary.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    (captures / "summary.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    return int(any("failure" in result for result in results))


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Cancelled:
        print(json.dumps({"failure": "native driver cancelled or exceeded its deadline"}), flush=True)
        raise SystemExit(130)
