param(
    [string]$Binary,
    [ValidateRange(100, 10000)]
    [int]$PowerShellHistoryBudgetMilliseconds = 1500,
    [ValidateRange(1000, 30000)]
    [int]$MaximumOwnedShutdownMilliseconds = 6000,
    [string]$ResourceReport,
    [string]$FrameCapture,
    [string]$TypographyCapture,
    [string]$SearchCapture,
    [string]$ResultCapture,
    [string]$ResultNavigationCapture,
    [string]$ModalCaptureDirectory,
    [ValidateRange(32, 4096)]
    [int64]$MaximumHandleGrowth = 384,
    [ValidateRange(8, 512)]
    [int64]$MaximumThreadGrowth = 48,
    [ValidateRange(67108864, 4294967296)]
    [int64]$MaximumPrivateBytesGrowth = 536870912,
    [ValidateRange(67108864, 4294967296)]
    [int64]$MaximumWorkingSetGrowth = 536870912,
    [ValidateRange(2, 128)]
    [int64]$MaximumDescendantProcessGrowth = 16,
    [ValidateRange(4, 256)]
    [int]$ImagePreviewLifecycleCycles = 16,
    [ValidateRange(0, 128)]
    [int64]$MaximumImageHandleGrowth = 32,
    [ValidateRange(0, 16)]
    [int64]$MaximumImageThreadGrowth = 2,
    [ValidateRange(8388608, 1073741824)]
    [int64]$MaximumImageMemoryGrowth = 134217728,
    [switch]$CloseConfirmationOnly,
    [switch]$TagCustomizationOnly,
    [switch]$ThemeGalleryOnly,
    [switch]$NamedProfilesOnly,
    [switch]$TagShapesOnly,
    [switch]$OutputColorsOnly,
    [switch]$CommandInputColorsOnly,
    [switch]$PlainOutputColorsOnly,
    [switch]$ClearShortcutOnly,
    [switch]$WordDeletionOnly,
    [switch]$MenuNavigationOnly,
    [string]$CommandInputWslDistro,
    [switch]$CommandInputPowerShell7,
    [switch]$ConnectionHubOnly,
    [switch]$UseCpuRenderer,
    [switch]$SessionRecoveryOnly,
    [switch]$QuickActionsOnly,
    [switch]$LiveBackdropOnly,
    [switch]$DependentControlsOnly,
    [switch]$TerminalAppearanceOnly,
    [switch]$OpacityOnly,
    [ValidateRange(0.0, 1.0)]
    [double]$InitialWindowOpacity = 1.0,
    [switch]$WindowControlChoicesOnly,
    [switch]$SharedColorPickerOnly,
    [switch]$FontPickerOnly,
    [switch]$AccessibilityOnly,
    [switch]$PixelOracleOnly
)

$ErrorActionPreference = 'Stop'
if ($ClearShortcutOnly -or $WordDeletionOnly -or $PlainOutputColorsOnly) { $CommandInputColorsOnly = $true }
if ($TagShapesOnly -or $MenuNavigationOnly) { $TagCustomizationOnly = $true }
# A Windows PowerShell child can inherit PowerShell 7's module search paths.
# Resolve Get-FileHash from the executing host for saved-file preservation checks.
Import-Module (Join-Path $PSHOME 'Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1') -ErrorAction Stop
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
# The source-built fixture must read the current preference owner's filename.
# Keeping a second schema number here let save/cancel checks inspect obsolete
# files (and sometimes incorrectly report an unchanged absent snapshot).
$preferenceSource = [IO.File]::ReadAllText((Join-Path $root 'apps/automexia-terminal/src/automexia/preferences.rs'))
$preferenceMatches = [regex]::Matches($preferenceSource, '(?m)^const PRIMARY_FILE: &str = "(user-preferences-v[1-9][0-9]*\.toml)";\r?$')
if ($preferenceMatches.Count -ne 1) { throw 'Cannot resolve the source-built preference owner' }
$preferenceRelativePath = 'state/' + $preferenceMatches[0].Groups[1].Value
if ([string]::IsNullOrWhiteSpace($Binary)) {
    $Binary = Join-Path $root 'target\debug\automexia.exe'
}
if (-not $PixelOracleOnly -and -not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    throw "Automexia test binary was not found at $Binary"
}
if ($OpacityOnly) {
    # Verify the actual GUI PE contract before cold HLSL compilation exercises it.
    $image = [IO.BinaryReader]::new([IO.File]::OpenRead((Resolve-Path $Binary).Path))
    try {
        $image.BaseStream.Position = 0x3c
        $pe = $image.ReadUInt32()
        $image.BaseStream.Position = $pe
        if ($image.ReadUInt32() -ne 0x4550) { throw 'Invalid native PE image' }
        $image.BaseStream.Position = $pe + 24
        if ($image.ReadUInt16() -ne 0x20b) { throw 'Opacity fixture requires a 64-bit GUI image' }
        $image.BaseStream.Position = $pe + 24 + 72
        if ($image.ReadUInt64() -ne 8388608) { throw 'GUI shader compiler stack reserve must be 8 MiB' }
    } finally { $image.Dispose() }
}

Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;
public static class AutomexiaResizeDriver {
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool MoveWindow(
        IntPtr hWnd, int x, int y, int width, int height, bool repaint);


    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(
        IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll")]
    private static extern uint MapVirtualKey(uint code, uint mapType);

    public static bool PostKeyTap(IntPtr hWnd, uint virtualKey, bool extended) {
        long scanCode = MapVirtualKey(virtualKey, 0);
        long down = 1L | (scanCode << 16);
        if (extended) {
            down |= 1L << 24;
        }
        long up = down | (1L << 30) | (1L << 31);
        return PostMessage(
                   hWnd, 0x0100, new IntPtr(virtualKey), new IntPtr(down)) &&
               PostMessage(
                   hWnd, 0x0101, new IntPtr(virtualKey), new IntPtr(up));
    }

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr hWnd, IntPtr processId);

    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AttachThreadInput(
        uint attachThread, uint attachToThread, bool attach);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool BringWindowToTop(IntPtr hWnd);

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    private static extern IntPtr GetWindow(IntPtr hWnd, uint command);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindowVisible(IntPtr hWnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsIconic(IntPtr hWnd);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr hWnd, System.Text.StringBuilder className, int maximum);

    [DllImport("user32.dll")]
    private static extern void keybd_event(
        byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);

    private static void SendKeyChange(
        uint virtualKey, bool extended, bool pressed) {
        const uint Extended = 0x0001;
        const uint KeyUp = 0x0002;
        uint flags = (extended ? Extended : 0) | (pressed ? 0 : KeyUp);
        keybd_event(
            (byte)virtualKey,
            (byte)MapVirtualKey(virtualKey, 0),
            flags,
            UIntPtr.Zero);
    }

    public static bool SendModifiedKeyTap(
        IntPtr hWnd, uint virtualKey, bool extended,
        bool control, bool shift) {
        // PostMessage does not update Windows' keyboard state, so winit cannot
        // observe modifiers from synthetic WM_KEYDOWN messages. This helper
        // drives the real foreground input path used by a physical keyboard.
        if (!ActivateWindow(hWnd)) {
            return false;
        }
        if (control) {
            SendKeyChange(0x11, false, true);
        }
        if (shift) {
            SendKeyChange(0x10, false, true);
        }
        SendKeyChange(virtualKey, extended, true);
        SendKeyChange(virtualKey, extended, false);
        if (shift) {
            SendKeyChange(0x10, false, false);
        }
        if (control) {
            SendKeyChange(0x11, false, false);
        }
        return GetForegroundWindow() == hWnd;
    }

    public static bool SendMenuEnter(IntPtr hWnd, bool repeat) {
        if (!ActivateWindow(hWnd)) return false;
        try {
            SendKeyChange(0x0D, false, true);
            // Keep the physical release behind the asynchronous child opening.
            System.Threading.Thread.Sleep(300);
            if (repeat) {
                for (int i = 0; i < 8; i++) {
                    SendKeyChange(0x0D, false, true);
                    System.Threading.Thread.Sleep(40);
                }
            }
        } finally {
            SendKeyChange(0x0D, false, false);
        }
        return GetForegroundWindow() == hWnd;
    }

    public static bool SendMenuBack(IntPtr hWnd, bool alt, bool repeat) {
        if (!ActivateWindow(hWnd)) return false;
        uint key = alt ? 0x25u : 0x08u;
        try {
            if (alt) SendKeyChange(0x12, false, true);
            SendKeyChange(key, alt, true);
            if (repeat) {
                for (int i = 0; i < 8; i++) {
                    System.Threading.Thread.Sleep(40);
                    SendKeyChange(key, alt, true);
                }
            }
        } finally {
            SendKeyChange(key, alt, false);
            if (alt) SendKeyChange(0x12, false, false);
        }
        return GetForegroundWindow() == hWnd;
    }

    [DllImport("user32.dll")]
    private static extern IntPtr GetKeyboardLayout(uint threadId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern short VkKeyScanEx(char character, IntPtr layout);

    private static NativeInput KeyboardInput(ushort virtualKey, bool release) {
        return new NativeInput {
            Type = 1,
            Data = new NativeInputData {
                Keyboard = new NativeKeyboardInput {
                    VirtualKey = virtualKey,
                    Flags = release ? 0x0002u : 0u,
                },
            },
        };
    }

    public static bool SendControlBurst(IntPtr hWnd, ushort key, int count) {
        if (count < 1 || count > 16 || !ActivateWindow(hWnd)) return false;
        var inputs = new List<NativeInput>(count * 4);
        for (int i = 0; i < count; i++) {
            inputs.Add(KeyboardInput(0x11, false));
            inputs.Add(KeyboardInput(key, false));
            inputs.Add(KeyboardInput(key, true));
            inputs.Add(KeyboardInput(0x11, true));
        }
        uint sent = SendInput((uint)inputs.Count, inputs.ToArray(),
            Marshal.SizeOf(typeof(NativeInput)));
        if (sent != inputs.Count) {
            SendInput(2, new NativeInput[] {
                KeyboardInput(key, true), KeyboardInput(0x11, true)
            }, Marshal.SizeOf(typeof(NativeInput)));
            return false;
        }
        return true;
    }

    public static bool SendRecoverySentinel(IntPtr hWnd) {
        return SendActionText(hWnd, "Write-Output RECOVERY_HISTORY_SENTINEL", true);
    }

    public static bool SendActionText(IntPtr hWnd, string text, bool enter) {
        if (text == null || text.Length > 4096) return false;
        if (!ActivateWindow(hWnd)) return false;
        IntPtr layout = GetKeyboardLayout(GetWindowThreadProcessId(hWnd, IntPtr.Zero));
        var inputs = new List<NativeInput>();
        foreach (char character in text) {
            if (char.IsControl(character)) return false;
            short mapped = VkKeyScanEx(character, layout);
            if (mapped == -1 || (mapped & 0x0600) != 0) return false;
            ushort key = (ushort)(mapped & 0xFF);
            bool shift = (mapped & 0x0100) != 0;
            if (shift) inputs.Add(KeyboardInput(0x10, false));
            inputs.Add(KeyboardInput(key, false));
            inputs.Add(KeyboardInput(key, true));
            if (shift) inputs.Add(KeyboardInput(0x10, true));
        }
        if (enter) {
            inputs.Add(KeyboardInput(0x0D, false));
            inputs.Add(KeyboardInput(0x0D, true));
        }
        if (GetForegroundWindow() != hWnd) return false;
        uint sent = SendInput((uint)inputs.Count, inputs.ToArray(), Marshal.SizeOf(typeof(NativeInput)));
        if (sent != inputs.Count) {
            var releases = new List<NativeInput>();
            foreach (NativeInput input in inputs) releases.Add(KeyboardInput(input.Data.Keyboard.VirtualKey, true));
            SendInput((uint)releases.Count, releases.ToArray(), Marshal.SizeOf(typeof(NativeInput)));
            return false;
        }
        return true;
    }

    public static bool SendProfileFixtureText(IntPtr hWnd, string text) {
        return text != null && text.Length <= 128 && SendActionText(hWnd, text, false);
    }

    public static bool ReplaceColorHex(IntPtr hWnd, string hex) {
        // Queue one real, ordered Ctrl+A and typed batch. Re-activating the
        // foreground window for every key attaches input threads, which resets
        // modifier state before winit necessarily consumes the queued events.
        if (hex == null || (hex.Length != 7 && hex.Length != 9) ||
            hex[0] != '#' ||
            !ActivateWindow(hWnd)) return false;
        IntPtr layout = GetKeyboardLayout(GetWindowThreadProcessId(hWnd, IntPtr.Zero));
        var inputs = new List<NativeInput>(80);
        var usedKeys = new HashSet<ushort>();
        inputs.Add(KeyboardInput(0x11, false));
        inputs.Add(KeyboardInput(0x41, false));
        inputs.Add(KeyboardInput(0x41, true));
        inputs.Add(KeyboardInput(0x11, true));
        foreach (char character in hex) {
            if (character != '#' && !Uri.IsHexDigit(character)) return false;
            short mapped = VkKeyScanEx(character, layout);
            if (mapped == -1) return false;
            ushort key = (ushort)(mapped & 0xFF);
            bool shift = (mapped & 0x0100) != 0;
            bool control = (mapped & 0x0200) != 0;
            bool alt = (mapped & 0x0400) != 0;
            if (control) inputs.Add(KeyboardInput(0x11, false));
            if (alt) inputs.Add(KeyboardInput(0x12, false));
            if (shift) inputs.Add(KeyboardInput(0x10, false));
            inputs.Add(KeyboardInput(key, false));
            inputs.Add(KeyboardInput(key, true));
            if (shift) inputs.Add(KeyboardInput(0x10, true));
            if (alt) inputs.Add(KeyboardInput(0x12, true));
            if (control) inputs.Add(KeyboardInput(0x11, true));
            usedKeys.Add(key);
        }
        if (GetForegroundWindow() != hWnd) return false;
        uint count = (uint)inputs.Count;
        uint sent = SendInput(count, inputs.ToArray(), Marshal.SizeOf(typeof(NativeInput)));
        if (sent != count) {
            // A partial batch must not leave a physical modifier or character
            // key down after the test reports failure.
            var releases = new List<NativeInput>();
            foreach (ushort key in usedKeys) releases.Add(KeyboardInput(key, true));
            releases.Add(KeyboardInput(0x41, true));
            releases.Add(KeyboardInput(0x10, true));
            releases.Add(KeyboardInput(0x12, true));
            releases.Add(KeyboardInput(0x11, true));
            SendInput((uint)releases.Count, releases.ToArray(),
                Marshal.SizeOf(typeof(NativeInput)));
            return false;
        }
        return GetForegroundWindow() == hWnd;
    }

    public static bool ActivateWindow(IntPtr hWnd) {
        if (GetForegroundWindow() == hWnd) return true;
        DateTime deadline = DateTime.UtcNow.AddSeconds(2);
        do {
            IntPtr foreground = GetForegroundWindow();
            uint currentThread = GetCurrentThreadId();
            uint foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, IntPtr.Zero);
            bool attached = foregroundThread != 0 &&
                foregroundThread != currentThread &&
                AttachThreadInput(currentThread, foregroundThread, true);
            try {
                BringWindowToTop(hWnd);
                SetForegroundWindow(hWnd);
            } finally {
                if (attached) {
                    AttachThreadInput(currentThread, foregroundThread, false);
                }
            }
            if (GetForegroundWindow() == hWnd) {
                return true;
            }
            System.Threading.Thread.Sleep(10);
        } while (DateTime.UtcNow < deadline);
        return false;
    }


    [StructLayout(LayoutKind.Sequential)]
    public struct Rect {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }



    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetClientRect(IntPtr hWnd, out Rect rect);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(IntPtr hWnd, out Rect rect);

    [StructLayout(LayoutKind.Sequential)]
    private struct Point {
        public int X;
        public int Y;
    }

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientToScreen(IntPtr hWnd, ref Point point);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ScreenToClient(IntPtr hWnd, ref Point point);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetCursorPos(out Point point);

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeMouseInput {
        public int X;
        public int Y;
        public uint MouseData;
        public uint Flags;
        public uint Time;
        public IntPtr ExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeKeyboardInput {
        public ushort VirtualKey;
        public ushort ScanCode;
        public uint Flags;
        public uint Time;
        public IntPtr ExtraInfo;
    }

    [StructLayout(LayoutKind.Explicit)]
    private struct NativeInputData {
        [FieldOffset(0)] public NativeMouseInput Mouse;
        [FieldOffset(0)] public NativeKeyboardInput Keyboard;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeInput {
        public uint Type;
        public NativeInputData Data;
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(
        uint count, [In] NativeInput[] inputs, int size);

    public static bool SendPhysicalLeftClick(IntPtr hWnd, int clientX, int clientY) {
        return SendPhysicalClick(hWnd, clientX, clientY, false);
    }
    public static bool SendPhysicalRightClick(IntPtr hWnd, int clientX, int clientY) {
        return SendPhysicalClick(hWnd, clientX, clientY, true);
    }
    private static bool SendPhysicalClick(IntPtr hWnd, int clientX, int clientY, bool right) {
        // WM_LBUTTONDOWN via PostMessage does not establish real OS button
        // state/capture. Use one bounded hardware-style click after checking
        // that the owned foreground window contains the physical pointer.
        if (GetForegroundWindow() != hWnd || !IsWindowVisible(hWnd)) return false;
        IntPtr previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (previous == IntPtr.Zero) return false;
        try {
            Rect client;
            Point pointer;
            if (!GetClientRect(hWnd, out client) || !GetCursorPos(out pointer) ||
                !ScreenToClient(hWnd, ref pointer) ||
                pointer.X < client.Left || pointer.X >= client.Right ||
                pointer.Y < client.Top || pointer.Y >= client.Bottom ||
                Math.Abs(pointer.X - clientX) > 2 ||
                Math.Abs(pointer.Y - clientY) > 2) return false;
            var inputs = new NativeInput[2];
            inputs[0].Type = 0;
            inputs[0].Data.Mouse.Flags = right ? 0x0008u : 0x0002u;
            inputs[1].Type = 0;
            inputs[1].Data.Mouse.Flags = right ? 0x0010u : 0x0004u;
            uint sent = SendInput(2, inputs, Marshal.SizeOf(typeof(NativeInput)));
            if (sent == 2) return true;
            if (sent == 1) {
                // Avoid leaving the real left button down on partial delivery.
                SendInput(1, new NativeInput[] { inputs[1] },
                    Marshal.SizeOf(typeof(NativeInput)));
            }
            return false;
        } finally {
            SetThreadDpiAwarenessContext(previous);
        }
    }

    private static void RequireExclusiveCaptureOwnership(IntPtr hWnd) {
        if (!ActivateWindow(hWnd)) {
            throw new InvalidOperationException(
                "Automexia could not own foreground focus for native capture");
        }

        Rect client;
        if (!GetClientRect(hWnd, out client)) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        Point origin = new Point { X = 0, Y = 0 };
        if (!ClientToScreen(hWnd, ref origin)) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        client.Left += origin.X;
        client.Right += origin.X;
        client.Top += origin.Y;
        client.Bottom += origin.Y;

        const uint PreviousWindow = 3;
        for (IntPtr other = GetWindow(hWnd, PreviousWindow);
             other != IntPtr.Zero;
             other = GetWindow(other, PreviousWindow)) {
            if (!IsWindowVisible(other) || IsIconic(other)) {
                continue;
            }
            var className = new System.Text.StringBuilder(128);
            GetClassName(other, className, className.Capacity);
            // Electron, IME, and shell infrastructure can expose visible
            // helper HWNDs above a foreground application without painting
            // into the captured region. The escaped PowerShell application
            // error is a standard native dialog (#32770), so reject that
            // enforceable class while foreground ownership covers normal
            // application windows.
            if (className.ToString() != "#32770") {
                continue;
            }
            Rect bounds;
            if (!GetWindowRect(other, out bounds)) {
                continue;
            }
            bool overlaps = client.Left < bounds.Right && bounds.Left < client.Right &&
                client.Top < bounds.Bottom && bounds.Top < client.Bottom;
            if (overlaps) {
                throw new InvalidOperationException(
                    "An unowned top-level window class " + className.ToString() +
                    " obscures the Automexia capture area");
            }
        }
    }

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetCursorPos(int x, int y);

    public static bool MovePointerToClient(IntPtr hWnd, int x, int y) {
        Point point = new Point { X = x, Y = y };
        return ClientToScreen(hWnd, ref point) && SetCursorPos(point.X, point.Y);
    }

    public static bool MovePhysicalWindow(IntPtr hWnd, int x, int y, int width, int height) {
        IntPtr previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (previous == IntPtr.Zero) return false;
        try { return MoveWindow(hWnd, x, y, width, height, true); }
        finally { SetThreadDpiAwarenessContext(previous); }
    }

    public static bool MovePhysicalPointerToClient(IntPtr hWnd, int x, int y) {
        IntPtr previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (previous == IntPtr.Zero) {
            return false;
        }
        try {
        return MovePointerToClient(hWnd, x, y);
        } finally {
            SetThreadDpiAwarenessContext(previous);
        }
    }

    public static bool PostMouseWheel(
        IntPtr hWnd, int clientX, int clientY, int delta) {
        Point point = new Point { X = clientX, Y = clientY };
        if (!ClientToScreen(hWnd, ref point)) {
            return false;
        }
        uint wheel = ((uint)(ushort)(short)delta) << 16;
        uint coordinates = (uint)(ushort)(short)point.X |
            ((uint)(ushort)(short)point.Y << 16);
        return PostMessage(
            hWnd,
            0x020A,
            new IntPtr(unchecked((int)wheel)),
            new IntPtr(unchecked((int)coordinates)));
    }

    public static void WritePreviewFixture(
        string path, int width, int height, bool jpeg) {
        using (var bitmap = new Bitmap(width, height, PixelFormat.Format32bppArgb)) {
            using (var graphics = Graphics.FromImage(bitmap)) {
                graphics.Clear(Color.FromArgb(255, 255, 226, 28));
                using (var cyan = new SolidBrush(Color.FromArgb(255, 25, 205, 255))) {
                    graphics.FillRectangle(cyan, 0, 0, width / 2, height / 2);
                    graphics.FillRectangle(
                        cyan, width / 2, height / 2,
                        width - width / 2, height - height / 2);
                }
                using (var magenta = new SolidBrush(Color.FromArgb(255, 236, 72, 153))) {
                    graphics.FillEllipse(
                        magenta, width / 4, height / 4,
                        Math.Max(4, width / 2), Math.Max(4, height / 2));
                }
                if (!jpeg) {
                    graphics.CompositingMode =
                        System.Drawing.Drawing2D.CompositingMode.SourceCopy;
                    using (var transparent = new SolidBrush(Color.FromArgb(0, 8, 24, 40))) {
                        graphics.FillRectangle(
                            transparent, 0, height / 2, width / 4, height - height / 2);
                    }
                    using (var translucent = new SolidBrush(Color.FromArgb(128, 255, 255, 255))) {
                        graphics.FillRectangle(
                            translucent, width / 4, height / 2,
                            Math.Max(4, width / 4), height - height / 2);
                    }
                }
            }
            bitmap.Save(path, jpeg ? ImageFormat.Jpeg : ImageFormat.Png);
        }
    }

    public static void WriteOpaqueWallpaperFixture(string path) {
        using (var bitmap = new Bitmap(32, 32, PixelFormat.Format32bppArgb)) {
            using (var graphics = Graphics.FromImage(bitmap)) {
                graphics.Clear(Color.FromArgb(255, 123, 77, 49));
            }
            bitmap.Save(path, ImageFormat.Png);
        }
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr GetDC(IntPtr hWnd);

    [DllImport("user32.dll")]
    private static extern int ReleaseDC(IntPtr hWnd, IntPtr hdc);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(
        IntPtr hWnd, IntPtr insertAfter, int x, int y, int width, int height, uint flags);

    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int index);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);

    [DllImport("gdi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool BitBlt(
        IntPtr destination, int x, int y, int width, int height,
        IntPtr source, int sourceX, int sourceY, uint operation);

    public sealed class FrameStats {
        public int Width;
        public int Height;
        public int SampleCount;
        public int DistinctColorBuckets;
        public int DominantColorBucket;
        public int LuminanceSpread;
        public int MaximumLuminance;
        public int MeanLuminance;
        public int MeanRed;
        public int MeanGreen;
        public int MeanBlue;
        public int BrightSampleCount;
        public int BrightForegroundSampleCount;
        public int TargetColorSampleCount;
        public long NonOpaquePixelCount;
        public string PixelDigest;
        public string DialogPixelDigest;
    }

    [DllImport("gdi32.dll", SetLastError = true)]
    private static extern IntPtr CreateCompatibleDC(IntPtr source);
    [DllImport("gdi32.dll", SetLastError = true)]
    private static extern IntPtr CreateCompatibleBitmap(IntPtr source, int width, int height);
    [DllImport("gdi32.dll", SetLastError = true)]
    private static extern IntPtr SelectObject(IntPtr dc, IntPtr value);
    [DllImport("gdi32.dll")]
    private static extern bool DeleteDC(IntPtr dc);
    [DllImport("gdi32.dll")]
    private static extern bool DeleteObject(IntPtr value);

    // Capture composited desktop RGB directly into a GDI-owned bitmap.
    // Graphics.GetHdc on a GDI+ bitmap uses a sentinel pattern; real colors
    // matching that pattern disappear during ReleaseHdc (Microsoft KB 311221).
    private static Bitmap CaptureDesktopBitmap(int x, int y, int width, int height) {
        if (width < 1 || height < 1 || (long)width * height > 67108864) {
            throw new ArgumentOutOfRangeException("width");
        }
        IntPtr screen = GetDC(IntPtr.Zero);
        IntPtr destination = IntPtr.Zero;
        IntPtr bitmap = IntPtr.Zero;
        IntPtr previous = IntPtr.Zero;
        try {
            if (screen == IntPtr.Zero) {
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            }
            destination = CreateCompatibleDC(screen);
            if (destination == IntPtr.Zero) {
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            }
            bitmap = CreateCompatibleBitmap(screen, width, height);
            if (bitmap == IntPtr.Zero) {
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            }
            previous = SelectObject(destination, bitmap);
            if (previous == IntPtr.Zero || previous == new IntPtr(-1)) {
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            }
            const uint SourceCopy = 0x00CC0020;
            const uint CaptureLayered = 0x40000000;
            if (!BitBlt(destination, 0, 0, width, height, screen, x, y, SourceCopy | CaptureLayered)) {
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            }
            // FromHbitmap copies the composited RGB before native handles retire.
            // The existing ARGB digest checks the resulting opaque PNG without
            // masking or tolerating any differences in visible RGB channels.
            return Image.FromHbitmap(bitmap);
        } finally {
            if (previous != IntPtr.Zero && previous != new IntPtr(-1)) {
                SelectObject(destination, previous);
            }
            if (bitmap != IntPtr.Zero) { DeleteObject(bitmap); }
            if (destination != IntPtr.Zero) { DeleteDC(destination); }
            if (screen != IntPtr.Zero) { ReleaseDC(IntPtr.Zero, screen); }
        }
    }

    [DllImport("gdi32.dll")]
    private static extern uint GetPixel(IntPtr dc, int x, int y);

    // Independent RGB oracle for a small, stable fixture region. This reads
    // the screen directly instead of reusing the bitmap/PNG capture path.
    public static void VerifyCapturedDesktopRegion(
        IntPtr hWnd, string path, int x, int y, int width, int height) {
        if (width < 1 || height < 1 || width > 32 || height > 32) {
            throw new ArgumentOutOfRangeException("width");
        }
        RequireExclusiveCaptureOwnership(hWnd);
        IntPtr previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (previous == IntPtr.Zero) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        IntPtr screen = IntPtr.Zero;
        try {
            Point origin = new Point { X = 0, Y = 0 };
            if (!ClientToScreen(hWnd, ref origin)) {
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            }
            screen = GetDC(IntPtr.Zero);
            if (screen == IntPtr.Zero) {
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            }
            using (var bitmap = new Bitmap(path)) {
                if (x < 0 || y < 0 || x > bitmap.Width - width || y > bitmap.Height - height) {
                    throw new ArgumentOutOfRangeException("x");
                }
                for (int py = y; py < y + height; py++) {
                    for (int px = x; px < x + width; px++) {
                        uint expected = GetPixel(screen, origin.X + px, origin.Y + py);
                        Color actual = bitmap.GetPixel(px, py);
                        uint rgb = (uint)(actual.R | actual.G << 8 | actual.B << 16);
                        if (expected == 0xffffffff || rgb != expected || actual.A != 255) {
                            throw new InvalidOperationException(
                                "Retained capture differs from desktop RGB at " + px + "," + py);
                        }
                    }
                }
            }
        } finally {
            if (screen != IntPtr.Zero) { ReleaseDC(IntPtr.Zero, screen); }
            SetThreadDpiAwarenessContext(previous);
        }
    }

    // Compositor oracle for the opacity fixture's owned solid backdrop. WGC
    // returns per-window alpha, which cannot prove desktop composition itself.
    public static int[] ReadPresentedClientPixel(IntPtr hWnd, int x, int y) {
        RequireExclusiveCaptureOwnership(hWnd);
        IntPtr previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (previous == IntPtr.Zero) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        IntPtr screen = IntPtr.Zero;
        try {
            Rect bounds;
            if (!GetClientRect(hWnd, out bounds) || x < 0 || y < 0 || x >= bounds.Right || y >= bounds.Bottom) {
                throw new ArgumentOutOfRangeException("x");
            }
            Point origin = new Point { X = x, Y = y };
            if (!ClientToScreen(hWnd, ref origin)) {
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            }
            screen = GetDC(IntPtr.Zero);
            if (screen == IntPtr.Zero) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            uint pixel = GetPixel(screen, origin.X, origin.Y);
            if (pixel == 0xffffffff) throw new InvalidOperationException("Composited pixel unavailable");
            return new int[] { (int)(pixel & 255), (int)((pixel >> 8) & 255), (int)((pixel >> 16) & 255) };
        } finally {
            if (screen != IntPtr.Zero) ReleaseDC(IntPtr.Zero, screen);
            SetThreadDpiAwarenessContext(previous);
        }
    }

    public static FrameStats CaptureClientFrame(IntPtr hWnd, string outputPath) {
        // An opt-in artifact must use physical client pixels. PowerShell is
        // normally DPI-unaware, so its logical GetClientRect dimensions crop
        // a 125%-225% display capture even though the application is correct.
        // Scope per-monitor-v2 awareness to this synchronous method only so
        // pointer-message tests retain their existing coordinate contract.
        if (!String.IsNullOrWhiteSpace(outputPath)) {
            IntPtr previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
            if (previous == IntPtr.Zero) {
                throw new System.ComponentModel.Win32Exception(
                    Marshal.GetLastWin32Error(),
                    "Could not enter per-monitor DPI awareness for retained capture");
            }
            try {
                DateTime deadline = DateTime.UtcNow.AddSeconds(3);
                FrameStats previousFrame = CaptureClientFrameCore(hWnd, null);
                bool framesEqual = false;
                do {
                    System.Threading.Thread.Sleep(25);
                    FrameStats current = CaptureClientFrameCore(hWnd, outputPath);
                    framesEqual = String.Equals(previousFrame.PixelDigest,
                        current.PixelDigest, StringComparison.Ordinal);
                    if (previousFrame.NonOpaquePixelCount == 0 &&
                        current.NonOpaquePixelCount == 0 && framesEqual) {
                        return current;
                    }
                    previousFrame = current;
                } while (DateTime.UtcNow < deadline);
                throw new InvalidOperationException(
                    "Automexia client frame did not reach two identical full-pixel captures (opaque)" +
                    "; final pair identical=" + framesEqual +
                    "; non-opaque pixels=" + previousFrame.NonOpaquePixelCount);
            } finally {
                SetThreadDpiAwarenessContext(previous);
            }
        }
        return CaptureClientFrameCore(hWnd, outputPath);
    }

    public static FrameStats CaptureStableCloseDialogFrame(IntPtr hWnd, string outputPath) {
        // The terminal cursor can blink behind the modal. Require exact pixel
        // stability in the centered dialog region without requiring the
        // unrelated prompt area to stop animating.
        IntPtr previousContext = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (previousContext == IntPtr.Zero) {
            throw new System.ComponentModel.Win32Exception(
                Marshal.GetLastWin32Error(),
                "Could not enter per-monitor DPI awareness for dialog capture");
        }
        try {
            DateTime deadline = DateTime.UtcNow.AddSeconds(3);
            FrameStats previousFrame = CaptureClientFrameCore(hWnd, null);
            do {
                System.Threading.Thread.Sleep(25);
                FrameStats current = CaptureClientFrameCore(hWnd, outputPath);
                if (previousFrame.NonOpaquePixelCount == 0 &&
                    current.NonOpaquePixelCount == 0 &&
                    String.Equals(previousFrame.DialogPixelDigest,
                        current.DialogPixelDigest, StringComparison.Ordinal)) {
                    return current;
                }
                previousFrame = current;
            } while (DateTime.UtcNow < deadline);
            throw new InvalidOperationException(
                "Automexia close dialog did not reach two identical dialog-region captures");
        } finally {
            SetThreadDpiAwarenessContext(previousContext);
        }
    }

    private static FrameStats CaptureClientFrameCore(IntPtr hWnd, string outputPath) {
        RequireExclusiveCaptureOwnership(hWnd);
        Rect rect;
        if (!GetClientRect(hWnd, out rect)) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        int width = rect.Right - rect.Left;
        int height = rect.Bottom - rect.Top;
        if (width < 1 || height < 1) {
            throw new InvalidOperationException("Automexia client frame has no drawable area");
        }

        Point origin = new Point { X = 0, Y = 0 };
        if (!ClientToScreen(hWnd, ref origin)) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }

        using (var bitmap = CaptureDesktopBitmap(origin.X, origin.Y, width, height)) {
            var buckets = new HashSet<int>();
            var bucketCounts = new Dictionary<int, int>();
            int dominantColorBucket = -1;
            int dominantColorCount = 0;
            int minimumLuminance = 255;
            int maximumLuminance = 0;
            int samples = 0;
            for (int y = 0; y < height; y += 8) {
                for (int x = 0; x < width; x += 8) {
                    Color color = bitmap.GetPixel(x, y);
                    int bucket = ((color.R >> 4) << 8) | ((color.G >> 4) << 4) | (color.B >> 4);
                    buckets.Add(bucket);
                    int count;
                    bucketCounts.TryGetValue(bucket, out count);
                    count++;
                    bucketCounts[bucket] = count;
                    if (count > dominantColorCount) {
                        dominantColorCount = count;
                        dominantColorBucket = bucket;
                    }
                    int luminance = (color.R * 54 + color.G * 183 + color.B * 19) >> 8;
                    minimumLuminance = Math.Min(minimumLuminance, luminance);
                    maximumLuminance = Math.Max(maximumLuminance, luminance);
                    samples++;
                }
            }
            long nonOpaquePixels = 0;
            string pixelDigest;
            string dialogPixelDigest;
            using (var hasher = System.Security.Cryptography.SHA256.Create())
            using (var dialogHasher = System.Security.Cryptography.SHA256.Create()) {
                byte[] dimensions = new byte[8];
                Buffer.BlockCopy(BitConverter.GetBytes(width), 0, dimensions, 0, 4);
                Buffer.BlockCopy(BitConverter.GetBytes(height), 0, dimensions, 4, 4);
                hasher.TransformBlock(dimensions, 0, dimensions.Length, null, 0);
                int dialogWidth = Math.Min(width, 640);
                int dialogHeight = Math.Min(height, 360);
                int dialogLeft = (width - dialogWidth) / 2;
                int dialogTop = (height - dialogHeight) / 2;
                dialogHasher.TransformBlock(dimensions, 0, dimensions.Length, null, 0);
                var bounds = new Rectangle(0, 0, width, height);
                BitmapData data = bitmap.LockBits(
                    bounds, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
                try {
                    byte[] row = new byte[checked(width * 4)];
                    for (int y = 0; y < height; y++) {
                        Marshal.Copy(
                            IntPtr.Add(data.Scan0, checked(y * data.Stride)),
                            row,
                            0,
                            row.Length);
                        for (int alpha = 3; alpha < row.Length; alpha += 4) {
                            if (row[alpha] != 255) {
                                nonOpaquePixels++;
                            }
                        }
                        hasher.TransformBlock(row, 0, row.Length, null, 0);
                        if (y >= dialogTop && y < dialogTop + dialogHeight) {
                            dialogHasher.TransformBlock(
                                row, dialogLeft * 4, dialogWidth * 4, null, 0);
                        }
                    }
                } finally {
                    bitmap.UnlockBits(data);
                }
                hasher.TransformFinalBlock(new byte[0], 0, 0);
                dialogHasher.TransformFinalBlock(new byte[0], 0, 0);
                pixelDigest = BitConverter.ToString(hasher.Hash).Replace("-", "");
                dialogPixelDigest = BitConverter.ToString(dialogHasher.Hash).Replace("-", "");
            }
            byte[] encoded;
            using (var stream = new MemoryStream()) {
                bitmap.Save(stream, ImageFormat.Png);
                encoded = stream.ToArray();
            }
            if (!String.IsNullOrWhiteSpace(outputPath)) {
                string directory = Path.GetDirectoryName(Path.GetFullPath(outputPath));
                if (!String.IsNullOrWhiteSpace(directory)) {
                    Directory.CreateDirectory(directory);
                }
                File.WriteAllBytes(outputPath, encoded);
            }
            return new FrameStats {
                Width = width,
                Height = height,
                SampleCount = samples,
                DistinctColorBuckets = buckets.Count,
                DominantColorBucket = dominantColorBucket,
                LuminanceSpread = maximumLuminance - minimumLuminance,
                NonOpaquePixelCount = nonOpaquePixels,
                PixelDigest = pixelDigest,
                DialogPixelDigest = dialogPixelDigest,
            };
        }
    }

    public static FrameStats CapturePhysicalClientRegionStats(
        IntPtr hWnd, int x, int y, int width, int height) {
        return CapturePhysicalClientRegionStats(
            hWnd, x, y, width, height, -1, -1, -1, 0);
    }

    public static FrameStats CapturePhysicalClientRegionStats(
        IntPtr hWnd, int x, int y, int width, int height,
        int targetRed, int targetGreen, int targetBlue, int tolerance) {
        IntPtr previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (previous == IntPtr.Zero) {
            throw new System.ComponentModel.Win32Exception(
                Marshal.GetLastWin32Error(),
                "Could not enter per-monitor DPI awareness for region capture");
        }
        try {
            return CaptureClientRegionStats(
                hWnd, x, y, width, height,
                targetRed, targetGreen, targetBlue, tolerance);
        } finally {
            SetThreadDpiAwarenessContext(previous);
        }
    }

    public static FrameStats CaptureClientRegionStats(
        IntPtr hWnd, int x, int y, int width, int height) {
        return CaptureClientRegionStats(
            hWnd, x, y, width, height, -1, -1, -1, 0);
    }

    public static FrameStats CaptureClientRegionStats(
        IntPtr hWnd, int x, int y, int width, int height,
        int targetRed, int targetGreen, int targetBlue, int tolerance) {
        if (tolerance < 0 || tolerance > 255) {
            throw new ArgumentOutOfRangeException("tolerance");
        }
        RequireExclusiveCaptureOwnership(hWnd);
        Rect rect;
        if (!GetClientRect(hWnd, out rect)) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        int clientWidth = rect.Right - rect.Left;
        int clientHeight = rect.Bottom - rect.Top;
        int left = Math.Max(0, x);
        int top = Math.Max(0, y);
        int right = Math.Min(clientWidth, x + width);
        int bottom = Math.Min(clientHeight, y + height);
        int clippedWidth = right - left;
        int clippedHeight = bottom - top;
        if (clippedWidth < 1 || clippedHeight < 1) {
            throw new InvalidOperationException("Image preview region is outside the client area");
        }

        Point origin = new Point { X = 0, Y = 0 };
        if (!ClientToScreen(hWnd, ref origin)) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }

        using (var bitmap = CaptureDesktopBitmap(
            origin.X + left, origin.Y + top, clippedWidth, clippedHeight)) {
            var buckets = new HashSet<int>();
            int minimumLuminance = 255;
            int maximumLuminance = 0;
            long luminanceTotal = 0;
            int brightSamples = 0;
            int brightForegroundSamples = 0;
            long redTotal = 0;
            long greenTotal = 0;
            long blueTotal = 0;
            int samples = 0;
            for (int py = 0; py < clippedHeight; py += 2) {
                for (int px = 0; px < clippedWidth; px += 2) {
                    Color color = bitmap.GetPixel(px, py);
                    int bucket =
                        ((color.R >> 4) << 8) |
                        ((color.G >> 4) << 4) |
                        (color.B >> 4);
                    buckets.Add(bucket);
                    int luminance =
                        (color.R * 54 + color.G * 183 + color.B * 19) >> 8;
                    minimumLuminance = Math.Min(minimumLuminance, luminance);
                    maximumLuminance = Math.Max(maximumLuminance, luminance);
                    luminanceTotal += luminance;
                    if (luminance >= 32) {
                        brightSamples++;
                    }
                    // The output-color native check needs to distinguish
                    // bright glyphs from a tinted but otherwise blank band.
                    if (luminance >= 160) {
                        brightForegroundSamples++;
                    }
                    redTotal += color.R;
                    greenTotal += color.G;
                    blueTotal += color.B;
                    samples++;
                }
            }
            return new FrameStats {
                Width = clippedWidth,
                Height = clippedHeight,
                SampleCount = samples,
                DistinctColorBuckets = buckets.Count,
                DominantColorBucket = -1,
                LuminanceSpread = maximumLuminance - minimumLuminance,
                MaximumLuminance = maximumLuminance,
                MeanLuminance = samples == 0 ? 0 : (int)(luminanceTotal / samples),
                MeanRed = samples == 0 ? 0 : (int)(redTotal / samples),
                MeanGreen = samples == 0 ? 0 : (int)(greenTotal / samples),
                MeanBlue = samples == 0 ? 0 : (int)(blueTotal / samples),
                BrightSampleCount = brightSamples,
                BrightForegroundSampleCount = brightForegroundSamples,
                TargetColorSampleCount = CountTargetPixels(bitmap, targetRed, targetGreen, targetBlue, tolerance),
            };
        }
    }

    public static bool SetCaptureTopmost(IntPtr hWnd, bool topmost) {
        IntPtr insertAfter = topmost ? new IntPtr(-1) : new IntPtr(-2);
        const uint NoMove = 0x0002;
        const uint NoSize = 0x0001;
        const uint NoActivate = 0x0010;
        const uint ShowWindow = 0x0040;
        return SetWindowPos(
            hWnd, insertAfter, 0, 0, 0, 0,
            NoMove | NoSize | NoActivate | ShowWindow);
    }

    public static bool MoveWindowTo(IntPtr hWnd, int x, int y) {
        const uint NoSize = 0x0001;
        const uint NoZOrder = 0x0004;
        const uint NoActivate = 0x0010;
        const uint ShowWindow = 0x0040;
        return SetWindowPos(
            hWnd, IntPtr.Zero, x, y, 0, 0,
            NoSize | NoZOrder | NoActivate | ShowWindow);
    }

    public static int PrimaryWidth() {
        return GetSystemMetrics(0);
    }

    public static int PrimaryHeight() {
        return GetSystemMetrics(1);
    }

    private static int CountTargetPixels(Bitmap bitmap, int red, int green, int blue, int tolerance) {
        if (red < 0 || green < 0 || blue < 0) return 0;
        int count = 0;
        // Color assertions inspect every pixel. The coarser luminance statistics
        // above are diagnostics only; a sampling grid can miss thin glyph ink.
        for (int y = 0; y < bitmap.Height; y++) {
            for (int x = 0; x < bitmap.Width; x++) {
                Color pixel = bitmap.GetPixel(x, y);
                if (Math.Abs(pixel.R - red) <= tolerance && Math.Abs(pixel.G - green) <= tolerance &&
                    Math.Abs(pixel.B - blue) <= tolerance) count++;
            }
        }
        return count;
    }

    public static void VerifyTargetColorOracle() {
        using (var bitmap = new Bitmap(4, 4)) {
            for (int y = 0; y < 4; y++) {
                for (int x = 0; x < 4; x++) {
                    bitmap.SetPixel(x, y, Color.FromArgb(90, 210, 230));
                    if (CountTargetPixels(bitmap, 90, 210, 230, 0) != 1)
                        throw new InvalidOperationException("Pixel oracle missed one physical pixel");
                    bitmap.SetPixel(x, y, Color.FromArgb(91, 210, 230));
                    if (CountTargetPixels(bitmap, 90, 210, 230, 0) != 0)
                        throw new InvalidOperationException("Pixel oracle hid a one-channel mutation");
                    bitmap.SetPixel(x, y, Color.Black);
                }
            }
        }
    }



}
'@
[AutomexiaResizeDriver]::VerifyTargetColorOracle()
if ($PixelOracleOnly) { Write-Host 'PASS: every pixel position and one-channel mutation'; return }
Add-Type -Path (Join-Path $PSScriptRoot 'windows-native-window-locator.cs')

function Assert-AutomexiaCloseSurface {
    param([IntPtr]$Window, [object]$Snapshot, [string]$Capture)
    [void][AutomexiaResizeDriver]::SetCaptureTopmost($Window, $true)
    try {
        [void][AutomexiaResizeDriver]::CaptureStableCloseDialogFrame($Window, $Capture)
        $scale = [double]$Snapshot.scale_factor
        $cardLeft = ([double]$Snapshot.window_width / $scale - 432) / 2
        $cardTop = ([double]$Snapshot.window_height / $scale - 224) / 2
        # Independent pixel oracles for the normal card and both button fills.
        # No field, selection border or covered label may leak into these areas.
        # The fixture uses the default #020B16 palette. Theme-aware dialogs use
        # that opaque background and a 3.5%-lightened surface for BOTH buttons;
        # destructive intent is conveyed by the close button's red outline.
        # Keep literal RGB/spread checks rather than sampling the renderer's state.
        foreach ($probe in @(
            @{ Name = 'card'; X = 32; Y = 120; W = 368; H = 24; RGB = @(2, 11, 22) },
            @{ Name = 'cancel'; X = 32; Y = 174; W = 8; H = 12; RGB = @(10, 19, 30) },
            @{ Name = 'close'; X = 232; Y = 174; W = 8; H = 12; RGB = @(10, 19, 30) }
        )) {
            $surface = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
                $Window, [int](($cardLeft + $probe.X) * $scale), [int](($cardTop + $probe.Y) * $scale),
                [int]($probe.W * $scale), [int]($probe.H * $scale))
            if ($surface.SampleCount -lt 20 -or $surface.LuminanceSpread -gt 2 -or
                [Math]::Abs($surface.MeanRed - $probe.RGB[0]) -gt 3 -or
                [Math]::Abs($surface.MeanGreen - $probe.RGB[1]) -gt 3 -or
                [Math]::Abs($surface.MeanBlue - $probe.RGB[2]) -gt 3) {
                throw "Close dialog $($probe.Name) is not opaque: RGB=$($surface.MeanRed)/$($surface.MeanGreen)/$($surface.MeanBlue), spread=$($surface.LuminanceSpread)"
            }
        }
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($Window, $false)
    }
}

function Get-ActiveAutomexiaPanel {
    param($Snapshot)
    return @($Snapshot.panels | Where-Object { [bool]$_.active })[0]
}

function Get-AutomexiaPaintedRow {
    param([object]$Panel, [int]$SourceRow)
    if ($null -eq $Panel.source_row_visual_origins -or $SourceRow -lt 0) {
        throw 'Native panel is missing painted row geometry'
    }
    $origins = @($Panel.source_row_visual_origins)
    if ($SourceRow -ge $origins.Count -or $null -eq $origins[$SourceRow]) {
        throw 'Native source row is hidden or outside the painted viewport'
    }
    $visual = [int]$origins[$SourceRow]
    if ($visual -lt 0 -or $visual -ge $origins.Count) {
        throw 'Native painted row is outside the viewport'
    }
    return $visual
}

function Test-AllAutomexiaPaneContexts {
    param($Snapshot)
    if ([int]$Snapshot.panel_count -le 0 -or
        @($Snapshot.panels).Count -ne [int]$Snapshot.panel_count) {
        return $false
    }
    foreach ($panel in @($Snapshot.panels)) {
        if ($null -eq $panel.context_session_id -or
            [int64]$panel.context_session_id -ne [int64]$panel.route_id -or
            @($panel.context_segments).Count -eq 0) {
            return $false
        }
    }
    return $true
}

function Assert-AutomexiaCommandResultPaintIsolation {
    param(
        [object]$Snapshot,
        [string]$Stage,
        [int]$MinimumPaints = 1
    )

    $paints = @($Snapshot.command_result_paints)
    if ($paints.Count -lt $MinimumPaints) {
        throw "$Stage published $($paints.Count) command-result paints; expected at least $MinimumPaints"
    }
    $scale = [Math]::Max(0.01, [double]$Snapshot.scale_factor)
    $logicalWidth = [double]$Snapshot.window_width / $scale
    $logicalHeight = [double]$Snapshot.window_height / $scale
    $promptPaints = @($Snapshot.prompt_context_paints)
    foreach ($promptPaint in $promptPaints) {
        $prompt = @($promptPaint)
        $promptRect = @($prompt[2])
        if ($prompt.Count -ne 3 -or $promptRect.Count -ne 4 -or
            @($promptRect | Where-Object {
                [double]::IsNaN([double]$_) -or
                    [double]::IsInfinity([double]$_)
            }).Count -ne 0 -or
            [double]$promptRect[0] -lt 0.0 -or [double]$promptRect[1] -lt 0.0 -or
            [double]$promptRect[2] -le 0.0 -or [double]$promptRect[3] -le 0.0 -or
            [double]$promptRect[0] + [double]$promptRect[2] -gt $logicalWidth + 1.0 -or
            [double]$promptRect[1] + [double]$promptRect[3] -gt $logicalHeight + 1.0) {
            throw "$Stage published malformed prompt-context paint geometry"
        }
    }
    $identities = [Collections.Generic.HashSet[string]]::new()
    for ($leftIndex = 0; $leftIndex -lt $paints.Count; $leftIndex++) {
        $left = @($paints[$leftIndex])
        $leftRect = @($left[2])
        if ($left.Count -ne 3 -or $leftRect.Count -ne 4 -or
            @($leftRect | Where-Object {
                [double]::IsNaN([double]$_) -or
                    [double]::IsInfinity([double]$_)
            }).Count -ne 0 -or
            [double]$leftRect[0] -lt 0.0 -or [double]$leftRect[1] -lt 0.0 -or
            [double]$leftRect[2] -le 0.0 -or [double]$leftRect[3] -le 0.0 -or
            [double]$leftRect[0] + [double]$leftRect[2] -gt $logicalWidth + 1.0 -or
            [double]$leftRect[1] + [double]$leftRect[3] -gt $logicalHeight + 1.0) {
            throw "$Stage published malformed command-result paint geometry"
        }
        if (-not $identities.Add([string]$left[1])) {
            throw "$Stage painted command-result identity $($left[1]) more than once"
        }
        for ($rightIndex = $leftIndex + 1; $rightIndex -lt $paints.Count; $rightIndex++) {
            $right = @($paints[$rightIndex])
            $rightRect = @($right[2])
            if ($right.Count -ne 3 -or $rightRect.Count -ne 4) {
                throw "$Stage published malformed command-result paint geometry"
            }
            $horizontalOverlap =
                [double]$leftRect[0] -lt [double]$rightRect[0] + [double]$rightRect[2] -and
                [double]$rightRect[0] -lt [double]$leftRect[0] + [double]$leftRect[2]
            $verticalOverlap =
                [double]$leftRect[1] -lt [double]$rightRect[1] + [double]$rightRect[3] -and
                [double]$rightRect[1] -lt [double]$leftRect[1] + [double]$leftRect[3]
            if ($horizontalOverlap -and $verticalOverlap) {
                throw "$Stage overlapped command-result identities $($left[1]) and $($right[1])"
            }
        }
        foreach ($promptPaint in $promptPaints) {
            $promptRect = @($promptPaint[2])
            $horizontalOverlap =
                [double]$leftRect[0] -lt [double]$promptRect[0] + [double]$promptRect[2] -and
                [double]$promptRect[0] -lt [double]$leftRect[0] + [double]$leftRect[2]
            $verticalOverlap =
                [double]$leftRect[1] -lt [double]$promptRect[1] + [double]$promptRect[3] -and
                [double]$promptRect[1] -lt [double]$leftRect[1] + [double]$leftRect[3]
            if ($horizontalOverlap -and $verticalOverlap) {
                throw "$Stage overlaps command-result identity $($left[1]) with prompt context"
            }
        }
    }
}

function Get-AutomexiaDescendantProcessIds {
    param([int]$RootProcessId)

    $processes = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $frontier = @($RootProcessId)
    $seen = [Collections.Generic.HashSet[int]]::new()
    while ($frontier.Count -gt 0) {
        $parent = [int]$frontier[0]
        if ($frontier.Count -eq 1) {
            $frontier = @()
        } else {
            $frontier = @($frontier[1..($frontier.Count - 1)])
        }
        foreach ($child in @($processes | Where-Object { [int]$_.ParentProcessId -eq $parent })) {
            $childId = [int]$child.ProcessId
            if ($seen.Add($childId)) {
                $frontier += $childId
            }
        }
    }
    return @($seen | Sort-Object)
}

function Get-AutomexiaOwnedProcessIds {
    param(
        [int]$RootProcessId,
        [string]$FixtureConfigRoot
    )

    $owned = [Collections.Generic.HashSet[int]]::new()
    foreach ($processId in @(Get-AutomexiaDescendantProcessIds $RootProcessId)) {
        [void]$owned.Add([int]$processId)
    }
    if (-not [string]::IsNullOrWhiteSpace($FixtureConfigRoot)) {
        foreach ($candidate in @(Get-CimInstance Win32_Process |
                Select-Object ProcessId, CommandLine)) {
            if ($null -ne $candidate.CommandLine -and
                $candidate.CommandLine.IndexOf(
                    $FixtureConfigRoot,
                    [StringComparison]::OrdinalIgnoreCase) -ge 0) {
                [void]$owned.Add([int]$candidate.ProcessId)
            }
        }
    }
    return @($owned | Sort-Object)
}

function Get-AutomexiaDescendantCount {
    param([int]$RootProcessId)

    return @(Get-AutomexiaOwnedProcessIds $RootProcessId $configRoot).Count
}

function Get-AutomexiaResourceSample {
    param([Diagnostics.Process]$AutomexiaProcess)

    $AutomexiaProcess.Refresh()
    if ($AutomexiaProcess.HasExited) {
        throw "Automexia exited while collecting resources during $script:testStage"
    }
    return [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        handle_count = [int64]$AutomexiaProcess.HandleCount
        thread_count = [int64]$AutomexiaProcess.Threads.Count
        private_bytes = [int64]$AutomexiaProcess.PrivateMemorySize64
        working_set_bytes = [int64]$AutomexiaProcess.WorkingSet64
        descendant_process_count = [int64](Get-AutomexiaDescendantCount $AutomexiaProcess.Id)
    }
}

function Test-AutomexiaImageResources {
    param(
        [object]$Snapshot,
        [bool]$Visible,
        [bool]$CpuRenderer,
        [int]$ExpectedCacheEntries,
        [int64]$ExpectedCacheBytes
    )

    $previewState = $Snapshot.image_preview
    if ($null -eq $previewState -or
        $null -eq $previewState.pixel_entries -or
        $null -eq $previewState.overlay_entries -or
        $null -eq $previewState.texture_entries -or
        $null -eq $previewState.texture_bytes -or
        $null -eq $previewState.thumbnail_cache_entries -or
        $null -eq $previewState.thumbnail_cache_bytes -or
        $null -eq $previewState.queued_requests -or
        $null -eq $previewState.completion_pending) {
        return $false
    }

    if ([int]$previewState.thumbnail_cache_entries -ne $ExpectedCacheEntries -or
        [int64]$previewState.thumbnail_cache_bytes -ne $ExpectedCacheBytes -or
        [int]$previewState.queued_requests -ne 0 -or
        [bool]$previewState.completion_pending) {
        return $false
    }

    if (-not $Visible) {
        return (
            -not [bool]$previewState.visible -and
            -not [bool]$previewState.overlay_present -and
            [int]$previewState.pixel_entries -eq 0 -and
            [int]$previewState.overlay_entries -eq 0 -and
            [int]$previewState.texture_entries -eq 0 -and
            [int64]$previewState.texture_bytes -eq 0)
    }

    if (-not [bool]$previewState.visible -or
        -not [bool]$previewState.overlay_present -or
        [int]$previewState.pixel_entries -ne 1 -or
        [int]$previewState.overlay_entries -ne 1) {
        return $false
    }
    if ($CpuRenderer) {
        return (
            [int]$previewState.texture_entries -eq 0 -and
            [int64]$previewState.texture_bytes -eq 0)
    }
    $dimensions = @($previewState.decoded_dimensions)
    if ($dimensions.Count -ne 2) {
        return $false
    }
    $expectedTextureBytes =
        [int64]$dimensions[0] * [int64]$dimensions[1] * 4
    # Snapshots are published before this frame's renderer preparation. The
    # initial pixel-fidelity check proves WGPU presentation; repeated lifecycle
    # samples may therefore observe either the bounded pre-upload state or the
    # exact settled texture, while dismissal still requires exact zero above.
    return (
        ([int]$previewState.texture_entries -eq 0 -and
         [int64]$previewState.texture_bytes -eq 0) -or
        ([int]$previewState.texture_entries -eq 1 -and
         [int64]$previewState.texture_bytes -eq $expectedTextureBytes))
}

function Wait-AutomexiaWindowCount {
    param(
        [int]$Expected,
        [int]$TimeoutMilliseconds = 15000
    )

    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        $process.Refresh()
        if ($process.HasExited) {
            throw "Automexia exited while waiting for $Expected visible windows during $script:testStage"
        }
        $windows = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id))
        if ($windows.Count -eq $Expected) {
            return $windows
        }
        Start-Sleep -Milliseconds 25
    } while ([DateTime]::UtcNow -lt $deadline)

    $windows = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id))
    $descriptions = @($windows | ForEach-Object { [AutomexiaNativeWindowLocator]::DescribeWindow($_) }) -join "; "
    throw "Expected $Expected visible Automexia windows during $script:testStage, found $($windows.Count): $descriptions"
}

