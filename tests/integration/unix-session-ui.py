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
import re
import os
from pathlib import Path
import signal
import socket
import shutil
import sys
import subprocess
import tempfile
import time
import tomllib


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
    # These fixtures use lambda for Bash/Zsh and a fixed Fish prompt. Prompt-active/A,
    # CWD, and Readline's PS1 can arrive in separate PTY reads. Admit a baseline
    # only after that input row is both in the live grid and presented, with the
    # same generation; otherwise an ordinary late prompt looks like UI mutation.
    panel = active(state)
    prompt_id = state.get("latest_prompt_id")
    expected_prompt = "fixture>" if panel.get("shell_name") == "fish" else "λ"
    return bool(panel.get("shell_integration") and state.get("prompt_active")
                and panel.get("current_directory") is not None
                and type(prompt_id) is int and prompt_id > 0
                and panel.get("raw_cursor_prompt_id") == prompt_id
                and panel.get("raw_cursor_line_text") == expected_prompt
                and panel.get("cursor_line_text") == expected_prompt)


def replace_search_query(key, previous_bytes: int, value: str) -> None:
    # Backspace on an empty search is the documented Back action. Do not
    # accidentally leave the page when clearing a query that is already empty.
    if previous_bytes:
        key("ctrl+a")
        key("BackSpace")
    for char in value:
        key("space" if char == " " else char)


def wait_for_native_state(read, predicate, message: str, seconds: float = 15, wake=None) -> dict:
    end = time.monotonic() + seconds
    value = {}
    while time.monotonic() < end:
        value = read()
        if value and predicate(value):
            return value
        # A new/restarted process may not have mapped a native window yet.
        # Only published frames can be safely targeted by the owned input driver.
        if wake is not None and type(value.get("sequence")) is int and value["sequence"] > 0:
            wake()
        time.sleep(.05)
    error = Failure(message)
    error.observed = native_failure_state(value)
    raise error


def native_pixel_args(converter: str, image: Path, x: int, y: int, *, macos: bool) -> list[str]:
    args = [converter, str(image)]
    if macos:
        # screencapture embeds the display ICC profile. Raw device bytes from
        # different hosted displays are not comparable to encoded theme sRGB.
        args += ["-profile", "/System/Library/ColorSync/Profiles/sRGB Profile.icc"]
    return args + ["-crop", f"1x1+{x}+{y}", "-depth", "8", "rgb:-"]


def presented_rgb(sample, expected: list[int], tolerance: int) -> list[int]:
    # The renderer snapshot can precede compositor presentation. Preserve the
    # original color tolerance and fail on a persistent mismatch after a bound.
    deadline = time.monotonic() + 6.0
    while True:
        actual = sample()
        if max(abs(a-b) for a, b in zip(actual, expected)) <= tolerance or time.monotonic() >= deadline:
            return actual
        time.sleep(.05)


def theme_gallery_ready(state: dict) -> bool:
    # A visible gallery can still contain only its initial configuration row.
    # Require all five bundled themes before testing keyboard preview/scrolling.
    gallery = state.get("settings", {}).get("gallery")
    return bool(gallery and gallery.get("busy") is False and gallery.get("count", 0) >= 6)


def caption_transition(state: dict) -> tuple[str, bool, int]:
    maximized = (state.get("chrome") or {}).get("maximized")
    sequence = state.get("sequence")
    check(type(maximized) is bool and type(sequence) is int and sequence >= 0,
          "native caption state unavailable")
    return ("Restore window" if maximized else "Maximize window", not maximized, sequence)


def caption_transition_presented(state: dict, maximized: bool, sequence: int, native_size) -> bool:
    # isZoomed can change before the renderer receives Resized. A new frame
    # with the opposite label alone may still expose the old button positions.
    return (state.get("sequence", 0) > sequence
            and (state.get("chrome") or {}).get("maximized") is maximized
            and native_geometry_ready(state, None, native_size))


def caption_pointer_ready(state: dict, driver) -> bool:
    if not native_geometry_ready(state, None, driver.window_size()):
        return False
    label, _, _ = caption_transition(state)
    chrome = state["chrome"]
    # AXSize can reach its destination while WindowServer is still animating.
    # Read the actual native hit target before posting any mouse input.
    return driver.pointer_caption_ready(label, chrome["controls_x"] + 1.5 * chrome["button_width"],
                                        chrome["header_height"] / 2)


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


def native_pointer_ready(state: dict, x: float, y: float, logical_size) -> bool:
    if not native_geometry_ready(state, None, logical_size):
        return False
    pointer = state.get("pointer") or {}
    scale = state["scale_factor"]
    return all(type(pointer.get(axis)) in (int, float) and math.isfinite(pointer[axis])
               and abs(pointer[axis] / scale - expected) <= 1
               for axis, expected in (("x", x), ("y", y)))


