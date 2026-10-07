#!/usr/bin/env python3
"""Independently verify actual header resource probes with FontTools.

This checks font programs and CID extraction, not assembled PDFs or public
book acceptance. Probe inputs are written by the staging integration tests.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.ttLib import TTFont


def outline(font, name):
    glyphs = font.getGlyphSet()
    pen = DecomposingRecordingPen(glyphs)
    glyphs[name].draw(pen)
    return pen.value


def verify(root, facts):
    uses = facts["uses"]
    totals = [0, 0]
    for index, info in enumerate(facts["fonts"]):
        paths = [root / info[k] for k in ("source", "subset")]
        assert all(p.parent == root and p.is_file() for p in paths)
        for kind, path in zip(("source", "subset"), paths):
            assert hashlib.sha256(path.read_bytes()).hexdigest() == info[kind + "_sha256"]
        rows = info["mapping"]
        assert rows and rows[0] == [0, 0]
        assert rows == sorted(rows) and len({r[0] for r in rows}) == len(rows)
        assert [r[1] for r in rows] == list(range(len(rows)))
        mapping = dict(rows)
        selected = info["selected"]
        assert selected == sorted(set(selected)) and 0 not in selected
        expected = {g for u in uses if u["font"] == index for g in u["gids"]}
        assert set(selected) == expected
        assert expected <= mapping.keys()
        if facts["repeated_only_gid"] is not None:
            assert facts["repeated_only_gid"] in selected
        bindings = info["bindings"]
        assert [b["gid"] for b in bindings] == selected
        assert [b["cid"] for b in bindings] == list(range(1, len(bindings) + 1))
        claims = {}
        for usage in uses:
            if usage["font"] == index and len(usage["gids"]) == len(usage["text"]) == 1:
                claims.setdefault(usage["gids"][0], set()).add(usage["text"])
        with TTFont(paths[0]) as original, TTFont(paths[1]) as subset:
            cff = "CFF " in original
            assert ("CFF " in subset) == cff
            totals[int(cff)] += 1
            assert subset["maxp"].numGlyphs == len(rows)
            assert original["head"].unitsPerEm == subset["head"].unitsPerEm
            source_order, subset_order = original.getGlyphOrder(), subset.getGlyphOrder()
            for gid, dense in rows:
                a, b = source_order[gid], subset_order[dense]
                assert outline(original, a) == outline(subset, b), (gid, dense, "outline")
                assert original["hmtx"].metrics[a] == subset["hmtx"].metrics[b], (gid, "metrics")
            for binding in bindings:
                gid = binding["gid"]
                assert binding["subset"] == mapping[gid]
                if cff:
                    assert binding["cid"] == binding["subset"]
                units = original["head"].unitsPerEm
                advance = original["hmtx"].metrics[source_order[gid]][0]
                assert binding["width"] == (advance * 1000 + units // 2) // units
                scalars = claims.get(gid, set())
                assert binding["unicode"] == (next(iter(scalars)) if len(scalars) == 1 else None)
            # Embedded TrueType programs intentionally omit cmap: the PDF CID
            # map and occurrence extraction above provide the text bindings.
            if cff or "cmap" in subset:
                expected_cmap = {c: mapping[original.getGlyphID(n)] for c, n in original.getBestCmap().items()
                                 if original.getGlyphID(n) in mapping and original.getGlyphID(n) != 0}
                actual_cmap = {c: subset.getGlyphID(n) for c, n in subset.getBestCmap().items()}
                assert actual_cmap == expected_cmap
    for usage in uses:
        bindings = facts["fonts"][usage["font"]]["bindings"]
        assert len(usage["cids"]) == len(usage["gids"])
        extracted = ""
        for cid, gid in zip(usage["cids"], usage["gids"]):
            assert cid > 0
            binding = bindings[cid - 1]
            assert binding["gid"] == gid
            extracted += binding["unicode"] or ""
        if extracted != usage["text"]:
            assert usage["actual_text"] == usage["text"]
            extracted = usage["actual_text"]
        else:
            assert usage["actual_text"] is None
        assert extracted == usage["text"]
    return totals, sum(len(f["mapping"]) for f in facts["fonts"]), len(uses)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("probes", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = args.probes.resolve()
    paths = sorted(root.glob("*.json"))
    assert paths, "no probes"
    totals, glyphs, uses, rejections, repeated = [0, 0], 0, 0, 0, 0
    for path in paths:
        facts = json.loads(path.read_text())
        fonts, gs, us = verify(root, facts)
        totals = [a + b for a, b in zip(totals, fonts)]
        glyphs += gs
        uses += us
        repeated += facts["repeated_only_gid"] is not None
        if args.self_test:
            for mode in range(3):
                bad = copy.deepcopy(facts)
                if mode == 0:
                    bad["fonts"][0]["subset_sha256"] = "0" * 64
                elif mode == 1:
                    bad["fonts"][0]["selected"].pop()
                else:
                    bad["fonts"][0]["mapping"][0][1] = 1
                try:
                    verify(root, bad)
                except AssertionError:
                    rejections += 1
                else:
                    raise AssertionError((path.name, mode, "tamper accepted"))
    print(f"PASS: {len(paths)} variant displays, {totals[0]} TrueType/{totals[1]} CFF subsets, "
          f"{glyphs} mapped glyphs, {uses} CID uses, {repeated} repeated-only glyph cases, {rejections} tamper rejections")


if __name__ == "__main__":
    main()
