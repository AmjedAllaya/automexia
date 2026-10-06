"""Owned-process AX/Quartz probes; requires preauthorized Accessibility access.

No permission prompts, AppleScript, shell evaluation or system-wide key posting.
Native macOS validation is required before admitting this adapter's cohort.
"""
import ctypes as ct
import time

from benchmark_model import BenchmarkError


class MacDriver:
    def __init__(self, process):
        self.process, self.application, self.window = process, None, None
        self.cf = ct.CDLL('/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation')
        self.ax = ct.CDLL('/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices')
        self.cg = ct.CDLL('/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics')
        pointer = ct.c_void_p
        signatures = {
            self.cf: {
                'CFStringCreateWithCString': ([pointer, ct.c_char_p, ct.c_uint32], pointer),
                'CFRelease': ([pointer], None), 'CFRetain': ([pointer], pointer),
                'CFArrayGetCount': ([pointer], ct.c_long),
                'CFArrayGetValueAtIndex': ([pointer, ct.c_long], pointer),
                'CFBooleanGetValue': ([pointer], ct.c_bool),
            },
            self.ax: {
                'AXIsProcessTrusted': ([], ct.c_bool),
                'AXUIElementCreateApplication': ([ct.c_int], pointer),
                'AXUIElementCopyAttributeValue': ([pointer, pointer, ct.POINTER(pointer)], ct.c_int),
                'AXUIElementSetAttributeValue': ([pointer, pointer, pointer], ct.c_int),
                'AXUIElementPerformAction': ([pointer, pointer], ct.c_int),
                'AXUIElementGetPid': ([pointer, ct.POINTER(ct.c_int)], ct.c_int),
                'AXUIElementSetMessagingTimeout': ([pointer, ct.c_float], ct.c_int),
                'AXValueCreate': ([ct.c_int, pointer], pointer),
            },
            self.cg: {
                'CGEventCreateKeyboardEvent': ([pointer, ct.c_uint16, ct.c_bool], pointer),
                'CGEventKeyboardSetUnicodeString': ([pointer, ct.c_ulong, ct.POINTER(ct.c_uint16)], None),
                'CGEventSetFlags': ([pointer, ct.c_uint64], None),
                'CGEventPostToPid': ([ct.c_int, pointer], None),
            },
        }
        for library, functions in signatures.items():
            for name, (args, result) in functions.items():
                getattr(library, name).argtypes, getattr(library, name).restype = args, result
        if not self.ax.AXIsProcessTrusted():
            raise BenchmarkError('macOS benchmark driver lacks preauthorized Accessibility access')
        try:
            self.application = self.ax.AXUIElementCreateApplication(process.pid)
            if not self.application or self.ax.AXUIElementSetMessagingTimeout(self.application, 2.0) != 0:
                raise BenchmarkError('macOS application observation unavailable')
            deadline = time.monotonic() + 15
            while self.window is None and time.monotonic() < deadline:
                windows = self.copy(self.application, 'AXWindows', optional=True)
                if windows:
                    try:
                        count = self.cf.CFArrayGetCount(windows)
                        if count == 1:
                            self.window = self.cf.CFRetain(self.cf.CFArrayGetValueAtIndex(windows, 0))
                        elif count > 1:
                            raise BenchmarkError('multiple owned benchmark windows')
                    finally:
                        self.cf.CFRelease(windows)
                if self.window is None:
                    time.sleep(.05)
            if not self.window:
                raise BenchmarkError('macOS native window unavailable')
            self.set(self.application, 'AXFrontmost', ct.c_void_p.in_dll(self.cf, 'kCFBooleanTrue').value)
            self.action(self.window, 'AXRaise')
            time.sleep(.2)
            self.check()
        except BaseException:
            self.dispose()
            raise

    def string(self, text):
        value = self.cf.CFStringCreateWithCString(None, text.encode('ascii'), 0x08000100)
        if not value:
            raise BenchmarkError('macOS probe attribute allocation failed')
        return value

    def copy(self, element, name, optional=False):
        attribute, value = self.string(name), ct.c_void_p()
        try:
            code = self.ax.AXUIElementCopyAttributeValue(element, attribute, ct.byref(value))
            if code != 0:
                if optional:
                    return None
                raise BenchmarkError('macOS probe attribute unavailable')
            return value.value
        finally:
            self.cf.CFRelease(attribute)

    def set(self, element, name, value):
        attribute = self.string(name)
        try:
            if self.ax.AXUIElementSetAttributeValue(element, attribute, value) != 0:
                raise BenchmarkError('macOS probe attribute could not be set')
        finally:
            self.cf.CFRelease(attribute)

    def action(self, element, name):
        action = self.string(name)
        try:
            if self.ax.AXUIElementPerformAction(element, action) != 0:
                raise BenchmarkError('macOS owned-window action failed')
        finally:
            self.cf.CFRelease(action)

    def check(self):
        owner = ct.c_int()
        if (self.process.poll() is not None or self.ax.AXUIElementGetPid(self.window, ct.byref(owner)) != 0
                or owner.value != self.process.pid):
            raise BenchmarkError('macOS owned window identity changed')
        front = self.copy(self.application, 'AXFrontmost')
        try:
            if not front or not self.cf.CFBooleanGetValue(front):
                raise BenchmarkError('benchmark lost owned window focus; input stopped')
        finally:
            if front:
                self.cf.CFRelease(front)

    def key(self, key):
        self.check()
        codes = {'F5': 96, 'F6': 97, 'F8': 100, 'Escape': 53,
                 's': 1, 'e': 14, 'a': 0, 'r': 15, 'c': 8, 'h': 4, 'n': 45, 'd': 2, 'l': 37, 'x': 7, 'q': 12}
        if key not in codes:
            raise BenchmarkError('unsupported fixed probe key')
        for down in (True, False):
            event = self.cg.CGEventCreateKeyboardEvent(None, codes[key], down)
            if not event:
                raise BenchmarkError('macOS keyboard probe unavailable')
            try:
                self.cg.CGEventSetFlags(event, 0)
                if len(key) == 1:
                    units = (ct.c_uint16 * 1)(ord(key))
                    self.cg.CGEventKeyboardSetUnicodeString(event, 1, units)
                self.cg.CGEventPostToPid(self.process.pid, event)
            finally:
                self.cf.CFRelease(event)

    def resize(self, width, height):
        self.check()
        class Size(ct.Structure):
            _fields_ = [('width', ct.c_double), ('height', ct.c_double)]
        size = Size(width, height)
        value = self.ax.AXValueCreate(2, ct.byref(size))  # kAXValueCGSizeType
        if not value:
            raise BenchmarkError('macOS resize value unavailable')
        try:
            self.set(self.window, 'AXSize', value)
        finally:
            self.cf.CFRelease(value)

    def close(self):
        self.check()
        button = self.copy(self.window, 'AXCloseButton')
        try:
            self.action(button, 'AXPress')
        finally:
            if button:
                self.cf.CFRelease(button)

    def dispose(self):
        for name in ('window', 'application'):
            value = getattr(self, name)
            if value:
                self.cf.CFRelease(value)
                setattr(self, name, None)
