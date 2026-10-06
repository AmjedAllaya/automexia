"""Thin native sensors and owned-window drivers for controlled desktop runs.

The caller owns the unreaped process inside qa_process containment. No numeric
PID is admitted from a user profile, and no terminal contents are inspected.
"""
import ctypes as ct
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from benchmark_model import BenchmarkError


class Resources:
    def __init__(self, process):
        self.process = process
        self.identity = None
        if os.name == 'nt':
            from ctypes import wintypes as wt
            class Memory(ct.Structure):
                _fields_ = [('cb', wt.DWORD), ('faults', wt.DWORD)] + [(name, ct.c_size_t) for name in
                    ('peak_rss', 'rss', 'peak_paged', 'paged', 'peak_nonpaged', 'nonpaged', 'pagefile', 'peak_pagefile')]
            self.memory = Memory
            self.api = ct.WinDLL('kernel32', use_last_error=True)
            self.api.K32GetProcessMemoryInfo.argtypes = [wt.HANDLE, ct.c_void_p, wt.DWORD]
            self.api.K32GetProcessMemoryInfo.restype = wt.BOOL
            self.api.GetProcessTimes.argtypes = [wt.HANDLE] + [ct.POINTER(ct.c_uint64)] * 4
            self.api.GetProcessTimes.restype = wt.BOOL
        elif sys.platform == 'darwin':
            # Apple's RUSAGE_INFO_V0 ABI: UUID followed by ten uint64 fields.
            class Usage(ct.Structure):
                _fields_ = [('uuid', ct.c_uint8 * 16)] + [(name, ct.c_uint64) for name in
                    ('user', 'system', 'idle_wakes', 'interrupt_wakes', 'pageins', 'wired',
                     'rss', 'footprint', 'start', 'exit')]
            self.usage = Usage
            self.api = ct.CDLL('/usr/lib/libproc.dylib')
            self.api.proc_pid_rusage.argtypes = [ct.c_int, ct.c_int, ct.c_void_p]
            self.api.proc_pid_rusage.restype = ct.c_int

    def sample(self):
        if self.process.poll() is not None:
            raise BenchmarkError('owned application exited before resource sample')
        if os.name == 'nt':
            value = self.memory(); value.cb = ct.sizeof(value)
            handle = int(self.process._handle)
            times = [ct.c_uint64() for _ in range(4)]
            if (not self.api.K32GetProcessMemoryInfo(handle, ct.byref(value), value.cb)
                    or not self.api.GetProcessTimes(handle, *(ct.byref(x) for x in times))):
                raise BenchmarkError('native process counters unavailable')
            rss, cpu, identity = value.rss, (times[2].value + times[3].value) * 100, times[0].value
        elif sys.platform == 'linux':
            data = Path(f'/proc/{self.process.pid}/stat').read_text()[:8192]
            fields = data[data.rfind(')') + 2:].split()
            # /proc stat fields 14,15,22,24; first field here is state (3).
            rss = int(fields[21]) * os.sysconf('SC_PAGE_SIZE')
            cpu = (int(fields[11]) + int(fields[12])) * 1_000_000_000 // os.sysconf('SC_CLK_TCK')
            identity = fields[19]
        elif sys.platform == 'darwin':
            value = self.usage()
            if self.api.proc_pid_rusage(self.process.pid, 0, ct.byref(value)) != 0:
                raise BenchmarkError('native process counters unavailable')
            rss, cpu, identity = value.rss, value.user + value.system, value.start
        else:
            raise BenchmarkError('resource adapter unsupported')
        if self.identity is not None and self.identity != identity:
            raise BenchmarkError('native process identity changed')
        self.identity = identity
        return rss, cpu, time.monotonic_ns()


class NvidiaGpu:
    """Optional NVML whole-device busy percentage; never substitute a zero."""
    def __init__(self, expected_name, expected_driver):
        library = str(Path(os.environ.get('SystemRoot', 'C:/Windows')) / 'System32' / 'nvml.dll') if os.name == 'nt' else 'libnvidia-ml.so.1'
        self.api = ct.CDLL(library)
        self.api.nvmlInit_v2.restype = ct.c_int
        self.api.nvmlShutdown.restype = ct.c_int
        if self.api.nvmlInit_v2() != 0:
            raise OSError('GPU sensor unavailable')
        self.active = True
        try:
            self.api.nvmlDeviceGetHandleByIndex_v2.argtypes = [ct.c_uint, ct.POINTER(ct.c_void_p)]
            self.api.nvmlDeviceGetName.argtypes = [ct.c_void_p, ct.c_void_p, ct.c_uint]
            self.api.nvmlSystemGetDriverVersion.argtypes = [ct.c_void_p, ct.c_uint]
            self.api.nvmlDeviceGetUtilizationRates.argtypes = [ct.c_void_p, ct.c_void_p]
            self.handle = ct.c_void_p()
            name, driver = ct.create_string_buffer(160), ct.create_string_buffer(160)
            if (self.api.nvmlDeviceGetHandleByIndex_v2(0, ct.byref(self.handle)) != 0
                    or self.api.nvmlDeviceGetName(self.handle, name, len(name)) != 0
                    or self.api.nvmlSystemGetDriverVersion(driver, len(driver)) != 0
                    or name.value.decode('ascii') != expected_name or driver.value.decode('ascii') != expected_driver):
                raise OSError('GPU sensor identity mismatch')
        except BaseException:
            self.close()
            raise

    def sample(self):
        value = (ct.c_uint * 2)()
        if self.api.nvmlDeviceGetUtilizationRates(self.handle, ct.byref(value)) != 0 or value[0] > 100:
            raise OSError('GPU sample unavailable')
        return int(value[0])

    def close(self):
        if self.active:
            self.api.nvmlShutdown()
            self.active = False


