"""Owned-process AX/Quartz probes; requires preauthorized Accessibility access.

No permission prompts, AppleScript, shell evaluation or system-wide key posting.
Pointer probes use the session event stream only after a native hit test verifies
the frontmost point belongs to the owned application.
Native macOS validation is required before admitting this adapter's cohort.
"""
import ctypes as ct
import math
import time

from benchmark_model import BenchmarkError


class Point(ct.Structure):
    _fields_ = [('x', ct.c_double), ('y', ct.c_double)]


class Size(ct.Structure):
    _fields_ = [('width', ct.c_double), ('height', ct.c_double)]


class MacDriver:
    CAPTION_LABELS = ('Minimize window', 'Maximize window', 'Restore window', 'Close window')

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
                'CFEqual': ([pointer, pointer], ct.c_bool),
                'CFDictionaryGetValue': ([pointer, pointer], pointer),
                'CFNumberGetValue': ([pointer, ct.c_int, pointer], ct.c_bool),
            },
            self.ax: {
                'AXIsProcessTrusted': ([], ct.c_bool),
                'AXUIElementCreateApplication': ([ct.c_int], pointer),
                'AXUIElementCreateSystemWide': ([], pointer),
                'AXUIElementCopyElementAtPosition': ([pointer, ct.c_float, ct.c_float, ct.POINTER(pointer)], ct.c_int),
                'AXUIElementCopyAttributeValue': ([pointer, pointer, ct.POINTER(pointer)], ct.c_int),
                'AXUIElementSetAttributeValue': ([pointer, pointer, pointer], ct.c_int),
                'AXUIElementPerformAction': ([pointer, pointer], ct.c_int),
                'AXUIElementGetPid': ([pointer, ct.POINTER(ct.c_int)], ct.c_int),
                'AXUIElementSetMessagingTimeout': ([pointer, ct.c_float], ct.c_int),
                'AXValueCreate': ([ct.c_int, pointer], pointer),
                'AXValueGetValue': ([pointer, ct.c_int, pointer], ct.c_bool),
            },
            self.cg: {
                'CGEventCreateKeyboardEvent': ([pointer, ct.c_uint16, ct.c_bool], pointer),
                'CGEventCreateMouseEvent': ([pointer, ct.c_uint32, Point, ct.c_uint32], pointer),
                'CGEventSetIntegerValueField': ([pointer, ct.c_uint32, ct.c_int64], None),
                'CGWindowListCopyWindowInfo': ([ct.c_uint32, ct.c_uint32], pointer),
                'CGEventKeyboardSetUnicodeString': ([pointer, ct.c_ulong, ct.POINTER(ct.c_uint16)], None),
                'CGEventSetFlags': ([pointer, ct.c_uint64], None),
                'CGEventPostToPid': ([ct.c_int, pointer], None),
                'CGEventPost': ([ct.c_uint32, pointer], None),
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

    @staticmethod
    def key_spec(key):
        codes = {'F5': 96, 'F6': 97, 'F8': 100, 'Escape': 53, 'Down': 125,
                 's': 1, 'e': 14, 'a': 0, 'r': 15, 'c': 8, 'h': 4,
                 'n': 45, 'd': 2, 'l': 37, 'x': 7, 'q': 12, 'p': 35}
        fields = key.split('+')
        modifiers, base = fields[:-1], fields[-1]
        masks = {'shift': 1 << 17, 'ctrl': 1 << 18, 'alt': 1 << 19, 'meta': 1 << 20}
        if base not in codes or len(modifiers) != len(set(modifiers)) or any(m not in masks for m in modifiers):
            raise BenchmarkError('unsupported fixed probe key')
        return codes[base], sum(masks[m] for m in modifiers), base

    def key(self, key):
        self.check()
        code, flags, base = self.key_spec(key)
        for down in (True, False):
            event = self.cg.CGEventCreateKeyboardEvent(None, code, down)
            if not event:
                raise BenchmarkError('macOS keyboard probe unavailable')
            try:
                self.cg.CGEventSetFlags(event, flags)
                if len(base) == 1 and not flags:
                    units = (ct.c_uint16 * 1)(ord(base))
                    self.cg.CGEventKeyboardSetUnicodeString(event, 1, units)
                self.cg.CGEventPostToPid(self.process.pid, event)
            finally:
                self.cf.CFRelease(event)

    @staticmethod
    def point_in_window(x, y, width, height):
        return (all(math.isfinite(value) for value in (x, y, width, height))
                and 0 <= x < width and 0 <= y < height)

    def owned_window_number(self):
        self.check()
        windows = self.cg.CGWindowListCopyWindowInfo(1, 0)  # on-screen windows
        if not windows:
            raise BenchmarkError('macOS window identifiers unavailable')
        try:
            count = self.cf.CFArrayGetCount(windows)
            if not 0 <= count <= 512:
                raise BenchmarkError('macOS window inventory exceeded fixture bound')
            matches = []
            keys = [ct.c_void_p.in_dll(self.cg, name).value
                    for name in ('kCGWindowOwnerPID', 'kCGWindowLayer', 'kCGWindowNumber')]
            for index in range(count):
                window = self.cf.CFArrayGetValueAtIndex(windows, index)
                values = []
                for key in keys:
                    number = ct.c_int64()
                    value = self.cf.CFDictionaryGetValue(window, key)
                    if not value or not self.cf.CFNumberGetValue(value, 4, ct.byref(number)):
                        break
                    values.append(number.value)
                if len(values) == 3 and values[0] == self.process.pid and values[1] == 0:
                    matches.append(values[2])
            if len(matches) != 1 or matches[0] <= 0:
                raise BenchmarkError('no unique owned macOS pointer window')
            return matches[0]
        finally:
            self.cf.CFRelease(windows)

    def pointer_hit_matches(self, point, caption=None):
        """Read-only readiness; actual input retains strict ownership checks."""
        if caption is not None and caption not in self.CAPTION_LABELS:
            raise BenchmarkError('unsupported fixed caption probe')
        self.check()
        if not math.isfinite(point.x) or not math.isfinite(point.y):
            raise BenchmarkError('macOS pointer point is invalid')
        system, hit = self.ax.AXUIElementCreateSystemWide(), ct.c_void_p()
        if not system:
            raise BenchmarkError('macOS pointer ownership unavailable')
        try:
            owner = ct.c_int()
            if (self.ax.AXUIElementSetMessagingTimeout(system, 2.0) != 0
                    or self.ax.AXUIElementCopyElementAtPosition(system, point.x, point.y, ct.byref(hit)) != 0
                    or not hit.value
                    or self.ax.AXUIElementGetPid(hit.value, ct.byref(owner)) != 0
                    or owner.value != self.process.pid):
                return False
            if caption is None:
                return True
            values = []
            try:
                for attribute in ('AXRole', 'AXTitle'):
                    value = self.copy(hit.value, attribute, optional=True)
                    if not value:
                        return False
                    values.append(value)
                for text in ('AXButton', caption):
                    values.append(self.string(text))
                return bool(self.cf.CFEqual(values[0], values[2])
                            and self.cf.CFEqual(values[1], values[3]))
            finally:
                for value in values:
                    self.cf.CFRelease(value)
        finally:
            if hit.value:
                self.cf.CFRelease(hit.value)
            self.cf.CFRelease(system)

    def check_pointer_owner(self, point):
        if not self.pointer_hit_matches(point):
            raise BenchmarkError('macOS pointer point is not owned; input stopped')

    def pointer_point(self, x, y):
        self.check()
        point, size = Point(), Size()
        for name, kind, result in [('AXPosition', 1, point), ('AXSize', 2, size)]:
            value = self.copy(self.window, name)
            try:
                if not self.ax.AXValueGetValue(value, kind, ct.byref(result)):
                    raise BenchmarkError('macOS owned window geometry unavailable')
            finally:
                self.cf.CFRelease(value)
        if not self.point_in_window(x, y, size.width, size.height):
            raise BenchmarkError('macOS pointer probe outside owned window')
        point.x += x
        point.y += y
        return point

    def pointer_caption_ready(self, label, x, y):
        if label not in self.CAPTION_LABELS:
            raise BenchmarkError('unsupported fixed caption probe')
        return self.pointer_hit_matches(self.pointer_point(x, y), label)

    def click(self, x, y):
        point = self.pointer_point(x, y)
        self.owned_window_number()  # Reject ambiguous or stale owned windows.
        events = []
        try:
            for event_type in (5, 1, 2):  # mouse moved, left down, left up
                event = self.cg.CGEventCreateMouseEvent(None, event_type, point, 0)
                if not event:
                    raise BenchmarkError('macOS pointer probe unavailable')
                events.append(event)
                self.cg.CGEventSetFlags(event, 0)
                self.cg.CGEventSetIntegerValueField(event, 1, 1)  # mouse click state
            self.check_pointer_owner(point)
            # Let WindowServer produce native window-relative mouse coordinates.
            # PID-posted keyboard events remain independent of this mouse path.
            self.cg.CGEventPost(1, events[0])  # kCGSessionEventTap
            time.sleep(.02)
            self.check_pointer_owner(point)
            try:
                self.cg.CGEventPost(1, events[1])
            finally:
                # Queue the release even if dispatch fails. Do not hit-test again
                # between down/up: maximize can move the window during the click.
                self.cg.CGEventPost(1, events[2])
        finally:
            for event in events:
                self.cf.CFRelease(event)

    def window_size(self):
        """Read logical native bounds independently of the renderer's snapshot."""
        self.check()
        value, size = self.copy(self.window, 'AXSize'), Size()
        try:
            if (not self.ax.AXValueGetValue(value, 2, ct.byref(size))
                    or not all(math.isfinite(v) and v > 0 for v in (size.width, size.height))):
                raise BenchmarkError('macOS owned window size unavailable')
            return size.width, size.height
        finally:
            self.cf.CFRelease(value)

    def resize(self, width, height):
        self.check()
        size = Size(width, height)
        value = self.ax.AXValueCreate(2, ct.byref(size))  # kAXValueCGSizeType
        if not value:
            raise BenchmarkError('macOS resize value unavailable')
        try:
            self.set(self.window, 'AXSize', value)
        finally:
            self.cf.CFRelease(value)

    def caption(self, label, press=False):
        """Find a fixed caption in the owned AX tree, optionally activate it."""
        if label not in self.CAPTION_LABELS:
            raise BenchmarkError('unsupported fixed caption probe')
        self.check()
        pending, retained, matches = [self.window], [], []
        expected, role = self.string(label), self.string('AXButton')
        visited = 0
        try:
            while pending:
                element = pending.pop()
                visited += 1
                if visited + len(pending) > 1024:
                    raise BenchmarkError('macOS caption tree exceeded fixture bound')
                owner = ct.c_int()
                if self.ax.AXUIElementGetPid(element, ct.byref(owner)) != 0 or owner.value != self.process.pid:
                    raise BenchmarkError('macOS caption tree ownership changed')
                title = self.copy(element, 'AXTitle', optional=True)
                kind = self.copy(element, 'AXRole', optional=True)
                try:
                    if title and kind and self.cf.CFEqual(title, expected) and self.cf.CFEqual(kind, role):
                        matches.append(element)
                finally:
                    for value in (title, kind):
                        if value:
                            self.cf.CFRelease(value)
                children = self.copy(element, 'AXChildren', optional=True)
                if children:
                    try:
                        count = self.cf.CFArrayGetCount(children)
                        if not 0 <= count <= 1024 - visited - len(pending):
                            raise BenchmarkError('macOS caption children exceeded fixture bound')
                        for index in range(count):
                            child = self.cf.CFRetain(self.cf.CFArrayGetValueAtIndex(children, index))
                            if not child:
                                raise BenchmarkError('macOS caption child unavailable')
                            retained.append(child)
                            pending.append(child)
                    finally:
                        self.cf.CFRelease(children)
            if len(matches) > 1:
                raise BenchmarkError('macOS caption identity is ambiguous')
            if matches and press:
                self.check()
                self.action(matches[0], 'AXPress')
            return bool(matches)
        finally:
            for value in retained + [expected, role]:
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
