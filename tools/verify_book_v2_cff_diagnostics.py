#!/usr/bin/env python3
"""Check the two original-Harano budget diagnostics using FontTools and source bytes."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from fontTools.ttLib import TTFont

HARANO = "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"


def check(probe, font, data):
    assert probe["font_sha256"] == HARANO
    assert probe["axis"] in ("operations", "segments")
    # Both deliberately tiny allowances fail in the mandatory .notdef glyph.
    assert probe["gid"] == 0
    assert probe["fd"] == font["CFF "].cff.topDictIndex[0].FDSelect.gidArray[0]
    cff = font.reader.tables["CFF "]
    assert 0 <= probe["table_byte"] < cff.length
    assert probe["font_byte"] == cff.offset + probe["table_byte"]
    byte = data[probe["font_byte"]]
    operator = None if byte == 28 or byte >= 32 else (0x0C00 | data[probe["font_byte"] + 1]) if byte == 12 else byte
    assert probe["operator"] == operator
    assert probe["limit"] == 1 and probe["observed"] == 2
    assert probe["source_chain_depth"] == 5
    assert font["OS/2"].fsType == 0
    fields = ["phase=charstring", "reason=budget_exceeded", "class=resource-budget",
              "requested_face_index=0", "table=CFF ", "offset_kind=field",
              f'font_byte={probe["font_byte"]}', f'table_byte={probe["table_byte"]}']
    if operator is not None:
        fields.append(f"cff_operator=0x{operator:04X}")
    fields += ["gid=0", f'fd={probe["fd"]}', "limit=1", "observed=2", "embedding=allowed", "fs_type=0x0000"]
    assert probe["note"] == "; ".join(fields)


def verify(directory, source, self_test):
    data = source.read_bytes()
    assert hashlib.sha256(data).hexdigest() == HARANO
    font = TTFont(source)
    count = rejected = 0
    for axis in ("operations", "segments"):
        probe = json.loads((directory / f"{axis}.json").read_text())
        assert probe["axis"] == axis
        check(probe, font, data)
        count += 1
        if self_test:
            for field, value in [("gid", 1), ("fd", 0), ("font_byte", 0), ("table_byte", 0),
                                 ("operator", 0), ("limit", 2), ("observed", 1), ("note", "lost")]:
                altered = copy.deepcopy(probe)
                altered[field] = value
                try:
                    check(altered, font, data)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError(f"accepted altered {field}")
    font.close()
    return count, rejected


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--font", type=Path, required=True)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    count, rejected = verify(args.directory, args.font, args.self_test)
    print(f"PASS: {count} original-Harano diagnostics / {rejected} alterations rejected; "
          "font hash, table/file offsets, original operators, GID/FD, permission and budget notes verified")
