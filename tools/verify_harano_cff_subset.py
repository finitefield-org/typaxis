#!/usr/bin/env python3
"""Independently compare every selected outline, advance and retained UVS."""
import argparse
import hashlib
import json
from pathlib import Path

import fontTools
from fontTools.pens.recordingPen import RecordingPen
from fontTools.ttLib import TTFont

EXPECTED = "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"


def outlines(charstring):
    pen = RecordingPen()
    charstring.draw(pen)
    data = bytearray()
    for operation, points in pen.value:
        symbol, count = {"moveTo": (b"M", 1), "lineTo": (b"L", 1), "curveTo": (b"C", 3), "closePath": (b"Z", 0)}[operation]
        assert len(points) == count
        data.extend(symbol)
        for point in points:
            for coordinate in point:
                fixed = coordinate * 65536
                assert int(fixed) == fixed
                data.extend(int(fixed).to_bytes(4, "big", signed=True))
    return bytes(data)


def uv_map(font):
    base = font.getBestCmap() or {}
    result = {}
    for table in font["cmap"].tables:
        if table.format != 14:
            continue
        for selector, pairs in table.uvsDict.items():
            for scalar, name in pairs:
                name = base.get(scalar) if name is None else name
                if name is not None:
                    result[selector, scalar] = font.getGlyphID(name)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("subset", type=Path)
    parser.add_argument("mapping", type=Path, help="source-GID dense-GID lines emitted by the Rust test")
    args = parser.parse_args()
    source_hash = hashlib.sha256(args.source.read_bytes()).hexdigest()
    if source_hash != EXPECTED:
        parser.error("requires unchanged original HaranoAjiMincho-Regular.otf")
    rows = [tuple(map(int, line.split())) for line in args.mapping.read_text().splitlines()]
    assert all(len(row) == 2 for row in rows)
    assert rows == sorted(rows) and rows[0] == (0, 0)
    assert len({a for a, _ in rows}) == len(rows)
    assert [b for _, b in rows] == list(range(len(rows)))
    mapping = dict(rows)
    state = hashlib.sha256()
    with TTFont(args.source) as source, TTFont(args.subset) as output:
        assert output["maxp"].numGlyphs == len(rows)
        original = source["CFF "].cff.topDictIndex[0]
        subset = output["CFF "].cff.topDictIndex[0]
        assert len(subset.FDArray) == 1 and len(subset.GlobalSubrs) == 0
        assert not getattr(subset.FDArray[0].Private, "Subrs", [])
        assert all(fd == 0 for fd in subset.FDSelect.gidArray)
        cids = [0 if name == ".notdef" else int(name.removeprefix("cid")) for name in subset.charset]
        assert cids == list(range(len(rows)))
        different_source_widths = 0
        used_fds = set()
        for source_gid, dense in rows:
            a = original.charset[source_gid]
            b = subset.charset[dense]
            original_outline = outlines(original.CharStrings[a])
            output_outline = outlines(subset.CharStrings[b])
            assert output_outline == original_outline, (source_gid, dense, "outline mismatch")
            advance, bearing = source["hmtx"].metrics[a]
            assert output["hmtx"].metrics[b] == (advance, bearing)
            assert subset.CharStrings[b].width == advance, (source_gid, "CFF/hmtx width mismatch")
            different_source_widths += original.CharStrings[a].width != advance
            used_fds.add(original.FDSelect.gidArray[source_gid])
            state.update(source_gid.to_bytes(2, "big") + dense.to_bytes(2, "big") + advance.to_bytes(2, "big") + original_outline)
        expected_base = {scalar: mapping[source.getGlyphID(name)] for scalar, name in source.getBestCmap().items() if source.getGlyphID(name) in mapping and source.getGlyphID(name) != 0}
        actual_base = {scalar: output.getGlyphID(name) for scalar, name in (output.getBestCmap() or {}).items()}
        assert actual_base == expected_base
        expected_uvs = {pair: mapping[gid] for pair, gid in uv_map(source).items() if gid in mapping and gid != 0}
        assert uv_map(output) == expected_uvs
        assert source["head"].unitsPerEm == output["head"].unitsPerEm == 1000
        assert source["OS/2"].fsType == output["OS/2"].fsType
        facts = {"algorithm": "typaxis.cff-v2-selected-subset-verification/1", "tool": {"name": "FontTools", "version": fontTools.__version__},
                 "source_sha256": source_hash, "subset_sha256": hashlib.sha256(args.subset.read_bytes()).hexdigest(),
                 "subset_bytes": args.subset.stat().st_size, "glyphs": len(rows), "source_fds": sorted(used_fds),
                 "source_width_disagreements": different_source_widths, "base_mappings": len(expected_base), "variation_pairs": len(expected_uvs),
                 "selected_outline_mapping_sha256": state.hexdigest()}
    print(json.dumps(facts, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