class WindowsDriver:
    def __init__(self, process):
        from ctypes import wintypes as wt
        self.process, self.handle = process, None
        self.api = ct.WinDLL('user32', use_last_error=True)
        self.callback = ct.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
        signatures = {
            'EnumWindows': ([self.callback, wt.LPARAM], wt.BOOL),
            'GetWindowThreadProcessId': ([wt.HWND, ct.POINTER(wt.DWORD)], wt.DWORD),
            'IsWindowVisible': ([wt.HWND], wt.BOOL),
            'SetForegroundWindow': ([wt.HWND], wt.BOOL),
            'GetForegroundWindow': ([], wt.HWND),
            'SetWindowPos': ([wt.HWND, wt.HWND, ct.c_int, ct.c_int, ct.c_int, ct.c_int, wt.UINT], wt.BOOL),
            'PostMessageW': ([wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM], wt.BOOL),
            'SendInput': ([wt.UINT, ct.c_void_p, ct.c_int], wt.UINT),
            'GetAsyncKeyState': ([ct.c_int], ct.c_short),
            'GetKeyState': ([ct.c_int], ct.c_short),
            'AttachThreadInput': ([wt.DWORD, wt.DWORD, wt.BOOL], wt.BOOL),
            'BringWindowToTop': ([wt.HWND], wt.BOOL),
            'GetClassNameW': ([wt.HWND, wt.LPWSTR, ct.c_int], ct.c_int),
            'GetWindowTextLengthW': ([wt.HWND], ct.c_int),
            'GetClientRect': ([wt.HWND, ct.POINTER(wt.RECT)], wt.BOOL),
        }
        for name, (args, result) in signatures.items():
            getattr(self.api, name).argtypes, getattr(self.api, name).restype = args, result
        class Keyboard(ct.Structure):
            _fields_ = [('vk', wt.WORD), ('scan', wt.WORD), ('flags', wt.DWORD), ('time', wt.DWORD), ('extra', ct.c_size_t)]
        class Mouse(ct.Structure):
            _fields_ = [('x', wt.LONG), ('y', wt.LONG), ('data', wt.DWORD), ('flags', wt.DWORD), ('time', wt.DWORD), ('extra', ct.c_size_t)]
        class Union(ct.Union):
            _fields_ = [('keyboard', Keyboard), ('mouse', Mouse)]
        class Input(ct.Structure):
            _fields_ = [('type', wt.DWORD), ('value', Union)]
        self.keyboard, self.input = Keyboard, Input
        dwm = ct.WinDLL('dwmapi', use_last_error=True)
        dwm.DwmGetWindowAttribute.argtypes = [wt.HWND, wt.DWORD, ct.c_void_p, wt.DWORD]
        dwm.DwmGetWindowAttribute.restype = ct.c_long
        deadline = time.monotonic() + 15
        while self.handle is None and time.monotonic() < deadline:
            found = []
            @self.callback
            def visit(window, _):
                owner = wt.DWORD()
                self.api.GetWindowThreadProcessId(window, ct.byref(owner))
                if owner.value == process.pid and self.api.IsWindowVisible(window):
                    cloaked, rect, name = wt.DWORD(), wt.RECT(), ct.create_unicode_buffer(256)
                    if (dwm.DwmGetWindowAttribute(window, 14, ct.byref(cloaked), ct.sizeof(cloaked)) == 0
                            and self.api.GetClientRect(window, ct.byref(rect))
                            and self.api.GetClassNameW(window, name, len(name)) > 0
                            and application_surface(name.value, rect.right-rect.left, rect.bottom-rect.top,
                                                    cloaked.value, self.api.GetWindowTextLengthW(window))):
                        found.append(window)
                return True
            self.api.EnumWindows(visit, 0)
            if len(found) == 1:
                self.handle = found[0]
            elif len(found) > 1:
                raise BenchmarkError('benchmark requires exactly one owned visible window')
            elif process.poll() is not None:
                raise BenchmarkError('application exited before native window appeared')
            else:
                time.sleep(.05)
        if self.handle is None:
            raise BenchmarkError('native window unavailable')
        # Same bounded focus handoff used by the repository's native GUI
        # harness. Detach before probing, and never send keys to another HWND.
        kernel = ct.WinDLL('kernel32', use_last_error=True)
        kernel.GetCurrentThreadId.restype = wt.DWORD
        current = kernel.GetCurrentThreadId()
        for _ in range(3):
            foreground = self.api.GetForegroundWindow()
            if foreground == self.handle:
                break
            thread = self.api.GetWindowThreadProcessId(foreground, None) if foreground else 0
            attached = bool(thread and thread != current and self.api.AttachThreadInput(current, thread, True))
            try:
                self.api.BringWindowToTop(self.handle)
                self.api.SetForegroundWindow(self.handle)
            finally:
                if attached and not self.api.AttachThreadInput(current, thread, False):
                    raise BenchmarkError('native focus handoff could not detach')
            time.sleep(.1)
        self.check()

    def check(self):
        from ctypes import wintypes as wt
        owner = wt.DWORD()
        self.api.GetWindowThreadProcessId(self.handle, ct.byref(owner))
        if (self.process.poll() is not None or owner.value != self.process.pid
                or self.api.GetForegroundWindow() != self.handle):
            raise BenchmarkError('benchmark lost owned window focus; input stopped')

    def key(self, key):
        self.check()
        if any(self.api.GetAsyncKeyState(vk) & 0x8000 for vk in (0x10, 0x11, 0x12, 0x5b, 0x5c)) or self.api.GetKeyState(0x14) & 1:
            raise BenchmarkError('physical modifiers or Caps Lock would change the fixed input probe')
        vk = {'F5': 0x74, 'F6': 0x75, 'F8': 0x77, 'Escape': 0x1b}.get(key, ord(key.upper()) if len(key) == 1 else 0)
        if not vk:
            raise BenchmarkError('unsupported fixed probe key')
        values = (self.input * 2)()
        for index in range(2):
            values[index].type = 1
            values[index].value.keyboard = self.keyboard(vk, 0, 2 if index else 0, 0, 0)
        if self.api.SendInput(2, values, ct.sizeof(self.input)) != 2:
            raise BenchmarkError('native input injection failed')

    def resize(self, width, height):
        self.check()
        if not self.api.SetWindowPos(self.handle, None, 0, 0, width, height, 0x16):
            raise BenchmarkError('native resize failed')

    def close(self):
        self.check()
        if not self.api.PostMessageW(self.handle, 0x10, 0, 0):
            raise BenchmarkError('native close failed')


