$script:AutomexiaCloudReparseTagBase = [Convert]::ToUInt32('9000001A', 16)
$script:AutomexiaCloudReparseTagMask = [Convert]::ToUInt32('FFFF0FFF', 16)
$script:AutomexiaNameSurrogateReparseTagMask = [Convert]::ToUInt32('20000000', 16)

function Initialize-AutomexiaReparseInspector {
    if ('Automexia.NativeFileSystem' -as [type]) { return }

    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace Automexia
{
    public static class NativeFileSystem
    {
        private const uint FileShareRead = 0x00000001;
        private const uint FileShareWrite = 0x00000002;
        private const uint FileShareDelete = 0x00000004;
        private const uint OpenExisting = 3;
        private const uint FileFlagBackupSemantics = 0x02000000;
        private const uint FileFlagOpenReparsePoint = 0x00200000;
        private const int FileAttributeTagInfo = 9;

        [StructLayout(LayoutKind.Sequential)]
        private struct FileAttributeTagInformation
        {
            internal uint FileAttributes;
            internal uint ReparseTag;
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern SafeFileHandle CreateFileW(
            string fileName,
            uint desiredAccess,
            uint shareMode,
            IntPtr securityAttributes,
            uint creationDisposition,
            uint flagsAndAttributes,
            IntPtr templateFile);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetFileInformationByHandleEx(
            SafeFileHandle file,
            int fileInformationClass,
            out FileAttributeTagInformation fileInformation,
            uint bufferSize);

        public static uint GetReparseTag(string path)
        {
            using (SafeFileHandle handle = CreateFileW(
                path,
                0,
                FileShareRead | FileShareWrite | FileShareDelete,
                IntPtr.Zero,
                OpenExisting,
                FileFlagBackupSemantics | FileFlagOpenReparsePoint,
                IntPtr.Zero))
            {
                if (handle.IsInvalid)
                {
                    throw new Win32Exception(
                        Marshal.GetLastWin32Error(),
                        "Unable to open the reparse point for inspection: " + path);
                }

                FileAttributeTagInformation information;
                uint size = (uint)Marshal.SizeOf(typeof(FileAttributeTagInformation));
                if (!GetFileInformationByHandleEx(
                    handle,
                    FileAttributeTagInfo,
                    out information,
                    size))
                {
                    throw new Win32Exception(
                        Marshal.GetLastWin32Error(),
                        "Unable to read the reparse tag: " + path);
                }

                return information.ReparseTag;
            }
        }
    }
}
'@
}

function Test-AutomexiaCloudReparseTag([uint32]$Tag) {
    return (($Tag -band $script:AutomexiaCloudReparseTagMask) -eq
        $script:AutomexiaCloudReparseTagBase)
}

function Test-AutomexiaNameSurrogateReparseTag([uint32]$Tag) {
    return (($Tag -band $script:AutomexiaNameSurrogateReparseTagMask) -ne 0)
}

function Get-AutomexiaReparseTag([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force
    if (-not $item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
        return [uint32]0
    }

    Initialize-AutomexiaReparseInspector
    return [Automexia.NativeFileSystem]::GetReparseTag($item.FullName)
}

function Assert-AutomexiaSupportedProfileReparsePoint([string]$Path, [uint32]$Tag) {
    if (Test-AutomexiaCloudReparseTag $Tag) { return }

    $formattedTag = '0x{0:X8}' -f $Tag
    if (Test-AutomexiaNameSurrogateReparseTag $Tag) {
        throw "Refusing linked PowerShell profile path ($formattedTag): $Path"
    }
    throw "Refusing unsupported PowerShell profile reparse point ($formattedTag): $Path"
}

function Assert-AutomexiaSafeProfilePathChain([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw 'PowerShell profile path is unavailable.'
    }

    $candidate = [IO.Path]::GetFullPath($Path)
    while (-not (Test-Path -LiteralPath $candidate)) {
        $parent = Split-Path -Parent $candidate
        if ([string]::IsNullOrWhiteSpace($parent) -or $parent -eq $candidate) {
            throw "PowerShell profile path has no existing ancestor: $Path"
        }
        $candidate = $parent
    }

    $item = Get-Item -LiteralPath $candidate -Force
    while ($null -ne $item) {
        if ($item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
            $tag = Get-AutomexiaReparseTag $item.FullName
            Assert-AutomexiaSupportedProfileReparsePoint $item.FullName $tag
        }
        $item = $item.Parent
    }
}
