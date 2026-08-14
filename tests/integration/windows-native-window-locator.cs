using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class AutomexiaNativeWindowLocator
{
    private delegate bool EnumWindowsCallback(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(EnumWindowsCallback callback, IntPtr lParam);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr hWnd);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int capacity);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(IntPtr hWnd, StringBuilder text, int capacity);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetClientRect(IntPtr hWnd, out Rect rect);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(
        IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential)]
    private struct Rect
    {
        internal int Left;
        internal int Top;
        internal int Right;
        internal int Bottom;
    }

    public static IntPtr[] VisibleApplicationWindows(int expectedProcessId)
    {
        var windows = new List<IntPtr>();
        EnumWindows((hWnd, _) =>
        {
            uint processId;
            GetWindowThreadProcessId(hWnd, out processId);
            if (processId != (uint)expectedProcessId || !IsWindowVisible(hWnd))
            {
                return true;
            }

            var title = new StringBuilder(512);
            var className = new StringBuilder(256);
            GetWindowText(hWnd, title, title.Capacity);
            GetClassName(hWnd, className, className.Capacity);
            Rect rect;
            if (title.Length > 0
                && !String.Equals(
                    className.ToString(),
                    "Winit Thread Event Target",
                    StringComparison.Ordinal)
                && GetClientRect(hWnd, out rect)
                && rect.Right - rect.Left >= 100
                && rect.Bottom - rect.Top >= 100)
            {
                windows.Add(hWnd);
            }
            return true;
        }, IntPtr.Zero);
        return windows.ToArray();
    }

    public static string DescribeWindow(IntPtr hWnd)
    {
        var title = new StringBuilder(512);
        var className = new StringBuilder(256);
        GetWindowText(hWnd, title, title.Capacity);
        GetClassName(hWnd, className, className.Capacity);
        Rect rect;
        GetClientRect(hWnd, out rect);
        return String.Format(
            "0x{0:X} class='{1}' title='{2}' client={3}x{4}",
            hWnd.ToInt64(),
            className,
            title,
            rect.Right - rect.Left,
            rect.Bottom - rect.Top);
    }
}