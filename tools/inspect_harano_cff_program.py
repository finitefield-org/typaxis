#!/usr/bin/env python3
"""Independent original-font structural facts; does not rewrite or subset a font."""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

import fontTools
from fontTools.ttLib import TTFont

EXPECTED = "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("font", type=Path)
    args = parser.parse_args()
    source = args.font.read_bytes()
    digest = hashlib.sha256(source).hexdigest()
    if digest != EXPECTED:
        parser.error("input is not the unchanged original HaranoAjiMincho-Regular.otf")
    with TTFont(args.font) as font:
        top = font["CFF "].cff.topDictIndex[0]
        fds = top.FDSelect.gidArray
        cids = [0 if name == ".notdef" else int(name.removeprefix("cid")) for name in top.charset]
        local = [len(getattr(fd.Private, "Subrs", [])) for fd in top.FDArray]
        facts = {
            "algorithm": "typaxis.harano-cff-program-facts/1",
            "tool": {"name": "FontTools", "version": fontTools.__version__},
            "source_sha256": digest,
            "source_bytes": len(source),
            "glyph_count": font["maxp"].numGlyphs,
            "units_per_em": font["head"].unitsPerEm,
            "font_dict_count": len(top.FDArray),
            "fd_usage": dict(sorted(Counter(fds).items())),
            "fd_by_gid_sha256": hashlib.sha256(bytes(fds)).hexdigest(),
            "cid_by_gid_be_u16_sha256": hashlib.sha256(b"".join(cid.to_bytes(2, "big") for cid in cids)).hexdigest(),
            "global_subroutines": len(top.GlobalSubrs),
            "local_subroutines_by_fd": local,
            "declared_subroutines": len(top.GlobalSubrs) + sum(local),
        }
    print(json.dumps(facts, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