def tag_geometry_saved(state: dict, identity: str, height: float) -> bool:
    # Observe the effect and its persistence acknowledgement in the SAME frame.
    # A pre-click save_pending=false is not evidence that this edit was saved.
    settings = state.get("settings") or {}
    return bool(settings.get("ready") and not settings.get("save_pending") and any(
        target.get("id") == identity and not target.get("roster")
        and len(target.get("bounds", [])) == 4 and target["bounds"][3] == height
        for target in settings.get("targets", [])))


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
    diagnostics = {"retries": 0, "removed": False}
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
                check(not Path(temporary.name).exists(), "owned test profile survived cleanup")
                diagnostics["removed"] = True
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
        for name in ("sway", "swaymsg", "xdotool", "xwininfo", "grim", "xvfb-run"):
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
                 sys.executable, "-c",
                 "import json,os,sys; from pathlib import Path; "
                 "Path(sys.argv[1]).write_text(json.dumps({k:os.environ[k] for k in ('DISPLAY','XAUTHORITY')})); "
                 "os.execv(sys.argv[2],sys.argv[2:])",
                 str(root / "input-display.json"), self.tools["sway"], "--config", str(config)],
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
                    # Inject through the compositor's persistent Xvfb keyboard.
                    # Creating/removing a wtype keyboard for each key emits
                    # Wayland focus-leave events that legitimately cancel drafts.
                    input_display = json.loads((root / "input-display.json").read_text())
                    check(set(input_display) == {"DISPLAY", "XAUTHORITY"} and
                          all(isinstance(v, str) and v for v in input_display.values()),
                          "invalid owned compositor input display")
                    self.input_env = dict(self.env, **input_display)
                    root_info = subprocess.run(
                        [self.tools["xwininfo"], "-root"], env=self.input_env,
                        check=True, capture_output=True, text=True, timeout=5).stdout
                    root_id = re.search(r"Window id: (0x[0-9a-fA-F]+)", root_info)
                    check(root_id is not None, "owned X11 root unavailable")
                    result = subprocess.run(
                        [self.tools["xdotool"], "search", "--onlyvisible", "--name", ""],
                        env=self.input_env, capture_output=True, text=True, timeout=5)
                    check(result.returncode in (0, 1), "owned compositor input unavailable")
                    windows = [w for w in result.stdout.split() if w != str(int(root_id.group(1), 16))]
                    if not windows:
                        time.sleep(.05)
                        continue  # IPC becomes ready before the X11 output maps.
                    check(len(windows) == 1 and windows[0].isdigit(), "no unique owned compositor window")
                    self.input_window = windows[0]
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
        check(chord in ("ctrl+shift+p", "Escape", "Down", "ctrl+shift+F1", "ctrl+shift+F3",
                        "alt+j", "alt+shift+j", "x", "a", "r", "0", "1", "2", "5", "ctrl+a", "F3", "Return",
                        "BackSpace", "space", *"connectTextsizeInheritshared"),
              "unsupported Wayland fixture key")
        subprocess.run(
            [self.tools["xdotool"], "windowfocus", self.input_window, "key", "--clearmodifiers", chord],
            env=self.input_env, check=True, capture_output=True, timeout=5)

    def click(self, process, x, y, read):
        window = self.window(process, focused=True)
        rect = window["rect"]
        check(all(math.isfinite(value) for value in (x, y)) and
              0 <= x < rect["width"] and 0 <= y < rect["height"], "pointer outside owned Wayland window")
        self.ipc("seat", "seat0", "cursor", "set", str(round(rect["x"] + x)), str(round(rect["y"] + y)))
        # Compositor IPC acceptance precedes client input/presentation. Do not
        # race a click against the application's previous pointer coordinates.
        # No wake motion is permitted here: it would move away from this target.
        wait_for_native_state(read, lambda s: native_pointer_ready(s, x, y, (rect["width"], rect["height"])),
                              "owned Wayland pointer did not reach target", 5)
        self.window(process, focused=True)
        self.ipc("seat", "seat0", "cursor", "press", "button1")
        self.ipc("seat", "seat0", "cursor", "release", "button1")

    def wake_control(self, process, sequence):
        window = self.window(process, focused=True)
        rect = window["rect"]
        check(rect["width"] >= 320 and rect["height"] >= 240, "invalid owned Wayland fixture size")
        # Stay in the empty left margin; no click, resize, or PTY input. Only a
        # native-test-hooks build with the private transport redraws on this move.
        self.ipc("seat", "seat0", "cursor", "set", str(round(rect["x"] + 12 + sequence % 2)),
                 str(round(rect["y"] + rect["height"] / 2)))

    def capture(self, process, destination):
        window = self.window(process, focused=True)
        rect = window["rect"]
        geometry = f"{rect['x']},{rect['y']} {rect['width']}x{rect['height']}"
        subprocess.run([self.tools["grim"], "-g", geometry, str(destination)], env=self.env,
                       check=True, capture_output=True, timeout=5)


def fixture_environment(home, config, program, control, snapshot, scale):
    # Keep display/toolchain capabilities only. Inherited Automexia, legacy
    # profile, benchmark and shell startup variables must never redirect a test.
    keys = ("PATH", "DISPLAY", "XAUTHORITY", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS",
            "VK_ICD_FILENAMES", "LIBGL_ALWAYS_SOFTWARE", "LANG", "LC_ALL")
    env = {name: os.environ[name] for name in keys if name in os.environ}
    env.update(HOME=str(home), XDG_CONFIG_HOME=str(home / ".config"),
               XDG_DATA_HOME=str(home / ".local/share"), XDG_CACHE_HOME=str(home / ".cache"),
               XDG_STATE_HOME=str(home / ".local/state"), RIO_CONFIG_HOME=str(home / "legacy-config"),
               USERPROFILE=str(home), LOCALAPPDATA=str(home), APPDATA=str(home),
               TMPDIR=str(home), TMP=str(home), TEMP=str(home),
               AUTOMEXIA_CONFIG_HOME=str(config), AUTOMEXIA_NATIVE_TEST_CONTROL=str(control),
               AUTOMEXIA_RESIZE_SNAPSHOT=str(snapshot), WINIT_UNIX_BACKEND="x11",
               WINIT_X11_SCALE_FACTOR=str(scale or 1), SHELL=program, HISTFILE="/dev/null", RUST_LOG="error",
               AUTOMEXIA_SHELL_INTEGRATION="1", AUTOMEXIA_CONTEXT_PATH_HINTS="0",
               PATH="/usr/local/bin:/usr/bin:/bin", USERNAME="fixture", LOGNAME="fixture")
    return env


