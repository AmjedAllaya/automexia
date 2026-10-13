"""Read-only check of branding actually embedded in compiled Windows MSIs."""

from __future__ import annotations

import argparse
import ctypes
import os
from pathlib import Path
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[2]


def verify(path: Path) -> None:
    if os.name != "nt":
        raise ValueError("compiled MSI inspection requires Windows")
    msi = ctypes.WinDLL("msi")
    handle = ctypes.c_uint
    pointer = ctypes.POINTER(handle)
    signatures = {
        "MsiOpenDatabaseW": [ctypes.c_wchar_p, ctypes.c_void_p, pointer],
        "MsiDatabaseOpenViewW": [handle, ctypes.c_wchar_p, pointer],
        "MsiViewExecute": [handle, handle],
        "MsiViewFetch": [handle, pointer],
        "MsiRecordGetStringW": [handle, ctypes.c_uint, ctypes.c_wchar_p, pointer],
        "MsiRecordReadStream": [handle, ctypes.c_uint, ctypes.c_void_p, pointer],
        "MsiCloseHandle": [handle],
    }
    for name, arguments in signatures.items():
        function = getattr(msi, name)
        function.argtypes = arguments
        function.restype = ctypes.c_uint

    def check(result: int) -> None:
        if result:
            raise ValueError(f"Windows Installer returned error {result}")

    def read_rows(query: str, columns: int, stream_column: int = 0) -> list:
        view = handle()
        check(msi.MsiDatabaseOpenViewW(database, query, ctypes.byref(view)))
        rows = []
        try:
            check(msi.MsiViewExecute(view, 0))
            for _ in range(4096):
                record = handle()
                result = msi.MsiViewFetch(view, ctypes.byref(record))
                if result == 259:  # ERROR_NO_MORE_ITEMS
                    return rows
                check(result)
                try:
                    row = []
                    for field in range(1, columns + 1):
                        if field == stream_column:
                            size = handle()
                            check(msi.MsiRecordReadStream(record, field, None, ctypes.byref(size)))
                            if size.value > 2 * 1024 * 1024:
                                raise ValueError("installer artwork exceeds 2 MiB")
                            buffer = ctypes.create_string_buffer(size.value)
                            check(msi.MsiRecordReadStream(record, field, buffer, ctypes.byref(size)))
                            row.append(buffer.raw[:size.value])
                        else:
                            size = handle(8192)
                            buffer = ctypes.create_unicode_buffer(size.value)
                            check(msi.MsiRecordGetStringW(record, field, buffer, ctypes.byref(size)))
                            row.append(buffer.value)
                    rows.append(row)
                finally:
                    msi.MsiCloseHandle(record)
            raise ValueError("installer table exceeds 4096 rows")
        finally:
            msi.MsiCloseHandle(view)

    database = handle()
    check(msi.MsiOpenDatabaseW(str(path.resolve()), None, ctypes.byref(database)))
    try:
        for name, filename in (("WixUI_Bmp_Banner", "banner.bmp"),
                               ("WixUI_Bmp_Dialog", "dialog.bmp")):
            rows = read_rows(f"SELECT `Data` FROM `Binary` WHERE `Name` = '{name}'", 1, 1)
            expected = (ROOT / "assets/brand/windows-installer" / filename).read_bytes()
            if len(rows) != 1 or rows[0][0] != expected:
                raise ValueError(f"{path.name}: missing or incorrect Automexia {filename}")
        controls = {(dialog, control): text for dialog, control, text in
                    read_rows("SELECT `Dialog_`, `Control`, `Text` FROM `Control`", 3)}
        strings = {item.attrib["Id"]: item.text for item in
                   ET.parse(ROOT / "packaging/windows/automexia-en-us.wxl").getroot()}
        strings["UserExitDescription"] = (strings["UserExitDescription1"] + " "
                                          + strings["UserExitDescription2"])
        for dialog in ("WelcomeDlg", "MaintenanceWelcomeDlg", "InstallDirDlg",
                       "LicenseAgreementDlg", "ExitDialog", "UserExit"):
            for control in ("Title", "Description"):
                if controls.get((dialog, control)) != strings[dialog + control]:
                    raise ValueError(f"{path.name}: incorrect {dialog} {control}")
    finally:
        msi.MsiCloseHandle(database)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, help="native packaging output directory")
    args = parser.parse_args()
    packages = []
    # cargo-packager may put its MSI one level below the requested output folder.
    for pattern in ("*.msi", "*/*.msi"):
        packages.extend(args.directory.glob(pattern))
    if not 1 <= len(packages) <= 16:
        parser.error("expected between 1 and 16 MSI files in the package output")
    try:
        for package in packages:
            verify(package)
    except (OSError, ValueError, ET.ParseError) as error:
        parser.error(str(error))
    print(f"PASS: Automexia artwork and setup text verified in {len(packages)} MSI(s)")


if __name__ == "__main__":
    main()
