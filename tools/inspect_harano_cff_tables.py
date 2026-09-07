#!/usr/bin/env python3
"""Independent vertical-metric and UVS facts for the unchanged original font."""
import argparse
import hashlib
import json
from pathlib import Path

import fontTools
from fontTools.ttLib import TTFont

EXPECTED = "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("font", type=Path)
    args = parser.parse_args()
    digest = hashlib.sha256(args.font.read_bytes()).hexdigest()
    if digest != EXPECTED:
        parser.error("input is not the unchanged original HaranoAjiMincho-Regular.otf")
    with TTFont(args.font) as font:
        vertical = bytearray()
        vorg = font["VORG"]
        for name in font.getGlyphOrder():
            advance, top = font["vmtx"].metrics[name]
            origin = vorg.VOriginRecords.get(name, vorg.defaultVertOriginY)
            vertical.extend(origin.to_bytes(2, "big", signed=True))
            vertical.extend(advance.to_bytes(2, "big"))
            vertical.extend(top.to_bytes(2, "big", signed=True))
        tables = [t for t in font["cmap"].tables if t.format == 14]
        assert len(tables) == 1
        sequences = tables[0].uvsDict
        base_map = font.getBestCmap()
        base_bytes = bytearray()
        for scalar, name in sorted(base_map.items()):
            base_bytes.extend(scalar.to_bytes(4, "big"))
            base_bytes.extend(font.getGlyphID(name).to_bytes(2, "big"))
        coverage = bytearray()
        resolved = bytearray()
        count = 0
        for selector, values in sorted(sequences.items()):
            for base, name in sorted(values):
                coverage.extend(selector.to_bytes(4, "big"))
                coverage.extend(base.to_bytes(4, "big"))
                coverage.append(int(name is not None))
                coverage.extend((0 if name is None else font.getGlyphID(name)).to_bytes(2, "big"))
                resolved.extend(selector.to_bytes(4, "big"))
                resolved.extend(base.to_bytes(4, "big"))
                resolved.extend(font.getGlyphID(base_map[base] if name is None else name).to_bytes(2, "big"))
                count += 1
        facts = {
            "algorithm": "typaxis.harano-cff-table-facts/1",
            "tool": {"name": "FontTools", "version": fontTools.__version__},
            "source_sha256": digest,
            "glyph_count": font["maxp"].numGlyphs,
            "vertical_metric_records": font["vhea"].numberOfVMetrics,
            "vertical_origin_overrides": len(vorg.VOriginRecords),
            "vertical_origin_advance_bearing_sha256": hashlib.sha256(vertical).hexdigest(),
            "variation_selectors": len(sequences),
            "variation_values": count,
            "variation_coverage_sha256": hashlib.sha256(coverage).hexdigest(),
            "base_mapping_count": len(base_map),
            "base_mapping_sha256": hashlib.sha256(base_bytes).hexdigest(),
            "resolved_variation_sha256": hashlib.sha256(resolved).hexdigest(),
        }
    print(json.dumps(facts, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
