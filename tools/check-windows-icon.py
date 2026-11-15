#!/usr/bin/env python3
"""Verify the built PE's application icon against Sheaf's source ICO, using only the standard library.

    python tools/check-windows-icon.py target/debug/sheaf-app.exe

This reads the actual RT_GROUP_ICON and RT_ICON resources, checks every frame's dimensions and
payload, and fails if the executable has no application icon or contains different artwork.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import struct


def unpack(data: bytes, offset: int, fmt: str) -> tuple:
    size = struct.calcsize(fmt)
    if offset < 0 or offset + size > len(data):
        raise ValueError("truncated icon or PE structure")
    return struct.unpack_from(fmt, data, offset)


def ico_frames(data: bytes) -> list[tuple[int, int, bytes]]:
    reserved, kind, count = unpack(data, 0, "<HHH")
    if reserved != 0 or kind != 1 or count == 0:
        raise ValueError("the icon input is not a nonempty ICO file")
    frames = []
    for i in range(count):
        w, h, _, _, _, _, size, offset = unpack(data, 6 + i * 16, "<BBBBHHII")
        if offset + size > len(data) or size == 0:
            raise ValueError("truncated ICO frame")
        frames.append((w or 256, h or 256, data[offset:offset + size]))
    return frames


def pe_resources(data: bytes) -> dict[tuple, bytes]:
    if data[:2] != b"MZ":
        raise ValueError("the executable has no DOS/PE header")
    pe, = unpack(data, 0x3C, "<I")
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("the executable has no PE signature")
    _, sections, _, _, _, optional_size, _ = unpack(data, pe + 4, "<HHIIIHH")
    optional = pe + 24
    magic, = unpack(data, optional, "<H")
    if magic == 0x10B:
        directory, count_offset = optional + 96, optional + 92
    elif magic == 0x20B:
        directory, count_offset = optional + 112, optional + 108
    else:
        raise ValueError("unsupported PE optional-header format")
    count, = unpack(data, count_offset, "<I")
    if count < 3 or directory + 24 > optional + optional_size:
        raise ValueError("the PE has no resource directory")
    resource_rva, resource_size = unpack(data, directory + 16, "<II")
    if not resource_rva or not resource_size:
        raise ValueError("the PE has no resource directory")
    segments = []
    for i in range(sections):
        section = optional + optional_size + i * 40
        _, rva, raw_size, raw_offset = unpack(data, section + 8, "<IIII")
        segments.append((rva, raw_size, raw_offset))

    def at_rva(rva: int, size: int) -> int:
        for begin, length, offset in segments:
            if begin <= rva and rva + size <= begin + length:
                result = offset + rva - begin
                if result + size <= len(data):
                    return result
        raise ValueError("resource data points outside the PE sections")

    base = at_rva(resource_rva, resource_size)

    def relative(offset: int, size: int) -> int:
        if offset < 0 or offset + size > resource_size:
            raise ValueError("resource directory points outside its section")
        return base + offset

    resources = {}

    def visit(offset: int, keys: tuple) -> None:
        if len(keys) >= 4:
            raise ValueError("unexpectedly deep resource directory")
        header = relative(offset, 16)
        named, numbered = unpack(data, header + 12, "<HH")
        for i in range(named + numbered):
            entry = relative(offset + 16 + i * 8, 8)
            name, child = unpack(data, entry, "<II")
            if name & 0x80000000:
                text = name & 0x7FFFFFFF
                length, = unpack(data, relative(text, 2), "<H")
                start = relative(text + 2, length * 2)
                key = data[start:start + length * 2].decode("utf-16-le")
            else:
                key = name
            if child & 0x80000000:
                visit(child & 0x7FFFFFFF, keys + (key,))
            else:
                value = relative(child, 16)
                rva, size, _, _ = unpack(data, value, "<IIII")
                start = at_rva(rva, size)
                resources[keys + (key,)] = data[start:start + size]

    visit(0, ())
    return resources


def verify(executable: Path, icon: Path) -> list[tuple[int, int]]:
    expected = ico_frames(icon.read_bytes())
    resources = pe_resources(executable.read_bytes())
    groups = [(key, value) for key, value in resources.items() if len(key) == 3 and key[:2] == (14, 1)]
    if not groups:
        raise ValueError("the PE has no application RT_GROUP_ICON with ID 1")
    for key, group in groups:
        reserved, kind, count = unpack(group, 0, "<HHH")
        if reserved != 0 or kind != 1 or count != len(expected):
            raise ValueError("the group icon does not contain the source ICO's frames")
        for i, (width, height, payload) in enumerate(expected):
            w, h, _, _, _, _, size, icon_id = unpack(group, 6 + i * 14, "<BBBBHHIH")
            if (w or 256, h or 256) != (width, height) or size != len(payload):
                raise ValueError(f"icon frame {i} dimensions or size differ from the source ICO")
            if resources.get((3, icon_id, key[2])) != payload:
                raise ValueError(f"embedded icon frame {i} does not match the source artwork")
    return [(width, height) for width, height, _ in expected]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("--icon", type=Path, default=Path(__file__).resolve().parents[1] / "crates/app/assets/icon/sheaf.ico")
    args = parser.parse_args()
    try:
        sizes = verify(args.executable, args.icon)
    except (OSError, ValueError, UnicodeError) as error:
        parser.exit(1, f"icon verification failed: {error}\n")
    print(f"Verified application icon ID 1: {len(sizes)} frames match {args.icon.name}: {sizes}")


if __name__ == "__main__":
    main()
