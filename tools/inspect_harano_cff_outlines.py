#!/usr/bin/env python3
"""Independently execute original Harano Type2 programs with FontTools."""
import argparse
import hashlib
import json
from pathlib import Path
import fontTools
from fontTools.pens.recordingPen import RecordingPen
from fontTools.ttLib import TTFont


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("font", type=Path)
    args = parser.parse_args()
    source = args.font.read_bytes()
    digest = hashlib.sha256(source).hexdigest()
    if digest != "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717":
        parser.error("requires unchanged original HaranoAjiMincho-Regular.otf")
    state = hashlib.sha256(b"typaxis.cff-v2-outline-records/1").digest()
    widths = hashlib.sha256()
    segments = 0
    different_widths = []
    with TTFont(args.font) as font:
        top = font["CFF "].cff.topDictIndex[0]
        for gid, name in enumerate(top.charset):
            charstring = top.CharStrings[name]
            pen = RecordingPen()
            charstring.draw(pen)
            advance = font["hmtx"].metrics[name][0]
            fixed_width = charstring.width * 65536
            if int(fixed_width) != fixed_width:
                raise ValueError(f"nonfixed width at GID {gid}")
            widths.update(int(fixed_width).to_bytes(4, "big", signed=True))
            if charstring.width != advance:
                different_widths.append([gid, charstring.width, advance])
            record = bytearray()
            for operation, points in pen.value:
                symbol, count = {"moveTo": (b"M", 1), "lineTo": (b"L", 1), "curveTo": (b"C", 3), "closePath": (b"Z", 0)}[operation]
                if len(points) != count:
                    raise ValueError(f"unexpected pen operands at GID {gid}")
                record.extend(symbol)
                for point in points:
                    for coordinate in point:
                        fixed = coordinate * 65536
                        if int(fixed) != fixed:
                            raise ValueError(f"nonfixed coordinate at GID {gid}")
                        record.extend(int(fixed).to_bytes(4, "big", signed=True))
                segments += 1
            state = hashlib.sha256(state + gid.to_bytes(2, "big") + advance.to_bytes(2, "big") + hashlib.sha256(record).digest()).digest()
        glyphs = len(top.charset)
    print(json.dumps({"algorithm": "typaxis.cff-v2-outline-records/1", "source_sha256": digest,
                      "tool": {"name": "FontTools", "version": fontTools.__version__},
                      "glyphs": glyphs, "segments": segments, "different_widths": different_widths, "outline_state_sha256": state.hex(), "source_width_fixed_be_i32_sha256": widths.hexdigest()}, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