$snapshotPath = Join-Path ([System.IO.Path]::GetTempPath()) (
    'automexia-resize-{0}.json' -f [guid]::NewGuid().ToString('N'))
$controlPath = Join-Path ([System.IO.Path]::GetTempPath()) (
    'automexia-control-{0}.txt' -f [guid]::NewGuid().ToString('N'))
$configRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
    'automexia-resize-config-{0}' -f [guid]::NewGuid().ToString('N'))
$previousSnapshot = $env:AUTOMEXIA_RESIZE_SNAPSHOT
$previousControl = $env:AUTOMEXIA_NATIVE_TEST_CONTROL
$previousConfigHome = $env:AUTOMEXIA_CONFIG_HOME
$previousVisualFixture = $env:AUTOMEXIA_VISUAL_TEST_FIXTURE
$process = $null
$window = [IntPtr]::Zero
$lastSnapshot = $null
$testStage = 'startup'
$ownedDescendantsAtShutdown = @()
$reportPath = $null
$report = $null

function Read-AutomexiaSnapshot {
    param(
        [int64]$AfterSequence = -1,
        [int]$TimeoutMilliseconds = 10000
    )

    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        if (Test-Path -LiteralPath $snapshotPath -PathType Leaf) {
            try {
                $snapshot = [IO.File]::ReadAllText(
                    $snapshotPath,
                    [Text.Encoding]::UTF8) | ConvertFrom-Json
                $script:lastSnapshot = $snapshot
                if ([int64]$snapshot.sequence -gt $AfterSequence) {
                    return $snapshot
                }
            } catch {
                # The render thread may be replacing this small JSON payload.
            }
        }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $deadline)

    if ($null -ne $process) {
        $process.Refresh()
        Write-Host "Automexia process exited: $($process.HasExited)"
    }
    Write-Host "Native resize test stage: $script:testStage"
    if ($null -ne $script:lastSnapshot) {
        Write-Host ($script:lastSnapshot | ConvertTo-Json -Depth 8)
    } elseif (Test-Path -LiteralPath $snapshotPath -PathType Leaf) {
        Write-Host 'Last raw renderer snapshot:'
        Write-Host ([IO.File]::ReadAllText($snapshotPath, [Text.Encoding]::UTF8))
    }
    throw "Timed out waiting for an Automexia renderer snapshot after sequence $AfterSequence during $script:testStage"
}

function Send-AutomexiaTestControl {
    param([string]$Control)
    $stagedControl = Join-Path (
        [System.IO.Path]::GetDirectoryName($controlPath)) (
        '.automexia-control-{0}.tmp' -f [guid]::NewGuid().ToString('N'))
    try {
        [System.IO.File]::WriteAllText(
            $stagedControl,
            $Control,
            [System.Text.UTF8Encoding]::new($false))
        [System.IO.File]::Delete($controlPath)
        [System.IO.File]::Move($stagedControl, $controlPath)
    } finally {
        [System.IO.File]::Delete($stagedControl)
    }
    if ($window -ne [IntPtr]::Zero) {
        # WM_PAINT is posted asynchronously. It wakes the feature-gated control
        # reader without adding a synchronous resize to the latency result.
        [void][AutomexiaResizeDriver]::PostMessage(
            $window, 0x000F, [IntPtr]::Zero, [IntPtr]::Zero)
    }
}