class X11Driver:
    def __init__(self, process):
        self.process = process
        self.program = shutil.which('xdotool')
        if not self.program or not os.environ.get('DISPLAY') or os.environ.get('WAYLAND_DISPLAY'):
            raise BenchmarkError('controlled X11 driver unavailable')
        deadline = time.monotonic() + 15
        self.window = None
        while time.monotonic() < deadline and self.window is None:
            found = self.call('search', '--onlyvisible', '--pid', str(process.pid), allow_empty=True).split()
            if len(found) == 1 and found[0].isdigit():
                self.window = found[0]
            elif len(found) > 1:
                raise BenchmarkError('multiple owned benchmark windows')
            else:
                time.sleep(.05)
        if self.window is None:
            raise BenchmarkError('native window unavailable')
        self.call('windowactivate', '--sync', self.window)
        self.check()

    def call(self, *args, allow_empty=False):
        # xdotool commands used here return only tiny numeric identities.
        result = subprocess.run([self.program, *args], stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.DEVNULL, timeout=3, check=False)
        if len(result.stdout) > 4096 or (result.returncode != 0 and not allow_empty):
            raise BenchmarkError('controlled X11 operation failed')
        return result.stdout.decode('ascii').strip()

    def check(self):
        if (self.process.poll() is not None or self.call('getwindowpid', self.window) != str(self.process.pid)
                or self.call('getactivewindow') != self.window):
            raise BenchmarkError('benchmark lost owned window focus; input stopped')

    def key(self, key):
        self.check(); self.call('key', key)

    def resize(self, width, height):
        self.check(); self.call('windowsize', self.window, str(width), str(height))

    def close(self):
        self.check(); self.call('windowclose', self.window)


def driver(process):
    if os.name == 'nt':
        return WindowsDriver(process)
    if sys.platform == 'linux':
        return X11Driver(process)
    if sys.platform == 'darwin':
        from macos_driver import MacDriver
        return MacDriver(process)
    raise BenchmarkError('interactive measurement adapter unsupported')


def application_surface(class_name, width, height, cloaked, title_length):
    """Match tests/integration/windows-native-window-locator.cs admission rules."""
    return (class_name != 'Winit Thread Event Target' and width >= 100 and height >= 100
            and cloaked == 0 and title_length > 0)