def run_case(binary: Path, captures: Path, backend: str, shell: str, scale: float | None, launch_mode: str,
             display_server: str = "x11", appearance_theme: str | None = None) -> dict:
    label = f"{backend}-{shell}-{scale if scale is not None else 'native'}-{launch_mode}"
    if display_server == "wayland":
        label = "wayland-" + label
    if appearance_theme is not None:
        label = f"appearance-{appearance_theme}-" + label
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
        theme_config = ""
        if appearance_theme not in (None, "default"):
            source = Path(__file__).resolve().parents[2] / "themes" / f"{appearance_theme}.toml"
            check(source.is_file() and source.parent.name == "themes", "unknown fixture theme")
            (config / "themes").mkdir()
            shutil.copyfile(source, config / "themes" / source.name)
            theme_config = f"theme = {json.dumps(appearance_theme)}\n"
        (config / "config.toml").write_text(
            theme_config + 'confirm-before-quit = false\n'
            'env-vars = ["AMX_NATIVE_PROFILE=PROFILE_LOADED"]\n'
            + fork_config + shell_config +
            f'[renderer]\n{renderer}\n[cursor]\nblinking = false\n'
            '[session-recovery]\nenabled = false\n', encoding="utf-8")
        control, snapshot = home / "control", home / "snapshot.json"
        control.write_text("", encoding="utf-8")
        env = fixture_environment(home, config, program, control, snapshot, scale)
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
            picker_history = []
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
                    record_picker_state(picker_history, value)
                    return value

                wake_sequence = 0
                def wake_wayland():
                    nonlocal wake_sequence
                    wake_sequence += 1
                    wayland.wake_control(process, wake_sequence)

                def wait(predicate, message: str, seconds: float = 15) -> dict:
                    # UI-only changes need the same native test-transport wake
                    # as commands; they must not await the passive status clock.
                    return wait_for_native_state(state, predicate, message, seconds,
                                                 wake_wayland if wayland else None)

                def attach_window():
                    nonlocal driver, handle
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
                attach_window()

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
                        native_size = driver.window_size()
                        driver.click(x, y, wait_for_motion=lambda: wait_for_native_state(
                            state, lambda s: native_pointer_ready(s, x, y, native_size),
                            "macOS pointer motion was not presented before click", 5))
                    elif wayland:
                        wayland.click(process, x, y, state)
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
                    elif appearance_theme is not None:
                        call("/usr/sbin/screencapture", "-x", "-o", "-l", str(driver.owned_window_number()),
                             str(output / f"{name}.png"))

                def command(text: str) -> dict:
                    previous = wait(integrated_ready, "shell input prompt not fully presented")
                    prior_id = previous.get("latest_prompt_id")
                    command_start = time.monotonic()
                    send("write-line:" + text)
                    value = wait(lambda s: integrated_ready(s) and s.get("latest_prompt_id") != prior_id,
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
                if appearance_theme is not None:
                    converter = shutil.which("magick") or shutil.which("convert")
                    check(converter is not None, "native appearance pixel reader missing")
                    clear = initial.get("appearance", {}).get("clear")
                    check(clear is not None and clear[3] == 1.0, "clean startup did not clear opaque")
                    rgb = [round(channel * 255) for channel in clear[:3]]

                    def body_pixel(name):
                        # An empty area below the prompt, away from the cursor,
                        # status, border, and scroll bar. Native PNGs are bounded
                        # fixture output, read by the existing image test tool.
                        capture(name)
                        size = state()
                        x, y = round(size["window_width"] * .80), round(size["window_height"] * .60)
                        raw = subprocess.run(native_pixel_args(converter, output / f"{name}.png", x, y,
                                                               macos=MACOS),
                                             capture_output=True, check=True, timeout=10).stdout
                        check(len(raw) == 3, "native pixel reader did not return RGB")
                        return list(raw)

                    def assert_body(name, alpha):
                        check(abs(state().get("appearance", {}).get("window_alpha", -1) - alpha) < .01,
                              "native clear alpha does not match resolved setting")
                        if wayland:
                            # Real compositor oracle. A golden captured against
                            # one wallpaper cannot tell transparency from colour.
                            for shade in (0, 255):
                                wayland.ipc("output", "*", "bg", "#000000" if shade == 0 else "#ffffff", "solid_color")
                                time.sleep(.15)
                                expected_rgb = [round(v * alpha + shade * (1-alpha)) for v in rgb]
                                actual = presented_rgb(lambda: body_pixel(f"{name}-{shade}"), expected_rgb, 4)
                                check(max(abs(a-b) for a,b in zip(actual, expected_rgb)) <= 4,
                                      f"native body blend mismatch: {name}, alpha={alpha}, actual={actual}, expected={expected_rgb}")
                        elif alpha == 1.0:
                            actual = presented_rgb(lambda: body_pixel(name), rgb, 6)
                            if max(abs(a-b) for a,b in zip(actual, rgb)) > 6:
                                error = Failure(f"opaque body pixel mismatch: {name}, actual={actual}, expected={rgb}")
                                error.observed = native_failure_state(state())
                                raise error

                    def setting_row(identity):
                        current = wait(lambda s: s.get("settings", {}).get("ready") and
                                       any(r["id"] == identity for r in s["settings"].get("controls", [])),
                                       "appearance row unavailable")
                        bounds = next(r["bounds"] for r in current["settings"]["controls"] if r["id"] == identity)
                        click(bounds[0]+bounds[2]/2, bounds[1]+bounds[3]/2)

                    def restart():
                        nonlocal process, driver, handle
                        if driver:
                            driver.dispose()
                            driver = None
                        os.killpg(process.pid, signal.SIGTERM)
                        process.wait(timeout=5)
                        control.write_text("", encoding="utf-8")
                        snapshot.unlink(missing_ok=True)
                        handle = None
                        process = subprocess.Popen([str(binary), "--working-dir", str(home)], cwd=home, env=env,
                                                   stdout=subprocess.DEVNULL, stderr=log, start_new_session=True)
                        attach_window()
                        wait(lambda s: integrated_ready(s) and native_geometry_ready(
                            s, scale, (1200, 800) if wayland else None), "restarted session geometry not ready")

                    assert_body("clean-startup", 1.0)
                    cell_rgb = None
                    if wayland and backend != "cpu" and appearance_theme == "default":
                        # Record the renderer's opaque ANSI colour first: existing
                        # output-colour policy may resolve ANSI against the theme.
                        command(r"printf '\033[48;2;255;0;0m\033[2J\033[Hcell blend\033[0m\n'")
                        cell_rgb = body_pixel("opaque-cell")
                        check(cell_rgb[0] > 150 and cell_rgb[0] > cell_rgb[1]*2,
                              "opaque ANSI cell fixture missing")
                        restart()
                    if backend != "cpu":
                        send("open-terminal-appearance")
                        setting_row("interface.background.opacity")
                        wait(lambda s: s.get("settings", {}).get("active_category") == "interface.background.opacity",
                             "background customization did not open")
                        setting_row("interface.background.opacity")
                        wait(lambda s: (s.get("settings", {}).get("numeric_editor") or {}).get("id") == "interface.background.opacity",
                             "opacity editor did not open")
                        key("ctrl+a")
                        key("5")
                        key("0")
                        key("Return")
                        wait(lambda s: s.get("appearance", {}).get("window_alpha") == .5 and
                             not s.get("settings", {}).get("save_pending"), "opacity did not apply and save")
                        key("Escape")
                        key("Escape")
                        wait(lambda s: not s.get("settings", {}).get("open"), "settings did not close")
                        assert_body("half-opacity", .5)
                        restart()
                        wait(lambda s: s.get("appearance", {}).get("window_alpha") == .5, "saved opacity lost after restart")
                        assert_body("saved-restart", .5)
                        if wayland and appearance_theme == "default":
                            send("open-terminal-appearance")
                            setting_row("interface.background.opacity")
                            setting_row("interface.background.opacity-cells")
                            key("Escape")
                            key("Escape")
                            wait(lambda s: not s.get("settings", {}).get("open"), "cell settings did not close")
                            # Explicit ANSI backgrounds use the same source-over
                            # equation as the clear, without multiplying alpha twice.
                            command(r"printf '\033[48;2;255;0;0m\033[2J\033[Hcell blend\033[0m\n'")
                            for shade in (0, 255):
                                wayland.ipc("output", "*", "bg", "#000000" if shade == 0 else "#ffffff", "solid_color")
                                time.sleep(.15)
                                actual = body_pixel(f"cell-opacity-{shade}")
                                cell_alpha = 128/255
                                expected_rgb = [round(c*cell_alpha + (v*.5 + shade*.5)*(1-cell_alpha))
                                                for c,v in zip(cell_rgb, rgb)]
                                check(max(abs(a-b) for a,b in zip(actual, expected_rgb)) <= 4,
                                      f"native ANSI cell blend mismatch: actual={actual}, expected={expected_rgb}")
                            restart()
                        send("open-terminal-appearance")
                        wait(lambda s: s.get("settings", {}).get("ready"), "settings did not reopen")
                        # Leave search focus without editing it, then use the
                        # same reset confirmation as a keyboard user.
                        key("Down")
                        key("r")
                        confirmation = wait(lambda s: s.get("settings", {}).get("ready") and
                                            (s["settings"].get("confirmation") or {}).get("accept", [0,0,0,0])[2] > 0,
                                            "reset confirmation absent")
                        bounds = confirmation["settings"]["confirmation"]["accept"]
                        click(bounds[0]+bounds[2]/2, bounds[1]+bounds[3]/2)
                        wait(lambda s: s.get("appearance", {}).get("window_alpha") == 1 and
                             not s.get("settings", {}).get("save_pending"), "reset did not restore opaque default")
                        key("Escape")
                        wait(lambda s: not s.get("settings", {}).get("open"), "reset settings did not close")
                        assert_body("reset", 1.0)
                        restart()
                        assert_body("reset-restart", 1.0)
                    # Config remains the test-owned source; no UI save rewrites it.
                    saved = config / "state/user-preferences-v16.toml"
                    if saved.exists():
                        preferences = tomllib.loads(saved.read_text(encoding="utf-8"))
                        check("opacity" not in preferences.get("visual", {}).get("interface", {}),
                              "reset persisted a replacement for inherited opacity")
                    return {"scenario": label, "renderer": expected, "scale": initial["scale_factor"],
                            "appearance": True, "clean_default": True, "native_pixels": True,
                            "saved_restart_reset": backend != "cpu", "compositor_blend": bool(wayland),
                            "fixture_cleanup": cleanup_diagnostics}

                result = command("printenv AMX_NATIVE_RC; printenv AMX_NATIVE_PROFILE")
                text = active(result).get("visible_text", "")
                check("RC_LOADED" in text and "PROFILE_LOADED" in text, "startup/profile environment lost")
                result = command("false")
                check(any(row.get("result_exit_code") == 1 for row in result.get("semantic_rows", [])), "failed command status lost")
                result = command("printf 'NAME  STATUS   AGE\\napi   Running  2d\\nweb   Pending  1d\\n'")
                check(result.get("inline_table_count", 0) >= 1, "inline table not rendered")
                # Captured GNU formats, with synthetic values and paths. The
                # blank corner and sparse mountpoint continuations are absent
                # from the simple uppercase-header smoke above.
                for rows in (
                    (
                        "               total        used        free      shared  buff/cache   available",
                        "Mem:            11Gi       958Mi         9Gi       125Mi       1.2Gi        10Gi",
                        "Swap:          8.0Gi       8.4Mi       8.0Gi",
                    ),
                    (
                        "NAME",
                        "    MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS",
                        "sda   8:0    0 722.9M  1 disk",
                        "sdb   8:16   0 159.4M  1 disk",
                        "sdc   8:32   0     8G  0 disk [SWAP]",
                        "sdd   8:48   0     1T  0 disk /mnt/example",
                        "                              /mnt/archive",
                        "                              /",
                    ),
                ):
                    command("clear")
                    result = command("printf '" + "\\n".join(rows) + "\\n'")
                    wait(lambda s: s.get("inline_table_count") == 1, "numeric inventory table not rendered")
                capture("numeric-table")
                baseline = wait(integrated_ready, "table source prompt not fully presented")
                unchanged_text = active(baseline).get("visible_text")
                unchanged_prompt = baseline.get("latest_prompt_id")
                unchanged_route = active(baseline).get("route_id")
                send("open-customizations")

                def table_control() -> list:
                    current = wait(lambda s: s.get("settings", {}).get("ready") and any(
                        control.get("id") == "terminal.inline_tables"
                        for control in s["settings"].get("controls", [])), "inline table control not reachable")
                    return next(control["bounds"] for control in current["settings"]["controls"]
                                if control["id"] == "terminal.inline_tables")

                x, y, w, h = table_control()
                click(x + w / 2, y + h / 2)
                wait(lambda s: s.get("settings", {}).get("active_category") == "terminal.inline_tables",
                     "inline table category did not open")
                for count in (0, 1, 0, 1):
                    x, y, w, h = table_control()
                    click(x + w / 2, y + h / 2)
                    toggled = wait(lambda s: s.get("inline_table_count") == count,
                                   "table toggle did not update existing output")
                    check(active(toggled).get("visible_text") == unchanged_text,
                          "table toggle changed source text")
                    check(toggled.get("latest_prompt_id") == unchanged_prompt,
                          "table toggle reran the command")
                key("Escape")
                wait(lambda s: s.get("settings", {}).get("active_category") is None, "table category did not close")
                key("Escape")
                wait(lambda s: not s.get("settings", {}).get("open"), "customizations did not close")
                # Real shared/per-tag edits must update the existing preview and
                # terminal without another command. This uses only the isolated
                # HOME/state owned by this scenario, on Linux and macOS alike.
                send("open-customizations")

                def tag_control(identity):
                    current = wait(lambda s: s.get("settings", {}).get("ready") and
                                   not s["settings"].get("save_pending") and any(
                                       c["id"] == identity for c in s["settings"].get("controls", [])),
                                   f"tag control unavailable: {identity}")
                    x, y, w, h = next(c["bounds"] for c in current["settings"]["controls"] if c["id"] == identity)
                    click(x+w/2, y+h/2)

                def tag_search(value):
                    current = wait(lambda s: s.get("settings", {}).get("ready"), "tag page not ready")
                    x,y,w,h = current["settings"]["search_button"]
                    click(x+w/2,y+h/2)
                    replace_search_query(key, current["settings"]["search_bytes"], value)
                    wait(lambda s: s.get("settings", {}).get("search_bytes") == len(value), "tag search did not update")

                def tag_size(identity, value):
                    tag_search("Text size")
                    tag_control(identity)
                    wait(lambda s: (s.get("settings", {}).get("numeric_editor") or {}).get("id") == identity,
                         "tag size editor did not open")
                    key("ctrl+a")
                    for digit in str(value):
                        key(digit)
                    key("Return")
                    wait(lambda s: not s.get("settings", {}).get("numeric_editor") and
                         not s.get("settings", {}).get("save_pending"), "tag size did not save")
                    tag_search("")

                def sample_tag(s, identity):
                    return next((t["bounds"] for t in s.get("settings", {}).get("targets", [])
                                 if t["id"] == identity and not t["roster"]), None)

                def reveal_tag_sample(identity):
                    current = wait(lambda s: s.get("settings", {}).get("ready"), "tag preview not ready")
                    if not current["settings"].get("preview_edit_mode"):
                        x, y, w, h = current["settings"]["edit_button"]
                        click(x+w/2, y+h/2)
                        current = wait(lambda s: s.get("settings", {}).get("ready") and
                                       s["settings"].get("preview_edit_mode"), "tag editing did not start")
                    # At fractional DPI the sample is scrollable. Use the real
                    # keyboard selection/reveal path instead of assuming the
                    # last User tag is in the initial viewport.
                    for _ in range(32):
                        selected = current["settings"].get("selected")
                        if selected == identity:
                            return wait(lambda s: s.get("settings", {}).get("ready") and
                                        sample_tag(s, identity) is not None, "selected tag sample was not revealed")
                        key("Down")
                        current = wait(lambda s: s.get("settings", {}).get("ready") and
                                       s["settings"].get("selected") != selected,
                                       "tag preview selection did not advance")
                    raise Failure("tag preview traversal exceeded its bounded inventory")

                tag_control("tags.enabled")
                wait(lambda s: s.get("settings", {}).get("active_category") == "tags.enabled", "information tags did not open")
                before_style = reveal_tag_sample("tags.slot.user.page")
                before_height = sample_tag(before_style,"tags.slot.user.page")[3]
                tag_size("tags.shared.size", 125)
                reveal_tag_sample("tags.slot.user.page")
                shared_style = wait(lambda s: sample_tag(s,"tags.slot.user.page") is not None and
                                    sample_tag(s,"tags.slot.user.page")[3] > before_height,
                                    "shared tag size did not change live preview")
                x,y,w,h = sample_tag(shared_style,"tags.slot.user.page")
                click(x+w/2,y+h/2)
                wait(lambda s: s.get("settings", {}).get("active_slot") == "tags.slot.user.page", "individual tag did not open")
                tag_size("tags.slot.user.style.size", 50)
                wait(lambda s: sample_tag(s,"tags.slot.user.page") is not None and
                     sample_tag(s,"tags.slot.user.page")[3] < before_height, "individual tag override did not preview")
                capture("tag-typography")
                tag_search("Inherit shared")
                tag_control("tags.slot.user.style.inherit")
                wait(lambda s: tag_geometry_saved(s, "tags.slot.user.page",
                     sample_tag(shared_style, "tags.slot.user.page")[3]),
                     "inherit did not restore shared geometry and complete its save")
                tag_search("")
                saved_tags = tomllib.loads((config / "state/user-preferences-v16.toml").read_text(encoding="utf-8"))["visual"]["information-bar"]["styles"]
                check(saved_tags["shared"]["size"] == 125 and "user" not in saved_tags.get("tags", {}),
                      "tag styles did not persist through their existing preference owner")
                for _ in range(3):
                    key("Escape")
                    time.sleep(.1)
                wait(lambda s: not s.get("settings", {}).get("open"), "tag customizations did not close")
                check(active(state()).get("visible_text") == unchanged_text and state().get("latest_prompt_id") == unchanged_prompt,
                      "tag styling changed source text or reran a command")
                if backend == "cpu" and shell == "default" and scale == 1.0 and launch_mode == "default":
                    # A route change must not republish stale presentation defaults.
                    send("open-customizations")
                    wait(lambda s: s.get("settings", {}).get("ready"), "table settings did not reopen")
                    x, y, w, h = table_control()
                    click(x + w / 2, y + h / 2)
                    wait(lambda s: s.get("settings", {}).get("active_category") == "terminal.inline_tables",
                         "table category did not reopen")
                    x, y, w, h = table_control()
                    click(x + w / 2, y + h / 2)
                    wait(lambda s: s.get("inline_table_count") == 0, "table disable did not apply")
                    key("Escape")
                    wait(lambda s: s.get("settings", {}).get("active_category") is None, "table category did not close")
                    key("Escape")
                    wait(lambda s: not s.get("settings", {}).get("open"), "table settings did not close")
                    send("local-tab")
                    wait(lambda s: active(s).get("local_tab_count") == 2 and integrated_ready(s),
                         "table persistence tab did not open")
                    send("select-local:0")
                    returned = wait(lambda s: active(s).get("route_id") == unchanged_route and
                                    s.get("latest_prompt_id") == unchanged_prompt,
                                    "original table terminal did not return")
                    check(returned.get("inline_table_count") == 0,
                          "switching terminals reverted the table preference")
                    send("close-local:1")
                    closed = wait(lambda s: active(s).get("local_tab_count") == 1 and
                                  active(s).get("visible_text") == unchanged_text,
                                  "closing the extra tab did not restore the exact original output")
                    check(closed.get("inline_table_count") == 0, "closing a tab reverted the table preference")
                    send("open-customizations")
                    wait(lambda s: s.get("settings", {}).get("ready"), "table settings did not reopen")
                    x, y, w, h = table_control()
                    click(x + w / 2, y + h / 2)
                    wait(lambda s: s.get("settings", {}).get("active_category") == "terminal.inline_tables",
                         "table category did not reopen")
                    x, y, w, h = table_control()
                    click(x + w / 2, y + h / 2)
                    wait(lambda s: s.get("inline_table_count") == 1, "table re-enable did not apply")
                    key("Escape")
                    wait(lambda s: s.get("settings", {}).get("active_category") is None, "table category did not close")
                    key("Escape")
                    wait(lambda s: not s.get("settings", {}).get("open"), "table settings did not close")
                wait(lambda s: bool(s.get("prompt_context_paints")), "information tags not painted")
                identity = active(state())
                os_name = identity.get("shell_os_name")
                check(bool(os_name), "native shell did not publish its OS identity")
                os_label = "".join(list(os_name)[:13]) + "…" if len(os_name) > 14 else os_name
                # The OS label must reach the core projection, even without DevOps.
                check(os_label in identity.get("context_segments", []), "native OS tag missing from information bar")
                expected_brand = {"Ubuntu": "Ubuntu", "macOS": "macOS", "Linux Mint": "Linux Mint", "LMDE": "Linux Mint"}.get(os_name)
                if expected_brand:
                    check(identity.get("tab_os_brand", {}).get("name") == expected_brand,
                          "native tab branding disagrees with shell OS identity")
                    check(bool(identity.get("tab_os_brand", {}).get("glyph")), "native OS logo missing")
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
                # Exercise the shared SSH picker through native input on X11,
                # Wayland and AppKit. Opening it must never create a session.
                picker_before = active(state())
                pane_count = len(state().get("panels", []))
                key("ctrl+shift+p")
                wait(lambda s: s.get("palette_enabled"), "SSH picker palette did not open")
                for character in "connect":
                    key(character)
                wait(lambda s: 0 < s.get("palette_total_results", 0) <= 2,
                     "SSH picker palette search did not settle")
                key("Return")
                wait(lambda s: s.get("connection_hub_active") and s.get("connection_hub_ssh_picker"),
                     "SSH picker action did not open its own view")
                capture("ssh-picker")
                key("Escape")
                picker_after = wait(lambda s: not s.get("connection_hub_active"),
                                    "SSH picker did not close")
                if picker_after.get("palette_enabled"):
                    key("Escape")
                    picker_after = wait(lambda s: not s.get("palette_enabled"),
                                        "SSH picker opener did not close")
                check(len(picker_after.get("panels", [])) == pane_count,
                      "SSH picker created an unexpected pane")
                check(active(picker_after).get("raw_cursor_line_text") == picker_before.get("raw_cursor_line_text"),
                      "SSH picker leaked native keys into terminal input")
                if MACOS:
                    for _ in range(2):
                        before = wait(lambda s: caption_pointer_ready(s, driver),
                                      "native caption pointer target did not settle")
                        _, maximized, presented_sequence = caption_transition(before)
                        chrome = before["chrome"]
                        click(chrome["controls_x"] + 1.5 * chrome["button_width"], chrome["header_height"] / 2)
                        wait(lambda s: caption_transition_presented(s, maximized, presented_sequence, driver.window_size()),
                             "native maximize/restore caption failed")
                if MACOS:
                    # Reading children activates the native adapter lazily. Wait
                    # for its real frame; snapshots cannot satisfy this oracle.
                    def await_caption(caption_label, present=True, press=False):
                        end = time.monotonic() + 8
                        while time.monotonic() < end:
                            if driver.caption(caption_label, press=press) is present:
                                return
                            time.sleep(.05)
                        raise Failure("native AX caption state did not settle")

                    for caption_label in ("Minimize window", "Close window"):
                        await_caption(caption_label)
                    for _ in range(2):
                        before = wait(lambda s: native_geometry_ready(s, None, driver.window_size()),
                                      "native AX caption geometry did not settle")
                        caption_label, maximized, presented_sequence = caption_transition(before)
                        # Find and press one retained native element in the same
                        # traversal; a separate presence query can become stale.
                        await_caption(caption_label, press=True)
                        wait(lambda s: caption_transition_presented(s, maximized, presented_sequence, driver.window_size()),
                             "native AX maximize/restore failed")
                key("ctrl+shift+F1")
                wait(lambda s: s.get("settings", {}).get("ready"), "settings not ready")
                capture("customizations")
                if MACOS:
                    await_caption("Close window", False)
                key("ctrl+shift+p")
                check(not (state().get("palette_enabled") and state().get("settings", {}).get("open")),
                      "two overlapping root menus")
                key("Escape")
                wait(lambda s: not s.get("settings", {}).get("open"), "settings did not close")
                key("ctrl+shift+F3")
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

                # All hosts use this picker. Native input must leave a chosen
                # swatch ready for shortcuts rather than silently editing hex.
                send("open-terminal-appearance")

                def color_editor(s):
                    return s.get("settings", {}).get("color_editor") or {}

                def color_row():
                    current = wait(lambda s: s.get("settings", {}).get("ready") and
                                   not s["settings"].get("save_pending") and any(
                                       c.get("id") == "interface.header.background"
                                       for c in s["settings"].get("controls", [])),
                                   "header color control not reachable")
                    bounds = next(c["bounds"] for c in current["settings"]["controls"]
                                  if c["id"] == "interface.header.background")
                    x, y, w, h = bounds
                    click(x + w / 2, y + h / 2)

                def palette_control(focus):
                    current = wait(lambda s: s.get("settings", {}).get("ready") and any(
                        c["focus"] == focus for c in color_editor(s).get("palette", {}).get("controls", [])),
                        "color palette control not reachable")
                    bounds = next(c["bounds"] for c in color_editor(current)["palette"]["controls"]
                                  if c["focus"] == focus)
                    x, y, w, h = bounds
                    click(x + w / 2, y + h / 2)

                def color_focus(focus):
                    return wait(lambda s: s.get("settings", {}).get("ready") and
                                color_editor(s).get("palette", {}).get("focus") == focus,
                                "color picker focus did not settle: " + focus)

                color_row()
                wait(lambda s: s.get("settings", {}).get("active_category") == "interface.header.background",
                     "header appearance page did not open")
                color_row()
                color_focus("Hex")
                palette_control("Suggested")
                suggested = color_focus("Swatch(0)")
                swatch = color_editor(suggested)["palette"]["controls"][-1]
                chosen = swatch["value"]
                palette_control(swatch["focus"])
                selected = color_focus("Apply")
                check(color_editor(selected)["draft_color"] == chosen, "pointer selection changed color")
                check(not color_editor(selected)["palette"]["browsing"], "selection trapped palette focus")
                capture("color-selected")
                key("x")  # Must not replace the selected value with a text character.
                selected = color_focus("Apply")
                check(color_editor(selected)["draft_color"] == chosen, "stray character replaced selected color")
                key("a")
                wait(lambda s: s.get("settings", {}).get("ready") and not color_editor(s) and
                     not s["settings"].get("save_pending"), "Apply shortcut did not save color")
                color_row()
                selected = color_focus("Hex")
                check(color_editor(selected)["draft_color"] == chosen,
                      f"Apply shortcut color mismatch: chosen={chosen}, reopened={color_editor(selected)['draft_color']}")
                key("F3")
                color_focus("FavoriteToggle")
                palette_control("Favorites")
                color_focus("Swatch(0)")
                key("Return")
                selected = color_focus("Apply")
                check(color_editor(selected)["draft_color"] == chosen, "keyboard favorite changed color")
                key("x")
                key("Escape")
                wait(lambda s: not color_editor(s), "Escape did not cancel chosen color")
                color_row()
                selected = color_focus("Hex")
                check(color_editor(selected)["draft_color"] == chosen, "cancellation changed saved color")
                key("Escape")
                wait(lambda s: not color_editor(s), "color editor did not close")
                key("Escape")
                wait(lambda s: s.get("settings", {}).get("active_category") is None,
                     "header appearance page did not close")
                key("Escape")
                wait(lambda s: not s.get("settings", {}).get("open"), "appearance did not close")

                key("meta+alt+shift+r" if MACOS else "alt+j")
                split = wait(lambda s: s.get("panel_count") == 2 and integrated_ready(s),
                             "native pane clone failed")
                check(active(split).get("launch_program") == (None if MACOS and shell == "default" else program), "clone changed native shell")
                check(active(split).get("current_directory") == str(home), "clone lost CWD")
                command("printenv AMX_NATIVE_RC")
                send("local-tab")
                tabbed = wait(lambda s: active(s).get("local_tab_count") == 2 and
                              integrated_ready(s), "native local tab failed")
                check(active(tabbed).get("launch_program") == (None if MACOS and shell == "default" else program), "tab changed native shell")
                if expected_brand:
                    check(len(active(tabbed).get("local_tab_brands", [])) == 2 and
                          all(brand == expected_brand for brand in active(tabbed)["local_tab_brands"]),
                          "cloned local tabs lost native OS branding")
                # Use the native key path for both distinct right-pane actions.
                key("meta+d" if MACOS else "alt+shift+j")
                wait(lambda s: s.get("panel_count") == 3 and integrated_ready(s),
                     "fresh right-pane keyboard shortcut failed")
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
                        "control_transport": ("file-consumed-on-render-with-native-pointer-wake" if wayland else
                                              "file-consumed-on-render-may-wait-for-idle-refresh"),
                        "startup_shell_ready_ms": ready_ms, "command_roundtrip_ms": command_samples,
                        "native_pixel_capture": not MACOS,
                        "display_server": "AppKit" if MACOS else display_server,
                        "fixture_cleanup": cleanup_diagnostics,
                        "shell_integration": True, "table": True, "numeric_tables": True, "table_live_toggle": True, "tags": True,
                        "palette": True, "ssh_picker": True, "settings": True, "themes": True, "color_picker_focus": True,
                        "split": True, "local_tab": True, "resize": True, "shared_caption_controls": True,
                        "header_pointer": True, "native_maximize_restore": True if MACOS else None,
                        "native_ax_caption": True if MACOS else None}
            except Exception as error:
                error.picker_history = picker_history
                if driver:
                    try:
                        error.native_focus = driver.focus_snapshot()
                    except BenchmarkError:
                        error.native_focus = {'observation_unavailable': True}
                raise
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
    appearance = state.get("appearance")
    if isinstance(appearance, dict):
        safe = {}
        alpha = appearance.get("window_alpha")
        if type(alpha) in (int, float) and math.isfinite(alpha) and 0 <= alpha <= 1:
            safe["window_alpha"] = alpha
        clear = appearance.get("clear")
        if (isinstance(clear, list) and len(clear) == 4 and
                all(type(v) in (int, float) and math.isfinite(v) and 0 <= v <= 1 for v in clear)):
            safe["clear"] = clear
        result["appearance"] = safe
    settings = state.get("settings")
    if isinstance(settings, dict):
        result["settings"] = {key: settings[key] for key in
                              ("open", "ready", "list_focused", "save_pending")
                              if type(settings.get(key)) is bool}
        result["settings"]["confirmation_visible"] = bool(settings.get("confirmation"))
        for key in ("catalog_revision", "last_color_dismissal", "previous_color_dismissal", "color_open_generation"):
            value = settings.get(key)
            if type(value) is int and 0 <= value <= 2**64 - 1:
                result["settings"][key] = value
        editor = settings.get("color_editor")
        if isinstance(editor, dict):
            palette = editor.get("palette") or {}
            focus = palette.get("focus")
            # Fixed control identities only; never include draft hex or values.
            if isinstance(focus, str) and (focus in (
                    "Favorites", "Suggested", "FavoriteToggle", "Hex", "Apply", "Cancel")
                    or re.fullmatch(r"Swatch\([0-9]{1,3}\)", focus)):
                result["settings"]["color_focus"] = focus
            if type(palette.get("browsing")) is bool:
                result["settings"]["color_browsing"] = palette["browsing"]
    for key in ("scale_factor", "window_width", "window_height", "palette_enabled", "prompt_active"):
        value = state.get(key)
        if isinstance(value, (int, float)) and math.isfinite(value):
            result[key] = value
    return result