try {
    [void](New-Item -ItemType Directory -Path $configRoot)
    # Exercise the installed flat layout, not repository-only relative fallbacks.
    # CMD identity contains account-specific Base64 values generated by the
    # installer, so the native test must reproduce that exact deployed contract.
    $integrationRoot = Join-Path $configRoot 'shell-integration'
    [void](New-Item -ItemType Directory -Path $integrationRoot)
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\powershell\automexia.ps1') -Destination $integrationRoot
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\powershell\automexia.format.ps1xml') -Destination $integrationRoot
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\cmd\automexia-ls.cmd') -Destination $integrationRoot
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\cmd\automexia-ls.ps1') -Destination $integrationRoot
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\cmd\automexia-alias-loader.ps1') -Destination $integrationRoot

    $cmdSource = [IO.File]::ReadAllText(
        (Join-Path $root 'shell-integration\cmd\automexia.cmd'),
        [Text.Encoding]::UTF8)
    $cmdSource = $cmdSource.Replace(
        '__AUTOMEXIA_CMD_USER_BASE64__',
        [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([Environment]::UserName)))
    $cmdSource = $cmdSource.Replace(
        '__AUTOMEXIA_CMD_PATH_BASE64__',
        [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($env:ComSpec)))
    $cmdSource = [regex]::Replace($cmdSource, "\r?\n", [Environment]::NewLine)
    [IO.File]::WriteAllText(
        (Join-Path $integrationRoot 'automexia.cmd'),
        $cmdSource,
        [Text.Encoding]::ASCII)

    $cmdListingFixture = Join-Path $configRoot 'cmd-listing-fixture'
    [void](New-Item -ItemType Directory -Path $cmdListingFixture)
    [void](New-Item -ItemType Directory -Path (Join-Path $cmdListingFixture 'apps'))
    [IO.File]::WriteAllText(
        (Join-Path $cmdListingFixture 'Cargo.toml'),
        '[workspace]',
        [Text.Encoding]::ASCII)

    $integration = (Join-Path $integrationRoot 'automexia.ps1').Replace('\', '/')
    $rendererConfig = if ($UseCpuRenderer) {
        "`n[renderer]`nuse-cpu = true`n"
    } else {
        ''
    }
    $wallpaperConfig = ''
    if ($OutputColorsOnly -and -not $UseCpuRenderer) {
        $wallpaperPath = Join-Path $configRoot 'opaque-output-wallpaper.png'
        [AutomexiaResizeDriver]::WriteOpaqueWallpaperFixture($wallpaperPath)
        $wallpaperConfigPath = $wallpaperPath.Replace('\', '/')
        $wallpaperConfig = "`n[window]`nbackground-image = { path = `"$wallpaperConfigPath`", opacity = 1.0 }`n"
    }
    if ($OpacityOnly) {
        $opacityLiteral = $InitialWindowOpacity.ToString([Globalization.CultureInfo]::InvariantCulture)
        $wallpaperConfig = "`n[window]`nopacity = $opacityLiteral`nblur = false`n[colors]`nbackground = '#406080'`n"
    }
    $shellProgram = if ($CommandInputColorsOnly -and $CommandInputPowerShell7) { 'pwsh.exe' } else { 'powershell.exe' }
    $inputHistorySetup = ''
    if ($CommandInputColorsOnly) {
        $inputHistory = (Join-Path $configRoot 'input-history.txt').Replace('\', '/')
        $inputHistorySetup = "; Set-PSReadLineOption -HistorySavePath '$inputHistory' -HistorySaveStyle SaveNothing"
    }
    $config = @"
confirm-before-quit = true

[shell]
program = "$shellProgram"
args = ["-NoLogo", "-NoProfile", "-NoExit", "-Command", ". '$integration'$inputHistorySetup"]
$rendererConfig
$wallpaperConfig
"@
    if ($TagShapesOnly) {
        # Solid backgrounds make reference silhouettes unambiguous in captures.
        # This applies only to the fixture's disposable configuration root.
        $config += "`n[presentation.tags]`nstyle = 'tinted'`nopacity = 100`n"
    }
    if ($ThemeGalleryOnly) {
        # Exercise dense table decoration in each applied dark/light theme.
        # These choices belong only to this disposable fixture configuration.
        $config += "`n[presentation]`ncommand-output-highlighting = false`noutput-highlighting = false`n[presentation.tables]`nborder-style = 'dashed'`nborder-weight = 'thick'`nbanding = 'columns'`nheader-bold = true`n"
    }
    if ($CommandInputColorsOnly) {
        # Literal palette oracle for Fish's native ANSI styles. It must not be
        # replaced by the terminal's purple plain-input fallback.
        # Moderate saturation keeps the literal pixel tolerance useful across
        # the native compositor's color-space conversion.
        $config += "`n[colors]`nyellow = '#B4D2B4'`ncyan = '#B4B4D2'`n"
    }
    if ($PlainOutputColorsOnly) {
        # Fixed non-default colours prove real output consumes customization,
        # independently of the current desktop theme or shell's own palette.
        $config += "`n[presentation]`ninline-tables = false`ncommand-output-highlighting = false`nkubernetes-highlighting = false`n[presentation.highlight]`nstyle = 'foreground'`n[presentation.highlight.colors]`ninfo = '#5AD2E6'`nerror = '#E6646E'`nwarning = '#E6BE64'`nsuccess = '#64DC96'`ndebug = '#8CA0DC'`n"
    }
    if ($SessionRecoveryOnly) {
        # The fixture owns an explicit starting directory even before inactive
        # shells publish their first complete metadata frame.
        $recoveryDirectory = $root.Replace('\', '/').Replace('"', '\"')
        $config = "working-dir = `"$recoveryDirectory`"`n" + $config
    } else { $config += "`n[session-recovery]`nenabled = false`n" }
    [System.IO.File]::WriteAllText(
        (Join-Path $configRoot 'config.toml'),
        $config,
        [System.Text.UTF8Encoding]::new($false))

    $env:AUTOMEXIA_RESIZE_SNAPSHOT = $snapshotPath
    $env:AUTOMEXIA_NATIVE_TEST_CONTROL = $controlPath
    $env:AUTOMEXIA_CONFIG_HOME = $configRoot
    $env:AUTOMEXIA_VISUAL_TEST_FIXTURE = 's1-standard-v1'
    $process = Start-Process -FilePath $Binary -WorkingDirectory $root -WindowStyle Hidden -PassThru

    # Process.MainWindowHandle can transiently select Winit's internal event
    # target because it is created before Automexia's titled application HWND.
    # Enumerate process windows and exclude that infrastructure window so all
    # resize, input, frame, and close assertions target the real terminal.
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    $applicationWindows = @()
    do {
        Start-Sleep -Milliseconds 50
        $process.Refresh()
        if (-not $process.HasExited) {
            $applicationWindows = @(
                [AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id))
        }
    } while ($applicationWindows.Count -eq 0 -and
             -not $process.HasExited -and
             [DateTime]::UtcNow -lt $deadline)
    if ($process.HasExited) {
        throw "Automexia exited before its native window became ready (exit $($process.ExitCode))"
    }
    if ($applicationWindows.Count -ne 1) {
        throw "Automexia exposed $($applicationWindows.Count) application windows during startup; expected 1"
    }
    $window = $applicationWindows[0]

    # A native window can be drawable before PowerShell has emitted its first
    # prompt. Wait for the prompt without sending input so this test also proves
    # that startup metadata/path discovery is automatic rather than action-led.
    $script:testStage = 'initial prompt'
    $initial = Read-AutomexiaSnapshot
    $promptDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (($null -eq $initial.latest_prompt_id -or
            [int]$initial.latest_prompt_start_count -ne 1 -or
            -not [bool]$initial.full_path_visible) -and
           [DateTime]::UtcNow -lt $promptDeadline) {
        $initial = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
    }
    if ($null -eq $initial.latest_prompt_id -or
        [int]$initial.latest_prompt_start_count -ne 1 -or
        -not [bool]$initial.full_path_visible) {
        Write-Host ($initial | ConvertTo-Json -Depth 4)
        throw 'PowerShell did not publish one complete prompt automatically after startup'
    }
    if ([string]$initial.visual_test_fixture -ne 's1-standard-v1' -or
        [string]$initial.visual_test_clock -ne '12:34' -or
        [bool]$initial.visual_test_animations_enabled) {
        Write-Host ($initial | ConvertTo-Json -Depth 4)
        throw 'The deterministic S1 visual fixture did not freeze clock and animation state'
    }
    if ($SessionRecoveryOnly) {
        . (Join-Path $PSScriptRoot 'session-recovery-windows.ps1')
        return
    }
    if ($QuickActionsOnly) {
        . (Join-Path $PSScriptRoot 'quick-actions-windows.ps1')
        return
    }
    if ($AccessibilityOnly) {
        . (Join-Path $PSScriptRoot 'accessibility-windows.ps1')
        return
    }
    $expectedRendererBackend = if ($UseCpuRenderer) { 'cpu' } else { 'wgpu' }
    if ([string]$initial.renderer_backend -ne $expectedRendererBackend) {
        throw "Native renderer backend mismatch: requested $expectedRendererBackend, actual $($initial.renderer_backend)"
    }

    $initialPanel = Get-ActiveAutomexiaPanel $initial
    # The first prompt can precede the background visual-fixture publication.
    # Require the complete fixed context before using it as the later CMD
    # comparison oracle; comparing a partial startup snapshot races discovery.
    # Namespace freshness remains metadata; the visible label has no decoration.
    $fixtureSegments = @('main', 'platform', 'eu-west-1', 'local', 'workspace', 'demo')
    $fixtureDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (@($fixtureSegments | Where-Object { $_ -notin @($initialPanel.context_segments) }).Count -gt 0 -and
           [DateTime]::UtcNow -lt $fixtureDeadline) {
        $initial = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
        $initialPanel = Get-ActiveAutomexiaPanel $initial
    }
    if (@($fixtureSegments | Where-Object { $_ -notin @($initialPanel.context_segments) }).Count -gt 0) {
        throw 'The complete deterministic context fixture was not published after startup'
    }
    $expectedContextSegmentsJson =
        @($initialPanel.context_segments) | ConvertTo-Json -Compress
    if ($null -eq $initialPanel -or [int]$initial.panel_count -ne 1) {
        throw 'The initial native session snapshot is incomplete'
    }
    if ([int64]$initialPanel.shell_pid -le 0) {
        throw 'The initial ConPTY child process ID was not recorded'
    }
    $blankPromptLine = [string]$initialPanel.raw_cursor_line_text
    $script:testStage = 'initial resource baseline'
    $resourceBaseline = Get-AutomexiaResourceSample $process

    if ($CommandInputColorsOnly) {
        . (Join-Path $PSScriptRoot 'command-input-colors-windows.ps1')
        Test-AutomexiaCommandInputColors
        return
    }
    if ($OutputColorsOnly) {
        . (Join-Path $PSScriptRoot 'output-colors-windows.ps1')
        Test-AutomexiaOutputColors
        return
    }

    if ($NamedProfilesOnly) {
        . (Join-Path $PSScriptRoot 'named-profiles-windows.ps1')
        Test-AutomexiaNamedProfiles
        return
    }
    if ($ThemeGalleryOnly) {
        . (Join-Path $PSScriptRoot 'theme-gallery-windows.ps1')
        Test-AutomexiaThemeGallery
        return
    }
    if ($LiveBackdropOnly) {
        . (Join-Path $PSScriptRoot 'live-backdrop-windows.ps1')
        Test-AutomexiaLiveBackdrop
        return
    }
    if ($DependentControlsOnly) {
        . (Join-Path $PSScriptRoot 'dependent-controls-windows.ps1')
        Test-AutomexiaDependentControls
        return
    }
    if ($OpacityOnly) {
        . (Join-Path $PSScriptRoot 'dependent-controls-windows.ps1')
        Test-AutomexiaLiveInterface -OpacityChecks
        return
    }
    if ($TerminalAppearanceOnly) {
        . (Join-Path $PSScriptRoot 'dependent-controls-windows.ps1')
        Test-AutomexiaLiveInterface
        return
    }
    if ($FontPickerOnly) {
        . (Join-Path $PSScriptRoot 'font-picker-windows.ps1')
        Test-AutomexiaFontPicker
        return
    }
    if ($SharedColorPickerOnly) {
        . (Join-Path $PSScriptRoot 'shared-color-picker-windows.ps1')
        Test-AutomexiaSharedColorPicker
        return
    }
    if ($WindowControlChoicesOnly) {
        . (Join-Path $PSScriptRoot 'window-control-choices-windows.ps1')
        Test-AutomexiaWindowControlChoices
        return
    }
    if ($TagCustomizationOnly) {
        $script:testStage = 'native information tag selection'
        function Wait-TagState([scriptblock]$Predicate) {
            $deadline = [DateTime]::UtcNow.AddSeconds(10)
            do {
                $state = Read-AutomexiaSnapshot
                if (& $Predicate $state) { return $state }
                Start-Sleep -Milliseconds 40
            } while ([DateTime]::UtcNow -lt $deadline)
            Write-Host ($state.settings | ConvertTo-Json -Depth 5 -Compress)
            if ($MenuNavigationOnly) {
                Write-Host ("Menu state: enabled={0}; summary={1}" -f $state.palette_enabled, $state.palette_accessibility_summary)
            }
            throw "Native tag customization did not reach the expected state during $script:testStage"
        }
        function Click-TagBounds($Bounds) {
            # Use the same physical-input owner as the output-color fixture.
            # Posting a second WM_MOUSEMOVE can race the real DPI-scaled move,
            # especially when successive dialogs share a button position.
            $x = [int][Math]::Round([double]$Bounds[0] + [double]$Bounds[2] * 0.5)
            $y = [int][Math]::Round([double]$Bounds[1] + [double]$Bounds[3] * 0.5)
            $scale = [double](Read-AutomexiaSnapshot).scale_factor
            $physicalX = [int][Math]::Round($x * $scale)
            $physicalY = [int][Math]::Round($y * $scale)
            if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
                -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $physicalX + 6, $physicalY)) {
                throw 'Could not move the native tag pointer'
            }
            $null = Wait-TagState { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - ($physicalX + 6) / $scale) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y) -le 1 }
            if (-not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $physicalX, $physicalY)) { throw 'Could not settle the native tag pointer' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - $x) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y) -le 1 }
            if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $physicalX, $physicalY)) {
                throw 'Native tag click lost foreground or physical client ownership'
            }
        }
        function Confirm-TagAction([switch]$CheckCancel) {
            $confirmation = Wait-TagState { param($s) $s.settings.ready -and $null -ne $s.settings.confirmation }
            if ($confirmation.settings.confirmation.accept_selected) { throw 'Confirmation must initially select Cancel' }
            if ((Get-CustomizationFixtureHashes) -ne $savedHashes) { throw 'Opening confirmation changed saved files' }
            if ($CheckCancel) {
                $before = $confirmation.settings.temporary_defaults
                if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x1B, $false, $false, $false)) { throw 'Confirmation Escape failed' }
                $null = Wait-TagState { param($s) $s.settings.ready -and $null -eq $s.settings.confirmation -and $s.settings.temporary_defaults -eq $before }
                if ((Get-CustomizationFixtureHashes) -ne $savedHashes) { throw 'Cancel changed saved files' }
                if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x52, $false, $false, $false)) { throw 'Reset shortcut failed' }
                $confirmation = Wait-TagState { param($s) $s.settings.ready -and $null -ne $s.settings.confirmation }
            }
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
                [void][IO.Directory]::CreateDirectory($captureRoot)
                $name = if ($confirmation.settings.confirmation.title -like 'Restore*') { 'settings-restore-confirm.png' } else { 'settings-reset-confirm.png' }
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot $name))
            }
            Click-TagBounds $confirmation.settings.confirmation.accept
            $null = Wait-TagState { param($s) $s.settings.ready -and $null -eq $s.settings.confirmation }
        }
        function Assert-DevOpsTagsHidden($State) {
            $corePages = @('tags.slot.ubuntu-wsl.page', 'tags.slot.windows.page', 'tags.slot.git.page', 'tags.slot.user.page', 'tags.add-slot')
            foreach ($target in $State.settings.targets) {
                if ($target.id -notin $corePages) { throw "DevOps off left a preview target for $($target.id)" }
            }
            foreach ($page in $corePages) {
                if (@($State.settings.targets | Where-Object { $_.roster -and $_.id -eq $page }).Count -ne 1) {
                    throw "DevOps off removed core roster target $page"
                }
            }
        }
        [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1200, 780, $true)
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)
        try {
            if ($MenuNavigationOnly) {
                function Send-MenuKey([int]$Key, [bool]$Alt = $false, [bool]$Repeat = $false) {
                    $sent = if ($Alt) { [AutomexiaResizeDriver]::SendMenuBack($window, $true, $false) }
                        elseif ($Key -eq 0x0D) { [AutomexiaResizeDriver]::SendMenuEnter($window, $Repeat) }
                        else { [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, ($Key -eq 0x25), $false, $false) }
                    if (-not $sent) {
                        throw 'Menu fixture lost foreground input ownership'
                    }
                }
                $script:testStage = 'menu navigation baseline'
                function Get-MenuPreferenceHashes {
                    foreach ($relative in @('config.toml', $preferenceRelativePath)) {
                        $path = Join-Path $configRoot $relative
                        if (Test-Path -LiteralPath $path) { (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash }
                        else { 'absent' }
                    }
                }
                $menuHashes = (Get-MenuPreferenceHashes) -join ':'
                $baseline = Read-AutomexiaSnapshot
                $baselinePanel = Get-ActiveAutomexiaPanel $baseline
                Send-AutomexiaTestControl 'open-palette:menu-reentry'
                $null = Wait-TagState { param($s) $s.palette_enabled -and $s.palette_total_results -eq 7 }
                for ($index = 0; $index -lt 5; $index++) { Send-MenuKey 0x28 }
                $null = Wait-TagState { param($s) $s.palette_selected_index -eq 5 }
                Send-MenuKey 0x0D
                $null = Wait-TagState { param($s) $s.palette_enabled -and $s.palette_total_results -eq 2 -and $s.palette_selected_index -eq 1 }
                foreach ($back in @('Backspace', 'AltLeft', 'Escape', 'Backspace')) {
                    $script:testStage = "first Enter opens customizations before $back"
                    Send-MenuKey 0x0D $false $true
                    $opened = Wait-TagState { param($s) $s.settings.ready -and -not $s.palette_enabled }
                    if ($null -ne $opened.settings.active_category) { throw 'Held Enter activated a child customization' }
                    switch ($back) {
                        'Backspace' { Send-MenuKey 0x08 }
                        'AltLeft' { Send-MenuKey 0x25 $true }
                        'Escape' { Send-MenuKey 0x1B }
                    }
                    $script:testStage = "one-level $back restores selected command"
                    $null = Wait-TagState { param($s) -not $s.settings.ready -and $s.palette_enabled -and $s.palette_selected_index -eq 1 -and $s.palette_total_results -eq 2 }
                    Write-Host "Menu re-entry: $back returned to its selected command"
                }
                $script:testStage = 'final first Enter reopens customizations'
                Send-MenuKey 0x0D
                $null = Wait-TagState { param($s) $s.settings.ready -and -not $s.palette_enabled }
                Send-MenuKey 0x1B
                $null = Wait-TagState { param($s) $s.palette_enabled }
                Send-MenuKey 0x1B
                $null = Wait-TagState { param($s) $s.palette_enabled -and $s.palette_total_results -eq 7 }
                Send-MenuKey 0x1B
                $null = Wait-TagState { param($s) -not $s.settings.ready -and -not $s.palette_enabled }
                foreach ($page in @(
                    @{ Id = 'theme'; Query = 'theme gallery'; Field = 'gallery' },
                    @{ Id = 'profiles'; Query = 'new terminal with profile'; Field = 'profiles' }
                )) {
                    Send-AutomexiaTestControl ('open-palette:menu-reentry-' + $page.Id)
                    $null = Wait-TagState { param($s) $s.palette_enabled -and $s.palette_total_results -eq 7 }
                    foreach ($letter in $page.Query.ToUpperInvariant().ToCharArray()) { Send-MenuKey ([int]$letter) }
                    $parent = Wait-TagState { param($s) $s.palette_enabled -and $s.palette_total_results -eq 1 }
                    for ($cycle = 0; $cycle -lt 2; $cycle++) {
                        $script:testStage = "first Enter opens $($page.Id) cycle $cycle"
                        Send-MenuKey 0x0D $false $true
                        $child = Wait-TagState { param($s) $s.settings.ready -and $null -ne $s.settings.($page.Field) -and -not $s.palette_enabled }
                        if ($child.window_tab_count -ne $baseline.window_tab_count) { throw 'Held Enter launched a profile' }
                        $script:testStage = "return from $($page.Id) cycle $cycle"
                        for ($level = 0; $level -lt 3 -and -not $child.palette_enabled; $level++) {
                            $previousPage = @($child.palette_enabled, $child.settings.open,
                                ($null -ne $child.settings.gallery), ($null -ne $child.settings.profiles),
                                $child.settings.active_category) -join ':'
                            Send-MenuKey 0x25 $true
                            $child = Wait-TagState { param($s)
                                (@($s.palette_enabled, $s.settings.open, ($null -ne $s.settings.gallery),
                                    ($null -ne $s.settings.profiles), $s.settings.active_category) -join ':') -ne $previousPage
                            }
                        }
                        $returned = Wait-TagState { param($s) $s.palette_enabled -and -not $s.settings.ready }
                        if ($returned.palette_accessibility_summary -ne $parent.palette_accessibility_summary) {
                            throw 'Child Back lost the parent query or selection'
                        }
                        Write-Host "Menu re-entry: $($page.Id) cycle $cycle passed with held Enter"
                    }
                    Send-MenuKey 0x1B
                    $null = Wait-TagState { param($s) -not $s.settings.ready -and -not $s.palette_enabled }
                }
                $finished = Read-AutomexiaSnapshot
                $afterPanel = Get-ActiveAutomexiaPanel $finished
                if ($afterPanel.route_id -ne $baselinePanel.route_id -or
                    $afterPanel.raw_cursor_line_text -ne $baselinePanel.raw_cursor_line_text -or
                    $finished.cursor_column -ne $baseline.cursor_column -or
                    $finished.cursor_row -ne $baseline.cursor_row -or
                    ((Get-MenuPreferenceHashes) -join ':') -ne $menuHashes) {
                    throw 'Menu re-entry changed terminal input or saved preferences'
                }
                Write-Host 'Menu re-entry: every first Enter opened; terminal input and saved preferences unchanged'
                [void][AutomexiaResizeDriver]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
                $null = Wait-TagState { param($s) $s.confirm_quit_active }
                Send-MenuKey 0x59
                if (-not $process.WaitForExit(6000)) { throw 'Menu fixture did not shut down' }
                $process = $null
                return
            }
            Send-AutomexiaTestControl 'open-customizations:tag-editor'
            $script:testStage = 'customizations root'
            $rootSettings = Wait-TagState { param($s) $s.settings.ready -and @($s.settings.controls | Where-Object id -eq 'tags.enabled').Count -eq 1 }
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
                [void][IO.Directory]::CreateDirectory($captureRoot)
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot 'customization-menu.png'))
            }
            Click-TagBounds ($rootSettings.settings.controls | Where-Object id -eq 'tags.enabled').bounds
            $script:testStage = 'information tag roster'
            $roster = Wait-TagState { param($s) $s.settings.ready -and @($s.settings.targets | Where-Object roster).Count -ge 13 }
            if ($TagShapesOnly) {
                . (Join-Path $PSScriptRoot 'tag-shapes-windows.ps1')
                Invoke-TagShapeScenario
                [void][AutomexiaResizeDriver]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
                $null = Wait-TagState { param($s) $s.confirm_quit_active }
                if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x59, $false, $false, $false)) { throw 'Shape fixture close failed' }
                if (-not $process.WaitForExit(6000)) { throw 'Shape fixture did not shut down' }
                $process = $null
                return
            }
            if ($roster.settings.devops_detection) {
                $toggle = @($roster.settings.controls | Where-Object id -eq 'extension.automexia.devops.context_status.enabled')
                if ($toggle.Count -ne 1) { throw 'Live DevOps detection toggle is not reachable' }
                Click-TagBounds $toggle[0].bounds
                $roster = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.devops_detection -and @($s.settings.targets | Where-Object roster).Count -eq 5 }
            }
            Assert-DevOpsTagsHidden $roster
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
                [void][IO.Directory]::CreateDirectory($captureRoot)
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot 'devops-tags-off.png'))
            }
            $ids = @('production', 'ubuntu-wsl', 'windows', 'git', 'kubernetes', 'docker', 'azure', 'aws', 'gcp', 'unknown-cloud', 'terraform', 'environment', 'user')
            # Ordinary edits above may still be saving. Establish disk baseline
            # only after that receipt, then exercise the real temporary owner.
            $roster = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.save_pending }
            $preferencePath = Join-Path $configRoot $preferenceRelativePath
            if (-not (Test-Path -LiteralPath $preferencePath -PathType Leaf)) {
                throw 'The native customization baseline was not saved'
            }
            $preferenceText = [IO.File]::ReadAllText($preferencePath)
            # Independent assertion against this fixture's single saved edit,
            # not a second TOML parser or a receipt-only preservation check.
            if ($preferenceText -notmatch '(?m)^\[\[extension-features\]\]\r?\nid = "extension\.automexia\.devops\.context_status\.enabled"\r?\nenabled = false\r?$') {
                throw 'The native customization baseline lacks the saved detection choice'
            }
            function Get-CustomizationFixtureHashes {
                $files = @((Get-Item -LiteralPath (Join-Path $configRoot 'config.toml')))
                $stateRoot = Join-Path $configRoot 'state'
                if (Test-Path -LiteralPath $stateRoot) {
                    $files += @(Get-ChildItem -LiteralPath $stateRoot -Filter '*preferences*.toml' -File)
                }
                return (($files | Sort-Object Name | ForEach-Object {
                    $_.Name + ':' + (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
                }) -join '|')
            }
            $savedHashes = Get-CustomizationFixtureHashes
            $script:testStage = 'temporary information-tag defaults'
            Click-TagBounds $roster.settings.reset_button
            Confirm-TagAction -CheckCancel
            $resetState = Wait-TagState { param($s) $s.settings.ready -and $s.settings.temporary_defaults -and $s.settings.tags_enabled }
            Click-TagBounds ($resetState.settings.controls | Where-Object id -eq 'tags.enabled').bounds
            $editedPreview = Wait-TagState { param($s) $s.settings.ready -and $s.settings.temporary_defaults -and -not $s.settings.tags_enabled }
            if ((Get-CustomizationFixtureHashes) -ne $savedHashes) { throw 'Temporary customization changed saved files' }

            # Closing the sheet must not end the application-owned preview or
            # replace its original saved snapshot with temporary choices.
            $script:testStage = 'close and reopen temporary customizations'
            [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)
            $null = Wait-TagState { param($s) $s.settings.ready -and @($s.settings.targets).Count -eq 0 }
            [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)
            $closedPreview = Wait-TagState { param($s) -not $s.settings.open }
            if ([string](Get-ActiveAutomexiaPanel $closedPreview).raw_cursor_line_text -ne $blankPromptLine) {
                throw 'Closing temporary customizations changed terminal input'
            }
            # The store belongs solely to this fixture. A regular file at the
            # directory boundary reproduces a real asynchronous inventory error.
            $packageProbe = Join-Path $configRoot 'ecosystem'
            if (Test-Path -LiteralPath $packageProbe) { throw 'Package failure fixture requires an absent store' }
            [IO.File]::WriteAllText($packageProbe, 'unavailable-store-fixture', [Text.Encoding]::ASCII)
            Send-AutomexiaTestControl 'open-customizations:temporary-reopen'
            $reopenedPreview = Wait-TagState { param($s) $s.settings.ready -and $s.settings.temporary_defaults -and -not $s.settings.tags_enabled -and $s.settings.package_settings_notice -eq 'unavailable' }
            if (@($reopenedPreview.settings.controls | Where-Object id -eq 'tags.enabled').Count -ne 1) {
                throw 'Unavailable package inventory hid core customizations'
            }
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
                [void][IO.Directory]::CreateDirectory($captureRoot)
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot 'package-unavailable-customizations.png'))
            }
            [IO.File]::Delete($packageProbe)
            $script:testStage = 'retry package inventory without losing temporary choices'
            [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)
            $null = Wait-TagState { param($s) -not $s.settings.open }
            Send-AutomexiaTestControl 'open-customizations:inventory-retry'
            $reopenedPreview = Wait-TagState { param($s) $s.settings.ready -and $s.settings.temporary_defaults -and -not $s.settings.tags_enabled -and $s.settings.package_inventory_ready -and $s.settings.package_settings_notice -eq 'none' }
            Click-TagBounds ($reopenedPreview.settings.controls | Where-Object id -eq 'tags.enabled').bounds
            $editedPreview = Wait-TagState { param($s) $s.settings.ready -and @($s.settings.targets | Where-Object roster).Count -ge 13 }
            $script:testStage = 'repeat feature reset without replacing saved choices'
            Click-TagBounds $editedPreview.settings.reset_button
            Confirm-TagAction
            $resetState = Wait-TagState { param($s) $s.settings.ready -and $s.settings.temporary_defaults -and $s.settings.tags_enabled }
            Click-TagBounds ($resetState.settings.controls | Where-Object id -eq 'tags.enabled').bounds
            $editedPreview = Wait-TagState { param($s) $s.settings.ready -and $s.settings.temporary_defaults -and -not $s.settings.tags_enabled }
            if ((Get-CustomizationFixtureHashes) -ne $savedHashes) { throw 'Repeated temporary reset changed saved files' }

            $script:testStage = 'global reset retains the original saved choices'
            [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)
            $rootPreview = Wait-TagState { param($s) $s.settings.ready -and @($s.settings.targets).Count -eq 0 }
            Click-TagBounds $rootPreview.settings.reset_button
            Confirm-TagAction
            $allDefaults = Wait-TagState { param($s) $s.settings.ready -and $s.settings.temporary_defaults -and $s.settings.tags_enabled -and $s.settings.devops_detection }
            if ((Get-CustomizationFixtureHashes) -ne $savedHashes) { throw 'Global temporary reset changed saved files' }
            $script:testStage = 'restore saved information-tag choices'
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x53, $false, $false, $false)) { throw 'Restore shortcut failed' }
            Confirm-TagAction
            $rootSettings = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.temporary_defaults -and $s.settings.tags_enabled -and -not $s.settings.devops_detection }
            Click-TagBounds ($rootSettings.settings.controls | Where-Object id -eq 'tags.enabled').bounds
            $roster = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.temporary_defaults -and $s.settings.tags_enabled -and -not $s.settings.devops_detection -and @($s.settings.targets | Where-Object roster).Count -eq 5 }
            Assert-DevOpsTagsHidden $roster
            if ((Get-CustomizationFixtureHashes) -ne $savedHashes) { throw 'Restore saved changed saved files' }
            # Restore deliberately returned to detection off. Re-enable it via
            # the real control before exercising all thirteen tag editors.
            $script:testStage = 're-enable DevOps tags'
            Click-TagBounds ($roster.settings.controls | Where-Object id -eq 'extension.automexia.devops.context_status.enabled').bounds
            $roster = Wait-TagState { param($s) $s.settings.ready -and $s.settings.devops_detection -and -not $s.settings.save_pending -and @($s.settings.targets | Where-Object roster).Count -eq 14 }
            $savedHashes = Get-CustomizationFixtureHashes
            foreach ($id in $ids) {
                $script:testStage = "pointer selection of $id"
                $page = "tags.slot.$id.page"
                if (@($roster.settings.targets | Where-Object { $_.id -eq $page -and -not $_.roster }).Count -eq 0) {
                    throw "Enabled tag $id is missing its native graphic sample"
                }
                $target = @($roster.settings.targets | Where-Object { $_.id -eq $page -and $_.roster })
                if ($target.Count -ne 1) { throw "Tag $id has no unique roster target" }
                Click-TagBounds $target[0].bounds
                $selected = Wait-TagState { param($s) $s.settings.ready -and $s.settings.active_slot -eq $page }
                if (@($selected.settings.controls | Where-Object id -eq "tags.slot.$id.enabled").Count -ne 1) {
                    throw "Tag $id did not open its enable control"
                }
                [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)
                $roster = Wait-TagState { param($s) $s.settings.ready -and $null -eq $s.settings.active_slot -and @($s.settings.targets | Where-Object roster).Count -ge 13 }
            }
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
                [void][IO.Directory]::CreateDirectory($captureRoot)
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot 'preview-hints.png'))
            }
            Click-TagBounds $roster.settings.edit_button
            $editing = Wait-TagState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode }
            $script:testStage = 'preview Done button'
            Click-TagBounds $editing.settings.edit_button
            $null = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.preview_edit_mode -and $null -eq $s.settings.active_slot }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x45, $false, $false, $false)) { throw 'Preview E shortcut after Done failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode }
            [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x24, $true)
            foreach ($id in $ids) {
                $script:testStage = "keyboard selection of $id"
                $page = "tags.slot.$id.page"
                $selected = Wait-TagState { param($s) $s.settings.ready -and $s.settings.selected -eq $page }
                [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x27, $true)
            }
            $selected = Wait-TagState { param($s) $s.settings.ready -and $s.settings.selected -eq 'tags.add-slot' }
            $script:testStage = 'preview keyboard mode entry, traversal and detail return'
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x1B, $false, $false, $false)) { throw 'Preview Escape failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.preview_edit_mode -and $s.settings.active_category -eq 'tags.enabled' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x45, $false, $false, $false)) { throw 'Preview E shortcut failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode -and $s.settings.selected -eq 'tags.add-slot' }
            # Forward wrap from Add must select the first tag; every subsequent
            # Tab must stay in preview, including offscreen or disabled tags.
            foreach ($id in $ids) {
                if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x09, $false, $false, $false)) { throw 'Preview Tab failed' }
                $page = "tags.slot.$id.page"
                $selected = Wait-TagState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode -and $s.settings.selected -eq $page -and $null -eq $s.settings.active_slot }
            }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x24, $true, $false, $false)) { throw 'Preview Home failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.selected -eq 'tags.slot.production.page' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x09, $false, $false, $true)) { throw 'Preview Shift+Tab failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode -and $s.settings.selected -eq 'tags.add-slot' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x09, $false, $false, $true)) { throw 'Preview reverse selection failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.selected -eq 'tags.slot.user.page' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x0D, $false, $false, $false)) { throw 'Preview activation failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.preview_edit_mode -and $s.settings.active_slot -eq 'tags.slot.user.page' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x45, $false, $false, $false)) { throw 'Detail E shortcut failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode -and $null -eq $s.settings.active_slot -and $s.settings.selected -eq 'tags.slot.user.page' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x0D, $false, $false, $false)) { throw 'Repeated preview activation failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.active_slot -eq 'tags.slot.user.page' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x1B, $false, $false, $false)) { throw 'Detail Escape failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.preview_edit_mode -and $null -eq $s.settings.active_slot -and $s.settings.active_category -eq 'tags.enabled' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x45, $false, $false, $false)) { throw 'Preview reentry failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x23, $true, $false, $false)) { throw 'Preview End failed' }
            $selected = Wait-TagState { param($s) $s.settings.ready -and $s.settings.selected -eq 'tags.add-slot' }
            if ((Get-CustomizationFixtureHashes) -ne $savedHashes) { throw 'Preview navigation changed saved settings' }
            $afterPanel = Get-ActiveAutomexiaPanel $selected
            if ([string]$afterPanel.raw_cursor_line_text -ne $blankPromptLine -or
                [int64]$afterPanel.route_id -ne [int64]$initialPanel.route_id) {
                throw 'Tag customization leaked input into the terminal'
            }
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
                [void][IO.Directory]::CreateDirectory($captureRoot)
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot 'tag-customization.png'))
            }
            if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
                $reportPath = [IO.Path]::GetFullPath($ResourceReport)
                [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($reportPath))
                [IO.File]::WriteAllText($reportPath, (@{
                    schema_version = 1; mode = 'tag-customization-only';
                    mouse_tags = $ids.Count; keyboard_tags = $ids.Count;
                    preview_shortcut_and_tab_cycle = $true; detail_escape_and_shortcut = $true;
                    preview_done_button = $true;
                    reset_restore_confirmation = $true; cancel_keeps_saved_files = $true;
                    unchanged_prompt = $true; devops_off_hides_samples_and_targets = $true;
                    devops_on_restores_tags = $true; scale = $selected.scale_factor;
                    temporary_reset_edit_restore = $true; saved_files_unchanged = $true;
                    temporary_close_reopen = $true; repeated_feature_and_global_reset = $true;
                    package_inventory_failure_recovery = $true;
                    frame = @($selected.window_width, $selected.window_height)
                } | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
            }
        } finally {
            [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
        }
        # Check painted pixels while Customizations is still open. A state flag
        # alone misses lower overlays painting over the confirmation card.
        [void][AutomexiaResizeDriver]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        $closing = Wait-TagState { param($s) $s.confirm_quit_active }
        $capture = if ([string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) { $null } else {
            Join-Path ([IO.Path]::GetFullPath($ModalCaptureDirectory)) 'customization-close-confirm.png'
        }
        Assert-AutomexiaCloseSurface $window $closing $capture
        [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)
        $restored = Wait-TagState { param($s) -not $s.confirm_quit_active -and $s.settings.ready -and $s.settings.selected -eq 'tags.add-slot' }
        if ((Get-CustomizationFixtureHashes) -ne $savedHashes -or
            [string](Get-ActiveAutomexiaPanel $restored).raw_cursor_line_text -ne $blankPromptLine) {
            throw 'Canceling close changed saved customizations or terminal input'
        }
        # Exercise the actual menu/Settings handoff, including held Back and a
        # second Back from the restored parent. A two-page toggle must fail here.
        $script:testStage = 'native menu back hierarchy'
        foreach ($altBack in @($true, $false)) {
            Send-AutomexiaTestControl "open-customizations:back-$altBack"
            $rootMenu = Wait-TagState { param($s) $s.settings.ready -and $null -eq $s.settings.active_category }
            Click-TagBounds ($rootMenu.settings.controls | Where-Object id -eq 'tags.enabled').bounds
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.active_category -eq 'tags.enabled' }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x45, $false, $false, $false)) { throw 'Menu preview entry failed' }
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode }
            if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $altBack, $true)) { throw 'Held menu Back lost focus' }
            $null = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.preview_edit_mode -and $s.settings.active_category -eq 'tags.enabled' }
            if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $altBack, $false)) { throw 'Category Back failed' }
            $rootMenu = Wait-TagState { param($s) $s.settings.ready -and $null -eq $s.settings.active_category }
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path ([IO.Path]::GetFullPath($ModalCaptureDirectory)) "menu-back-$altBack.png"))
            }
            if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $altBack, $true)) { throw 'Root Back failed' }
            $null = Wait-TagState { param($s) -not $s.settings.open -and -not $s.palette_enabled }
            Send-AutomexiaTestControl "open-palette:back-$altBack"
            $null = Wait-TagState { param($s) $s.palette_enabled }
            # Category order is Tabs, Panes, Search, Input, Appearance, Customizations.
            for ($i = 0; $i -lt 5; $i++) { [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x28, $true, $false, $false) }
            [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x0D, $false, $false, $false)
            $null = Wait-TagState { param($s) $s.palette_enabled -and $s.palette_accessibility_summary -like 'Customizations;*' }
            [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x28, $true, $false, $false)
            [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x0D, $false, $false, $false)
            $null = Wait-TagState { param($s) $s.settings.ready -and $null -eq $s.settings.active_category }
            if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $altBack, $true)) { throw 'Return to invoking menu failed' }
            $null = Wait-TagState { param($s) -not $s.settings.open -and $s.palette_enabled -and $s.palette_accessibility_summary -like 'Customizations;*' }
            if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $altBack, $false)) { throw 'Repeated parent Back failed' }
            $null = Wait-TagState { param($s) -not $s.settings.open -and $s.palette_enabled -and $s.palette_accessibility_summary -like 'Command categories;*' }
            if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $altBack, $true)) { throw 'Menu dismissal failed' }
            $returned = Wait-TagState { param($s) -not $s.settings.open -and -not $s.palette_enabled }
            if ((Get-CustomizationFixtureHashes) -ne $savedHashes -or
                [string](Get-ActiveAutomexiaPanel $returned).raw_cursor_line_text -ne $blankPromptLine) {
                throw 'Back navigation changed saved files or leaked held input to the terminal'
            }
        }
        if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
            $report = Get-Content -LiteralPath $ResourceReport -Raw | ConvertFrom-Json
            $report | Add-Member -NotePropertyName opaque_close_card_and_buttons -NotePropertyValue $true
            $report | Add-Member -NotePropertyName cancel_restores_customizations -NotePropertyValue $true
            $report | Add-Member -NotePropertyName menu_back_hierarchy_and_held_input -NotePropertyValue $true
            [IO.File]::WriteAllText([IO.Path]::GetFullPath($ResourceReport), ($report | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
        }
        [void][AutomexiaResizeDriver]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        $null = Wait-TagState { param($s) $s.confirm_quit_active }
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x59, $false, $false, $false)) {
            throw 'Could not confirm the native tag fixture close with Y'
        }
        if (-not $process.WaitForExit(6000)) { throw 'Tag customization fixture did not shut down' }
        $process = $null
        Write-Host 'Native tag customization passed: 13 pointer selections, 13 keyboard selections, unchanged terminal input'
        return
    }

    if ($CloseConfirmationOnly) {
        # Do not depend on OS-cascaded startup placement: part of a default
        # window can be below the monitor, yielding transparent capture rows.
        # Use the same on-screen dimensions as the customization fixture.
        [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1200, 780, $true)
        $sized = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        while (([Math]::Abs($sized.window_width / $sized.scale_factor - 1200) -gt 1 -or
                [Math]::Abs($sized.window_height / $sized.scale_factor - 780) -gt 1) -and
                [DateTime]::UtcNow -lt $deadline) {
            $sized = Read-AutomexiaSnapshot -AfterSequence ([int64]$sized.sequence)
        }
        if ([Math]::Abs($sized.window_width / $sized.scale_factor - 1200) -gt 1 -or
            [Math]::Abs($sized.window_height / $sized.scale_factor - 780) -gt 1) {
            throw 'Close fixture did not reach its controlled window size'
        }
        $script:testStage = 'focused native WM_CLOSE confirmation'
        if (-not [AutomexiaResizeDriver]::PostMessage(
                $window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
            throw 'Could not post the first native close request'
        }
        $opened = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not [bool]$opened.confirm_quit_active -and
               [DateTime]::UtcNow -lt $deadline) {
            $opened = Read-AutomexiaSnapshot -AfterSequence ([int64]$opened.sequence)
        }
        $process.Refresh()
        if (-not [bool]$opened.confirm_quit_active -or $process.HasExited -or
            -not [AutomexiaResizeDriver]::IsWindowVisible($window)) {
            throw 'Native WM_CLOSE did not open the Automexia confirmation'
        }
        $presented = Read-AutomexiaSnapshot -AfterSequence ([int64]$opened.sequence)
        if (-not [bool]$presented.confirm_quit_active) {
            throw 'The close confirmation disappeared before presentation'
        }
        $capture = if ([string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
            $null
        } else {
            $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
            New-Item -ItemType Directory -Force -Path $captureRoot | Out-Null
            Join-Path $captureRoot 'native-close-confirm-focused.png'
        }
        if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
            throw 'Could not expose the focused close overlay for capture'
        }
        try {
            Start-Sleep -Milliseconds 100
            $frame = [AutomexiaResizeDriver]::CaptureStableCloseDialogFrame($window, $capture)
        } finally {
            [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
        }
        if ($frame.SampleCount -lt 100 -or
            $frame.DistinctColorBuckets -lt 8 -or
            $frame.LuminanceSpread -lt 32) {
            throw 'The native close overlay frame is blank or unreadable'
        }
        Assert-AutomexiaCloseSurface $window $presented $capture
        if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)) {
            throw 'Could not cancel the native close with Escape'
        }
        $cancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$opened.sequence)
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        while ([bool]$cancelled.confirm_quit_active -and
               [DateTime]::UtcNow -lt $deadline) {
            $cancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelled.sequence)
        }
        $process.Refresh()
        if ([bool]$cancelled.confirm_quit_active -or $process.HasExited -or
            -not [AutomexiaResizeDriver]::IsWindowVisible($window)) {
            throw 'Escape did not restore the running window'
        }
        # A retained search footer uses the same modal text phase as settings.
        # It must neither show through Quit nor consume Quit's first Escape.
        Send-AutomexiaTestControl 'open-pane-search:close-regression'
        $search = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelled.sequence)
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not $search.search_active -and [DateTime]::UtcNow -lt $deadline) {
            $search = Read-AutomexiaSnapshot -AfterSequence ([int64]$search.sequence)
        }
        if (-not $search.search_active) { throw 'Close regression could not open search' }
        if (-not [AutomexiaResizeDriver]::PostMessage(
                $window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
            throw 'Could not post the repeated native close request'
        }
        $reopened = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelled.sequence)
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not [bool]$reopened.confirm_quit_active -and
               [DateTime]::UtcNow -lt $deadline) {
            $reopened = Read-AutomexiaSnapshot -AfterSequence ([int64]$reopened.sequence)
        }
        if (-not [bool]$reopened.confirm_quit_active) {
            throw 'Repeated native close did not reopen the confirmation'
        }
        $searchCapture = if ([string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) { $null } else {
            Join-Path ([IO.Path]::GetFullPath($ModalCaptureDirectory)) 'search-close-confirm.png'
        }
        Assert-AutomexiaCloseSurface $window $reopened $searchCapture
        [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)
        $searchRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$reopened.sequence)
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        while ($searchRestored.confirm_quit_active -and [DateTime]::UtcNow -lt $deadline) {
            $searchRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchRestored.sequence)
        }
        if ($searchRestored.confirm_quit_active -or -not $searchRestored.search_active) {
            throw 'Canceling Quit did not preserve the covered search'
        }
        [void][AutomexiaResizeDriver]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        $reopened = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchRestored.sequence)
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not $reopened.confirm_quit_active -and [DateTime]::UtcNow -lt $deadline) {
            $reopened = Read-AutomexiaSnapshot -AfterSequence ([int64]$reopened.sequence)
        }
        if (-not $reopened.confirm_quit_active) { throw 'Close over search did not reopen' }
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
                $window, 0x59, $false, $false, $false)) {
            throw 'Could not confirm the native close with Y'
        }
        if (-not $process.WaitForExit(15000)) {
            $afterY = Read-AutomexiaSnapshot
            throw "Confirming the native close did not exit Automexia (overlay active: $([bool]$afterY.confirm_quit_active), sequence: $($afterY.sequence))"
        }
        if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
            $reportPath = [IO.Path]::GetFullPath($ResourceReport)
            New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($reportPath)) | Out-Null
            [IO.File]::WriteAllText($reportPath, (@{
                schema_version = 1
                mode = 'close-confirmation-only'
                cancelled_and_reopened = $true
                opaque_card_and_buttons = $true
                cancel_restores_search = $true
                frame = @($frame.Width, $frame.Height)
                distinct_color_buckets = $frame.DistinctColorBuckets
                artifact = if ($null -eq $capture) { $null } else { [IO.Path]::GetFileName($capture) }
            } | ConvertTo-Json -Depth 3), [Text.UTF8Encoding]::new($false))
        }
        Write-Host 'Focused native WM_CLOSE confirmation passed'
        $process = $null
        return
    }

    if ($ConnectionHubOnly) {
        $modalCaptureRoot = if ([string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
            $null
        } else {
            [IO.Path]::GetFullPath($ModalCaptureDirectory)
        }
        if ($null -ne $modalCaptureRoot) {
            New-Item -ItemType Directory -Force -Path $modalCaptureRoot | Out-Null
        }
        $rendererName = if ($UseCpuRenderer) { 'cpu' } else { 'wgpu' }
        $setupCapturePath = if ($null -eq $modalCaptureRoot) {
            $null
        } else {
            Join-Path $modalCaptureRoot "connection-hub-$rendererName.png"
        }
        $directCapturePath = if ($null -eq $modalCaptureRoot) {
            $null
        } else {
            Join-Path $modalCaptureRoot "connection-hub-direct-$rendererName.png"
        }

        $script:testStage = 'focused connection hub setup composition'
        $hubControl = 'open-connection-hub:focused-native-hub'
        Send-AutomexiaTestControl $hubControl
        $hubSetup = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
        $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (([string]$hubSetup.last_control -ne $hubControl -or
                -not [bool]$hubSetup.connection_hub_active -or
                [string]$hubSetup.connection_hub_route -ne 'results') -and
               [DateTime]::UtcNow -lt $hubDeadline) {
            $hubSetup = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubSetup.sequence)
        }
        if (-not [bool]$hubSetup.connection_hub_active -or
            [string]$hubSetup.connection_hub_route -ne 'results') {
            Write-Host ($hubSetup | ConvertTo-Json -Depth 8)
            throw 'Focused native Connection Hub did not open on Results'
        }
        $hubSetupPresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubSetup.sequence)
        $terminalBefore = Get-ActiveAutomexiaPanel $hubSetupPresented
        if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
            throw 'Could not expose Automexia for focused setup capture'
        }
        try {
            Start-Sleep -Milliseconds 100
            $setupFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
                $window, $setupCapturePath)
        } finally {
            [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
        }
        if ($setupFrame.Width -lt 100 -or $setupFrame.Height -lt 100 -or
            $setupFrame.SampleCount -lt 100 -or
            $setupFrame.DistinctColorBuckets -lt 8 -or
            $setupFrame.LuminanceSpread -lt 32) {
            throw 'Focused native Connection Hub setup frame is blank or unreadable'
        }

        $script:testStage = 'credential source keyboard workflow'
        if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x4B, $false)) {
            throw 'Could not deliver the native vaults shortcut'
        }
        $vault = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubSetupPresented.sequence)
        $vaultDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not [bool]$vault.connection_hub_credentials_active -and [DateTime]::UtcNow -lt $vaultDeadline) {
            $vault = Read-AutomexiaSnapshot -AfterSequence ([int64]$vault.sequence)
        }
        if (-not [bool]$vault.connection_hub_credentials_active) { throw 'K did not open credential sources' }
        [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x4E, $false)
        $vaultDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not [bool]$vault.connection_hub_credentials_editing -and [DateTime]::UtcNow -lt $vaultDeadline) {
            $vault = Read-AutomexiaSnapshot -AfterSequence ([int64]$vault.sequence)
        }
        if (-not [bool]$vault.connection_hub_credentials_editing) { throw 'N did not start a new source' }
        # F belongs to the focused editor; it must not open the outer Hub file picker.
        [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x46, $false)
        $vault = Read-AutomexiaSnapshot -AfterSequence ([int64]$vault.sequence)
        if (-not [bool]$vault.connection_hub_credentials_editing) { throw 'Typing escaped the vault editor' }
        if ($null -ne $modalCaptureRoot) {
            [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)
            try { [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $modalCaptureRoot "credential-editor-$rendererName.png")) }
            finally { [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false) }
        }
        foreach ($unusedTab in 1..2) { [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x09, $false) }
        [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x0D, $false)
        $vaultDeadline = [DateTime]::UtcNow.AddSeconds(8)
        while (([bool]$vault.connection_hub_credentials_editing -or [int]$vault.connection_hub_credentials_count -ne 1) -and [DateTime]::UtcNow -lt $vaultDeadline) {
            $vault = Read-AutomexiaSnapshot -AfterSequence ([int64]$vault.sequence)
        }
        if ([bool]$vault.connection_hub_credentials_editing -or [int]$vault.connection_hub_credentials_count -ne 1) { throw 'Keyboard source save did not finish' }
        if ($null -ne $modalCaptureRoot) {
            [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)
            try { [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $modalCaptureRoot "credential-sources-$rendererName.png")) }
            finally { [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false) }
        }
        if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $true, $true)) { throw 'Held Alt+Left from vaults lost focus' }
        $vaultDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ([bool]$vault.connection_hub_credentials_active -and [DateTime]::UtcNow -lt $vaultDeadline) {
            $vault = Read-AutomexiaSnapshot -AfterSequence ([int64]$vault.sequence)
        }
        if ([bool]$vault.connection_hub_credentials_active -or -not [bool]$vault.connection_hub_active) { throw 'Escape did not restore Hub focus' }

        $script:testStage = 'saved connection keyboard workflow'
        [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x4E, $false)
        $saved = Read-AutomexiaSnapshot -AfterSequence ([int64]$vault.sequence)
        $savedDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not [bool]$saved.connection_hub_profiles_active -and [DateTime]::UtcNow -lt $savedDeadline) {
            $saved = Read-AutomexiaSnapshot -AfterSequence ([int64]$saved.sequence)
        }
        if (-not [bool]$saved.connection_hub_profiles_active) { throw 'N did not open saved connections' }
        [void][AutomexiaResizeDriver]::PostKeyTap($window, 0x4E, $false)
        $savedDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not [bool]$saved.connection_hub_profiles_editing -and [DateTime]::UtcNow -lt $savedDeadline) {
            $saved = Read-AutomexiaSnapshot -AfterSequence ([int64]$saved.sequence)
        }
        if (-not [bool]$saved.connection_hub_profiles_editing) { throw 'N did not open the saved connection form' }
        foreach ($fieldText in @('Team shell', 'shell.example.test', 'operator', '2222')) {
            if (-not [AutomexiaResizeDriver]::SendActionText($window, $fieldText, $false)) { throw 'Could not type saved connection field' }
            $saved = Read-AutomexiaSnapshot -AfterSequence ([int64]$saved.sequence)
            # Text uses the real input queue. Posting WM_KEYDOWN for Tab can
            # overtake its final character and insert it into the next field.
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x09, $false, $false, $false)) { throw 'Could not move between saved connection fields' }
            $saved = Read-AutomexiaSnapshot -AfterSequence ([int64]$saved.sequence)
        }
        # Source is focused. Choose the external source just added, then Save.
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x27, $true, $false, $false)) { throw 'Could not select the saved credential source' }
        $saved = Read-AutomexiaSnapshot -AfterSequence ([int64]$saved.sequence)
        if ($null -ne $modalCaptureRoot) {
            [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)
            try { [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $modalCaptureRoot "saved-connection-editor-$rendererName.png")) }
            finally { [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false) }
        }
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x09, $false, $false, $false) -or
            -not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x0D, $false, $false, $false)) { throw 'Could not save the connection with the keyboard' }
        $savedDeadline = [DateTime]::UtcNow.AddSeconds(8)
        while (([bool]$saved.connection_hub_profiles_editing -or [int]$saved.connection_hub_profiles_count -ne 1) -and [DateTime]::UtcNow -lt $savedDeadline) {
            $saved = Read-AutomexiaSnapshot -AfterSequence ([int64]$saved.sequence)
        }
        if ([bool]$saved.connection_hub_profiles_editing -or [int]$saved.connection_hub_profiles_count -ne 1) { throw 'Saved connection was not published after save' }
        $savedLibrary = Get-Content -LiteralPath (Join-Path $configRoot 'connections/library.v1.json') -Raw | ConvertFrom-Json
        if ([string]$savedLibrary.profiles.profiles[0].display_name -cne 'Team shell') {
            throw 'The saved connection name did not preserve the exact typed text, including spaces'
        }
        if ([string]$savedLibrary.profiles.profiles[0].transport.host -cne 'shell.example.test' -or
            [string]$savedLibrary.profiles.profiles[0].transport.user -cne 'operator' -or
            [int]$savedLibrary.profiles.profiles[0].transport.port -ne 2222) {
            throw 'The saved connection fields did not preserve exact typed text across Tab navigation'
        }
        if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $false, $true)) { throw 'Held Backspace from saved connections lost focus' }
        $savedDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ([bool]$saved.connection_hub_profiles_active -and [DateTime]::UtcNow -lt $savedDeadline) {
            $saved = Read-AutomexiaSnapshot -AfterSequence ([int64]$saved.sequence)
        }
        if ([bool]$saved.connection_hub_profiles_active -or -not [bool]$saved.connection_hub_active) { throw 'Saved connections did not return to Hub' }

        $script:testStage = 'focused connection hub direct-entry composition'
        if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x4C, $false)) {
            throw 'Could not deliver the native Connection Hub L mnemonic'
        }
        $hubDirect = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubSetupPresented.sequence)
        $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ((-not [bool]$hubDirect.connection_hub_active -or
                -not [bool]$hubDirect.connection_hub_literal_entry) -and
               [DateTime]::UtcNow -lt $hubDeadline) {
            $hubDirect = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubDirect.sequence)
        }
        if (-not [bool]$hubDirect.connection_hub_active -or
            -not [bool]$hubDirect.connection_hub_literal_entry) {
            Write-Host ($hubDirect | ConvertTo-Json -Depth 8)
            throw 'Native L did not open the focused direct-entry editor'
        }
        $hubDirectPresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubDirect.sequence)
        if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
            throw 'Could not expose Automexia for focused direct-entry capture'
        }
        try {
            Start-Sleep -Milliseconds 100
            $directFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
                $window, $directCapturePath)
        } finally {
            [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
        }
        if ($directFrame.Width -lt 100 -or $directFrame.Height -lt 100 -or
            $directFrame.SampleCount -lt 100 -or
            $directFrame.DistinctColorBuckets -lt 8 -or
            $directFrame.LuminanceSpread -lt 32) {
            throw 'Focused native Connection Hub direct frame is blank or unreadable'
        }

        $hubScale = [double]$hubDirectPresented.scale_factor
        $logicalWidth = [double]$hubDirectPresented.window_width / $hubScale
        $logicalHeight = [double]$hubDirectPresented.window_height / $hubScale
        $margin = if ($logicalWidth -lt 420.0 -or $logicalHeight -lt 320.0) {
            6.0
        } else {
            18.0
        }
        $cardWidth = [Math]::Min(840.0, [Math]::Max(1.0, $logicalWidth - $margin * 2.0))
        $cardHeight = [Math]::Min(420.0, [Math]::Max(1.0, $logicalHeight - $margin * 2.0))
        $cardX = [Math]::Max(0.0, ($logicalWidth - $cardWidth) * 0.5)
        $cardY = [Math]::Max(0.0, ($logicalHeight - $cardHeight) * 0.5)
        $inner = if ($cardWidth -lt 650.0) { 12.0 } else { 20.0 }
        $closeX = [int][Math]::Round(($cardX + $cardWidth - $inner - 20.0) * $hubScale)
        $closeY = [int][Math]::Round(($cardY + 32.0) * $hubScale)
        # Keep the owned fixture above other desktop windows for the entire
        # pointer transaction, not only its screenshot. Otherwise native leave/
        # movement messages can change the hit between the snapshot and click.
        if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true) -or
            -not [AutomexiaResizeDriver]::ActivateWindow($window)) {
            throw 'Could not keep the focused Hub pointer target visible'
        }
        try {
        # Use only the DPI-aware physical input owner. A second posted move is
        # virtualized by Windows and can race in at an extra scale-factor offset.
        $hubPointerReady = $hubDirectPresented
        foreach ($targetX in @(($closeX + 6), $closeX)) {
            if (-not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $targetX, $closeY)) {
                throw 'Could not move to the focused direct-entry close target'
            }
            $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
            do {
                $hubPointerReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubPointerReady.sequence)
            } while (([Math]::Abs([double]$hubPointerReady.pointer.x - $targetX) -gt 1 -or
                      [Math]::Abs([double]$hubPointerReady.pointer.y - $closeY) -gt 1) -and
                     [DateTime]::UtcNow -lt $hubDeadline)
            if ([Math]::Abs([double]$hubPointerReady.pointer.x - $targetX) -gt 1 -or
                [Math]::Abs([double]$hubPointerReady.pointer.y - $closeY) -gt 1) {
                throw 'The Hub pointer did not reach the exact physical target'
            }
        }
        if ([string]$hubPointerReady.connection_hub_pointer_hit -ne
            'CancelLiteralDestination') {
            Write-Host ($hubPointerReady | ConvertTo-Json -Depth 8)
            throw 'The physical pointer did not resolve to direct-entry Cancel'
        }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick(
                $window, $closeX, $closeY)) {
            throw 'Could not click the focused direct-entry close target'
        }
        $hubCancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubPointerReady.sequence)
        $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ((-not [bool]$hubCancelled.connection_hub_active -or
                [bool]$hubCancelled.connection_hub_literal_entry) -and
               [DateTime]::UtcNow -lt $hubDeadline) {
            $hubCancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubCancelled.sequence)
        }
        $terminalAfterCancel = Get-ActiveAutomexiaPanel $hubCancelled
        if (-not [bool]$hubCancelled.connection_hub_active -or
            [bool]$hubCancelled.connection_hub_literal_entry -or
            [string]$hubCancelled.connection_hub_route -ne 'results' -or
            [int64]$terminalAfterCancel.route_id -ne [int64]$terminalBefore.route_id -or
            [int]$hubCancelled.display_offset -ne [int]$hubSetupPresented.display_offset -or
            [string]$terminalAfterCancel.raw_cursor_line_text -ne
                [string]$terminalBefore.raw_cursor_line_text) {
            Write-Host ($hubCancelled | ConvertTo-Json -Depth 8)
            throw 'Native close did not cancel only direct entry or changed terminal state'
        }
        } finally {
            [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
        }

        if (-not [AutomexiaResizeDriver]::SendMenuBack($window, $true, $true)) {
            throw 'Could not deliver held Connection Hub Back after nested cancellation'
        }
        $hubClosed = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubCancelled.sequence)
        $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ([bool]$hubClosed.connection_hub_active -and
               [DateTime]::UtcNow -lt $hubDeadline) {
            $hubClosed = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubClosed.sequence)
        }
        if ([bool]$hubClosed.connection_hub_active) {
            throw 'Focused Connection Hub remained active after Escape'
        }
        if ([string](Get-ActiveAutomexiaPanel $hubClosed).raw_cursor_line_text -ne
            [string]$terminalBefore.raw_cursor_line_text) {
            throw 'Held Hub Back leaked into terminal input'
        }

        if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
            $reportPath = [IO.Path]::GetFullPath($ResourceReport)
            $reportDirectory = [IO.Path]::GetDirectoryName($reportPath)
            if (-not [string]::IsNullOrWhiteSpace($reportDirectory)) {
                New-Item -ItemType Directory -Force -Path $reportDirectory | Out-Null
            }
            $focusedReport = [ordered]@{
                schema_version = 1
                mode = 'connection-hub-only'
                renderer = $rendererName
                fixture = [string]$initial.visual_test_fixture
                credential_source_saved = $true
                source_bound_profile_saved = $true
                setup = [ordered]@{
                    frame = @($setupFrame.Width, $setupFrame.Height)
                    distinct_color_buckets = $setupFrame.DistinctColorBuckets
                    luminance_spread = $setupFrame.LuminanceSpread
                    artifact = if ($null -eq $setupCapturePath) { $null } else {
                        [IO.Path]::GetFileName($setupCapturePath)
                    }
                }
                direct_entry = [ordered]@{
                    frame = @($directFrame.Width, $directFrame.Height)
                    distinct_color_buckets = $directFrame.DistinctColorBuckets
                    luminance_spread = $directFrame.LuminanceSpread
                    close_cancelled_nested_only = $true
                    terminal_state_preserved = $true
                    artifact = if ($null -eq $directCapturePath) { $null } else {
                        [IO.Path]::GetFileName($directCapturePath)
                    }
                }
            } | ConvertTo-Json -Depth 5
            [IO.File]::WriteAllText(
                $reportPath, $focusedReport,
                [Text.UTF8Encoding]::new($false))
        }
        Write-Host "Focused native Connection Hub $rendererName visual/input assurance passed"
        return
    }

    # Create a top-level tab through the exact Ctrl+T lifecycle. The renderer
    # snapshot is taken immediately after the control is consumed, before any
    # later OS resize can accidentally repair stale geometry.
    $script:testStage = 'create top-level tab at current viewport'
    $windowTabControl = 'window-tab:top-create'
    Send-AutomexiaTestControl $windowTabControl
    $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
    $topTabDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (([string]$topTab.last_control -ne $windowTabControl -or
            [int]$topTab.window_tab_count -ne 2 -or
            [int]$topTab.active_window_tab_index -ne 1 -or
            [int]$topTab.panel_count -ne 1) -and
           [DateTime]::UtcNow -lt $topTabDeadline) {
        $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$topTab.sequence)
    }
    if ([string]$topTab.last_control -ne $windowTabControl -or
        [int]$topTab.window_tab_count -ne 2 -or
        [int]$topTab.active_window_tab_index -ne 1 -or
        [int]$topTab.panel_count -ne 1) {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'Ctrl+T lifecycle did not create and select exactly one top-level tab'
    }
    if ([Math]::Abs([double]$topTab.grid_width - [double]$topTab.window_width) -gt 1.0 -or
        [Math]::Abs([double]$topTab.grid_height - [double]$topTab.window_height) -gt 1.0) {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'A new top-level tab inherited terminal dimensions instead of the current window viewport'
    }
    if ([string]$topTab.active_tab_profile -notmatch '(?i)powershell|pwsh') {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'A new top-level tab did not expose its PowerShell launch identity before shell output'
    }
    $topPanel = Get-ActiveAutomexiaPanel $topTab
    $topRect = @($topPanel.layout_rect)
    $expectedBottom = [double]$topTab.grid_height - [double]$topTab.grid_margin.bottom
    $actualBottom = [double]$topTab.grid_margin.top + [double]$topRect[1] + [double]$topRect[3]
    $configuredBottomInset = $expectedBottom - $actualBottom
    if ($topRect.Count -ne 4 -or $configuredBottomInset -lt -1.0 -or $configuredBottomInset -gt 32.0) {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'The new-tab pane/footer boundary does not reach the current viewport bottom'
    }

    # Wait without input for the new shell too. This makes the following
    # history checks use the newly created session and catches blank first-frame
    # regressions independently of profile startup speed.
    while (($null -eq $topTab.latest_prompt_id -or
            [int]$topTab.latest_prompt_start_count -ne 1 -or
            -not [bool]$topTab.full_path_visible) -and
           [DateTime]::UtcNow -lt $topTabDeadline) {
        $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$topTab.sequence)
    }
    if ($null -eq $topTab.latest_prompt_id -or
        [int]$topTab.latest_prompt_start_count -ne 1 -or
        -not [bool]$topTab.full_path_visible) {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'The new Ctrl+T session did not publish its complete prompt automatically'
    }
    # Queue physical Ctrl+T presses without waiting for shell startup or a
    # rendered frame. Check every published frame, including inactive tabs,
    # then close and repeat while prior PTYs are retiring.
    for ($burst = 0; $burst -lt 2; $burst++) {
        $script:testStage = 'rapid top-level tab startup and retirement'
        if (-not [AutomexiaResizeDriver]::SendControlBurst($window, 0x54, 6)) {
            throw 'Could not queue the native new-tab burst'
        }
        $burstDeadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$topTab.sequence)
            $titles = @($topTab.window_tab_titles)
            if ($titles.Count -ne [int]$topTab.window_tab_count -or
                @($titles | Where-Object { $_ -cne 'PowerShell' }).Count -ne 0) {
                throw 'A startup frame exposed a placeholder or wrong profile in the tab strip'
            }
        } while ([int]$topTab.window_tab_count -lt 8 -and [DateTime]::UtcNow -lt $burstDeadline)
        if ([int]$topTab.window_tab_count -ne 8 -or [int]$topTab.active_window_tab_index -ne 7) {
            Write-Host ("Rapid-tab observation: burst={0} tabs={1} selected={2} sequence={3}" -f $burst, $topTab.window_tab_count, $topTab.active_window_tab_index, $topTab.sequence)
            throw 'Rapid new-tab input was lost or selected the wrong tab'
        }
        if (-not [AutomexiaResizeDriver]::SendControlBurst($window, 0x73, 6)) {
            throw 'Could not queue the native close-tab burst'
        }
        $burstDeadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$topTab.sequence)
        } while ([int]$topTab.window_tab_count -gt 2 -and [DateTime]::UtcNow -lt $burstDeadline)
        if ([int]$topTab.window_tab_count -ne 2 -or [int]$topTab.active_window_tab_index -ne 1 -or
            @($topTab.window_tab_titles | Where-Object { $_ -cne 'PowerShell' }).Count -ne 0) {
            throw 'Rapid closing or late shell output changed the surviving tabs'
        }
    }
    if ([int]$topTab.owned_route_count -ne 10) {
        throw 'Rapid closing did not retain exactly two visible and eight undoable sessions'
    }
    # Ctrl+F4 deliberately retains live sessions for Undo Close (bounded to
    # eight). Purge these fixtures via the same owner as the inspector's
    # confirmed Clear action before unrelated resource-growth measurements.
    $clearParkedControl = 'clear-parked-tabs:rapid-tab-fixtures'
    Send-AutomexiaTestControl $clearParkedControl
    $clearParkedDeadline = [DateTime]::UtcNow.AddSeconds(6)
    do {
        $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$topTab.sequence)
    } while (([string]$topTab.last_control -ne $clearParkedControl -or
        [int]$topTab.owned_route_count -ne 2) -and [DateTime]::UtcNow -lt $clearParkedDeadline)
    if ([string]$topTab.last_control -ne $clearParkedControl -or
        [int]$topTab.owned_route_count -ne 2) {
        throw 'Rapid-tab fixture cleanup did not retire every retained route'
    }
    $initial = $topTab
    $initialPanel = Get-ActiveAutomexiaPanel $initial

    # Exercise the same terminal-owned word extension used by
    # Ctrl+Shift+Left. The binding table independently proves the chord; this
    # renderer-neutral check proves the live PowerShell cursor anchors a real
    # selection without leaking input into ConPTY.
    $selectionToken = 'AMX_SELECTION_PROBE_74129'
    $selectionCommand = "Write-Output $selectionToken"
    $selectionTypeControl = "write-text:selection-type:$selectionCommand"
    $script:testStage = 'keyboard word selection typing'
    Send-AutomexiaTestControl $selectionTypeControl
    $selectionTyped = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
    $selectionDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$selectionTyped.last_control -ne $selectionTypeControl -or
            -not ([string](Get-ActiveAutomexiaPanel $selectionTyped).raw_cursor_logical_line_text).Contains($selectionToken)) -and
           [DateTime]::UtcNow -lt $selectionDeadline) {
        $selectionTyped = Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionTyped.sequence)
    }
    if ([string]$selectionTyped.last_control -ne $selectionTypeControl -or
        -not ([string](Get-ActiveAutomexiaPanel $selectionTyped).raw_cursor_logical_line_text).Contains($selectionToken)) {
        Write-Host ($selectionTyped | ConvertTo-Json -Depth 10)
        throw 'PowerShell did not render the keyboard-selection probe'
    }

    $selectionControl = 'extend-selection:keyboard-word-left:word-left'
    Send-AutomexiaTestControl $selectionControl
    $selectionExtended = Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionTyped.sequence)
    $selectionDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$selectionExtended.last_control -ne $selectionControl -or
            [string](Get-ActiveAutomexiaPanel $selectionExtended).selection_text -ne $selectionToken) -and
           [DateTime]::UtcNow -lt $selectionDeadline) {
        $selectionExtended = Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionExtended.sequence)
    }
    if ([string]$selectionExtended.last_control -ne $selectionControl -or
        [string](Get-ActiveAutomexiaPanel $selectionExtended).selection_text -ne $selectionToken) {
        Write-Host ($selectionExtended | ConvertTo-Json -Depth 10)
        throw 'Ctrl+Shift+Left semantics did not select exactly one PowerShell word'
    }
    if (-not [bool](Get-ActiveAutomexiaPanel $selectionExtended).selection_rendered) {
        throw 'Keyboard selection reached VT state but not the renderer snapshot'
    }

    # A bare arrow is shell input and therefore exits terminal selection mode
    # before the key is forwarded. Use a real window message so this covers
    # the native Windows/ConPTY input path rather than a test-only clear call.
    $script:testStage = 'bare arrow exits keyboard selection'
    if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x25, $true)) {
        throw 'Could not deliver the native Left Arrow selection-exit probe'
    }
    $selectionArrowCleared =
        Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionExtended.sequence)
    $selectionDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ((-not [string]::IsNullOrEmpty(
                [string](Get-ActiveAutomexiaPanel $selectionArrowCleared).selection_text) -or
            [bool](Get-ActiveAutomexiaPanel $selectionArrowCleared).selection_rendered) -and
           [DateTime]::UtcNow -lt $selectionDeadline) {
        $selectionArrowCleared =
            Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionArrowCleared.sequence)
    }
    if (-not [string]::IsNullOrEmpty(
            [string](Get-ActiveAutomexiaPanel $selectionArrowCleared).selection_text) -or
        [bool](Get-ActiveAutomexiaPanel $selectionArrowCleared).selection_rendered) {
        throw 'Bare Left Arrow did not exit keyboard selection mode'
    }

    # Recreate a real selection, then exercise the same paste/input seam used
    # by printable text, IME commits and unbracketed paste. The payload must be
    # visible in PowerShell and both VT/render selection state must clear.
    $selectionAgainControl = 'extend-selection:keyboard-input-exit:word-left'
    Send-AutomexiaTestControl $selectionAgainControl
    $selectionAgain =
        Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionArrowCleared.sequence)
    $selectionDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$selectionAgain.last_control -ne $selectionAgainControl -or
            [string]::IsNullOrEmpty(
                [string](Get-ActiveAutomexiaPanel $selectionAgain).selection_text)) -and
           [DateTime]::UtcNow -lt $selectionDeadline) {
        $selectionAgain =
            Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionAgain.sequence)
    }
    if ([string]::IsNullOrEmpty(
            [string](Get-ActiveAutomexiaPanel $selectionAgain).selection_text) -or
        -not [bool](Get-ActiveAutomexiaPanel $selectionAgain).selection_rendered) {
        throw 'Could not recreate keyboard selection for the text-input exit probe'
    }

    $selectionInputSuffix = '__EXIT__'
    $selectionInputControl =
        "input-text:keyboard-selection-input-exit:$selectionInputSuffix"
    Send-AutomexiaTestControl $selectionInputControl
    $selectionCleared =
        Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionAgain.sequence)
    $selectionDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$selectionCleared.last_control -ne $selectionInputControl -or
            -not [string]::IsNullOrEmpty(
                [string](Get-ActiveAutomexiaPanel $selectionCleared).selection_text) -or
            [bool](Get-ActiveAutomexiaPanel $selectionCleared).selection_rendered -or
            -not ([string](Get-ActiveAutomexiaPanel $selectionCleared).raw_cursor_logical_line_text).Contains(
                $selectionInputSuffix)) -and
           [DateTime]::UtcNow -lt $selectionDeadline) {
        $selectionCleared =
            Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionCleared.sequence)
    }
    if (-not [string]::IsNullOrEmpty(
            [string](Get-ActiveAutomexiaPanel $selectionCleared).selection_text) -or
        [bool](Get-ActiveAutomexiaPanel $selectionCleared).selection_rendered -or
        -not ([string](Get-ActiveAutomexiaPanel $selectionCleared).raw_cursor_logical_line_text).Contains(
            $selectionInputSuffix)) {
        Write-Host ($selectionCleared | ConvertTo-Json -Depth 10)
        throw 'Text input did not exit selection mode and reach PowerShell'
    }
    $selectionSubmitControl = 'write-line:selection-submit:'
    Send-AutomexiaTestControl $selectionSubmitControl
    $selectionDone = Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionCleared.sequence)
    $selectionDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([string]$selectionDone.last_control -ne $selectionSubmitControl -or
            [int64]$selectionDone.latest_prompt_id -le [int64]$initial.latest_prompt_id) -and
           [DateTime]::UtcNow -lt $selectionDeadline) {
        $selectionDone = Read-AutomexiaSnapshot -AfterSequence ([int64]$selectionDone.sequence)
    }
    if ([int64]$selectionDone.latest_prompt_id -le [int64]$initial.latest_prompt_id) {
        Write-Host ($selectionDone | ConvertTo-Json -Depth 10)
        throw 'PowerShell did not complete the keyboard-selection probe command'
    }
    $initial = $selectionDone
    # A surface from an earlier command is not evidence that the current
    # command was grouped. Exercise representative PowerShell command classes
    # and require a new semantic result identity, exact owning prompt, output
    # token, exit state, and painted surface for every case.
    $resultCommandCases = @(
        [pscustomobject]@{
            Name = 'single-success'
            Command = "Write-Output 'AMX_RESULT_SINGLE_78101'"
            Tokens = @('AMX_RESULT_SINGLE_78101')
            HasOutput = $true
            ExitCode = 0
        },
        [pscustomobject]@{
            Name = 'multiline-success'
            Command = "Write-Output 'AMX_RESULT_MULTI_A_78102'; Write-Output 'AMX_RESULT_MULTI_B_78102'"
            Tokens = @('AMX_RESULT_MULTI_A_78102', 'AMX_RESULT_MULTI_B_78102')
            ExitCode = 0
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'external-process-success'
            Command = 'cmd.exe /D /C "echo AMX_RESULT_EXTERNAL_78103"'
            Tokens = @('AMX_RESULT_EXTERNAL_78103')
            ExitCode = 0
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'error-output'
            Command = "Write-Error 'AMX_RESULT_ERROR_78104'"
            Tokens = @('AMX_RESULT_ERROR_78104')
            ExitCode = 1
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'parameter-binding-error-ls-ll'
            Command = 'ls -ll'
            Tokens = @('ParameterBindingException')
            ExitCode = 1
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'provider-pipeline-success'
            Command = 'Get-Item -LiteralPath Cargo.toml | ForEach-Object { Write-Output ''AMX_RESULT_PROVIDER_78105''; Write-Output $_.Name }'
            Tokens = @('AMX_RESULT_PROVIDER_78105', 'Cargo.toml')
            ExitCode = 0
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'native-stderr-failure'
            Command = 'cmd.exe /D /C "echo AMX_RESULT_NATIVE_STDERR_78106 1>&2 & exit /b 7"'
            Tokens = @('AMX_RESULT_NATIVE_STDERR_78106')
            ExitCode = 7
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'no-output-success'
            Command = '$null = Get-Item -LiteralPath Cargo.toml'
            Tokens = @()
            ExitCode = 0
            HasOutput = $false
        }
    )
    $viewportRows = [Math]::Max(4, [int]$initial.rows)
    $overflowHeights = @(
        [Math]::Max(1, $viewportRows - 2)
        [Math]::Max(1, $viewportRows - 1)
        $viewportRows
        $viewportRows + 1
        ($viewportRows * 2) - 1
        $viewportRows * 2
        ($viewportRows * 2) + 1
    ) | Select-Object -Unique
    $overflowIndex = 0
    foreach ($outputRows in $overflowHeights) {
        $overflowIndex += 1
        $marker = "AMX_RESULT_OVERFLOW_$($overflowIndex.ToString('D2'))"
        $resultCommandCases += [pscustomobject]@{
            Name = "viewport-overflow-$outputRows"
            Command = "1..$outputRows | ForEach-Object { Write-Output ('$marker' + '_' + `$_) }"
            Tokens = @("$marker`_$outputRows")
            ExitCode = 0
            HasOutput = $true
            OutputRows = $outputRows
            OwnerMustBeOffscreen = $outputRows -ge $viewportRows
        }
    }
    $resultCommandEvidence = @()
    $resultProbe = $initial

    foreach ($case in $resultCommandCases) {
        $ownerMustBeOffscreen = $null -ne $case.PSObject.Properties['OwnerMustBeOffscreen'] -and
            [bool]$case.OwnerMustBeOffscreen
        $outputRowsEvidence = if ($null -eq $case.PSObject.Properties['OutputRows']) {
            $null
        } else {
            [int]$case.OutputRows
        }
        $previousPromptId = [int64]$resultProbe.latest_prompt_id
        $previousResultKey = if ($null -eq $resultProbe.command_result_key) {
            -1
        } else {
            [int64]$resultProbe.command_result_key
        }
        $caseControl = "write-line:result-$($case.Name):$($case.Command)"
        $script:testStage = "command-result $($case.Name)"
        Send-AutomexiaTestControl $caseControl
        $caseReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$resultProbe.sequence)
        $caseDeadline = [DateTime]::UtcNow.AddSeconds(10)
        do {
            $casePanel = Get-ActiveAutomexiaPanel $caseReady
            $allTokensVisible = $true
            foreach ($token in $case.Tokens) {
                if (-not ([string]$casePanel.visible_text).Contains($token)) {
                    $allTokensVisible = $false
                    break
                }
            }
            $resultMatches = if ([bool]$case.HasOutput) {
                $null -ne $caseReady.command_result_key -and
                    [int64]$caseReady.command_result_key -gt $previousResultKey -and
                    [int64]$caseReady.command_result_generation -eq $previousPromptId -and
                    [int]$caseReady.command_result_exit_code -eq [int]$case.ExitCode -and
                    [int64]$caseReady.command_result_completed_at_unix_ms -gt 1600000000000 -and
                    [string]$caseReady.command_result_label -like
                        '*2026-08-26 12:34:56' -and
                    $null -ne $caseReady.command_result_surface
            } else {
                $semanticResult = @($caseReady.semantic_rows | Where-Object {
                    $_.has_result -and
                        [int64]$_.generation -eq $previousPromptId -and
                        [int]$_.result_exit_code -eq [int]$case.ExitCode -and
                        [int64]$_.result_completed_at_unix_ms -gt 1600000000000
                })
                # A preceding output group may remain visible by design. The
                # silent command itself must publish semantic completion but
                # must not become the selected paintable result.
                $semanticResult.Count -gt 0 -and
                    ($null -eq $caseReady.command_result_generation -or
                        [int64]$caseReady.command_result_generation -ne $previousPromptId)
            }
            $caseComplete = (
                [string]$caseReady.last_control -eq $caseControl -and
                [int64]$caseReady.latest_prompt_id -gt $previousPromptId -and
                $resultMatches -and
                $allTokensVisible)
            if (-not $caseComplete) {
                $caseReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$caseReady.sequence)
            }
        } while (-not $caseComplete -and [DateTime]::UtcNow -lt $caseDeadline)
        if (-not $caseComplete) {
            Write-Host ($caseReady | ConvertTo-Json -Depth 10)
            throw "Command-result case '$($case.Name)' did not publish fresh, truthful, visible output grouping"
        }
        if ($ownerMustBeOffscreen) {
            $visibleOwner = @($caseReady.semantic_rows | Where-Object {
                $_.has_result -and [int64]$_.generation -eq $previousPromptId
            })
            if ($visibleOwner.Count -ne 0) {
                throw "Viewport-overflow case '$($case.Name)' did not move its source result owner above the visible snapshot"
            }
            $visibleBoundary = @($caseReady.semantic_rows | Where-Object {
                $null -ne $_.boundary_result_id -and
                    [int64]$_.boundary_result_id -eq [int64]$caseReady.command_result_key -and
                    [int64]$_.boundary_source_generation -eq $previousPromptId -and
                    [int64]$_.boundary_completed_at_unix_ms -eq
                        [int64]$caseReady.command_result_completed_at_unix_ms
            })
            if ($visibleBoundary.Count -ne 1) {
                throw "Viewport-overflow case '$($case.Name)' did not retain exactly one visible terminal-owned result boundary"
            }
        }
        # Report the evidence owned by this command, not the most recent
        # paintable surface. Silent commands intentionally leave the preceding
        # output surface visible, so copying the selected surface here would
        # falsely attribute that older result to the silent completion.
        $evidenceGeneration = if ([bool]$case.HasOutput) {
            [int64]$caseReady.command_result_generation
        } else {
            $previousPromptId
        }
        $evidenceKey = if ([bool]$case.HasOutput) {
            [int64]$caseReady.command_result_key
        } else {
            $null
        }
        $resultCommandEvidence += [ordered]@{
            name = $case.Name
            generation = $evidenceGeneration
            key = $evidenceKey
            exit_code = [int]$case.ExitCode
            has_output = [bool]$case.HasOutput
            painted = [bool]$case.HasOutput
            output_rows = $outputRowsEvidence
            source_owner_offscreen = $ownerMustBeOffscreen
            completed_at_unix_ms = if ([bool]$case.HasOutput) {
                [int64]$caseReady.command_result_completed_at_unix_ms
            } else {
                [int64](@($caseReady.semantic_rows | Where-Object {
                    $_.has_result -and [int64]$_.generation -eq $previousPromptId
                })[0].result_completed_at_unix_ms)
            }
            timestamp_label = if ([bool]$case.HasOutput) {
                [string]$caseReady.command_result_label
            } else {
                $null
            }
        }
        $resultProbe = $caseReady
    }
    $initial = $resultProbe
    $initialPanel = Get-ActiveAutomexiaPanel $initial
    # Prove native PowerShell history navigation remains interactive after a
    # completed command. The recall path uses real window messages below; the
    # feature-gated controls only seed/cancel deterministically and publish
    # renderer-neutral snapshots without OCR.
    $historyToken = 'AMX_HISTORY_73491'
    $historyCommand = "Write-Output '$historyToken'"
    $historyControl = "write-line:history-seed:$historyCommand"
    $script:testStage = 'history seed command'
    Send-AutomexiaTestControl $historyControl
    $historyReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
    $controlDeadline = [DateTime]::UtcNow.AddSeconds(3)
    while ([string]$historyReady.last_control -ne $historyControl -and
           [DateTime]::UtcNow -lt $controlDeadline) {
        $historyReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    }
    if ([string]$historyReady.last_control -ne $historyControl) {
        Write-Host ($historyReady | ConvertTo-Json -Depth 8)
        throw 'The native driver did not observe the history seed control input'
    }
    $visualAnimationsEnabled = [bool]$historyReady.visual_test_animations_enabled
    $historyDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([int64]$historyReady.latest_prompt_id -le [int64]$initial.latest_prompt_id -or
            ($visualAnimationsEnabled -and
                [int64]$historyReady.command_result_pulse_generation -le
                    [int64]$initial.command_result_pulse_generation) -or
            $null -eq $historyReady.command_result_surface) -and
           [DateTime]::UtcNow -lt $historyDeadline) {
        $historyReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    }
    if ([int64]$historyReady.latest_prompt_id -le [int64]$initial.latest_prompt_id -or
        $null -eq $historyReady.command_result_surface) {
        Write-Host ($historyReady | ConvertTo-Json -Depth 8)
        throw 'PowerShell completion did not publish a new prompt and result surface'
    }
    if ($visualAnimationsEnabled -and
        [int64]$historyReady.command_result_pulse_generation -le
            [int64]$initial.command_result_pulse_generation) {
        throw 'PowerShell completion did not publish the one-shot result glow'
    }
    if (-not $visualAnimationsEnabled -and
        [int64]$historyReady.command_result_pulse_generation -ne
            [int64]$initial.command_result_pulse_generation) {
        throw 'The deterministic visual fixture unexpectedly published an animation pulse'
    }

    $resultSurface = @($historyReady.command_result_surface)
    if ($null -ne $historyReady.command_result_accent) {
        throw 'Completed output still publishes the removed vertical rail geometry'
    }
    $resultDivider = @($historyReady.command_result_divider)
    if ($resultSurface.Count -ne 4 -or
        $resultDivider.Count -ne 4) {
        Write-Host ($historyReady | ConvertTo-Json -Depth 8)
        throw 'Completed output did not publish surface and divider geometry'
    }
    $resultSurfaceBottom =
        [double]$resultSurface[1] + [double]$resultSurface[3]
    $resultMarkerGap = [double]$resultDivider[1] - $resultSurfaceBottom
    if ([double]$resultSurface[2] -lt 4.0 -or
        [double]$resultSurface[3] -lt 1.0 -or
        [double]$resultDivider[2] -lt 1.0 -or
        [double]$resultDivider[2] -gt 48.0 -or
        [double]$resultDivider[2] -gt ([double]$resultSurface[2] + 4.0) * 0.25 -or
        [double]$resultDivider[0] -le [double]$resultSurface[0] -or
        ([double]$resultDivider[0] + [double]$resultDivider[2]) -ge
            ([double]$resultSurface[0] + [double]$resultSurface[2]) -or
        $resultMarkerGap -lt 0.5 -or
        $resultMarkerGap -gt 1.5) {
        Write-Host ($historyReady | ConvertTo-Json -Depth 8)
        throw 'Command-result geometry must fill the final output row and place a short inset marker in the reserved prompt row'
    }
    $resultOpacity = @($historyReady.command_result_opacity)
    if ($resultOpacity.Count -ne 3 -or
        [double]$resultOpacity[0] -lt 0.05 -or
        [double]$resultOpacity[0] -gt 0.10 -or
        [double]$resultOpacity[1] -lt 0.35 -or
        [double]$resultOpacity[1] -gt 0.60 -or
        [double]$resultOpacity[2] -lt 0.10 -or
        [double]$resultOpacity[2] -gt 0.18) {
        Write-Host ($historyReady | ConvertTo-Json -Depth 8)
        throw 'Command-result paint regressed to an imperceptible opacity'
    }
    if ([int]$historyReady.command_result_pulse_duration_ms -ne 540) {
        throw 'Command-result lightening no longer lasts the requested 540 milliseconds'
    }
    $resultPulseHold =
        [double]$historyReady.command_result_pulse_hold_fraction
    if ($resultPulseHold -lt 0.32 -or $resultPulseHold -gt 0.34) {
        throw 'Command-result lightening no longer holds before its single fade'
    }

    # Freeze the live editor as well as renderer-owned clocks. A valid result
    # surface can otherwise be compared beside different pending shell input,
    # making a full-frame backend diff fail for an unrelated but real pixel
    # difference. Escape clears without executing; the sentinel is removed
    # after navigation and every transition must retain it byte-for-byte.
    $captureInputSentinel = 'AMX_CAPTURE_INPUT_59217'
    $captureClearControl = 'write-hex:result-capture-clear:1b'
    Send-AutomexiaTestControl $captureClearControl
    $captureClear = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    $captureInputDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$captureClear.last_control -ne $captureClearControl -or
            [string](Get-ActiveAutomexiaPanel $captureClear).raw_cursor_line_text -cne
                $blankPromptLine) -and
           [DateTime]::UtcNow -lt $captureInputDeadline) {
        $captureClear = Read-AutomexiaSnapshot -AfterSequence ([int64]$captureClear.sequence)
    }
    if ([string]$captureClear.last_control -ne $captureClearControl -or
        [string](Get-ActiveAutomexiaPanel $captureClear).raw_cursor_line_text -cne
            $blankPromptLine) {
        throw 'The native result capture could not restore the exact blank prompt line'
    }
    $captureInputControl = "write-text:result-capture-input:$captureInputSentinel"
    # The blank snapshot trims the prompt's trailing separator, while PSReadLine
    # materializes that one cell as soon as editable text exists.
    $captureExpectedLine = $blankPromptLine + ' ' + $captureInputSentinel
    Send-AutomexiaTestControl $captureInputControl
    $captureInputReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$captureClear.sequence)
    $captureInputDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$captureInputReady.last_control -ne $captureInputControl -or
            [string](Get-ActiveAutomexiaPanel $captureInputReady).raw_cursor_line_text -cne
                $captureExpectedLine) -and
           [DateTime]::UtcNow -lt $captureInputDeadline) {
        $captureInputReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$captureInputReady.sequence)
    }
    if ([string]$captureInputReady.last_control -ne $captureInputControl -or
        [string](Get-ActiveAutomexiaPanel $captureInputReady).raw_cursor_line_text -cne
            $captureExpectedLine) {
        $captureActualLine =
            [string](Get-ActiveAutomexiaPanel $captureInputReady).raw_cursor_line_text
        throw ('The native result capture could not establish deterministic live shell input ' +
            '(control={0}; actual-length={1}; expected-length={2}; sentinel-suffix={3})' -f
            ([string]$captureInputReady.last_control -eq $captureInputControl),
            $captureActualLine.Length,
            $captureExpectedLine.Length,
            $captureActualLine.EndsWith(
                $captureInputSentinel, [StringComparison]::Ordinal))
    }
    $historyReady = $captureInputReady

    $resultFramePath = if ([string]::IsNullOrWhiteSpace($ResultCapture)) {
        $null
    } else {
        [IO.Path]::GetFullPath($ResultCapture)
    }
    if ($null -ne $resultFramePath) {
        $resultFrameDirectory = [IO.Path]::GetDirectoryName($resultFramePath)
        if (-not [string]::IsNullOrWhiteSpace($resultFrameDirectory)) {
            New-Item -ItemType Directory -Force -Path $resultFrameDirectory | Out-Null
        }
    }
    $resultScale = [double]$historyReady.scale_factor
    $resultRegionX =
        [int][Math]::Floor([double]$resultSurface[0] * $resultScale)
    $resultRegionY =
        [int][Math]::Floor([double]$resultSurface[1] * $resultScale)
    $resultRegionWidth = [int][Math]::Ceiling(
        (([double]$resultDivider[0] + [double]$resultDivider[2]) -
         [double]$resultSurface[0]) * $resultScale)
    $resultRegionHeight = [int][Math]::Ceiling(
        (([double]$resultDivider[1] + [double]$resultDivider[3]) -
         [double]$resultSurface[1]) * $resultScale)
    if ($resultRegionWidth -lt 8 -or $resultRegionHeight -lt 8) {
        throw "Command-result painted region is unusable: $resultRegionWidth x $resultRegionHeight"
    }
    # Sample output glyphs independently from the divider and status
    # decoration. This prevents structural paint from masquerading as visible
    # command text in a native frame.
    $resultGlyphX = [int][Math]::Floor(
        ([double]$resultSurface[0] + 4.0) * $resultScale)
    $resultGlyphY = $resultRegionY
    $resultGlyphWidth = [int][Math]::Floor(
        [Math]::Min([double]$resultSurface[2] * 0.55, 560.0) * $resultScale)
    $resultGlyphHeight = [int][Math]::Ceiling(
        [double]$resultSurface[3] * $resultScale)
    if ($resultGlyphWidth -lt 64 -or $resultGlyphHeight -lt 8) {
        throw 'Command-result glyph sample is too small'
    }
    # Compare blank pixels in the resting surface with the following reserved
    # prompt row outside its short marker. Text diversity cannot satisfy this.
    $resultSampleX = [int][Math]::Floor(
        ([double]$resultSurface[0] + [double]$resultSurface[2] * 0.60) * $resultScale)
    $resultSampleWidth = [int][Math]::Floor(
        [double]$resultSurface[2] * 0.25 * $resultScale)
    $resultSurfaceSampleY = [int][Math]::Floor(
        ([double]$resultSurface[1] + [double]$resultSurface[3] * 0.20) * $resultScale)
    $resultSurfaceSampleHeight = [int][Math]::Max(2, [Math]::Floor(
        [double]$resultSurface[3] * 0.60 * $resultScale))
    $resultReservedRowSampleY = [int][Math]::Ceiling(
        ($resultSurfaceBottom + 1.5) * $resultScale)
    $resultReservedRowSampleHeight = [int][Math]::Max(2, [Math]::Ceiling(
        1.0 * $resultScale))
    if ($resultSampleWidth -lt 32 -or
        $resultSurfaceSampleHeight -lt 2 -or
        $resultReservedRowSampleHeight -lt 2) {
        throw 'Command-result blank-pixel contrast samples are too small'
    }
    $script:testStage = 'command-result composited surface'
    $resultCaptureDeadline = [DateTime]::UtcNow.AddSeconds(5)
    $resultCaptureAttempts = 0
    $resultPixelsValid = $false
    if (-not [AutomexiaResizeDriver]::MoveWindowTo($window, 20, 20)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not place Automexia fully on-screen for command-result capture (Win32 error $code)"
    }
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for command-result capture (Win32 error $code)"
    }
    $resultSurfaceBackground = $null
    $resultReservedRowBackground = $null
    $resultGlyphPixels = $null
    try {
        do {
            $resultCaptureAttempts++
            Start-Sleep -Milliseconds 50
            $resultFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
                $window, $resultFramePath)
            $resultPixels =
                [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
                    $window,
                    $resultRegionX,
                    $resultRegionY,
                    $resultRegionWidth,
                    $resultRegionHeight)
            $resultGlyphPixels =
                [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
                    $window,
                    $resultGlyphX,
                    $resultGlyphY,
                    $resultGlyphWidth,
                    $resultGlyphHeight)
            $resultSurfaceBackground =
                [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
                    $window,
                    $resultSampleX,
                    $resultSurfaceSampleY,
                    $resultSampleWidth,
                    $resultSurfaceSampleHeight)
            $resultReservedRowBackground =
                [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
                    $window,
                    $resultSampleX,
                    $resultReservedRowSampleY,
                    $resultSampleWidth,
                    $resultReservedRowSampleHeight)
            $resultPaintDelta =
                [Math]::Abs([int]$resultSurfaceBackground.MeanRed -
                    [int]$resultReservedRowBackground.MeanRed) +
                [Math]::Abs([int]$resultSurfaceBackground.MeanGreen -
                    [int]$resultReservedRowBackground.MeanGreen) +
                [Math]::Abs([int]$resultSurfaceBackground.MeanBlue -
                    [int]$resultReservedRowBackground.MeanBlue)
            $resultPixelsValid = (
                $resultFrame.Width -ge 100 -and
                $resultFrame.Height -ge 100 -and
                $resultFrame.NonOpaquePixelCount -eq 0 -and
                $resultPixels.SampleCount -ge 32 -and
                $resultGlyphPixels.SampleCount -ge 32 -and
                $resultGlyphPixels.DistinctColorBuckets -ge 8 -and
                $resultGlyphPixels.LuminanceSpread -ge 96 -and
                $resultPixels.DistinctColorBuckets -ge 4 -and
                $resultPixels.LuminanceSpread -ge 32 -and
                $resultPaintDelta -ge 16)
        } while (-not $resultPixelsValid -and
                 [DateTime]::UtcNow -lt $resultCaptureDeadline)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if (-not $resultPixelsValid) {
        throw "Command-result pixels did not settle after $resultCaptureAttempts attempts: samples=$($resultPixels.SampleCount), buckets=$($resultPixels.DistinctColorBuckets), spread=$($resultPixels.LuminanceSpread), glyph-buckets=$($resultGlyphPixels.DistinctColorBuckets), glyph-spread=$($resultGlyphPixels.LuminanceSpread), blank-pixel-delta=$resultPaintDelta, surface-rgb=$($resultSurfaceBackground.MeanRed)/$($resultSurfaceBackground.MeanGreen)/$($resultSurfaceBackground.MeanBlue), reserved-row-rgb=$($resultReservedRowBackground.MeanRed)/$($resultReservedRowBackground.MeanGreen)/$($resultReservedRowBackground.MeanBlue)"
    }
    if ($resultPaintDelta -lt 16) {
        throw "Command-result resting paint is not perceptible against its reserved row: RGB delta $resultPaintDelta"
    }

    # Resize immediately before the public shortcut sequence. This preserves
    # the reported real-world ordering and proves the freshly reflowed frame,
    # not a stable pre-resize frame, owns each command badge exactly once.
    $commandJumpBaseline = $historyReady
    $commandJumpRestoreWidth = [int]$commandJumpBaseline.window_width
    $commandJumpRestoreHeight = [int]$commandJumpBaseline.window_height
    $commandJumpResizeWidth = [Math]::Max(
        520, [Math]::Min(900, $commandJumpRestoreWidth - 180))
    $commandJumpResizeHeight = [Math]::Max(
        360, [Math]::Min(640, $commandJumpRestoreHeight - 120))
    $script:testStage = 'resize before native command navigation'
    if (-not [AutomexiaResizeDriver]::MoveWindow(
            $window, 40, 40, $commandJumpResizeWidth,
            $commandJumpResizeHeight, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "MoveWindow failed before command navigation with Win32 error $code"
    }
    $commandJumpResized = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpBaseline.sequence)
    $commandJumpDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int]$commandJumpResized.columns -eq [int]$commandJumpBaseline.columns -and
           [int]$commandJumpResized.rows -eq [int]$commandJumpBaseline.rows -and
           [DateTime]::UtcNow -lt $commandJumpDeadline) {
        $commandJumpResized = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpResized.sequence)
    }
    if ([int]$commandJumpResized.columns -eq [int]$commandJumpBaseline.columns -and
        [int]$commandJumpResized.rows -eq [int]$commandJumpBaseline.rows) {
        throw 'Native resize did not change terminal geometry before command navigation'
    }
    Assert-AutomexiaCommandResultPaintIsolation `
        $commandJumpResized 'freshly reflowed command-result frame'
    $commandJumpBaseline = $commandJumpResized

    # Use real foreground Ctrl+Shift+Arrow input. The selected PowerShell pane
    # must move between OSC 133 command marks while its command line, prompt
    # generation, route, and PTY-visible state remain unchanged. A downward
    # jump must reverse direction and stop cleanly at the live prompt boundary.
    $commandJumpPanel = Get-ActiveAutomexiaPanel $commandJumpBaseline
    $commandJumpRawLine = [string]$commandJumpPanel.raw_cursor_line_text
    $commandJumpPrompt = [int64]$commandJumpPanel.raw_cursor_prompt_id
    $commandJumpRoute = [int64]$commandJumpPanel.route_id
    $script:testStage = 'native previous command jump'
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
            $window, 0x26, $true, $true, $true)) {
        throw 'Could not deliver native Ctrl+Shift+Up command navigation'
    }
    $commandJumpPrevious = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpBaseline.sequence)
    $commandJumpDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int]$commandJumpPrevious.display_offset -le
           [int]$commandJumpBaseline.display_offset -and
           [DateTime]::UtcNow -lt $commandJumpDeadline) {
        $commandJumpPrevious = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpPrevious.sequence)
    }
    if ([int]$commandJumpPrevious.display_offset -le
        [int]$commandJumpBaseline.display_offset) {
        Write-Host ($commandJumpPrevious | ConvertTo-Json -Depth 10)
        throw 'Ctrl+Shift+Up did not jump to the previous command marker'
    }
    $commandJumpPreviousPanel = Get-ActiveAutomexiaPanel $commandJumpPrevious
    if ([int64]$commandJumpPreviousPanel.raw_cursor_prompt_id -ne $commandJumpPrompt -or
        [int64]$commandJumpPreviousPanel.route_id -ne $commandJumpRoute -or
        [string]$commandJumpPreviousPanel.raw_cursor_line_text -ne $commandJumpRawLine) {
        throw 'Previous-command navigation changed prompt, route, or PTY-visible input state'
    }
    Assert-AutomexiaCommandResultPaintIsolation `
        $commandJumpPrevious 'previous-command result frame'
    $resultNavigationFramePath = if ([string]::IsNullOrWhiteSpace($ResultNavigationCapture)) {
        $null
    } else {
        [IO.Path]::GetFullPath($ResultNavigationCapture)
    }
    if ($null -ne $resultNavigationFramePath) {
        $resultNavigationDirectory = [IO.Path]::GetDirectoryName($resultNavigationFramePath)
        if (-not [string]::IsNullOrWhiteSpace($resultNavigationDirectory)) {
            New-Item -ItemType Directory -Force -Path $resultNavigationDirectory | Out-Null
        }
    }
    $script:testStage = 'resized command-navigation composited frame'
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        throw 'Could not expose Automexia for resized command-navigation capture'
    }
    try {
        Start-Sleep -Milliseconds 100
        $resultNavigationFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
            $window, $resultNavigationFramePath)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if ($resultNavigationFrame.Width -lt 100 -or
        $resultNavigationFrame.Height -lt 100 -or
        $resultNavigationFrame.SampleCount -lt 100 -or
        $resultNavigationFrame.DistinctColorBuckets -lt 8 -or
        $resultNavigationFrame.LuminanceSpread -lt 32) {
        throw 'Resized command-navigation composited frame is blank or unreadable'
    }

    $script:testStage = 'native next command jump'
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
            $window, 0x28, $true, $true, $true)) {
        throw 'Could not deliver native Ctrl+Shift+Down command navigation'
    }
    $commandJumpNext = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpPrevious.sequence)
    $commandJumpDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int]$commandJumpNext.display_offset -ge
           [int]$commandJumpPrevious.display_offset -and
           [DateTime]::UtcNow -lt $commandJumpDeadline) {
        $commandJumpNext = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpNext.sequence)
    }
    if ([int]$commandJumpNext.display_offset -ge
        [int]$commandJumpPrevious.display_offset) {
        Write-Host ($commandJumpNext | ConvertTo-Json -Depth 10)
        throw 'Ctrl+Shift+Down did not jump toward the next command marker'
    }
    Assert-AutomexiaCommandResultPaintIsolation `
        $commandJumpNext 'next-command result frame'
    for ($jump = 0; $jump -lt 64 -and
         [int]$commandJumpNext.display_offset -ne 0; $jump++) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
                $window, 0x28, $true, $true, $true)) {
            throw 'Could not continue native next-command navigation'
        }
        $commandJumpNext = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpNext.sequence)
        Assert-AutomexiaCommandResultPaintIsolation `
            $commandJumpNext "next-command result frame $jump"
    }
    if ([int]$commandJumpNext.display_offset -ne 0) {
        throw 'Next-command navigation did not reach the live prompt boundary within 64 marked commands'
    }
    $commandJumpNextPanel = Get-ActiveAutomexiaPanel $commandJumpNext
    if ([int64]$commandJumpNextPanel.raw_cursor_prompt_id -ne $commandJumpPrompt -or
        [int64]$commandJumpNextPanel.route_id -ne $commandJumpRoute -or
        [string]$commandJumpNextPanel.raw_cursor_line_text -ne $commandJumpRawLine) {
        throw 'Next-command navigation changed prompt, route, or PTY-visible input state'
    }
    if (-not [AutomexiaResizeDriver]::MoveWindow(
            $window, 40, 40, $commandJumpRestoreWidth,
            $commandJumpRestoreHeight, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "MoveWindow failed while restoring command-navigation size with Win32 error $code"
    }
    $commandJumpRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpNext.sequence)
    $commandJumpDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int]$commandJumpRestored.columns -eq [int]$commandJumpNext.columns -and
           [int]$commandJumpRestored.rows -eq [int]$commandJumpNext.rows -and
           [DateTime]::UtcNow -lt $commandJumpDeadline) {
        $commandJumpRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpRestored.sequence)
    }
    if ([int]$commandJumpRestored.columns -eq [int]$commandJumpNext.columns -and
        [int]$commandJumpRestored.rows -eq [int]$commandJumpNext.rows) {
        throw 'Native command-navigation test did not restore terminal geometry'
    }
    $commandJumpRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpRestored.sequence)
    Assert-AutomexiaCommandResultPaintIsolation `
        $commandJumpRestored 'restored command-result frame'
    $captureReleaseControl = 'write-hex:result-capture-release:1b5b36373b34363b333b313b383b315f1b5b36373b34363b303b303b383b315f'
    Send-AutomexiaTestControl $captureReleaseControl
    $captureReleased = Read-AutomexiaSnapshot -AfterSequence ([int64]$commandJumpRestored.sequence)
    $captureInputDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$captureReleased.last_control -ne $captureReleaseControl -or
            [string](Get-ActiveAutomexiaPanel $captureReleased).raw_cursor_line_text -cne
                $blankPromptLine) -and
           [DateTime]::UtcNow -lt $captureInputDeadline) {
        $captureReleased = Read-AutomexiaSnapshot -AfterSequence ([int64]$captureReleased.sequence)
    }
    if ([string]$captureReleased.last_control -ne $captureReleaseControl -or
        [string](Get-ActiveAutomexiaPanel $captureReleased).raw_cursor_line_text -cne
            $blankPromptLine) {
        $releasedLine =
            [string](Get-ActiveAutomexiaPanel $captureReleased).raw_cursor_line_text
        throw ('The native result-capture input sentinel was not cleared without execution ' +
            '(control={0}; actual-length={1}; blank-length={2}; still-has-sentinel={3})' -f
            ([string]$captureReleased.last_control -eq $captureReleaseControl),
            $releasedLine.Length,
            $blankPromptLine.Length,
            ($releasedLine.IndexOf(
                $captureInputSentinel, [StringComparison]::Ordinal) -ge 0))
    }
    $historyReady = $captureReleased

    # Establish whether latency is in generic frontend -> PTY delivery or in a
    # PSReadLine history action. A printable key uses the same channel, ConPTY,
    # VT parser, damage, and renderer path as normal interactive typing.
    $typingTimer = [Diagnostics.Stopwatch]::StartNew()
    $typingControl = 'write-text:latency-printable:q'
    Send-AutomexiaTestControl $typingControl
    $typingSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    $typingDeadline = [DateTime]::UtcNow.AddSeconds(3)
    $typingControlObservedMilliseconds = $null
    while (([string](Get-ActiveAutomexiaPanel $typingSnapshot).cursor_line_text) -notlike '*q*' -and
           [DateTime]::UtcNow -lt $typingDeadline) {
        if ($null -eq $typingControlObservedMilliseconds -and
            [string]$typingSnapshot.last_control -eq $typingControl) {
            $typingControlObservedMilliseconds = $typingTimer.ElapsedMilliseconds
        }
        $typingSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$typingSnapshot.sequence)
    }
    $typingTimer.Stop()
    if ($null -eq $typingControlObservedMilliseconds -and
        [string]$typingSnapshot.last_control -eq $typingControl) {
        $typingControlObservedMilliseconds = $typingTimer.ElapsedMilliseconds
    }
    if ($null -eq $typingControlObservedMilliseconds) {
        throw 'The native driver did not observe the printable input control'
    }
    $typingShellMilliseconds = [Math]::Max(
        0, $typingTimer.ElapsedMilliseconds - $typingControlObservedMilliseconds)
    if (([string](Get-ActiveAutomexiaPanel $typingSnapshot).cursor_line_text) -notlike '*q*') {
        throw 'Printable input did not reach PowerShell and the renderer'
    }
    if ($typingShellMilliseconds -gt 500) {
        throw "PowerShell plain typed input exceeded 500 ms ($typingShellMilliseconds ms; $($typingTimer.ElapsedMilliseconds) ms total)"
    }
    $eraseControl = 'write-hex:latency-erase:1b5b383b31343b383b313b303b315f1b5b383b31343b303b303b303b315f'
    Send-AutomexiaTestControl $eraseControl
    $erased = Read-AutomexiaSnapshot -AfterSequence ([int64]$typingSnapshot.sequence)
    $eraseDeadline = [DateTime]::UtcNow.AddSeconds(3)
    while ((([string](Get-ActiveAutomexiaPanel $erased).cursor_line_text) -like '*q*' -or
            [string]$erased.last_control -ne $eraseControl) -and
           [DateTime]::UtcNow -lt $eraseDeadline) {
        $erased = Read-AutomexiaSnapshot -AfterSequence ([int64]$erased.sequence)
    }
    if (([string](Get-ActiveAutomexiaPanel $erased).cursor_line_text) -like '*q*') {
        throw 'Backspace did not clear the printable latency probe'
    }

    $upTimer = [Diagnostics.Stopwatch]::StartNew()
    # The Windows key encoder is covered by Rust unit tests. Inject its CSI Up
    # sequence through Automexia's input queue here so this headless native test
    # deterministically covers ConPTY, PSReadLine, VT, damage, and rendering
    # without depending on the desktop foreground-lock policy.
    $script:testStage = 'Up Arrow recall'
    Send-AutomexiaTestControl 'write-hex:history-up:1b5b41'
    $upRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    $upDeadline = [DateTime]::UtcNow.AddSeconds(3)
    $upRawObservedMilliseconds = $null
    while (([string](Get-ActiveAutomexiaPanel $upRecall).cursor_line_text) -notlike "*$historyToken*" -and
           [DateTime]::UtcNow -lt $upDeadline) {
        if ($null -eq $upRawObservedMilliseconds -and
            [string](Get-ActiveAutomexiaPanel $upRecall).raw_cursor_line_text -like "*$historyToken*") {
            $upRawObservedMilliseconds = $upTimer.ElapsedMilliseconds
        }
        $upRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$upRecall.sequence)
    }
    if ($null -eq $upRawObservedMilliseconds -and
        [string](Get-ActiveAutomexiaPanel $upRecall).raw_cursor_line_text -like "*$historyToken*") {
        $upRawObservedMilliseconds = $upTimer.ElapsedMilliseconds
    }
    $upTimer.Stop()
    $upShellMilliseconds = $upTimer.ElapsedMilliseconds
    if (([string](Get-ActiveAutomexiaPanel $upRecall).cursor_line_text) -notlike "*$historyToken*") {
        Write-Host ($upRecall | ConvertTo-Json -Depth 8)
        throw 'Up Arrow did not recall the latest PowerShell command'
    }
    if ($upShellMilliseconds -gt $PowerShellHistoryBudgetMilliseconds) {
        throw "Up Arrow recall exceeded the $PowerShellHistoryBudgetMilliseconds ms Windows PowerShell budget ($upShellMilliseconds ms; raw terminal observed at $upRawObservedMilliseconds ms)"
    }
    # The control contains distinct down/up records back-to-back, matching a
    # physical tap without holding the key through the repeat interval.
    $upReleased = $upRecall

    # Cancel the recalled line, clear its old screen occurrence, then search by
    # a unique fragment. Seeing it afterward proves Ctrl+R produced a live
    # PSReadLine match instead of merely finding stale terminal output.
    $script:testStage = 'cancel recalled history'
    Send-AutomexiaTestControl 'write-hex:history-cancel-up:1b5b36373b34363b333b313b383b315f1b5b36373b34363b303b303b383b315f'
    $cancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$upReleased.sequence)
    $cancelDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int64]$cancelled.latest_prompt_id -le [int64]$historyReady.latest_prompt_id -and
           [DateTime]::UtcNow -lt $cancelDeadline) {
        $cancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelled.sequence)
    }
    $script:testStage = 'clear history display'
    Send-AutomexiaTestControl 'write-line:history-clear:Clear-Host'
    $cleared = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelled.sequence)
    $clearDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int64]$cleared.latest_prompt_id -le [int64]$cancelled.latest_prompt_id -and
           [DateTime]::UtcNow -lt $clearDeadline) {
        $cleared = Read-AutomexiaSnapshot -AfterSequence ([int64]$cleared.sequence)
    }

    # Ctrl+R: VK_R=82, scan=19, Unicode Ctrl+R=18, left-control state=8.
    # Send the physical key-down and the search text as separate controls. In
    # DECSET 9001 mode every typed character must remain a Win32 input record;
    # mixing raw UTF-8 into that stream is invalid and previously added a
    # misleading ConsoleHost timeout to this latency measurement.
    $searchTimer = [Diagnostics.Stopwatch]::StartNew()
    $ctrlRControl = 'write-hex:history-search-open:1b5b38323b31393b31383b313b383b315f1b5b38323b31393b303b303b383b315f'
    $script:testStage = 'Ctrl+R history search open'
    Send-AutomexiaTestControl $ctrlRControl
    $searchOpened = Read-AutomexiaSnapshot -AfterSequence ([int64]$cleared.sequence)
    $searchOpenDeadline = [DateTime]::UtcNow.AddSeconds(3)
    while ([string]$searchOpened.last_control -ne $ctrlRControl -and
           [DateTime]::UtcNow -lt $searchOpenDeadline) {
        $searchOpened = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchOpened.sequence)
    }
    if ([string]$searchOpened.last_control -ne $ctrlRControl) {
        throw 'The native driver did not observe the Ctrl+R key-down event'
    }
    $searchControl = "write-text:history-search-text:$historyToken"
    $script:testStage = 'Ctrl+R history search text'
    Send-AutomexiaTestControl $searchControl
    $searchRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchOpened.sequence)
    $searchDeadline = [DateTime]::UtcNow.AddSeconds(3)
    $searchControlObservedMilliseconds = $null
    while (([string](Get-ActiveAutomexiaPanel $searchRecall).cursor_line_text) -notlike "*$historyToken*" -and
           [DateTime]::UtcNow -lt $searchDeadline) {
        if ($null -eq $searchControlObservedMilliseconds -and
            [string]$searchRecall.last_control -eq $searchControl) {
            $searchControlObservedMilliseconds = $searchTimer.ElapsedMilliseconds
        }
        $searchRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchRecall.sequence)
    }
    $searchTimer.Stop()
    if ($null -eq $searchControlObservedMilliseconds -and
        [string]$searchRecall.last_control -eq $searchControl) {
        $searchControlObservedMilliseconds = $searchTimer.ElapsedMilliseconds
    }
    if ($null -eq $searchControlObservedMilliseconds) {
        throw 'The native driver did not observe the Ctrl+R control input'
    }
    $searchShellMilliseconds = [Math]::Max(
        0, $searchTimer.ElapsedMilliseconds - $searchControlObservedMilliseconds)
    if (([string](Get-ActiveAutomexiaPanel $searchRecall).cursor_line_text) -notlike "*$historyToken*") {
        Write-Host ($searchRecall | ConvertTo-Json -Depth 8)
        throw 'Ctrl+R did not find the seeded PowerShell history command'
    }
    if ($searchShellMilliseconds -gt $PowerShellHistoryBudgetMilliseconds) {
        throw "Ctrl+R search exceeded the $PowerShellHistoryBudgetMilliseconds ms Windows PowerShell budget ($searchShellMilliseconds ms; $($searchTimer.ElapsedMilliseconds) ms including test-control delivery)"
    }
    $script:testStage = 'cancel history search'
    $searchReleased = $searchRecall
    $cancelSearchControl = 'write-hex:history-cancel-search:1b5b36373b34363b333b313b383b315f1b5b36373b34363b303b303b383b315f'
    Send-AutomexiaTestControl $cancelSearchControl
    $historyDone = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchReleased.sequence)
    $cancelSearchDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$historyDone.last_control -ne $cancelSearchControl -or
            [int64]$historyDone.latest_prompt_id -le [int64]$searchReleased.latest_prompt_id -or
            ([string](Get-ActiveAutomexiaPanel $historyDone).cursor_line_text) -like "*$historyToken*" -or
            ([string](Get-ActiveAutomexiaPanel $historyDone).visible_text) -like '*bck-i-search*') -and
           [DateTime]::UtcNow -lt $cancelSearchDeadline) {
        $historyDone = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
    }
    if ([string]$historyDone.last_control -ne $cancelSearchControl -or
        [int64]$historyDone.latest_prompt_id -le [int64]$searchReleased.latest_prompt_id -or
        ([string](Get-ActiveAutomexiaPanel $historyDone).cursor_line_text) -like "*$historyToken*" -or
        ([string](Get-ActiveAutomexiaPanel $historyDone).visible_text) -like '*bck-i-search*') {
        Write-Host ($historyDone | ConvertTo-Json -Depth 10)
        throw 'Ctrl+R teardown did not restore a clean PowerShell prompt'
    }

    # Create a tab inside the selected pane through the same implementation
    # path as Ctrl+Alt+T. It must own a new ConPTY/route while preserving the
    # selected PowerShell launch intent and must not add another split panel.
    $script:testStage = 'create pane-local tab'
    Send-AutomexiaTestControl 'local-tab:local-create'
    $localCreated = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
    $localDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (([int]$localCreated.panel_count -ne 1 -or
            [int](Get-ActiveAutomexiaPanel $localCreated).local_tab_count -ne 2 -or
            [int64](Get-ActiveAutomexiaPanel $localCreated).route_id -eq [int64]$initialPanel.route_id -or
            -not [bool]$localCreated.full_path_visible) -and
           [DateTime]::UtcNow -lt $localDeadline) {
        $localCreated = Read-AutomexiaSnapshot -AfterSequence ([int64]$localCreated.sequence)
    }
    $localPanel = Get-ActiveAutomexiaPanel $localCreated
    if ([int]$localCreated.panel_count -ne 1 -or [int]$localPanel.local_tab_count -ne 2) {
        Write-Host ($localCreated | ConvertTo-Json -Depth 10)
        throw 'Ctrl+Alt+T local-tab path did not create exactly one sibling in the selected pane'
    }
    $localRoutes = @($localPanel.local_tabs | ForEach-Object { [int64]$_.route_id } | Sort-Object -Unique)
    $localPids = @($localPanel.local_tabs | ForEach-Object { [int64]$_.shell_pid } | Sort-Object -Unique)
    if ($localRoutes.Count -ne 2 -or $localPids.Count -ne 2 -or $localPids[0] -le 0) {
        throw 'Pane-local PowerShell tabs reused a route or ConPTY process'
    }
    if ($null -eq $localPanel.local_tab_rail_rect) {
        Write-Host ($localCreated | ConvertTo-Json -Depth 10)
        throw 'Pane-local tabs did not reserve a rail inside their owning pane'
    }
    $localLayout = @($localPanel.layout_rect)
    $localRail = @($localPanel.local_tab_rail_rect)
    $localTerminal = @($localPanel.terminal_rect)
    if ($localRail.Count -ne 4 -or $localTerminal.Count -ne 4 -or
        [double]$localRail[0] -ne [double]$localLayout[0] -or
        [double]$localRail[1] -ne [double]$localLayout[1] -or
        [double]$localRail[2] -ne [double]$localLayout[2] -or
        [double]$localTerminal[1] -ne
            ([double]$localLayout[1] + [double]$localRail[3]) -or
        [double]$localTerminal[3] -ge [double]$localLayout[3]) {
        Write-Host ($localCreated | ConvertTo-Json -Depth 10)
        throw 'Pane-local rail and terminal content rectangles overlap or escape the pane'
    }
    if ($localPanel.current_directory -ne $initialPanel.current_directory -or
        $localPanel.launch_program -ne $initialPanel.launch_program -or
        $localPanel.profile_identity -ne $initialPanel.profile_identity -or
        (($localPanel.launch_args | ConvertTo-Json -Compress) -ne
         ($initialPanel.launch_args | ConvertTo-Json -Compress))) {
        Write-Host ($localCreated | ConvertTo-Json -Depth 10)
        throw 'Pane-local tab did not preserve the selected PowerShell profile and directory'
    }

    # Exercise the same wraparound selection methods used by Alt+PageUp and
    # Alt+PageDown. Binding-table tests independently prove those chords map to
    # these actions; the native snapshot proves route focus changes only inside
    # the owning pane and preserves both PTYs.
    $createdLocalRoute = [int64]$localPanel.route_id
    $script:testStage = 'previous pane-local tab navigation'
    Send-AutomexiaTestControl 'select-local-prev:local-prev'
    $localPrevious = Read-AutomexiaSnapshot -AfterSequence ([int64]$localCreated.sequence)
    $localPreviousDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64](Get-ActiveAutomexiaPanel $localPrevious).route_id -ne
           [int64]$initialPanel.route_id -and
           [DateTime]::UtcNow -lt $localPreviousDeadline) {
        $localPrevious = Read-AutomexiaSnapshot -AfterSequence ([int64]$localPrevious.sequence)
    }
    if ([int64](Get-ActiveAutomexiaPanel $localPrevious).route_id -ne
        [int64]$initialPanel.route_id) {
        Write-Host ($localPrevious | ConvertTo-Json -Depth 10)
        throw 'Previous pane-local tab navigation did not wrap to the source tab'
    }

    $script:testStage = 'next pane-local tab navigation'
    Send-AutomexiaTestControl 'select-local-next:local-next'
    $localNext = Read-AutomexiaSnapshot -AfterSequence ([int64]$localPrevious.sequence)
    $localNextDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64](Get-ActiveAutomexiaPanel $localNext).route_id -ne
           $createdLocalRoute -and [DateTime]::UtcNow -lt $localNextDeadline) {
        $localNext = Read-AutomexiaSnapshot -AfterSequence ([int64]$localNext.sequence)
    }
    if ([int64](Get-ActiveAutomexiaPanel $localNext).route_id -ne $createdLocalRoute -or
        [int](Get-ActiveAutomexiaPanel $localNext).local_tab_count -ne 2) {
        Write-Host ($localNext | ConvertTo-Json -Depth 10)
        throw 'Next pane-local tab navigation escaped its pane or lost a sibling PTY'
    }

    # Return to the source and close the inactive sibling by index. This is the
    # native regression for the old cascade-close failure: the active source
    # route/PID and the window must survive unchanged.
    $script:testStage = 'select pane-local source'
    Send-AutomexiaTestControl 'select-local:local-source:0'
    $localSource = Read-AutomexiaSnapshot -AfterSequence ([int64]$localNext.sequence)
    $localSourceDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64](Get-ActiveAutomexiaPanel $localSource).route_id -ne
           [int64]$initialPanel.route_id -and [DateTime]::UtcNow -lt $localSourceDeadline) {
        $localSource = Read-AutomexiaSnapshot -AfterSequence ([int64]$localSource.sequence)
    }
    $script:testStage = 'close inactive pane-local tab'
    Send-AutomexiaTestControl 'close-local:local-close-inactive:1'
    $localClosed = Read-AutomexiaSnapshot -AfterSequence ([int64]$localSource.sequence)
    $localCloseDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([int](Get-ActiveAutomexiaPanel $localClosed).local_tab_count -ne 1 -or
            [int64](Get-ActiveAutomexiaPanel $localClosed).route_id -ne [int64]$initialPanel.route_id) -and
           [DateTime]::UtcNow -lt $localCloseDeadline) {
        $localClosed = Read-AutomexiaSnapshot -AfterSequence ([int64]$localClosed.sequence)
    }
    $survivingLocalPanel = Get-ActiveAutomexiaPanel $localClosed
    if ([int]$localClosed.panel_count -ne 1 -or
        [int]$survivingLocalPanel.local_tab_count -ne 1 -or
        [int64]$survivingLocalPanel.route_id -ne [int64]$initialPanel.route_id -or
        [int64]$survivingLocalPanel.shell_pid -ne [int64]$initialPanel.shell_pid) {
        Write-Host ($localClosed | ConvertTo-Json -Depth 10)
        throw 'Closing an inactive pane-local tab changed or closed the active source session'
    }
    if ($null -ne $survivingLocalPanel.local_tab_rail_rect -or
        [double]$survivingLocalPanel.terminal_rect[1] -ne
            [double]$survivingLocalPanel.layout_rect[1]) {
        Write-Host ($localClosed | ConvertTo-Json -Depth 10)
        throw 'Single-tab pane retained stale local-tab rail geometry'
    }
    if ($null -eq $localClosed.retired_renderer_grid_count -or
        $null -eq $localClosed.renderer_grid_count -or
        $null -eq $localClosed.owned_route_count -or
        [int]$localClosed.retired_renderer_grid_count -ne 0 -or
        [int]$localClosed.renderer_grid_count -gt [int]$localClosed.owned_route_count) {
        Write-Host ('Renderer grid ownership after inactive tab close: ' + (@{
            renderer_grid_count = $localClosed.renderer_grid_count
            owned_route_count = $localClosed.owned_route_count
            retired_renderer_grid_count = $localClosed.retired_renderer_grid_count
        } | ConvertTo-Json -Compress))
        throw 'Closing an inactive pane-local tab retained a renderer grid without a live route'
    }
    $historyDone = $localClosed

    # Enter the real interactive CMD child through PowerShell's bare cmd alias.
    # It must remain inside this ConPTY and publish the same renderer metadata
    # without waiting for a second keypress.
    $script:testStage = 'interactive CMD startup parity'
    $cmdTypeControl = 'write-text:cmd-type:cmd'
    Send-AutomexiaTestControl $cmdTypeControl
    $cmdTyped = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
    $cmdTypeDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while ((([string]$cmdTyped.last_control -ne $cmdTypeControl) -or
            -not ([string](Get-ActiveAutomexiaPanel $cmdTyped).cursor_line_text).Contains('cmd')) -and
           [DateTime]::UtcNow -lt $cmdTypeDeadline) {
        $cmdTyped = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdTyped.sequence)
    }
    if ([string]$cmdTyped.last_control -ne $cmdTypeControl -or
        -not ([string](Get-ActiveAutomexiaPanel $cmdTyped).cursor_line_text).Contains('cmd')) {
        Write-Host ($cmdTyped | ConvertTo-Json -Depth 10)
        throw 'Interactive PowerShell did not visibly accept the CMD command text'
    }

    $cmdEnterControl = 'write-line:cmd-submit:'
    Send-AutomexiaTestControl $cmdEnterControl
    $cmdReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdTyped.sequence)
    $cmdDeadline = [DateTime]::UtcNow.AddSeconds(20)
    # CMD publishes metadata before its final prompt glyph. Wait for the same
    # complete prompt that the assertion below requires, not an intermediate frame.
    while (([string]$cmdReady.last_control -ne $cmdEnterControl -or
            (Get-ActiveAutomexiaPanel $cmdReady).shell_name -ne 'CMD' -or
            -not [bool](Get-ActiveAutomexiaPanel $cmdReady).shell_integration -or
            -not [bool](Get-ActiveAutomexiaPanel $cmdReady).shell_prompt_active -or
            (@((Get-ActiveAutomexiaPanel $cmdReady).context_segments) |
                ConvertTo-Json -Compress) -ne $expectedContextSegmentsJson -or
            (Get-ActiveAutomexiaPanel $cmdReady).shell_user -ne [Environment]::UserName -or
            [IO.Path]::GetFileName([string](Get-ActiveAutomexiaPanel $cmdReady).shell_path) -ine 'cmd.exe' -or
            -not ([string](Get-ActiveAutomexiaPanel $cmdReady).cursor_line_text).Contains([char]0x03BB) -or
            -not [bool]$cmdReady.full_path_visible) -and
           [DateTime]::UtcNow -lt $cmdDeadline) {
        $cmdReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdReady.sequence)
    }
    $cmdPanel = Get-ActiveAutomexiaPanel $cmdReady
    if ([string]$cmdReady.last_control -ne $cmdEnterControl -or
        $cmdPanel.shell_name -ne 'CMD' -or
        -not [bool]$cmdPanel.shell_integration -or
        -not [bool]$cmdPanel.shell_prompt_active -or
        (@($cmdPanel.context_segments) | ConvertTo-Json -Compress) -ne
            $expectedContextSegmentsJson -or
        -not [bool]$cmdReady.full_path_visible -or
        $cmdPanel.shell_user -ne [Environment]::UserName -or
        [IO.Path]::GetFileName([string]$cmdPanel.shell_path) -ine 'cmd.exe' -or
        -not ([string]$cmdPanel.cursor_line_text).Contains([char]0x03BB)) {
        Write-Host ($cmdReady | ConvertTo-Json -Depth 10)
        # Report only executable names and process relationships; arguments and
        # paths can contain private shell/provider data.
        $cmdStartupProcesses = @(Get-AutomexiaOwnedProcessIds $process.Id $configRoot |
            Select-Object -First 32 | ForEach-Object {
                Get-CimInstance Win32_Process -Filter "ProcessId = $_" |
                    Select-Object Name, ProcessId, ParentProcessId
            })
        Write-Host ($cmdStartupProcesses | ConvertTo-Json -Compress)
        throw 'Interactive CMD did not publish its shell, user, path, prompt, and complete working directory automatically'
    }

    $cmdPreviousResultKey = if ($null -eq $cmdReady.command_result_key) {
        -1
    } else {
        [int64]$cmdReady.command_result_key
    }

    # Prove the display-only DOSKEY helper is active in the real pane and keeps
    # category/file glyphs directly beside names.
    $folderGlyph = [char]::ConvertFromUtf32(0xF19F6)
    $rustGlyph = [char]0xE7A8
    $script:testStage = 'interactive CMD icon listing'
    Send-AutomexiaTestControl ('write-line:cmd-list:ls "{0}"' -f $cmdListingFixture)
    $cmdListing = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdReady.sequence)
    $cmdListingDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while ((-not ((Get-ActiveAutomexiaPanel $cmdListing).visible_text.Contains("$folderGlyph apps\")) -or
            -not ((Get-ActiveAutomexiaPanel $cmdListing).visible_text.Contains("$rustGlyph Cargo.toml")) -or
            -not [bool](Get-ActiveAutomexiaPanel $cmdListing).shell_prompt_active -or
            $null -eq $cmdListing.command_result_key -or
            [int64]$cmdListing.command_result_key -le $cmdPreviousResultKey -or
            $null -ne $cmdListing.command_result_generation -or
            $null -ne $cmdListing.command_result_exit_code -or
            [int64]$cmdListing.command_result_completed_at_unix_ms -le 1600000000000 -or
            [string]$cmdListing.command_result_label -notlike '*2026-08-26 12:34:56' -or
            $null -eq $cmdListing.command_result_surface -or
            $null -eq $cmdListing.command_result_divider) -and
           [DateTime]::UtcNow -lt $cmdListingDeadline) {
        $cmdListing = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdListing.sequence)
    }
    $cmdListingPanel = Get-ActiveAutomexiaPanel $cmdListing
    if (-not $cmdListingPanel.visible_text.Contains("$folderGlyph apps\") -or
        -not $cmdListingPanel.visible_text.Contains("$rustGlyph Cargo.toml")) {
        Write-Host ($cmdListing | ConvertTo-Json -Depth 10)
        throw 'Interactive CMD ls did not render category and Rust icons immediately beside names'
    }
    if ($null -eq $cmdListing.command_result_key -or
        [int64]$cmdListing.command_result_key -le $cmdPreviousResultKey -or
        $null -ne $cmdListing.command_result_generation -or
        $null -ne $cmdListing.command_result_exit_code -or
        [int64]$cmdListing.command_result_completed_at_unix_ms -le 1600000000000 -or
        [string]$cmdListing.command_result_label -notlike '*2026-08-26 12:34:56' -or
        $null -eq $cmdListing.command_result_surface -or
        $null -eq $cmdListing.command_result_divider) {
        Write-Host ($cmdListing | ConvertTo-Json -Depth 10)
        throw 'Interactive CMD output did not publish a fresh neutral result surface'
    }

    # Exit must restore the parent metadata on PowerShell's very next prompt;
    # stale CMD identity is a failure even if another keystroke would repair it.
    $script:testStage = 'restore PowerShell after CMD exit'
    Send-AutomexiaTestControl 'write-line:cmd-exit:exit'
    $powerShellRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdListing.sequence)
    $restoreDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (((Get-ActiveAutomexiaPanel $powerShellRestored).shell_name -ne 'PowerShell' -or
            -not [bool](Get-ActiveAutomexiaPanel $powerShellRestored).shell_prompt_active -or
            -not [bool]$powerShellRestored.full_path_visible) -and
           [DateTime]::UtcNow -lt $restoreDeadline) {
        $powerShellRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$powerShellRestored.sequence)
    }
    $restoredPanel = Get-ActiveAutomexiaPanel $powerShellRestored
    if ($restoredPanel.shell_name -ne 'PowerShell' -or
        -not [bool]$restoredPanel.shell_prompt_active -or
        -not [bool]$powerShellRestored.full_path_visible) {
        Write-Host ($powerShellRestored | ConvertTo-Json -Depth 10)
        throw 'PowerShell metadata did not replace CMD identity immediately after exit'
    }
    $historyDone = $powerShellRestored

    # Clear-Host intentionally removes old semantic rows earlier in this
    # scenario. Seed fresh, multi-line marked commands immediately before the
    # split so the isolation check cannot pass or fail based on incidental
    # scrollback retained by a shell version or window height.
    for ($seedIndex = 1; $seedIndex -le 2; $seedIndex++) {
        $previousPrompt = [int64]$historyDone.latest_prompt_id
        $script:testStage = "seed selected-pane command navigation $seedIndex"
        Send-AutomexiaTestControl (
            "write-line:split-history-${seedIndex}:" +
            "1..24 | ForEach-Object { 'AUTOMEXIA_SPLIT_HISTORY_{0:D2}_' -f `$_ }"
        )
        $seededHistory = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
        $seedDeadline = [DateTime]::UtcNow.AddSeconds(10)
        while (([int64]$seededHistory.latest_prompt_id -le $previousPrompt -or
                [int](Get-ActiveAutomexiaPanel $seededHistory).display_offset -ne 0) -and
               [DateTime]::UtcNow -lt $seedDeadline) {
            $seededHistory = Read-AutomexiaSnapshot -AfterSequence ([int64]$seededHistory.sequence)
        }
        if ([int64]$seededHistory.latest_prompt_id -le $previousPrompt -or
            [int](Get-ActiveAutomexiaPanel $seededHistory).display_offset -ne 0) {
            throw "PowerShell did not publish split-navigation history seed $seedIndex"
        }
        $historyDone = $seededHistory
    }

    # Deterministic binding tests prove bare Ctrl+R clones while Ctrl+Alt+R
    # sends shell history search. This feature-gated,
    # renderer-neutral control invokes the same clone-right action path without
    # relying on focus-sensitive synthetic keyboard input.
    $script:testStage = 'clone split right'
    Send-AutomexiaTestControl 'clone-right:1'
    $rightClone = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
    $cloneDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (([int]$rightClone.panel_count -ne 2 -or
            $null -eq (Get-ActiveAutomexiaPanel $rightClone).shell_user -or
            -not [bool](Get-ActiveAutomexiaPanel $rightClone).shell_prompt_active -or
            -not [bool]$rightClone.full_path_visible) -and
           [DateTime]::UtcNow -lt $cloneDeadline) {
        $rightClone = Read-AutomexiaSnapshot -AfterSequence ([int64]$rightClone.sequence)
    }
    $rightPanel = Get-ActiveAutomexiaPanel $rightClone
    if ([int]$rightClone.panel_count -ne 2 -or $null -eq $rightPanel -or
        -not [bool]$rightPanel.shell_prompt_active) {
        Write-Host ($rightClone | ConvertTo-Json -Depth 8)
        throw 'The clone-right action did not create a prompt-ready independent right split'
    }
    if ([int64]$rightPanel.route_id -eq [int64]$initialPanel.route_id -or
        [int64]$rightPanel.shell_pid -eq [int64]$initialPanel.shell_pid -or
        [int64]$rightPanel.shell_pid -le 0) {
        throw 'The right clone reused its source route or ConPTY process'
    }
    if ($rightPanel.current_directory -ne $initialPanel.current_directory -or
        $rightPanel.launch_program -ne $initialPanel.launch_program -or
        $rightPanel.profile_identity -ne $initialPanel.profile_identity -or
        (($rightPanel.launch_args | ConvertTo-Json -Compress) -ne
         ($initialPanel.launch_args | ConvertTo-Json -Compress))) {
        Write-Host ($rightClone | ConvertTo-Json -Depth 8)
        throw 'The right clone did not preserve PowerShell/profile/current-directory launch intent'
    }

    # Validate the geometric focus path used by Alt+Left/Alt+Right. The
    # movement must stop within this grid and select the visual neighbour,
    # without replacing either independent route.
    $rightRoute = [int64]$rightPanel.route_id
    $script:testStage = 'geometric focus left'
    Send-AutomexiaTestControl 'select-pane:focus-left:left'
    $focusedLeft = Read-AutomexiaSnapshot -AfterSequence ([int64]$rightClone.sequence)
    $focusLeftDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64](Get-ActiveAutomexiaPanel $focusedLeft).route_id -ne
           [int64]$initialPanel.route_id -and [DateTime]::UtcNow -lt $focusLeftDeadline) {
        $focusedLeft = Read-AutomexiaSnapshot -AfterSequence ([int64]$focusedLeft.sequence)
    }
    if ([int64](Get-ActiveAutomexiaPanel $focusedLeft).route_id -ne
        [int64]$initialPanel.route_id) {
        Write-Host ($focusedLeft | ConvertTo-Json -Depth 10)
        throw 'Geometric left navigation did not focus the source pane'
    }

    $script:testStage = 'geometric focus right'
    Send-AutomexiaTestControl 'select-pane:focus-right:right'
    $focusedRight = Read-AutomexiaSnapshot -AfterSequence ([int64]$focusedLeft.sequence)
    $focusRightDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64](Get-ActiveAutomexiaPanel $focusedRight).route_id -ne
           $rightRoute -and [DateTime]::UtcNow -lt $focusRightDeadline) {
        $focusedRight = Read-AutomexiaSnapshot -AfterSequence ([int64]$focusedRight.sequence)
    }
    if ([int64](Get-ActiveAutomexiaPanel $focusedRight).route_id -ne $rightRoute -or
        @($focusedRight.panels | ForEach-Object { [int64]$_.route_id } | Sort-Object -Unique).Count -ne 2) {
        Write-Host ($focusedRight | ConvertTo-Json -Depth 10)
        throw 'Geometric right navigation escaped the grid or changed pane routes'
    }

    # Input and visible history must remain isolated. Write a marker only to
    # the clone, then return to the source with the unchanged Shift+F6 split
    # navigation shortcut and prove the marker is absent there.
    $marker = 'AUTOMEXIA_CLONE_ONLY_73491'
    $script:testStage = 'write to cloned split'
    # Command echo already contains the marker. Keep a deliberate delay so this
    # fixture proves completion readiness before asserting idle-pane isolation.
    Send-AutomexiaTestControl "write-line:2:Start-Sleep -Milliseconds 1500; Write-Output $marker"
    $cloneOutput = Read-AutomexiaSnapshot -AfterSequence ([int64]$focusedRight.sequence)
    $outputDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ((-not ((Get-ActiveAutomexiaPanel $cloneOutput).visible_text -like "*$marker*") -or
            -not [bool](Get-ActiveAutomexiaPanel $cloneOutput).shell_prompt_active -or
            [int64]$cloneOutput.latest_prompt_id -le [int64]$focusedRight.latest_prompt_id) -and
           [DateTime]::UtcNow -lt $outputDeadline) {
        $cloneOutput = Read-AutomexiaSnapshot -AfterSequence ([int64]$cloneOutput.sequence)
    }
    if (-not ((Get-ActiveAutomexiaPanel $cloneOutput).visible_text -like "*$marker*")) {
        throw 'The cloned PowerShell PTY did not receive its independent input'
    }
    if (-not [bool](Get-ActiveAutomexiaPanel $cloneOutput).shell_prompt_active -or
        [int64]$cloneOutput.latest_prompt_id -le [int64]$focusedRight.latest_prompt_id) {
        throw 'Clone marker was observed before the command completed and a fresh prompt became active'
    }

    $script:testStage = 'return to source split'
    Send-AutomexiaTestControl 'select-prev:3'
    $sourceAgain = Read-AutomexiaSnapshot -AfterSequence ([int64]$cloneOutput.sequence)
    $sourceDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64](Get-ActiveAutomexiaPanel $sourceAgain).route_id -ne
           [int64]$initialPanel.route_id -and [DateTime]::UtcNow -lt $sourceDeadline) {
        $sourceAgain = Read-AutomexiaSnapshot -AfterSequence ([int64]$sourceAgain.sequence)
    }
    $sourcePanel = Get-ActiveAutomexiaPanel $sourceAgain
    if ([int64]$sourcePanel.route_id -ne [int64]$initialPanel.route_id) {
        throw 'Shift+F6 did not return focus to the source split'
    }
    if ($sourcePanel.visible_text -like "*$marker*") {
        throw 'Clone-only terminal output contaminated the source session'
    }

    # The same public shortcut must affect only the selected source pane. The
    # independent clone has its own terminal and must retain its exact offset.
    $rightBeforeCommandJump = @($sourceAgain.panels | Where-Object {
        [int64]$_.route_id -eq $rightRoute
    })[0]
    $sourceBeforeCommandJump = Get-ActiveAutomexiaPanel $sourceAgain
    $script:testStage = 'selected-pane previous command isolation'
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
            $window, 0x26, $true, $true, $true)) {
        throw 'Could not deliver selected-pane Ctrl+Shift+Up navigation'
    }
    $sourceCommandJump = Read-AutomexiaSnapshot -AfterSequence ([int64]$sourceAgain.sequence)
    $sourceCommandJumpDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int](Get-ActiveAutomexiaPanel $sourceCommandJump).display_offset -le
           [int]$sourceBeforeCommandJump.display_offset -and
           [DateTime]::UtcNow -lt $sourceCommandJumpDeadline) {
        $sourceCommandJump = Read-AutomexiaSnapshot -AfterSequence ([int64]$sourceCommandJump.sequence)
    }
    $sourceAfterCommandJump = Get-ActiveAutomexiaPanel $sourceCommandJump
    $rightAfterCommandJump = @($sourceCommandJump.panels | Where-Object {
        [int64]$_.route_id -eq $rightRoute
    })[0]
    if ([int]$sourceAfterCommandJump.display_offset -le
        [int]$sourceBeforeCommandJump.display_offset -or
        [int]$rightAfterCommandJump.display_offset -ne
        [int]$rightBeforeCommandJump.display_offset) {
        throw ("Command navigation changed the wrong pane or failed to move the selected pane " +
            "(source {0}->{1}; clone {2}->{3})" -f
            [int]$sourceBeforeCommandJump.display_offset,
            [int]$sourceAfterCommandJump.display_offset,
            [int]$rightBeforeCommandJump.display_offset,
            [int]$rightAfterCommandJump.display_offset)
    }
    if ([string]$sourceAfterCommandJump.raw_cursor_line_text -ne
        [string]$sourceBeforeCommandJump.raw_cursor_line_text -or
        [string]$rightAfterCommandJump.raw_cursor_line_text -ne
        [string]$rightBeforeCommandJump.raw_cursor_line_text -or
        [int64]$sourceAfterCommandJump.raw_cursor_prompt_id -ne
        [int64]$sourceBeforeCommandJump.raw_cursor_prompt_id -or
        [int64]$rightAfterCommandJump.raw_cursor_prompt_id -ne
        [int64]$rightBeforeCommandJump.raw_cursor_prompt_id) {
        throw ('Selected-pane command navigation changed cursor state ' +
            '(source prompt {0}->{1}, clone prompt {2}->{3}; source text changed={4}, clone text changed={5})' -f
            [int64]$sourceBeforeCommandJump.raw_cursor_prompt_id,
            [int64]$sourceAfterCommandJump.raw_cursor_prompt_id,
            [int64]$rightBeforeCommandJump.raw_cursor_prompt_id,
            [int64]$rightAfterCommandJump.raw_cursor_prompt_id,
            ([string]$sourceAfterCommandJump.raw_cursor_line_text -ne [string]$sourceBeforeCommandJump.raw_cursor_line_text),
            ([string]$rightAfterCommandJump.raw_cursor_line_text -ne [string]$rightBeforeCommandJump.raw_cursor_line_text))
    }
    $sourceRestored = $sourceCommandJump
    for ($jump = 0; $jump -lt 64 -and
         [int](Get-ActiveAutomexiaPanel $sourceRestored).display_offset -ne 0; $jump++) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
                $window, 0x28, $true, $true, $true)) {
            throw 'Could not restore the selected pane after command navigation'
        }
        $sourceRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$sourceRestored.sequence)
    }
    if ([int](Get-ActiveAutomexiaPanel $sourceRestored).display_offset -ne 0) {
        throw 'Selected-pane next-command navigation did not restore the live prompt boundary'
    }
    $rightRestored = @($sourceRestored.panels | Where-Object {
        [int64]$_.route_id -eq $rightRoute
    })[0]
    if ([int]$rightRestored.display_offset -ne
        [int]$rightBeforeCommandJump.display_offset) {
        throw 'Restoring selected-pane command navigation changed the unfocused pane'
    }
    $sourceAgain = $sourceRestored

    # Add a lower independent clone before the storm so layout, prompt, PTY,
    # and session isolation are exercised together under rapid resizing.
    $script:testStage = 'clone split down'
    Send-AutomexiaTestControl 'clone-down:4'
    $lowerClone = Read-AutomexiaSnapshot -AfterSequence ([int64]$sourceAgain.sequence)
    $lowerDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (([int]$lowerClone.panel_count -ne 3 -or
            -not [bool]$lowerClone.full_path_visible -or
            -not (Test-AllAutomexiaPaneContexts $lowerClone)) -and
           [DateTime]::UtcNow -lt $lowerDeadline) {
        $lowerClone = Read-AutomexiaSnapshot -AfterSequence ([int64]$lowerClone.sequence)
    }
    if ([int]$lowerClone.panel_count -ne 3) {
        Write-Host ($lowerClone | ConvertTo-Json -Depth 8)
        throw 'The clone-down action did not create an independent lower split'
    }
    $routeIds = @($lowerClone.panels | ForEach-Object { [int64]$_.route_id } | Sort-Object -Unique)
    $processIds = @($lowerClone.panels | ForEach-Object { [int64]$_.shell_pid } | Sort-Object -Unique)
    if ($routeIds.Count -ne 3 -or $processIds.Count -ne 3 -or $processIds[0] -le 0) {
        throw 'Cloned panels do not have three independent routes and ConPTY processes'
    }
    if (-not (Test-AllAutomexiaPaneContexts $lowerClone)) {
        Write-Host ($lowerClone | ConvertTo-Json -Depth 8)
        throw 'Every visible pane must expose a route-matched operational context snapshot'
    }
    $sizes = @(
        @(320, 220),
        @(1920, 1080),
        @(420, 260),
        @(1600, 900),
        @(280, 200),
        @(1280, 720)
    )
    $script:testStage = 'resize storm'
    for ($iteration = 0; $iteration -lt 240; $iteration++) {
        $size = $sizes[$iteration % $sizes.Count]
        if (-not [AutomexiaResizeDriver]::MoveWindow(
                $window, 40, 40, $size[0], $size[1], $true)) {
            $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            throw "MoveWindow failed during iteration $iteration with Win32 error $code"
        }
        if ($iteration -eq 80) {
            # Create another clone while window-size events are still arriving;
            # this exercises descriptor launch, Taffy layout, ConPTY startup,
            # and prompt seeding in the middle of the storm.
            Send-AutomexiaTestControl 'clone-right:5'
            Start-Sleep -Milliseconds 150
        }
        if ($iteration -eq 160) {
            Send-AutomexiaTestControl 'select-prev:6'
        }
        Start-Sleep -Milliseconds 4
    }

    $storm = Read-AutomexiaSnapshot -AfterSequence ([int64]$lowerClone.sequence)
    $stormDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int]$storm.panel_count -ne 4 -and [DateTime]::UtcNow -lt $stormDeadline) {
        $storm = Read-AutomexiaSnapshot -AfterSequence ([int64]$storm.sequence)
    }

    $finalWindowWidth = [Math]::Min(
        1400, [AutomexiaResizeDriver]::PrimaryWidth())
    $finalWindowHeight = [Math]::Min(
        900, [AutomexiaResizeDriver]::PrimaryHeight())
    if (-not [AutomexiaResizeDriver]::MoveWindow(
        $window, 0, 0, $finalWindowWidth, $finalWindowHeight, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Final MoveWindow failed with Win32 error $code"
    }
    $script:testStage = 'final restored size'
    $final = Read-AutomexiaSnapshot -AfterSequence ([int64]$storm.sequence)
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([double]$final.window_width -lt 1200 -or
            [double]$final.window_height -lt 700 -or
            $null -eq $final.latest_prompt_id -or
            [int]$final.latest_prompt_start_count -ne 1 -or
            -not [bool]$final.full_path_visible) -and
           [DateTime]::UtcNow -lt $deadline) {
        $final = Read-AutomexiaSnapshot -AfterSequence ([int64]$final.sequence)
    }
    $process.Refresh()

    if ($process.HasExited) { throw 'Automexia exited during the resize storm' }
    if ([int]$final.columns -lt 1 -or [int]$final.rows -lt 1) {
        throw "Invalid final grid dimensions: $($final.columns)x$($final.rows)"
    }
    if ([int]$final.cursor_column -lt 0 -or [int]$final.cursor_column -ge [int]$final.columns) {
        throw "Final cursor column $($final.cursor_column) is outside the grid"
    }
    if ([int]$final.cursor_row -lt 0 -or [int]$final.cursor_row -ge [int]$final.rows) {
        throw "Final cursor row $($final.cursor_row) is outside the grid"
    }
    if ([int]$final.latest_prompt_start_count -gt 1) {
        Write-Host ($final | ConvertTo-Json -Depth 8)
        throw "The active prompt was duplicated $($final.latest_prompt_start_count) times"
    }
    if ($null -ne $final.active_prompt_gap_rows -and
        [int]$final.active_prompt_gap_rows -gt 1) {
        Write-Host ($final | ConvertTo-Json -Depth 4)
        throw "Resize left $($final.active_prompt_gap_rows) blank rows between completed output and the active prompt"
    }
    if ([bool]$final.prompt_active -and -not [bool]$final.full_path_visible) {
        Write-Host ($final | ConvertTo-Json -Depth 4)
        throw 'The complete active path did not return after restoring a usable window size'
    }
    if ([int]$final.panel_count -ne 4) {
        throw 'A cloned session disappeared during the resize storm'
    }
    $finalRoutes = @($final.panels | ForEach-Object { [int64]$_.route_id } | Sort-Object -Unique)
    $finalProcesses = @($final.panels | ForEach-Object { [int64]$_.shell_pid } | Sort-Object -Unique)
    if ($finalRoutes.Count -ne 4 -or $finalProcesses.Count -ne 4 -or $finalProcesses[0] -le 0) {
        throw 'Interleaved clone/resize operations lost route or PTY isolation'
    }
    if (-not (Test-AllAutomexiaPaneContexts $final)) {
        Write-Host ($final | ConvertTo-Json -Depth 8)
        throw 'A visible pane lost or inherited another route operational context during resize'
    }

    # The native fixture intentionally omits font overrides. Assert the real
    # renderer inherited the product defaults in every independent pane and
    # retained the same zoom-reset baseline through cloning and resize storms.
    foreach ($panel in @($final.panels)) {
        if ([Math]::Abs([double]$panel.font_size - 18.0) -gt 0.01 -or
            [Math]::Abs([double]$panel.original_font_size - 18.0) -gt 0.01 -or
            [Math]::Abs([double]$panel.line_height - 1.22) -gt 0.001 -or
            [double]$panel.scaled_font_size -le 0.0) {
            Write-Host ($panel | ConvertTo-Json -Depth 8)
            throw 'A native pane did not retain the balanced typography defaults'
        }
    }

    # Capture the clean, settled four-pane workspace before any fullscreen or
    # preview overlay changes its composition. Pixel statistics reject blank
    # frames automatically; an explicit path additionally retains a PNG for
    # human typography review without making CI store terminal contents.
    $typographyFramePath = if ([string]::IsNullOrWhiteSpace($TypographyCapture)) {
        $null
    } else {
        [IO.Path]::GetFullPath($TypographyCapture)
    }
    if ($null -ne $typographyFramePath) {
        $typographyDirectory = [IO.Path]::GetDirectoryName($typographyFramePath)
        if (-not [string]::IsNullOrWhiteSpace($typographyDirectory)) {
            New-Item -ItemType Directory -Force -Path $typographyDirectory | Out-Null
        }
    }
    $script:testStage = 'balanced typography composited frame'
    $typographyDeadline = [DateTime]::UtcNow.AddSeconds(5)
    $typographyAttempts = 0
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for typography capture (Win32 error $code)"
    }
    try {
        do {
            $typographyAttempts++
            $typographyFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
                $window, $typographyFramePath)
            $typographyFrameValid = (
                $typographyFrame.Width -ge 100 -and
                $typographyFrame.Height -ge 100 -and
                $typographyFrame.SampleCount -ge 100 -and
                $typographyFrame.DistinctColorBuckets -ge 8 -and
                $typographyFrame.LuminanceSpread -ge 32)
            if (-not $typographyFrameValid) {
                Start-Sleep -Milliseconds 100
            }
        } while (-not $typographyFrameValid -and
                 [DateTime]::UtcNow -lt $typographyDeadline)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if (-not $typographyFrameValid) {
        throw "Balanced typography frame remained blank or low-detail after $typographyAttempts attempts"
    }

    # Enter and leave the exact borderless-fullscreen path used by F11 and
    # Alt+Enter. The renderer's dominant composited color must not change, the
    # client must cover the display, and Windows must expose Automexia's scoped
    # DisplayRequired request only for the fullscreen lifetime.
    $script:testStage = 'fullscreen display brightness stability'
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for the windowed brightness sample (Win32 error $code)"
    }
    try {
        Start-Sleep -Milliseconds 100
        $windowedBrightnessFrame = [AutomexiaResizeDriver]::CaptureClientFrame($window, $null)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }

    $fullscreenControl = 'toggle-fullscreen:9050'
    Send-AutomexiaTestControl $fullscreenControl
    $fullscreenSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$final.sequence)
    $fullscreenDeadline = [DateTime]::UtcNow.AddSeconds(10)
    $fullscreenBrightnessFrame = $null
    $fullscreenSettled = $false
    $fullscreenOccluded = $false
    $fullscreenOcclusionReason = $null
    do {
        if ([string]$fullscreenSnapshot.last_control -ne $fullscreenControl -or
            -not [bool]$fullscreenSnapshot.fullscreen_display_request_active) {
            $fullscreenSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$fullscreenSnapshot.sequence)
        }
        try {
            $fullscreenBrightnessFrame =
                [AutomexiaResizeDriver]::CaptureClientFrame($window, $null)
            $fullscreenOccluded = $false
        } catch {
            if (-not $_.Exception.Message.Contains('unowned top-level window')) {
                throw
            }
            # The taskbar/compositor can briefly remain above a window while
            # borderless fullscreen is settling. Retry within the existing
            # bounded readiness deadline; a persistent titled surface fails.
            $fullscreenOccluded = $true
            $fullscreenOcclusionReason = $_.Exception.Message
            $fullscreenSettled = $false
            Start-Sleep -Milliseconds 50
            continue
        }
        $fullscreenSettled = (
            [Math]::Abs($fullscreenBrightnessFrame.Width - [AutomexiaResizeDriver]::PrimaryWidth()) -le 2 -and
            [Math]::Abs($fullscreenBrightnessFrame.Height - [AutomexiaResizeDriver]::PrimaryHeight()) -le 2)
        $fullscreenReady = (
            $fullscreenSettled -and
            [string]$fullscreenSnapshot.last_control -eq $fullscreenControl -and
            [bool]$fullscreenSnapshot.fullscreen_display_request_active)
        if (-not $fullscreenReady) {
            Start-Sleep -Milliseconds 50
        }
    } while (-not $fullscreenReady -and [DateTime]::UtcNow -lt $fullscreenDeadline)
    if ($fullscreenOccluded -or -not $fullscreenSettled -or
        [string]$fullscreenSnapshot.last_control -ne $fullscreenControl -or
        -not [bool]$fullscreenSnapshot.fullscreen_display_request_active) {
        throw "Fullscreen did not settle unobscured with an active DisplayRequired request at the display bounds: $fullscreenOcclusionReason"
    }

    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for the fullscreen brightness sample (Win32 error $code)"
    }
    try {
        Start-Sleep -Milliseconds 100
        $fullscreenBrightnessFrame = [AutomexiaResizeDriver]::CaptureClientFrame($window, $null)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if ($windowedBrightnessFrame.DominantColorBucket -lt 0 -or
        $fullscreenBrightnessFrame.DominantColorBucket -ne $windowedBrightnessFrame.DominantColorBucket) {
        throw "Fullscreen changed the rendered dominant color bucket from $($windowedBrightnessFrame.DominantColorBucket) to $($fullscreenBrightnessFrame.DominantColorBucket)"
    }

    $restoreControl = 'toggle-fullscreen:9051'
    Send-AutomexiaTestControl $restoreControl
    $restoredSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$fullscreenSnapshot.sequence)
    $restoreDeadline = [DateTime]::UtcNow.AddSeconds(10)
    $restoredBrightnessFrame = $null
    $restoredSettled = $false
    $restoreOccluded = $false
    $restoreOcclusionReason = $null
    do {
        if ([string]$restoredSnapshot.last_control -ne $restoreControl -or
            [bool]$restoredSnapshot.fullscreen_display_request_active) {
            $restoredSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$restoredSnapshot.sequence)
        }
        try {
            $restoredBrightnessFrame =
                [AutomexiaResizeDriver]::CaptureClientFrame($window, $null)
            $restoreOccluded = $false
        } catch {
            if (-not $_.Exception.Message.Contains('unowned top-level window')) {
                throw
            }
            $restoreOccluded = $true
            $restoreOcclusionReason = $_.Exception.Message
            $restoredSettled = $false
            Start-Sleep -Milliseconds 50
            continue
        }
        $restoredSettled = (
            [Math]::Abs($restoredBrightnessFrame.Width - $windowedBrightnessFrame.Width) -le 2 -and
            [Math]::Abs($restoredBrightnessFrame.Height - $windowedBrightnessFrame.Height) -le 2)
        $restoreReady = (
            $restoredSettled -and
            [string]$restoredSnapshot.last_control -eq $restoreControl -and
            -not [bool]$restoredSnapshot.fullscreen_display_request_active)
        if (-not $restoreReady) {
            Start-Sleep -Milliseconds 50
        }
    } while (-not $restoreReady -and [DateTime]::UtcNow -lt $restoreDeadline)
    if ($restoreOccluded -or -not $restoredSettled -or
        [string]$restoredSnapshot.last_control -ne $restoreControl -or
        [bool]$restoredSnapshot.fullscreen_display_request_active) {
        throw "Fullscreen exit did not restore unobscured windowed bounds and release DisplayRequired: $restoreOcclusionReason"
    }
    if ($restoredBrightnessFrame.DominantColorBucket -ne $windowedBrightnessFrame.DominantColorBucket) {
        throw "Fullscreen exit did not restore the rendered dominant color bucket: $($restoredBrightnessFrame.DominantColorBucket)"
    }

    $requestReleased = -not [bool]$restoredSnapshot.fullscreen_display_request_active
    # Exercise the user-facing path rather than bypassing input through the
    # feature-gated preview control: print two real filenames, hover the first,
    # click it to pin, and use Right Arrow to browse to the second. Snapshot
    # geometry drives Win32 pointer coordinates deterministically without OCR.
    $script:testStage = 'native image hover click and arrow browsing'
    # Generate privacy-safe, high-contrast fixtures at runtime. The odd-width
    # JPEG exercises a non-256-aligned RGBA row on the real WGPU upload path;
    # the PNG proves alpha-capable decoding through the same interaction.
    $previewAssetSmall = Join-Path $configRoot 'preview bright (64).png'
    $previewAsset = Join-Path $configRoot 'preview bright (127).jpg'
    [AutomexiaResizeDriver]::WritePreviewFixture(
        $previewAssetSmall, 64, 64, $false)
    [AutomexiaResizeDriver]::WritePreviewFixture(
        $previewAsset, 127, 93, $true)
    $previewDirectory = Split-Path -Parent $previewAsset
    $previewTokenSmall = Split-Path -Leaf $previewAssetSmall
    $previewTokenLarge = Split-Path -Leaf $previewAsset
    # Keep each target on its own logical output row. The 29-column stress
    # pane can display either filename intact, so keyboard browsing tests real
    # path discovery instead of a synthetic token split by terminal reflow.
    $previewCwdControl = "write-line:preview-cwd:Set-Location '$previewDirectory'"
    Send-AutomexiaTestControl $previewCwdControl
    $previewCwd = Read-AutomexiaSnapshot -AfterSequence ([int64]$final.sequence)
    $previewCwdDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([string]$previewCwd.last_control -ne $previewCwdControl -or
            [int64]$previewCwd.latest_prompt_id -le [int64]$final.latest_prompt_id -or
            [string](Get-ActiveAutomexiaPanel $previewCwd).current_directory -ne
                $previewDirectory.Replace('\', '/') -or
            -not [bool](Get-ActiveAutomexiaPanel $previewCwd).shell_prompt_active) -and
           [DateTime]::UtcNow -lt $previewCwdDeadline) {
        $previewCwd = Read-AutomexiaSnapshot -AfterSequence ([int64]$previewCwd.sequence)
    }
    if ([string]$previewCwd.last_control -ne $previewCwdControl -or
        [string](Get-ActiveAutomexiaPanel $previewCwd).current_directory -ne
            $previewDirectory.Replace('\', '/')) {
        Write-Host ($previewCwd | ConvertTo-Json -Depth 10)
        throw 'Native image preview fixture did not enter the image directory'
    }

    # Exercise the exact filesystem-listing workflow: PowerShell's native ls
    # objects feed a display-only name projection, and both names fit within
    # the intentionally narrow pane without inheriting stale command cells.
    $previewControl = 'write-line:preview-list:ls ''preview bright*'' | % { "$([char]0xF1C5) $($_.Name)" }'
    Send-AutomexiaTestControl $previewControl
    $preview = Read-AutomexiaSnapshot -AfterSequence ([int64]$previewCwd.sequence)
    $previewDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([string]$preview.last_control -ne $previewControl -or
            [int64]$preview.latest_prompt_id -le [int64]$previewCwd.latest_prompt_id -or
            -not ([string](Get-ActiveAutomexiaPanel $preview).visible_text).Contains($previewTokenSmall) -or
            -not ([string](Get-ActiveAutomexiaPanel $preview).visible_text).Contains($previewTokenLarge) -or
            -not [bool](Get-ActiveAutomexiaPanel $preview).shell_prompt_active) -and
           [DateTime]::UtcNow -lt $previewDeadline) {
        $preview = Read-AutomexiaSnapshot -AfterSequence ([int64]$preview.sequence)
    }
    $previewPanel = Get-ActiveAutomexiaPanel $preview
    $previewRows = @(([string]$previewPanel.visible_text) -split [char]10)
    $previewRow = -1
    $previewColumn = -1
    for ($row = 0; $row -lt $previewRows.Count; $row++) {
        $column = $previewRows[$row].IndexOf(
            $previewTokenSmall, [StringComparison]::OrdinalIgnoreCase)
        if ($column -ge 0) {
            $previewRow = $row
            $previewColumn = $column
            break
        }
    }
    if ($previewRow -lt 0 -or $previewColumn -lt 0 -or
        [int]$previewPanel.cell_width -le 0 -or
        [int]$previewPanel.cell_height -le 0) {
        Write-Host ($preview | ConvertTo-Json -Depth 10)
        throw 'Native image filename did not produce usable renderer-neutral hit geometry'
    }
    $previewX = [int][Math]::Floor(
        [double]$previewPanel.grid_origin[0] +
        (($previewColumn + 1.5) * [double]$previewPanel.cell_width))
    $previewPaintedRow = Get-AutomexiaPaintedRow -Panel $previewPanel -SourceRow $previewRow
    $previewY = [int][Math]::Floor(
        [double]$previewPanel.grid_origin[1] +
        (($previewPaintedRow + 0.5) * [double]$previewPanel.cell_height))
    $previewScale = [double]$preview.scale_factor
    if ($previewScale -le 0.0) {
        throw 'Native snapshot did not publish a valid window scale factor'
    }
    # WM_MOUSEMOVE client coordinates are DPI-virtualized before winit emits
    # its physical position. Convert the renderer's physical hit geometry
    # back to message coordinates exactly once.
    $messagePreviewX = [int][Math]::Round($previewX / $previewScale)
    $messagePreviewY = [int][Math]::Round($previewY / $previewScale)
    $mouseLParam = [IntPtr]((
        [int64]($messagePreviewY -band 0xFFFF) -shl 16) -bor
        [int64]($messagePreviewX -band 0xFFFF))
    # Enter from a different grid row first. CursorMoved is deliberately
    # coalesced within one terminal cell in production, so a native hover test
    # must model an actual pointer transition instead of reposting whatever
    # cell the previous stress stage happened to leave behind.
    $preHoverY = [Math]::Max(
        1, $previewY - [int]$previewPanel.cell_height)
    $messagePreHoverY = [int][Math]::Round($preHoverY / $previewScale)
    $preHoverLParam = [IntPtr]((
        [int64]($messagePreHoverY -band 0xFFFF) -shl 16) -bor
        [int64]($messagePreviewX -band 0xFFFF))
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        throw 'Could not expose Automexia for native image pointer input'
    }
    if (-not [AutomexiaResizeDriver]::MovePointerToClient(
        $window, $messagePreviewX, $messagePreHoverY) -or
        -not [AutomexiaResizeDriver]::PostMessage(
        $window, 0x0200, [IntPtr]::Zero, $preHoverLParam)) {
        throw 'Could not deliver the native pre-hover transition'
    }
    $preHovered = Read-AutomexiaSnapshot -AfterSequence ([int64]$preview.sequence)
    $preHoverDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([Math]::Abs([double]$preHovered.pointer.x - $previewX) -gt 1.0 -or
            [Math]::Abs([double]$preHovered.pointer.y - $preHoverY) -gt 1.0) -and
           [DateTime]::UtcNow -lt $preHoverDeadline) {
        $preHovered = Read-AutomexiaSnapshot -AfterSequence ([int64]$preHovered.sequence)
    }
    if ([Math]::Abs([double]$preHovered.pointer.x - $previewX) -gt 1.0 -or
        [Math]::Abs([double]$preHovered.pointer.y - $preHoverY) -gt 1.0) {
        Write-Host ($preHovered | ConvertTo-Json -Depth 10)
        throw 'Native pre-hover transition did not reach the terminal grid'
    }
    if (-not [AutomexiaResizeDriver]::MovePointerToClient(
        $window, $messagePreviewX, $messagePreviewY) -or
        -not [AutomexiaResizeDriver]::PostMessage(
        $window, 0x0200, [IntPtr]::Zero, $mouseLParam)) {
        throw 'Could not deliver the native hover event to the rendered image filename'
    }
    $hovered = Read-AutomexiaSnapshot -AfterSequence ([int64]$preHovered.sequence)
    $hoverDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ((-not [bool]$hovered.image_preview.visible -or
            -not [bool]$hovered.image_preview.overlay_present) -and
           [DateTime]::UtcNow -lt $hoverDeadline) {
        $hovered = Read-AutomexiaSnapshot -AfterSequence ([int64]$hovered.sequence)
    }
    if (-not [bool]$hovered.image_preview.visible -or
        -not [bool]$hovered.image_preview.overlay_present -or
        [int]$hovered.image_preview.decoded_dimensions[0] -ne 64 -or
        [int]$hovered.image_preview.decoded_dimensions[1] -ne 64) {
        Write-Host ($hovered | ConvertTo-Json -Depth 10)
        throw 'Plain hover did not decode and display the 64px image path'
    }

    if (-not [AutomexiaResizeDriver]::PostMessage(
        $window, 0x0201, [IntPtr]1, $mouseLParam) -or
        -not [AutomexiaResizeDriver]::PostMessage(
        $window, 0x0202, [IntPtr]::Zero, $mouseLParam)) {
        throw 'Could not post the native click pair used to pin image quick look'
    }
    $pinned = Read-AutomexiaSnapshot -AfterSequence ([int64]$hovered.sequence)
    $pinDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ((-not [bool]$pinned.image_preview.pinned) -and
           [DateTime]::UtcNow -lt $pinDeadline) {
        $pinned = Read-AutomexiaSnapshot -AfterSequence ([int64]$pinned.sequence)
    }
    if (-not [bool]$pinned.image_preview.pinned -or
        [string]$pinned.image_preview.candidate -notlike "*$previewTokenSmall*") {
        Write-Host ($pinned | ConvertTo-Json -Depth 10)
        throw 'Native click did not pin the image path before keyboard browsing'
    }
    if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x27, $true)) {
        throw 'Could not post Right Arrow to browse the pinned image preview'
    }
    $preview = Read-AutomexiaSnapshot -AfterSequence ([int64]$pinned.sequence)
    $browseDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ((-not [bool]$preview.image_preview.visible -or
            -not [bool]$preview.image_preview.overlay_present -or
            -not [bool]$preview.image_preview.pinned -or
            [string]$preview.image_preview.candidate -notlike "*$previewTokenLarge*" -or
            [int]$preview.image_preview.decoded_dimensions[0] -ne 127 -or
            [int]$preview.image_preview.decoded_dimensions[1] -ne 93) -and
           [DateTime]::UtcNow -lt $browseDeadline) {
        $preview = Read-AutomexiaSnapshot -AfterSequence ([int64]$preview.sequence)
    }
    if (-not [bool]$preview.image_preview.visible -or
        -not [bool]$preview.image_preview.overlay_present -or
        -not [bool]$preview.image_preview.pinned -or
        [string]$preview.image_preview.candidate -notlike "*$previewTokenLarge*" -or
        [int]$preview.image_preview.decoded_dimensions[0] -ne 127 -or
        [int]$preview.image_preview.decoded_dimensions[1] -ne 93) {
        Write-Host ($preview | ConvertTo-Json -Depth 10)
        throw 'Click-to-pin and Right Arrow did not browse to the next visible image path'
    }

    $overlayRect = @($preview.image_preview.overlay_rect)
    if ($overlayRect.Count -ne 4) {
        Write-Host ($preview | ConvertTo-Json -Depth 10)
        throw 'Native image quick look did not publish its painted image rectangle'
    }
    # Compare fully contained physical pixels. Rounding a fractional origin
    # outward includes card/background edge pixels on CPU while WGPU blends
    # that same edge; those are not samples of the image body under test.
    $overlayX = [int][Math]::Ceiling([double]$overlayRect[0])
    $overlayY = [int][Math]::Ceiling([double]$overlayRect[1])
    $overlayWidth = [int][Math]::Floor([double]$overlayRect[0] + [double]$overlayRect[2]) - $overlayX
    $overlayHeight = [int][Math]::Floor([double]$overlayRect[1] + [double]$overlayRect[3]) - $overlayY
    if ($overlayWidth -lt 8 -or $overlayHeight -lt 8) {
        throw "Native image quick look published an unusable image rectangle: $overlayWidth x $overlayHeight"
    }

    # A visible overlay record is insufficient: the former CPU ordering bug
    # painted the nearly opaque card after the image, leaving a technically
    # present but black preview. Inspect only the image body and require real
    # color/luminance variation from the checked-in Automexia mark.
    $script:testStage = 'native image preview visible pixel fidelity'
    $pixelDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $previewPixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
            $window, $overlayX, $overlayY, $overlayWidth, $overlayHeight)
        $brightRatio = if ($previewPixels.SampleCount -eq 0) {
            0.0
        } else {
            [double]$previewPixels.BrightSampleCount / $previewPixels.SampleCount
        }
        $previewPixelsVisible = (
            $previewPixels.SampleCount -ge 64 -and
            $previewPixels.DistinctColorBuckets -ge 4 -and
            $previewPixels.LuminanceSpread -ge 32 -and
            $previewPixels.MeanLuminance -ge 20 -and
            $brightRatio -ge 0.10)
        if (-not $previewPixelsVisible) {
            Start-Sleep -Milliseconds 100
        }
    } while (-not $previewPixelsVisible -and [DateTime]::UtcNow -lt $pixelDeadline)
    if (-not $previewPixelsVisible) {
        throw "Image preview body is blank or obscured: $($previewPixels.Width)x$($previewPixels.Height), samples=$($previewPixels.SampleCount), buckets=$($previewPixels.DistinctColorBuckets), mean-luminance=$($previewPixels.MeanLuminance), spread=$($previewPixels.LuminanceSpread), bright-ratio=$([Math]::Round($brightRatio, 3))"
    }

    $framePath = if ([string]::IsNullOrWhiteSpace($FrameCapture)) {
        $null
    } else {
        [IO.Path]::GetFullPath($FrameCapture)
    }
    # The renderer snapshot is published before the GPU presentation is
    # necessarily observable in the desktop compositor. Keep the visual
    # threshold strict, but allow one bounded presentation-settle window
    # instead of treating a transient single-color capture as the final frame.
    $script:testStage = 'final painted frame settle'
    $frameDeadline = [DateTime]::UtcNow.AddSeconds(5)
    $frameStopwatch = [Diagnostics.Stopwatch]::StartNew()
    $frameAttempts = 0
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for composited frame capture (Win32 error $code)"
    }
    try {
        do {
            $frameAttempts++
            $frameStats = [AutomexiaResizeDriver]::CaptureClientFrame(
                $window, $framePath)
            $frameIsValid = (
                $frameStats.Width -ge 100 -and
                $frameStats.Height -ge 100 -and
                $frameStats.SampleCount -ge 100 -and
                $frameStats.DistinctColorBuckets -ge 8 -and
                $frameStats.LuminanceSpread -ge 32)
            if (-not $frameIsValid) {
                Start-Sleep -Milliseconds 100
            }
        } while (-not $frameIsValid -and [DateTime]::UtcNow -lt $frameDeadline)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
        $frameStopwatch.Stop()
    }
    if (-not $frameIsValid) {
        throw "Final painted client frame did not settle within 5 seconds after $frameAttempts attempts: $($frameStats.Width)x$($frameStats.Height), samples=$($frameStats.SampleCount), buckets=$($frameStats.DistinctColorBuckets), luminance-spread=$($frameStats.LuminanceSpread)"
    }

    $script:testStage = 'dismiss native image quick look with Escape'
    if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)) {
        throw 'Could not post Escape to dismiss the pinned image preview'
    }
    $dismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$preview.sequence)
    $dismissDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([bool]$dismissed.image_preview.visible -or
            [bool]$dismissed.image_preview.overlay_present) -and
           [DateTime]::UtcNow -lt $dismissDeadline) {
        $dismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$dismissed.sequence)
    }
    if ([bool]$dismissed.image_preview.visible -or
        [bool]$dismissed.image_preview.overlay_present) {
        throw 'Native image quick look did not remove its GPU overlay after dismissal'
    }

    $expectedPreviewCacheEntries = 2
    $expectedPreviewCacheBytes = (64 * 64 * 4) + (127 * 93 * 4)
    $resourceReleaseDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (-not (Test-AutomexiaImageResources $dismissed $false ([bool]$UseCpuRenderer) $expectedPreviewCacheEntries $expectedPreviewCacheBytes) -and
           [DateTime]::UtcNow -lt $resourceReleaseDeadline) {
        $dismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$dismissed.sequence)
    }
    if (-not (Test-AutomexiaImageResources $dismissed $false ([bool]$UseCpuRenderer) $expectedPreviewCacheEntries $expectedPreviewCacheBytes)) {
        Write-Host ($dismissed | ConvertTo-Json -Depth 10)
        throw 'Image dismissal left preview pixels, overlays, GPU textures, queued work, or completion state alive'
    }

    $script:testStage = 'repeated native image preview resource lifecycle'
    $imageResourceBaseline = Get-AutomexiaResourceSample $process
    $imageLifecycleFinal = $dismissed
    for ($cycle = 1; $cycle -le $ImagePreviewLifecycleCycles; $cycle++) {
        # The real pointer hover/click/arrow path above proves user input. The
        # repeated leak soak uses the feature-gated control so Windows cannot
        # coalesce adjacent WM_MOUSEMOVE pairs and hide a resource result.
        $cycleControl = "preview-image:${cycle}:$previewAssetSmall"
        Send-AutomexiaTestControl $cycleControl
        $cycleVisible =
            Read-AutomexiaSnapshot -AfterSequence ([int64]$imageLifecycleFinal.sequence)
        $cycleVisibleDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (([string]$cycleVisible.last_control -ne $cycleControl -or
                -not (Test-AutomexiaImageResources $cycleVisible $true ([bool]$UseCpuRenderer) $expectedPreviewCacheEntries $expectedPreviewCacheBytes) -or
                [int]$cycleVisible.image_preview.decoded_dimensions[0] -ne 64 -or
                [int]$cycleVisible.image_preview.decoded_dimensions[1] -ne 64) -and
               [DateTime]::UtcNow -lt $cycleVisibleDeadline) {
            $cycleVisible =
                Read-AutomexiaSnapshot -AfterSequence ([int64]$cycleVisible.sequence)
        }
        if ([string]$cycleVisible.last_control -ne $cycleControl -or
            -not (Test-AutomexiaImageResources $cycleVisible $true ([bool]$UseCpuRenderer) $expectedPreviewCacheEntries $expectedPreviewCacheBytes) -or
            [int]$cycleVisible.image_preview.decoded_dimensions[0] -ne 64 -or
            [int]$cycleVisible.image_preview.decoded_dimensions[1] -ne 64) {
            Write-Host ($cycleVisible | ConvertTo-Json -Depth 10)
            throw "Preview lifecycle cycle $cycle did not converge to one bounded live image resource"
        }

        if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)) {
            throw "Could not dismiss preview lifecycle cycle $cycle"
        }
        $cycleDismissed =
            Read-AutomexiaSnapshot -AfterSequence ([int64]$cycleVisible.sequence)
        $cycleDismissDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not (Test-AutomexiaImageResources $cycleDismissed $false ([bool]$UseCpuRenderer) $expectedPreviewCacheEntries $expectedPreviewCacheBytes) -and
               [DateTime]::UtcNow -lt $cycleDismissDeadline) {
            $cycleDismissed =
                Read-AutomexiaSnapshot -AfterSequence ([int64]$cycleDismissed.sequence)
        }
        if (-not (Test-AutomexiaImageResources $cycleDismissed $false ([bool]$UseCpuRenderer) $expectedPreviewCacheEntries $expectedPreviewCacheBytes)) {
            Write-Host ($cycleDismissed | ConvertTo-Json -Depth 10)
            throw "Preview lifecycle cycle $cycle leaked pixels, overlays, textures, queue items, or completion state"
        }
        $imageLifecycleFinal = $cycleDismissed
    }
    Start-Sleep -Milliseconds 250
    $imageResourceFinal = Get-AutomexiaResourceSample $process
    $imageResourceLimits = [ordered]@{
        handle_growth = $MaximumImageHandleGrowth
        thread_growth = $MaximumImageThreadGrowth
        private_bytes_growth = $MaximumImageMemoryGrowth
        working_set_growth = $MaximumImageMemoryGrowth
    }
    $imageResourceDelta = [ordered]@{
        handle_growth =
            $imageResourceFinal.handle_count - $imageResourceBaseline.handle_count
        thread_growth =
            $imageResourceFinal.thread_count - $imageResourceBaseline.thread_count
        private_bytes_growth =
            $imageResourceFinal.private_bytes - $imageResourceBaseline.private_bytes
        working_set_growth =
            $imageResourceFinal.working_set_bytes - $imageResourceBaseline.working_set_bytes
    }
    foreach ($name in $imageResourceLimits.Keys) {
        if ([int64]$imageResourceDelta[$name] -gt
            [int64]$imageResourceLimits[$name]) {
            throw "Repeated image preview resource ceiling exceeded for $name"
        }
    }

    # Pane and workspace search are one continuous session. Feature-gated
    # controls exercise the same screen methods as the typed actions while the
    # Rust binding tests own the physical Ctrl/Cmd chords. The snapshot exposes
    # only query byte length, semantic status, and announcement generation; it
    # never serializes the query or terminal contents.
    $script:testStage = 'continuous scoped search session'
    $openPaneSearch = 'open-pane-search:scope-session'
    Send-AutomexiaTestControl $openPaneSearch
    $paneSearch = Read-AutomexiaSnapshot -AfterSequence ([int64]$imageLifecycleFinal.sequence)
    $paneSearchDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$paneSearch.last_control -ne $openPaneSearch -or
            -not [bool]$paneSearch.search_active -or
            [string]$paneSearch.search_scope -ne 'pane' -or
            [string]$paneSearch.search_focus -ne 'query') -and
           [DateTime]::UtcNow -lt $paneSearchDeadline) {
        $paneSearch = Read-AutomexiaSnapshot -AfterSequence ([int64]$paneSearch.sequence)
    }
    if ([string]$paneSearch.last_control -ne $openPaneSearch -or
        -not [bool]$paneSearch.search_active -or
        [string]$paneSearch.search_scope -ne 'pane' -or
        [string]$paneSearch.search_focus -ne 'query' -or
        [int]$paneSearch.search_query_bytes -ne 0) {
        Write-Host ($paneSearch | ConvertTo-Json -Depth 8)
        throw 'Pane search did not open as a focused empty continuous session'
    }

    $query = 'automexia-scope-retained-42'
    $queryHex = -join (
        [Text.Encoding]::UTF8.GetBytes($query) |
            ForEach-Object { $_.ToString('x2') })
    $setQueryControl = "set-search-query-hex:scope-query:$queryHex"
    $cursorColumnBeforeSearch = [int]$paneSearch.cursor_column
    $cursorRowBeforeSearch = [int]$paneSearch.cursor_row
    Send-AutomexiaTestControl $setQueryControl
    $querySearch = Read-AutomexiaSnapshot -AfterSequence ([int64]$paneSearch.sequence)
    $queryDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$querySearch.last_control -ne $setQueryControl -or
            [int]$querySearch.search_query_bytes -ne
                [Text.Encoding]::UTF8.GetByteCount($query) -or
            [string]::IsNullOrWhiteSpace(
                [string]$querySearch.search_result_status)) -and
           [DateTime]::UtcNow -lt $queryDeadline) {
        $querySearch = Read-AutomexiaSnapshot -AfterSequence ([int64]$querySearch.sequence)
    }
    if ([string]$querySearch.last_control -ne $setQueryControl -or
        [int]$querySearch.search_query_bytes -ne
            [Text.Encoding]::UTF8.GetByteCount($query) -or
        [int]$querySearch.cursor_column -ne $cursorColumnBeforeSearch -or
        [int]$querySearch.cursor_row -ne $cursorRowBeforeSearch) {
        Write-Host ($querySearch | ConvertTo-Json -Depth 8)
        throw 'Search query input was not retained exclusively by the search session'
    }

    $paneAnnouncementGeneration =
        [int64]$querySearch.search_announcement_generation
    $openWorkspaceSearch = 'open-workspace-search:scope-expand'
    Send-AutomexiaTestControl $openWorkspaceSearch
    $workspaceSearch = Read-AutomexiaSnapshot -AfterSequence ([int64]$querySearch.sequence)
    $workspaceDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$workspaceSearch.last_control -ne $openWorkspaceSearch -or
            [string]$workspaceSearch.search_scope -ne 'workspace' -or
            [string]$workspaceSearch.search_focus -ne 'query' -or
            [int]$workspaceSearch.search_query_bytes -ne
                [Text.Encoding]::UTF8.GetByteCount($query)) -and
           [DateTime]::UtcNow -lt $workspaceDeadline) {
        $workspaceSearch = Read-AutomexiaSnapshot -AfterSequence ([int64]$workspaceSearch.sequence)
    }
    if ([string]$workspaceSearch.last_control -ne $openWorkspaceSearch -or
        [string]$workspaceSearch.search_scope -ne 'workspace' -or
        [string]$workspaceSearch.search_focus -ne 'query' -or
        [int]$workspaceSearch.search_query_bytes -ne
            [Text.Encoding]::UTF8.GetByteCount($query) -or
        [int64]$workspaceSearch.search_announcement_generation -le
            $paneAnnouncementGeneration -or
        [string]$workspaceSearch.search_live_announcement -notlike
            '*all visible panes*') {
        Write-Host ($workspaceSearch | ConvertTo-Json -Depth 8)
        throw 'Pane-to-workspace search switching lost query, focus, count, or announcement state'
    }

    $focusScopeControl = 'focus-search-scope:scope-keyboard'
    Send-AutomexiaTestControl $focusScopeControl
    $scopeFocused = Read-AutomexiaSnapshot -AfterSequence ([int64]$workspaceSearch.sequence)
    $focusDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$scopeFocused.last_control -ne $focusScopeControl -or
            [string]$scopeFocused.search_focus -ne 'scope') -and
           [DateTime]::UtcNow -lt $focusDeadline) {
        $scopeFocused = Read-AutomexiaSnapshot -AfterSequence ([int64]$scopeFocused.sequence)
    }
    if ([string]$scopeFocused.search_focus -ne 'scope') {
        Write-Host ($scopeFocused | ConvertTo-Json -Depth 8)
        throw 'Search scope control did not receive keyboard focus'
    }

    $workspaceGeneration =
        [int64]$scopeFocused.search_announcement_generation
    $refocusWorkspace = 'open-workspace-search:scope-refocus'
    Send-AutomexiaTestControl $refocusWorkspace
    $workspaceRefocused = Read-AutomexiaSnapshot -AfterSequence ([int64]$scopeFocused.sequence)
    $refocusDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$workspaceRefocused.last_control -ne $refocusWorkspace -or
            [string]$workspaceRefocused.search_focus -ne 'query') -and
           [DateTime]::UtcNow -lt $refocusDeadline) {
        $workspaceRefocused = Read-AutomexiaSnapshot -AfterSequence ([int64]$workspaceRefocused.sequence)
    }
    if ([string]$workspaceRefocused.search_scope -ne 'workspace' -or
        [string]$workspaceRefocused.search_focus -ne 'query' -or
        [int]$workspaceRefocused.search_query_bytes -ne
            [Text.Encoding]::UTF8.GetByteCount($query) -or
        [int64]$workspaceRefocused.search_announcement_generation -ne
            $workspaceGeneration) {
        Write-Host ($workspaceRefocused | ConvertTo-Json -Depth 8)
        throw 'The already-active workspace shortcut was not an idempotent query refocus'
    }

    $returnPaneSearch = 'open-pane-search:scope-contract'
    Send-AutomexiaTestControl $returnPaneSearch
    $paneReturned = Read-AutomexiaSnapshot -AfterSequence ([int64]$workspaceRefocused.sequence)
    $returnDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$paneReturned.last_control -ne $returnPaneSearch -or
            [string]$paneReturned.search_scope -ne 'pane' -or
            [string]$paneReturned.search_focus -ne 'query') -and
           [DateTime]::UtcNow -lt $returnDeadline) {
        $paneReturned = Read-AutomexiaSnapshot -AfterSequence ([int64]$paneReturned.sequence)
    }
    if ([string]$paneReturned.search_scope -ne 'pane' -or
        [int]$paneReturned.search_query_bytes -ne
            [Text.Encoding]::UTF8.GetByteCount($query) -or
        [int64]$paneReturned.search_announcement_generation -le
            $workspaceGeneration -or
        [string]$paneReturned.search_live_announcement -notlike
            '*current pane*') {
        Write-Host ($paneReturned | ConvertTo-Json -Depth 8)
        throw 'Workspace-to-pane search switching lost ownership, query, focus, or announcement state'
    }

    # Capture the real pane-footer composition while this continuous session is
    # still active. This complements renderer-neutral geometry assertions with
    # a native compositing check and an optional human-review artifact.
    $searchFramePath = if ([string]::IsNullOrWhiteSpace($SearchCapture)) {
        $null
    } else {
        [IO.Path]::GetFullPath($SearchCapture)
    }
    if ($null -ne $searchFramePath) {
        $searchFrameDirectory = [IO.Path]::GetDirectoryName($searchFramePath)
        if (-not [string]::IsNullOrWhiteSpace($searchFrameDirectory)) {
            New-Item -ItemType Directory -Force -Path $searchFrameDirectory | Out-Null
        }
    }
    $searchPresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$paneReturned.sequence)
    $searchRect = @($searchPresented.search_surface)
    if ($searchRect.Count -ne 4) {
        Write-Host ($searchPresented | ConvertTo-Json -Depth 8)
        throw 'Scoped search did not publish its painted surface rectangle'
    }
    $searchScale = [double]$searchPresented.scale_factor
    $searchX = [int][Math]::Floor([double]$searchRect[0] * $searchScale)
    $searchY = [int][Math]::Floor([double]$searchRect[1] * $searchScale)
    $searchWidth = [int][Math]::Ceiling([double]$searchRect[2] * $searchScale)
    $searchHeight = [int][Math]::Ceiling([double]$searchRect[3] * $searchScale)
    if ($searchWidth -lt 100 -or $searchHeight -lt 20) {
        throw "Scoped search published an unusable surface: $searchWidth x $searchHeight"
    }

    $searchFrameDeadline = [DateTime]::UtcNow.AddSeconds(5)
    $searchFrameAttempts = 0
    $searchFrameValid = $false
    $searchSurfaceValid = $false
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for scoped-search capture (Win32 error $code)"
    }
    try {
        do {
            $searchFrameAttempts++
            Start-Sleep -Milliseconds 100
            $searchFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
                $window, $searchFramePath)
            $searchSurfacePixels =
                [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
                    $window, $searchX, $searchY, $searchWidth, $searchHeight)
            $searchFrameValid = (
                $searchFrame.Width -ge 100 -and
                $searchFrame.Height -ge 100 -and
                $searchFrame.SampleCount -ge 100 -and
                $searchFrame.DistinctColorBuckets -ge 8 -and
                $searchFrame.LuminanceSpread -ge 32)
            $searchSurfaceValid = (
                $searchSurfacePixels.SampleCount -ge 32 -and
                $searchSurfacePixels.DistinctColorBuckets -ge 6 -and
                $searchSurfacePixels.LuminanceSpread -ge 32)
        } while ((-not $searchFrameValid -or -not $searchSurfaceValid) -and
                 [DateTime]::UtcNow -lt $searchFrameDeadline)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if (-not $searchFrameValid -or -not $searchSurfaceValid) {
        throw "Scoped-search frame did not settle after $searchFrameAttempts attempts: frame=$($searchFrame.Width)x$($searchFrame.Height), frame-buckets=$($searchFrame.DistinctColorBuckets), surface-buckets=$($searchSurfacePixels.DistinctColorBuckets), surface-spread=$($searchSurfacePixels.LuminanceSpread)"
    }

    $closeSearchControl = 'close-search:scope-session'
    Send-AutomexiaTestControl $closeSearchControl
    $searchClosed = Read-AutomexiaSnapshot -AfterSequence ([int64]$paneReturned.sequence)
    $closeSearchDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$searchClosed.last_control -ne $closeSearchControl -or
            [bool]$searchClosed.search_active) -and
           [DateTime]::UtcNow -lt $closeSearchDeadline) {
        $searchClosed = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchClosed.sequence)
    }
    if ([bool]$searchClosed.search_active) {
        Write-Host ($searchClosed | ConvertTo-Json -Depth 8)
        throw 'Search teardown left an input-owning session active'
    }

    # Both command surfaces are true modals: exactly one may be active, the
    # feature-gated snapshot must acknowledge it, and a real composited client
    # capture must remain visibly nonblank. Sugarloaf unit tests separately
    # assert that the modal primitive/text suffix is physically submitted after
    # pane borders, footers, scrollbars, and all ordinary UI labels.
    $modalCaptureRoot = if ([string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
        $null
    } else {
        [IO.Path]::GetFullPath($ModalCaptureDirectory)
    }
    if ($null -ne $modalCaptureRoot) {
        New-Item -ItemType Directory -Force -Path $modalCaptureRoot | Out-Null
    }
    $paletteCaptureName = if ($UseCpuRenderer) {
        'palette-cpu.png'
    } else {
        'palette-wgpu.png'
    }
    $quitCaptureName = if ($UseCpuRenderer) {
        'confirm-quit-cpu.png'
    } else {
        'confirm-quit-wgpu.png'
    }
    $nativeQuitCaptureName = if ($UseCpuRenderer) {
        'native-close-confirm-cpu.png'
    } else {
        'native-close-confirm-wgpu.png'
    }
    $hubCaptureName = if ($UseCpuRenderer) {
        'connection-hub-cpu.png'
    } else {
        'connection-hub-wgpu.png'
    }
    $hubDirectCaptureName = if ($UseCpuRenderer) {
        'connection-hub-direct-cpu.png'
    } else {
        'connection-hub-direct-wgpu.png'
    }
    $paletteCapturePath = if ($null -eq $modalCaptureRoot) {
        $null
    } else {
        Join-Path $modalCaptureRoot $paletteCaptureName
    }
    $quitCapturePath = if ($null -eq $modalCaptureRoot) {
        $null
    } else {
        Join-Path $modalCaptureRoot $quitCaptureName
    }
    $nativeQuitCapturePath = if ($null -eq $modalCaptureRoot) {
        $null
    } else {
        Join-Path $modalCaptureRoot $nativeQuitCaptureName
    }
    $hubCapturePath = if ($null -eq $modalCaptureRoot) {
        $null
    } else {
        Join-Path $modalCaptureRoot $hubCaptureName
    }
    $hubDirectCapturePath = if ($null -eq $modalCaptureRoot) {
        $null
    } else {
        Join-Path $modalCaptureRoot $hubDirectCaptureName
    }

    $script:testStage = 'connection hub native section navigation and composition'
    $hubControl = 'open-connection-hub:modal-hub'
    Send-AutomexiaTestControl $hubControl
    $hubSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$imageLifecycleFinal.sequence)
    $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$hubSnapshot.last_control -ne $hubControl -or
            -not [bool]$hubSnapshot.connection_hub_active -or
            [string]$hubSnapshot.connection_hub_route -ne 'results' -or
            [bool]$hubSnapshot.palette_enabled -or
            [bool]$hubSnapshot.confirm_quit_active) -and
           [DateTime]::UtcNow -lt $hubDeadline) {
        $hubSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubSnapshot.sequence)
    }
    if ([string]$hubSnapshot.last_control -ne $hubControl -or
        -not [bool]$hubSnapshot.connection_hub_active -or
        [string]$hubSnapshot.connection_hub_route -ne 'results' -or
        [bool]$hubSnapshot.palette_enabled -or
        [bool]$hubSnapshot.confirm_quit_active) {
        Write-Host ($hubSnapshot | ConvertTo-Json -Depth 8)
        throw 'The Connection Hub did not acquire exclusive modal ownership'
    }
    $hubPresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubSnapshot.sequence)
    $hubInputBaseline = $hubPresented
    $hubTerminalBefore = Get-ActiveAutomexiaPanel $hubInputBaseline

    foreach ($section in @(
            [ordered]@{ key = 0x57; route = 'workspaces'; label = 'W' },
            [ordered]@{ key = 0x50; route = 'providers'; label = 'P' },
            [ordered]@{ key = 0x43; route = 'results'; label = 'C' })) {
        $script:testStage = "connection hub native $($section.label) section mnemonic"
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
                $window, [uint32]$section.key, $false, $false, $false)) {
            $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            throw "Could not inject Connection Hub $($section.label) mnemonic (Win32 error $code)"
        }
        $nextHub = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubPresented.sequence)
        $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ((-not [bool]$nextHub.connection_hub_active -or
                [string]$nextHub.connection_hub_route -ne [string]$section.route) -and
               [DateTime]::UtcNow -lt $hubDeadline) {
            $nextHub = Read-AutomexiaSnapshot -AfterSequence ([int64]$nextHub.sequence)
        }
        if (-not [bool]$nextHub.connection_hub_active -or
            [string]$nextHub.connection_hub_route -ne [string]$section.route) {
            throw ("Connection Hub {0} did not select {1} " +
                "(active={2}; actual-route={3})" -f
                $section.label,
                $section.route,
                [bool]$nextHub.connection_hub_active,
                [string]$nextHub.connection_hub_route)
        }
        $hubPresented = $nextHub
    }

    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for Connection Hub capture (Win32 error $code)"
    }
    try {
        Start-Sleep -Milliseconds 100
        $hubFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
            $window, $hubCapturePath)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if ($hubFrame.Width -lt 100 -or
        $hubFrame.Height -lt 100 -or
        $hubFrame.SampleCount -lt 100 -or
        $hubFrame.DistinctColorBuckets -lt 8 -or
        $hubFrame.LuminanceSpread -lt 32) {
        throw "Connection Hub composited frame is blank or unreadable: $($hubFrame.Width)x$($hubFrame.Height), buckets=$($hubFrame.DistinctColorBuckets), spread=$($hubFrame.LuminanceSpread)"
    }

    $script:testStage = 'connection hub native direct-entry composition'
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
            $window, 0x4C, $false, $false, $false)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not inject Connection Hub L mnemonic (Win32 error $code)"
    }
    $hubDirect = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubPresented.sequence)
    $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ((-not [bool]$hubDirect.connection_hub_active -or
            -not [bool]$hubDirect.connection_hub_literal_entry) -and
           [DateTime]::UtcNow -lt $hubDeadline) {
        $hubDirect = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubDirect.sequence)
    }
    if (-not [bool]$hubDirect.connection_hub_active -or
        -not [bool]$hubDirect.connection_hub_literal_entry) {
        Write-Host ($hubDirect | ConvertTo-Json -Depth 8)
        throw 'Connection Hub L did not open the direct-entry editor'
    }
    $hubDirectPresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubDirect.sequence)
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for direct-entry capture (Win32 error $code)"
    }
    try {
        Start-Sleep -Milliseconds 100
        $hubDirectFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
            $window, $hubDirectCapturePath)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if ($hubDirectFrame.Width -lt 100 -or
        $hubDirectFrame.Height -lt 100 -or
        $hubDirectFrame.SampleCount -lt 100 -or
        $hubDirectFrame.DistinctColorBuckets -lt 8 -or
        $hubDirectFrame.LuminanceSpread -lt 32) {
        throw "Connection Hub direct-entry frame is blank or unreadable: $($hubDirectFrame.Width)x$($hubDirectFrame.Height), buckets=$($hubDirectFrame.DistinctColorBuckets), spread=$($hubDirectFrame.LuminanceSpread)"
    }

    # Click the exact top-right close target through the production physical-to-
    # logical conversion. It must cancel only the nested editor and preserve the
    # Hub plus terminal state.
    $hubScale = [double]$hubDirectPresented.scale_factor
    $hubLogicalWidth = [double]$hubDirectPresented.window_width / $hubScale
    $hubLogicalHeight = [double]$hubDirectPresented.window_height / $hubScale
    $hubMargin = if ($hubLogicalWidth -lt 420.0 -or $hubLogicalHeight -lt 320.0) {
        6.0
    } else {
        18.0
    }
    $hubCardWidth = [Math]::Min(840.0, [Math]::Max(1.0, $hubLogicalWidth - $hubMargin * 2.0))
    $hubCardHeight = [Math]::Min(420.0, [Math]::Max(1.0, $hubLogicalHeight - $hubMargin * 2.0))
    $hubCardX = [Math]::Max(0.0, ($hubLogicalWidth - $hubCardWidth) * 0.5)
    $hubCardY = [Math]::Max(0.0, ($hubLogicalHeight - $hubCardHeight) * 0.5)
    $hubInner = if ($hubCardWidth -lt 650.0) { 12.0 } else { 20.0 }
    $hubCloseX = [int][Math]::Round(($hubCardX + $hubCardWidth - $hubInner - 20.0) * $hubScale)
    $hubCloseY = [int][Math]::Round(($hubCardY + 32.0) * $hubScale)
    $hubCloseLParam = [IntPtr](($hubCloseX -band 0xffff) -bor (($hubCloseY -band 0xffff) -shl 16))
    if (-not [AutomexiaResizeDriver]::MovePhysicalPointerToClient(
            $window, $hubCloseX, $hubCloseY) -or
        -not [AutomexiaResizeDriver]::PostMessage(
            $window, 0x0200, [IntPtr]::Zero, $hubCloseLParam)) {
        throw 'Could not hover the Connection Hub direct-entry close target'
    }
    # SetCursorPos may enqueue a second native move after the posted move. Wait
    # for the renderer-owned hit test to confirm the exact target before the
    # press so foreground scheduling cannot turn this into an inert click.
    $hubCloseHover = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubDirectPresented.sequence)
    $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([string]$hubCloseHover.connection_hub_pointer_hit -ne 'CancelLiteralDestination' -and
           [DateTime]::UtcNow -lt $hubDeadline) {
        $hubCloseHover = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubCloseHover.sequence)
    }
    if ([string]$hubCloseHover.connection_hub_pointer_hit -ne 'CancelLiteralDestination') {
        throw 'Connection Hub direct-entry close target did not acquire pointer hover'
    }
    if (-not [AutomexiaResizeDriver]::PostMessage(
            $window, 0x0201, [IntPtr]1, $hubCloseLParam) -or
        -not [AutomexiaResizeDriver]::PostMessage(
            $window, 0x0202, [IntPtr]::Zero, $hubCloseLParam)) {
        throw 'Could not click the Connection Hub direct-entry close target'
    }
    $hubDirectCancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubCloseHover.sequence)
    $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ((-not [bool]$hubDirectCancelled.connection_hub_active -or
            [bool]$hubDirectCancelled.connection_hub_literal_entry) -and
           [DateTime]::UtcNow -lt $hubDeadline) {
        $hubDirectCancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubDirectCancelled.sequence)
    }
    if (-not [bool]$hubDirectCancelled.connection_hub_active -or
        [bool]$hubDirectCancelled.connection_hub_literal_entry -or
        [string]$hubDirectCancelled.connection_hub_route -ne 'results') {
        throw ("Direct-entry close did not cancel only the nested editor " +
            "(hub={0}; literal={1}; route={2}; last-hit={3})" -f
            [bool]$hubDirectCancelled.connection_hub_active,
            [bool]$hubDirectCancelled.connection_hub_literal_entry,
            [string]$hubDirectCancelled.connection_hub_route,
            [string]$hubDirectCancelled.connection_hub_last_hit)
    }
    $hubPresented = $hubDirectCancelled

    $script:testStage = 'connection hub native dismissal and PTY isolation'
    if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not inject Connection Hub Escape (Win32 error $code)"
    }
    $hubDismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubPresented.sequence)
    $hubDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([bool]$hubDismissed.connection_hub_active -and
           [DateTime]::UtcNow -lt $hubDeadline) {
        $hubDismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$hubDismissed.sequence)
    }
    $hubTerminalAfter = Get-ActiveAutomexiaPanel $hubDismissed
    if ([bool]$hubDismissed.connection_hub_active -or
        [int64]$hubTerminalAfter.route_id -ne [int64]$hubTerminalBefore.route_id -or
        [int]$hubDismissed.display_offset -ne [int]$hubInputBaseline.display_offset -or
        [int]$hubDismissed.cursor_column -ne [int]$hubInputBaseline.cursor_column -or
        [int]$hubDismissed.cursor_row -ne [int]$hubInputBaseline.cursor_row -or
        [string]$hubTerminalAfter.raw_cursor_line_text -ne
            [string]$hubTerminalBefore.raw_cursor_line_text) {
        Write-Host ($hubDismissed | ConvertTo-Json -Depth 8)
        throw 'Connection Hub navigation changed terminal state or remained active'
    }

    $script:testStage = 'topmost command palette composition'
    $paletteControl = 'open-palette:modal-layer'
    Send-AutomexiaTestControl $paletteControl
    $paletteSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$imageLifecycleFinal.sequence)
    $paletteDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$paletteSnapshot.last_control -ne $paletteControl -or
            -not [bool]$paletteSnapshot.palette_enabled -or
            [bool]$paletteSnapshot.confirm_quit_active) -and
           [DateTime]::UtcNow -lt $paletteDeadline) {
        $paletteSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$paletteSnapshot.sequence)
    }
    if ([string]$paletteSnapshot.last_control -ne $paletteControl -or
        -not [bool]$paletteSnapshot.palette_enabled -or
        [bool]$paletteSnapshot.confirm_quit_active) {
        Write-Host ($paletteSnapshot | ConvertTo-Json -Depth 8)
        throw 'The command palette did not acquire exclusive modal ownership'
    }
    # The snapshot that acknowledges a control is published before that dirty
    # frame is presented. Wait one additional renderer generation.
    $palettePresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$paletteSnapshot.sequence)
    if ([int]$palettePresented.palette_total_results -ne 7 -or
        [int]$palettePresented.palette_selected_index -ne 0 -or
        [int]$palettePresented.palette_scroll_offset -ne 0 -or
        -not ([string]$palettePresented.palette_accessibility_summary).StartsWith('Command categories; 7 results;')) {
        throw 'The native command palette did not open its seven-category root'
    }
    $paletteRootPanel = Get-ActiveAutomexiaPanel $palettePresented
    $paletteRoot = $palettePresented
    # Browse the real hierarchy before testing overflow; the short category
    # root must not be padded with fake commands just to make a wheel test pass.
    if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x28, $true)) {
        throw 'Could not select the native Panes and Sessions category'
    }
    $paletteDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $palettePresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$palettePresented.sequence)
    } while ([int]$palettePresented.palette_selected_index -ne 1 -and
             [DateTime]::UtcNow -lt $paletteDeadline)
    if ([int]$palettePresented.palette_selected_index -ne 1 -or
        -not [bool]$palettePresented.palette_enabled) {
        throw 'Native category selection did not become ready'
    }
    if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x0D, $false)) {
        throw 'Could not enter the native Panes and Sessions category'
    }
    $paletteDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $palettePresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$palettePresented.sequence)
    } while (-not ([string]$palettePresented.palette_accessibility_summary).StartsWith('Panes & Sessions;') -and
             [DateTime]::UtcNow -lt $paletteDeadline)
    $paletteCategoryPanel = Get-ActiveAutomexiaPanel $palettePresented
    if (-not [bool]$palettePresented.palette_enabled -or
        -not ([string]$palettePresented.palette_accessibility_summary).StartsWith('Panes & Sessions;') -or
        [int]$palettePresented.palette_total_results -ne 12 -or
        [int64]$paletteCategoryPanel.route_id -ne [int64]$paletteRootPanel.route_id -or
        [int]$palettePresented.display_offset -ne [int]$paletteRoot.display_offset -or
        [int]$palettePresented.cursor_column -ne [int]$paletteRoot.cursor_column -or
        [int]$palettePresented.cursor_row -ne [int]$paletteRoot.cursor_row -or
        [string]$paletteCategoryPanel.raw_cursor_line_text -ne [string]$paletteRootPanel.raw_cursor_line_text) {
        throw 'Native category entry changed terminal state, ran a command, or lost its Back row'
    }
    $palettePresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$palettePresented.sequence)
    if ([int]$palettePresented.palette_total_results -le
            [int]$palettePresented.palette_visible_results -or
        [int]$palettePresented.palette_scroll_offset -ne 0) {
        Write-Host ($palettePresented | ConvertTo-Json -Depth 8)
        throw 'The native command-palette fixture must begin at the top of an overflowing list'
    }
    $palettePanelBeforeWheel = Get-ActiveAutomexiaPanel $palettePresented
    $wheelClientX = [int]([double]$palettePresented.window_width / 2.0)
    $wheelClientY = [int]([double]$palettePresented.window_height / 2.0)

    $script:testStage = 'command palette native mouse wheel down'
    if (-not [AutomexiaResizeDriver]::PostMouseWheel(
            $window, $wheelClientX, $wheelClientY, -360)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not inject command-palette wheel-down input (Win32 error $code)"
    }
    $paletteWheelDown = Read-AutomexiaSnapshot -AfterSequence ([int64]$palettePresented.sequence)
    $paletteWheelDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ((-not [bool]$paletteWheelDown.palette_enabled -or
            [int]$paletteWheelDown.palette_scroll_offset -le 0) -and
           [DateTime]::UtcNow -lt $paletteWheelDeadline) {
        $paletteWheelDown =
            Read-AutomexiaSnapshot -AfterSequence ([int64]$paletteWheelDown.sequence)
    }
    $palettePanelAfterWheelDown = Get-ActiveAutomexiaPanel $paletteWheelDown
    $paletteSelectionEnd = [int]$paletteWheelDown.palette_scroll_offset +
        [int]$paletteWheelDown.palette_visible_results
    if (-not [bool]$paletteWheelDown.palette_enabled -or
        [int]$paletteWheelDown.palette_scroll_offset -le 0 -or
        [int]$paletteWheelDown.palette_selected_index -lt
            [int]$paletteWheelDown.palette_scroll_offset -or
        [int]$paletteWheelDown.palette_selected_index -ge $paletteSelectionEnd -or
        [int64]$palettePanelAfterWheelDown.route_id -ne
            [int64]$palettePanelBeforeWheel.route_id -or
        [int]$paletteWheelDown.display_offset -ne [int]$palettePresented.display_offset -or
        [int]$paletteWheelDown.cursor_column -ne [int]$palettePresented.cursor_column -or
        [int]$paletteWheelDown.cursor_row -ne [int]$palettePresented.cursor_row -or
        [string]$palettePanelAfterWheelDown.raw_cursor_line_text -ne
            [string]$palettePanelBeforeWheel.raw_cursor_line_text) {
        Write-Host ($paletteWheelDown | ConvertTo-Json -Depth 8)
        throw 'Native palette wheel-down escaped its modal list or hid the selected command'
    }

    $script:testStage = 'command palette native mouse wheel up'
    if (-not [AutomexiaResizeDriver]::PostMouseWheel(
            $window, $wheelClientX, $wheelClientY, 360)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not inject command-palette wheel-up input (Win32 error $code)"
    }
    $paletteWheelUp = Read-AutomexiaSnapshot -AfterSequence ([int64]$paletteWheelDown.sequence)
    $paletteWheelDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ((-not [bool]$paletteWheelUp.palette_enabled -or
            [int]$paletteWheelUp.palette_scroll_offset -ne 0) -and
           [DateTime]::UtcNow -lt $paletteWheelDeadline) {
        $paletteWheelUp =
            Read-AutomexiaSnapshot -AfterSequence ([int64]$paletteWheelUp.sequence)
    }
    $palettePanelAfterWheelUp = Get-ActiveAutomexiaPanel $paletteWheelUp
    if (-not [bool]$paletteWheelUp.palette_enabled -or
        [int]$paletteWheelUp.palette_scroll_offset -ne 0 -or
        [int64]$palettePanelAfterWheelUp.route_id -ne
            [int64]$palettePanelBeforeWheel.route_id -or
        [int]$paletteWheelUp.display_offset -ne [int]$palettePresented.display_offset -or
        [int]$paletteWheelUp.cursor_column -ne [int]$palettePresented.cursor_column -or
        [int]$paletteWheelUp.cursor_row -ne [int]$palettePresented.cursor_row -or
        [string]$palettePanelAfterWheelUp.raw_cursor_line_text -ne
            [string]$palettePanelBeforeWheel.raw_cursor_line_text) {
        Write-Host ($paletteWheelUp | ConvertTo-Json -Depth 8)
        throw 'Native palette wheel-up did not return to the top without terminal side effects'
    }
    $palettePresented = $paletteWheelUp
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for palette capture (Win32 error $code)"
    }
    try {
        Start-Sleep -Milliseconds 100
        $paletteFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
            $window, $paletteCapturePath)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if ($paletteFrame.Width -lt 100 -or
        $paletteFrame.Height -lt 100 -or
        $paletteFrame.SampleCount -lt 100 -or
        $paletteFrame.DistinctColorBuckets -lt 8 -or
        $paletteFrame.LuminanceSpread -lt 32) {
        throw "Command palette composited frame is blank or unreadable: $($paletteFrame.Width)x$($paletteFrame.Height), buckets=$($paletteFrame.DistinctColorBuckets), spread=$($paletteFrame.LuminanceSpread)"
    }

    $script:testStage = 'topmost close confirmation composition'
    $quitControl = 'confirm-quit:modal-layer'
    Send-AutomexiaTestControl $quitControl
    $quitSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$palettePresented.sequence)
    $quitDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$quitSnapshot.last_control -ne $quitControl -or
            [bool]$quitSnapshot.palette_enabled -or
            -not [bool]$quitSnapshot.confirm_quit_active) -and
           [DateTime]::UtcNow -lt $quitDeadline) {
        $quitSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$quitSnapshot.sequence)
    }
    if ([string]$quitSnapshot.last_control -ne $quitControl -or
        [bool]$quitSnapshot.palette_enabled -or
        -not [bool]$quitSnapshot.confirm_quit_active) {
        Write-Host ($quitSnapshot | ConvertTo-Json -Depth 8)
        throw 'The close confirmation did not replace the palette as the exclusive modal'
    }
    $quitPresented = Read-AutomexiaSnapshot -AfterSequence ([int64]$quitSnapshot.sequence)
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for close-confirmation capture (Win32 error $code)"
    }
    try {
        Start-Sleep -Milliseconds 100
        $quitFrame = [AutomexiaResizeDriver]::CaptureClientFrame(
            $window, $quitCapturePath)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if ($quitFrame.Width -lt 100 -or
        $quitFrame.Height -lt 100 -or
        $quitFrame.SampleCount -lt 100 -or
        $quitFrame.DistinctColorBuckets -lt 8 -or
        $quitFrame.LuminanceSpread -lt 32) {
        throw "Close confirmation composited frame is blank or unreadable: $($quitFrame.Width)x$($quitFrame.Height), buckets=$($quitFrame.DistinctColorBuckets), spread=$($quitFrame.LuminanceSpread)"
    }

    $script:testStage = 'modal dismissal restores terminal'
    $dismissModalControl = 'dismiss-modal:modal-layer'
    Send-AutomexiaTestControl $dismissModalControl
    $modalDismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$quitPresented.sequence)
    $modalDismissDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$modalDismissed.last_control -ne $dismissModalControl -or
            [bool]$modalDismissed.palette_enabled -or
            [bool]$modalDismissed.confirm_quit_active) -and
           [DateTime]::UtcNow -lt $modalDismissDeadline) {
        $modalDismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$modalDismissed.sequence)
    }
    if ([string]$modalDismissed.last_control -ne $dismissModalControl -or
        [bool]$modalDismissed.palette_enabled -or
        [bool]$modalDismissed.confirm_quit_active) {
        Write-Host ($modalDismissed | ConvertTo-Json -Depth 8)
        throw 'Modal dismissal left a hidden input-blocking surface active'
    }

    # Custom-chrome hit geometry is covered deterministically in Rust across
    # physical DPI scales. Exercise the native OS teardown route here without
    # relying on foreground-locked desktop pointer injection.
    $script:testStage = 'native close isolates Ctrl+Shift+N window'
    Send-AutomexiaTestControl 'new-window:9001'
    $windows = Wait-AutomexiaWindowCount -Expected 2
    $newWindow = @($windows | Where-Object { $_ -ne $window })[0]
    $windowCloseTimer = [Diagnostics.Stopwatch]::StartNew()
    if (-not [AutomexiaResizeDriver]::PostMessage(
        $newWindow, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw 'Could not post WM_CLOSE to the secondary Automexia window'
    }
    while ([AutomexiaResizeDriver]::IsWindowVisible($newWindow) -and
           $windowCloseTimer.ElapsedMilliseconds -lt 500) {
        Start-Sleep -Milliseconds 5
    }
    if ([AutomexiaResizeDriver]::IsWindowVisible($newWindow)) {
        throw 'Secondary window dismissal waited for resource cleanup'
    }
    $remaining = Wait-AutomexiaWindowCount -Expected 1
    $process.Refresh()
    if ($process.HasExited -or $remaining[0] -ne $window) {
        throw 'Native window close terminated or replaced the original Automexia window'
    }

    $script:testStage = 'post-storm resource ceiling'
    $resourceLimits = [ordered]@{
        handle_growth = $MaximumHandleGrowth
        thread_growth = $MaximumThreadGrowth
        private_bytes_growth = $MaximumPrivateBytesGrowth
        working_set_growth = $MaximumWorkingSetGrowth
        descendant_process_growth = $MaximumDescendantProcessGrowth
    }
    $resourceSettleDeadline = [DateTime]::UtcNow.AddSeconds(5)
    $resourceSettleTimer = [Diagnostics.Stopwatch]::StartNew()
    $resourceTimeline = [System.Collections.Generic.List[object]]::new()
    $resourceInitialSample = $null
    do {
        Start-Sleep -Milliseconds 100
        $resourceFinal = Get-AutomexiaResourceSample $process
        if ($null -eq $resourceInitialSample) { $resourceInitialSample = $resourceFinal }
        $descendantProcessGrowth =
            $resourceFinal.descendant_process_count -
            $resourceBaseline.descendant_process_count
        $resourceDelta = [ordered]@{
            handle_growth = $resourceFinal.handle_count - $resourceBaseline.handle_count
            thread_growth = $resourceFinal.thread_count - $resourceBaseline.thread_count
            private_bytes_growth = $resourceFinal.private_bytes - $resourceBaseline.private_bytes
            working_set_growth = $resourceFinal.working_set_bytes - $resourceBaseline.working_set_bytes
            descendant_process_growth = $descendantProcessGrowth
        }
        $resourceTimeline.Add([ordered]@{
            elapsed_milliseconds = $resourceSettleTimer.ElapsedMilliseconds
            handle_count = $resourceFinal.handle_count
            thread_count = $resourceFinal.thread_count
            private_bytes = $resourceFinal.private_bytes
            working_set_bytes = $resourceFinal.working_set_bytes
            descendant_process_count = $resourceFinal.descendant_process_count
        })
        $resourceExceeded = $false
        foreach ($name in $resourceLimits.Keys) {
            if ([int64]$resourceDelta[$name] -gt [int64]$resourceLimits[$name]) {
                $resourceExceeded = $true
            }
        }
    } while ($resourceExceeded -and [DateTime]::UtcNow -lt $resourceSettleDeadline)
    foreach ($name in $resourceLimits.Keys) {
        if ([int64]$resourceDelta[$name] -gt [int64]$resourceLimits[$name]) {
            Write-Host ('Native resource ceiling diagnostic: ' + (@{
                failed_metric = $name
                baseline = $resourceBaseline
                initial_sample = $resourceInitialSample
                failing_sample = $resourceFinal
                timeline = $resourceTimeline.ToArray()
            } | ConvertTo-Json -Depth 5 -Compress))
            throw "Native resource ceiling exceeded for $name`: $($resourceDelta[$name]) > $($resourceLimits[$name])"
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
        $reportPath = [IO.Path]::GetFullPath($ResourceReport)
        $reportDirectory = [IO.Path]::GetDirectoryName($reportPath)
        if (-not [string]::IsNullOrWhiteSpace($reportDirectory)) {
            New-Item -ItemType Directory -Force -Path $reportDirectory | Out-Null
        }
        $report = [ordered]@{
            schema_version = 1
            renderer_backend = [string]$initial.renderer_backend
            panel_count_at_final_sample = [int]$final.panel_count
            baseline = $resourceBaseline
            final = $resourceFinal
            delta = $resourceDelta
            ceilings = $resourceLimits
            resource_settle = [ordered]@{
                initial_sample = $resourceInitialSample
                elapsed_milliseconds = $resourceSettleTimer.ElapsedMilliseconds
                timeline = $resourceTimeline.ToArray()
            }
            typography_frame = [ordered]@{
                width = $typographyFrame.Width
                height = $typographyFrame.Height
                sample_count = $typographyFrame.SampleCount
                distinct_color_buckets = $typographyFrame.DistinctColorBuckets
                luminance_spread = $typographyFrame.LuminanceSpread
                attempts = $typographyAttempts
                artifact = if ($null -eq $typographyFramePath) {
                    $null
                } else {
                    [IO.Path]::GetFileName($typographyFramePath)
                }
            }
            fullscreen_brightness = [ordered]@{
                windowed_size = @($windowedBrightnessFrame.Width, $windowedBrightnessFrame.Height)
                fullscreen_size = @($fullscreenBrightnessFrame.Width, $fullscreenBrightnessFrame.Height)
                restored_size = @($restoredBrightnessFrame.Width, $restoredBrightnessFrame.Height)
                dominant_color_bucket = $windowedBrightnessFrame.DominantColorBucket
                display_request_released = $requestReleased
            }
            painted_frame = [ordered]@{
                width = $frameStats.Width
                height = $frameStats.Height
                sample_count = $frameStats.SampleCount
                distinct_color_buckets = $frameStats.DistinctColorBuckets
                dominant_color_bucket = $frameStats.DominantColorBucket
                luminance_spread = $frameStats.LuminanceSpread
                attempts = $frameAttempts
                settle_milliseconds = $frameStopwatch.ElapsedMilliseconds
                artifact = if ($null -eq $framePath) { $null } else { [IO.Path]::GetFileName($framePath) }
            }
            scoped_search_frame = [ordered]@{
                renderer = if ($UseCpuRenderer) { 'cpu' } else { 'wgpu' }
                width = $searchFrame.Width
                height = $searchFrame.Height
                sample_count = $searchFrame.SampleCount
                distinct_color_buckets = $searchFrame.DistinctColorBuckets
                luminance_spread = $searchFrame.LuminanceSpread
                attempts = $searchFrameAttempts
                surface_sample_count = $searchSurfacePixels.SampleCount
                surface_distinct_color_buckets = $searchSurfacePixels.DistinctColorBuckets
                surface_luminance_spread = $searchSurfacePixels.LuminanceSpread
                artifact = if ($null -eq $searchFramePath) {
                    $null
                } else {
                    [IO.Path]::GetFileName($searchFramePath)
                }
            }
            command_result_surface = [ordered]@{
                renderer = if ($UseCpuRenderer) { 'cpu' } else { 'wgpu' }
                pulse_generation = [int64]$historyReady.command_result_pulse_generation
                surface = $resultSurface
                divider = $resultDivider
                reserved_row_marker_gap = $resultMarkerGap
                attempts = $resultCaptureAttempts
                region_size = @($resultRegionWidth, $resultRegionHeight)
                region_sample_count = $resultPixels.SampleCount
                region_distinct_color_buckets = $resultPixels.DistinctColorBuckets
                region_luminance_spread = $resultPixels.LuminanceSpread
                glyph_sample_count = $resultGlyphPixels.SampleCount
                glyph_distinct_color_buckets = $resultGlyphPixels.DistinctColorBuckets
                glyph_luminance_spread = $resultGlyphPixels.LuminanceSpread
                opacity = $resultOpacity
                pulse_duration_milliseconds = [int]$historyReady.command_result_pulse_duration_ms
                pulse_hold_fraction = $resultPulseHold
                single_cycle = $true
                blank_surface_mean_rgb = @(
                    $resultSurfaceBackground.MeanRed,
                    $resultSurfaceBackground.MeanGreen,
                    $resultSurfaceBackground.MeanBlue)
                blank_reserved_row_mean_rgb = @(
                    $resultReservedRowBackground.MeanRed,
                    $resultReservedRowBackground.MeanGreen,
                    $resultReservedRowBackground.MeanBlue)
                blank_surface_reserved_row_rgb_delta = $resultPaintDelta
                representative_commands = $resultCommandEvidence
                completed_at_unix_ms = [int64]$historyReady.command_result_completed_at_unix_ms
                timestamp_label = [string]$historyReady.command_result_label
                resize_navigation = [ordered]@{
                    resized_grid = @(
                        [int]$commandJumpResized.columns,
                        [int]$commandJumpResized.rows)
                    previous_offset = [int]$commandJumpPrevious.display_offset
                    next_offset = [int]$commandJumpNext.display_offset
                    resized_paints = @($commandJumpResized.command_result_paints)
                    previous_paints = @($commandJumpPrevious.command_result_paints)
                    next_paints = @($commandJumpNext.command_result_paints)
                    restored_paints = @($commandJumpRestored.command_result_paints)
                    captured_frame = [ordered]@{
                        width = [int]$resultNavigationFrame.Width
                        height = [int]$resultNavigationFrame.Height
                        distinct_color_buckets = [int]$resultNavigationFrame.DistinctColorBuckets
                        luminance_spread = [int]$resultNavigationFrame.LuminanceSpread
                        artifact = if ($null -eq $resultNavigationFramePath) {
                            $null
                        } else {
                            [IO.Path]::GetFileName($resultNavigationFramePath)
                        }
                    }
                }
                artifact = if ($null -eq $resultFramePath) {
                    $null
                } else {
                    [IO.Path]::GetFileName($resultFramePath)
                }
            }
            modal_composition = [ordered]@{
                renderer = if ($UseCpuRenderer) { 'cpu' } else { 'wgpu' }
                palette = [ordered]@{
                    width = $paletteFrame.Width
                    height = $paletteFrame.Height
                    distinct_color_buckets = $paletteFrame.DistinctColorBuckets
                    luminance_spread = $paletteFrame.LuminanceSpread
                    wheel_down_offset = [int]$paletteWheelDown.palette_scroll_offset
                    wheel_down_selected_index =
                        [int]$paletteWheelDown.palette_selected_index
                    wheel_up_offset = [int]$paletteWheelUp.palette_scroll_offset
                    terminal_display_offset_preserved =
                        [int]$paletteWheelDown.display_offset -eq
                            [int]$palettePresented.display_offset
                    artifact = if ($null -eq $paletteCapturePath) {
                        $null
                    } else {
                        [IO.Path]::GetFileName($paletteCapturePath)
                    }
                }
                connection_hub = [ordered]@{
                    width = $hubFrame.Width
                    height = $hubFrame.Height
                    distinct_color_buckets = $hubFrame.DistinctColorBuckets
                    luminance_spread = $hubFrame.LuminanceSpread
                    section_sequence = @('results', 'workspaces', 'providers', 'results')
                    direct_entry = [ordered]@{
                        width = $hubDirectFrame.Width
                        height = $hubDirectFrame.Height
                        distinct_color_buckets = $hubDirectFrame.DistinctColorBuckets
                        luminance_spread = $hubDirectFrame.LuminanceSpread
                        close_cancelled_nested_only = $true
                        artifact = if ($null -eq $hubDirectCapturePath) {
                            $null
                        } else {
                            [IO.Path]::GetFileName($hubDirectCapturePath)
                        }
                    }
                    terminal_state_preserved = $true
                    artifact = if ($null -eq $hubCapturePath) {
                        $null
                    } else {
                        [IO.Path]::GetFileName($hubCapturePath)
                    }
                }
                close_confirmation = [ordered]@{
                    width = $quitFrame.Width
                    height = $quitFrame.Height
                    distinct_color_buckets = $quitFrame.DistinctColorBuckets
                    luminance_spread = $quitFrame.LuminanceSpread
                    artifact = if ($null -eq $quitCapturePath) {
                        $null
                    } else {
                        [IO.Path]::GetFileName($quitCapturePath)
                    }
                }
                native_close_confirmation = [ordered]@{
                    width = $nativeQuitFrame.Width
                    height = $nativeQuitFrame.Height
                    distinct_color_buckets = $nativeQuitFrame.DistinctColorBuckets
                    luminance_spread = $nativeQuitFrame.LuminanceSpread
                    cancelled_and_reopened = $true
                    artifact = if ($null -eq $nativeQuitCapturePath) {
                        $null
                    } else {
                        [IO.Path]::GetFileName($nativeQuitCapturePath)
                    }
                }
                exclusive_state_restored = (
                    -not [bool]$modalDismissed.palette_enabled -and
                    -not [bool]$modalDismissed.confirm_quit_active -and
                    -not [bool]$modalDismissed.connection_hub_active)
            }
            image_preview_pixels = [ordered]@{
                width = $previewPixels.Width
                height = $previewPixels.Height
                sample_count = $previewPixels.SampleCount
                distinct_color_buckets = $previewPixels.DistinctColorBuckets
                mean_luminance = $previewPixels.MeanLuminance
                luminance_spread = $previewPixels.LuminanceSpread
                bright_ratio = $brightRatio
            }
            image_preview_lifecycle = [ordered]@{
                cycles = $ImagePreviewLifecycleCycles
                renderer = if ($UseCpuRenderer) { 'cpu' } else { 'wgpu' }
                cache_entries = $expectedPreviewCacheEntries
                cache_bytes = $expectedPreviewCacheBytes
                baseline = $imageResourceBaseline
                final = $imageResourceFinal
                delta = $imageResourceDelta
                ceilings = $imageResourceLimits
            }
        } | ConvertTo-Json -Depth 5
    }

    $successSummary = (
        'Native CMD/resize/history/fullscreen/multi-window stress passed: sequence {0}, grid {1}x{2}, prompt {3}, Up shell/VT {4}ms ({5}ms total), Ctrl+R shell/VT {6}ms ({7}ms total)' -f
        $final.sequence, $final.columns, $final.rows, $final.latest_prompt_id,
        $upShellMilliseconds, $upTimer.ElapsedMilliseconds,
        $searchShellMilliseconds, $searchTimer.ElapsedMilliseconds)

    # Capture exact process identities before closing the owner. Once a child
    # becomes orphaned, count-only sampling and parent-tree traversal can no
    # longer prove that the original route released it.
    $script:testStage = 'native final-window close confirmation and cancellation'
    $beforeNativeClose = Read-AutomexiaSnapshot
    if (-not [AutomexiaResizeDriver]::PostMessage(
            $window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw 'Could not post WM_CLOSE to the final Automexia window'
    }
    $nativeClose = Read-AutomexiaSnapshot -AfterSequence ([int64]$beforeNativeClose.sequence)
    $nativeCloseDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (-not [bool]$nativeClose.confirm_quit_active -and
           [DateTime]::UtcNow -lt $nativeCloseDeadline) {
        $nativeClose = Read-AutomexiaSnapshot -AfterSequence ([int64]$nativeClose.sequence)
    }
    $process.Refresh()
    if (-not [bool]$nativeClose.confirm_quit_active -or
        $process.HasExited -or
        -not [AutomexiaResizeDriver]::IsWindowVisible($window)) {
        throw 'WM_CLOSE bypassed the in-app confirmation or left a native dialog blocking it'
    }
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        throw 'Could not expose the native close confirmation for capture'
    }
    try {
        Start-Sleep -Milliseconds 100
        $nativeQuitFrame = [AutomexiaResizeDriver]::CaptureStableCloseDialogFrame(
            $window, $nativeQuitCapturePath)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if ($nativeQuitFrame.SampleCount -lt 100 -or
        $nativeQuitFrame.DistinctColorBuckets -lt 8 -or
        $nativeQuitFrame.LuminanceSpread -lt 32) {
        throw 'The native close confirmation frame is blank or unreadable'
    }
    if (-not [AutomexiaResizeDriver]::PostKeyTap($window, 0x1B, $false)) {
        throw 'Could not cancel the native close confirmation with Escape'
    }
    $cancelledClose = Read-AutomexiaSnapshot -AfterSequence ([int64]$nativeClose.sequence)
    $cancelDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([bool]$cancelledClose.confirm_quit_active -and
           [DateTime]::UtcNow -lt $cancelDeadline) {
        $cancelledClose = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelledClose.sequence)
    }
    $process.Refresh()
    if ([bool]$cancelledClose.confirm_quit_active -or $process.HasExited -or
        -not [AutomexiaResizeDriver]::IsWindowVisible($window)) {
        throw 'Cancelling the native close did not restore the running window'
    }

    $script:testStage = 'application process-tree shutdown'
    $ownedDescendantsAtShutdown = @(
        Get-AutomexiaOwnedProcessIds $process.Id $configRoot)
    if (-not $process.CloseMainWindow()) {
        throw 'Automexia did not accept the native close request'
    }
    $confirmedClose = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelledClose.sequence)
    $confirmDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (-not [bool]$confirmedClose.confirm_quit_active -and
           [DateTime]::UtcNow -lt $confirmDeadline) {
        $confirmedClose = Read-AutomexiaSnapshot -AfterSequence ([int64]$confirmedClose.sequence)
    }
    if (-not [bool]$confirmedClose.confirm_quit_active) {
        throw 'The final native close did not reopen the in-app confirmation'
    }
    $shutdownTimer = [Diagnostics.Stopwatch]::StartNew()
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap(
            $window, 0x59, $false, $false, $false)) {
        throw 'Could not accept the native close confirmation with Y'
    }
    # Window retirement and child/resource teardown are different observations.
    # A process-exit ceiling alone previously allowed seconds of visible lag.
    while ([AutomexiaResizeDriver]::IsWindowVisible($window) -and
           $shutdownTimer.ElapsedMilliseconds -lt 500) {
        Start-Sleep -Milliseconds 5
    }
    $dismissalMilliseconds = $shutdownTimer.ElapsedMilliseconds
    if ([AutomexiaResizeDriver]::IsWindowVisible($window)) {
        throw 'Final window dismissal waited for resource cleanup'
    }
    if (-not $process.WaitForExit(15000)) {
        throw 'Automexia did not exit within the native shutdown budget'
    }
    $ownedExitDeadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        $remainingOwnedProcesses = @($ownedDescendantsAtShutdown | Where-Object {
            $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue)
        })
        if ($remainingOwnedProcesses.Count -eq 0) {
            break
        }
        Start-Sleep -Milliseconds 25
    } while ([DateTime]::UtcNow -lt $ownedExitDeadline)
    $shutdownTimer.Stop()
    if ($remainingOwnedProcesses.Count -ne 0) {
        throw "Automexia shutdown left $($remainingOwnedProcesses.Count) owned descendant processes"
    }
    if ($shutdownTimer.ElapsedMilliseconds -gt $MaximumOwnedShutdownMilliseconds) {
        throw "Automexia owned shutdown exceeded the multi-session wall-clock ceiling: $($shutdownTimer.ElapsedMilliseconds)ms > ${MaximumOwnedShutdownMilliseconds}ms"
    }
    if ($null -ne $reportPath -and $null -ne $report) {
        $reportData = $report | ConvertFrom-Json
        $reportData | Add-Member -NotePropertyName owned_process_tree_shutdown -NotePropertyValue ([ordered]@{
            descendant_count = $ownedDescendantsAtShutdown.Count
            dismissal_milliseconds = $dismissalMilliseconds
            dismissal_ceiling_milliseconds = 500
            elapsed_milliseconds = $shutdownTimer.ElapsedMilliseconds
            ceiling_milliseconds = $MaximumOwnedShutdownMilliseconds
            owner_exited = $true
            descendants_exited = $true
        })
        $temporaryReport = "$reportPath.$PID.tmp"
        [IO.File]::WriteAllText(
            $temporaryReport,
            ($reportData | ConvertTo-Json -Depth 7),
            [Text.UTF8Encoding]::new($false))
        Move-Item -LiteralPath $temporaryReport -Destination $reportPath -Force
    }
    $process = $null
    Write-Host ($successSummary + ', owned shutdown ' +
        $shutdownTimer.ElapsedMilliseconds + 'ms')
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        $ownedDescendantsAtShutdown = @(
            Get-AutomexiaOwnedProcessIds $process.Id $configRoot)
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(15000)) {
            Stop-Process -Id $process.Id -Force
        }
    }
    foreach ($ownedProcessId in $ownedDescendantsAtShutdown) {
        if ($null -ne (Get-Process -Id $ownedProcessId -ErrorAction SilentlyContinue)) {
            try {
                Stop-Process -Id $ownedProcessId -Force
            } catch {
                # A descendant can finish between lookup and cleanup. Preserve
                # the original assertion failure while retaining other errors.
                if ($_.FullyQualifiedErrorId -notlike 'NoProcessFoundForGivenId,*') {
                    throw
                }
            }
        }
    }
    if ($null -eq $previousSnapshot) {
        Remove-Item Env:AUTOMEXIA_RESIZE_SNAPSHOT -ErrorAction SilentlyContinue
    } else {
        $env:AUTOMEXIA_RESIZE_SNAPSHOT = $previousSnapshot
    }
    if ($null -eq $previousControl) {
        Remove-Item Env:AUTOMEXIA_NATIVE_TEST_CONTROL -ErrorAction SilentlyContinue
    } else {
        $env:AUTOMEXIA_NATIVE_TEST_CONTROL = $previousControl
    }
    if ($null -eq $previousConfigHome) {
        Remove-Item Env:AUTOMEXIA_CONFIG_HOME -ErrorAction SilentlyContinue
    } else {
        $env:AUTOMEXIA_CONFIG_HOME = $previousConfigHome
    }
    if ($null -eq $previousVisualFixture) {
        Remove-Item Env:AUTOMEXIA_VISUAL_TEST_FIXTURE -ErrorAction SilentlyContinue
    } else {
        $env:AUTOMEXIA_VISUAL_TEST_FIXTURE = $previousVisualFixture
    }
    Remove-Item -LiteralPath $snapshotPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $controlPath -Force -ErrorAction SilentlyContinue
    $resolvedFixtureRoot = [IO.Path]::GetFullPath($configRoot)
    $resolvedTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $resolvedFixtureRoot.StartsWith($resolvedTempRoot, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetFileName($resolvedFixtureRoot) -notmatch '^automexia-resize-config-[0-9a-f]{32}$') {
        throw 'Refusing cleanup outside the owned temporary configuration root'
    }
    Remove-Item -LiteralPath $resolvedFixtureRoot -Recurse -Force -ErrorAction SilentlyContinue
}
