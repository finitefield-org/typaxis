#!/usr/bin/env python3
"""Generate the compact Unicode 16 data table used by typaxis-linebreak.

The inputs are the unmodified Unicode Character Database files named below.
The generated Rust table deliberately carries only the properties required by
UAX #14 revision 53. Network access is not performed by this script.
"""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


UNICODE_VERSION = "16.0.0"
EXPECTED_SHA256 = {
    "LineBreak.txt": "e97e4259d0d20fab150b9c7b4b28abfae5cd78ca97e7f4ac6ed20d685d5f4a7c",
    "UnicodeData.txt": "ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f",
    "EastAsianWidth.txt": "43adc76c0686a42cb370764eb8cfe2b2a45b10b855e5572a2db4a0eecce15d5b",
    "emoji-data.txt": "f1365a5173eee18e1f98b240cdc492e84a25f1ce7e0c9d1094eb29c41a22696a",
}

CLASSES = [
    "AI", "AK", "AL", "AP", "AS", "B2", "BA", "BB", "BK", "CB",
    "CJ", "CL", "CM", "CP", "CR", "EB", "EM", "EX", "GL", "H2",
    "H3", "HL", "HY", "ID", "IN", "IS", "JL", "JT", "JV", "LF",
    "NL", "NS", "NU", "OP", "PO", "PR", "QU", "RI", "SA", "SG",
    "SP", "SY", "VF", "VI", "WJ", "XX", "ZW", "ZWJ",
]
CLASS_ID = {name: index for index, name in enumerate(CLASSES)}

CLASS_MASK = 0x3F
INITIAL_PUNCTUATION = 1 << 6
FINAL_PUNCTUATION = 1 << 7
EAST_ASIAN = 1 << 8
MARK = 1 << 9
UNASSIGNED_EXTENDED_PICTOGRAPHIC = 1 << 10
CODEPOINT_LIMIT = 0x110000


def checked_bytes(path: Path) -> bytes:
    data = path.read_bytes()
    actual = hashlib.sha256(data).hexdigest()
    expected = EXPECTED_SHA256[path.name]
    if actual != expected:
        raise ValueError(f"{path}: SHA-256 {actual}, expected {expected}")
    return data


def codepoint_range(field: str) -> tuple[int, int]:
    bounds = field.strip().split("..")
    start = int(bounds[0], 16)
    end = int(bounds[-1], 16)
    if start > end or end >= CODEPOINT_LIMIT:
        raise ValueError(f"invalid code point range: {field}")
    return start, end


def property_rows(data: bytes):
    for raw_line in data.decode("utf-8").splitlines():
        content = raw_line.split("#", 1)[0].strip()
        if not content:
            continue
        fields = content.split(";")
        if len(fields) < 2:
            raise ValueError(f"malformed property row: {raw_line}")
        yield codepoint_range(fields[0]), fields[1].strip()


def set_flag(values: list[int], start: int, end: int, flag: int) -> None:
    for codepoint in range(start, end + 1):
        values[codepoint] |= flag


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ucd-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    inputs = {
        name: checked_bytes(args.ucd_dir / name) for name in EXPECTED_SHA256
    }
    values = [CLASS_ID["XX"]] * CODEPOINT_LIMIT

    # UAX #14 default values for undesignated CJK and currency positions.
    defaults = [
        (0x3400, 0x4DBF, "ID"),
        (0x4E00, 0x9FFF, "ID"),
        (0xF900, 0xFAFF, "ID"),
        (0x1F000, 0x1FAFF, "ID"),
        (0x1FC00, 0x1FFFD, "ID"),
        (0x20000, 0x2FFFD, "ID"),
        (0x30000, 0x3FFFD, "ID"),
        (0x20A0, 0x20CF, "PR"),
    ]
    for start, end, class_name in defaults:
        class_id = CLASS_ID[class_name]
        values[start : end + 1] = [class_id] * (end - start + 1)

    for (start, end), class_name in property_rows(inputs["LineBreak.txt"]):
        class_id = CLASS_ID[class_name]
        for codepoint in range(start, end + 1):
            values[codepoint] = (values[codepoint] & ~CLASS_MASK) | class_id

    assigned = bytearray(CODEPOINT_LIMIT)
    unicode_data = inputs["UnicodeData.txt"].decode("utf-8").splitlines()
    pending_first: tuple[int, str] | None = None
    for line in unicode_data:
        fields = line.split(";")
        if len(fields) < 3:
            raise ValueError(f"malformed UnicodeData row: {line}")
        codepoint = int(fields[0], 16)
        name = fields[1]
        category = fields[2]
        if name.endswith(", First>"):
            pending_first = (codepoint, category)
            continue
        if name.endswith(", Last>"):
            if pending_first is None or pending_first[1] != category:
                raise ValueError(f"unmatched UnicodeData range end: {line}")
            start = pending_first[0]
            pending_first = None
        else:
            if pending_first is not None:
                raise ValueError("unterminated UnicodeData range")
            start = codepoint
        assigned[start : codepoint + 1] = b"\x01" * (codepoint - start + 1)
        flag = {
            "Pi": INITIAL_PUNCTUATION,
            "Pf": FINAL_PUNCTUATION,
            "Mn": MARK,
            "Mc": MARK,
        }.get(category)
        if flag is not None:
            set_flag(values, start, codepoint, flag)
    if pending_first is not None:
        raise ValueError("unterminated UnicodeData range")

    for (start, end), width in property_rows(inputs["EastAsianWidth.txt"]):
        if width in {"F", "W", "H"}:
            set_flag(values, start, end, EAST_ASIAN)

    for (start, end), property_name in property_rows(inputs["emoji-data.txt"]):
        if property_name != "Extended_Pictographic":
            continue
        for codepoint in range(start, end + 1):
            if not assigned[codepoint]:
                values[codepoint] |= UNASSIGNED_EXTENDED_PICTOGRAPHIC

    ranges: list[tuple[int, int, int]] = []
    start = 0
    current = values[0]
    for codepoint, value in enumerate(values[1:], 1):
        if value != current:
            ranges.append((start, codepoint - 1, current))
            start = codepoint
            current = value
    ranges.append((start, CODEPOINT_LIMIT - 1, current))

    lines = [
        "// @generated by tools/generate_unicode_linebreak.py; do not edit.",
        f'pub(super) const UNICODE_LINE_BREAK_VERSION: &str = "{UNICODE_VERSION}";',
        "pub(super) static UNICODE_LINE_BREAK_RANGES: &[(u32, u32, u16)] = &[",
    ]
    lines.extend(
        f"    (0x{start:06X}, 0x{end:06X}, 0x{value:04X}),"
        for start, end, value in ranges
    )
    lines.append("];")
    lines.append("")
    args.output.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