def record_picker_state(history: list, state: dict) -> None:
    # Keep only fixed semantic state transitions. No colors, text, paths or IDs.
    settings = native_failure_state(state).get("settings", {})
    keys = ("catalog_revision", "last_color_dismissal", "previous_color_dismissal",
            "color_open_generation", "color_focus", "color_browsing")
    current = {key: settings[key] for key in keys if key in settings}
    if "color_focus" not in current and not history:
        return
    if not history or history[-1] != current:
        history.append(current)
        del history[:-32]


def failure_details(error: BaseException) -> dict:
    # Never publish exception messages, filesystem paths, command text or
    # arbitrary traceback frames. Fixed owners, line numbers and errno identify
    # a failing operation without exposing the isolated session's contents.
    message = str(error)[:240] if isinstance(error, (Failure, BenchmarkError)) else type(error).__name__
    result = {"failure": message}
    focus = getattr(error, "native_focus", None)
    if isinstance(focus, dict):
        result["native_focus"] = {key: focus[key] for key in (
            "process_alive", "window_owned", "app_frontmost", "owned_window_focused",
            "app_frontmost_unavailable", "owned_window_focused_unavailable", "observation_unavailable")
            if type(focus.get(key)) is bool}
    if isinstance(error, Failure) and hasattr(error, "observed"):
        result["observed"] = error.observed
    if getattr(error, "picker_history", None):
        result["picker_history"] = error.picker_history
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
    parser.add_argument("--backend", choices=("cpu", "vulkan", "webgpu", "metal"), action="append")
    parser.add_argument("--shell", choices=("default", "bash", "zsh", "fish"), action="append")
    parser.add_argument("--scale", type=float, action="append")
    parser.add_argument("--launch-mode", choices=("default", "fork"), action="append")
    parser.add_argument("--display-server", choices=("x11", "wayland"), default="x11")
    parser.add_argument("--appearance-only", action="store_true")
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
    if not MACOS and "metal" in (args.backend or []):
        parser.error("Metal requires macOS")
    if MACOS and args.scale:
        parser.error("macOS records its real display scale; controlled raster tests cover synthetic scales")
    for backend in args.backend or (["cpu", "webgpu"] if MACOS else ["cpu", "vulkan"]):
        for shell in args.shell or ["default", "bash", "zsh", "fish"]:
            for scale in args.scale or ([None] if MACOS else [1.0, 1.5]):
                for launch_mode in args.launch_mode or (["default", "fork"] if MACOS else ["default"]):
                    try:
                        themes = ["default", "aurora-night", "solar-dusk", "forest-operator", "arctic-glass", "arctic-day"] if args.appearance_only else [None]
                        for theme in themes:
                            result = run_case(args.binary.resolve(), captures, backend, shell, scale, launch_mode, args.display_server, theme)
                            if theme != themes[-1]:
                                results.append(result)
                                print(json.dumps(result), flush=True)
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
