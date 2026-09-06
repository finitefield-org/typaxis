#!/usr/bin/env python3
"""Extend the repository's ASCII-only no-MATH test fonts with a real bullet."""
import hashlib
import json
from pathlib import Path
from fontTools import version
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont, TTCollection

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "samples/machine-package/profiles/basic-document-1/combined/job"
OUTPUT = ROOT / "samples/machine-package/staging/production-book-1/vmb-book/list-fonts"


def add_bullet(font):
    font.recalcTimestamp = False
    assert 0x2022 not in font.getBestCmap()
    assert "MATH" not in font
    order = list(font.getGlyphOrder())
    pen = TTGlyphPen(None)
    # Quadratic circle: visible, asymmetric with respect to baseline and distinct
    # from a whitespace fallback. Existing ASCII glyph IDs/widths are unchanged.
    pen.moveTo((400, 350))
    pen.qCurveTo((400, 500), (250, 500))
    pen.qCurveTo((100, 500), (100, 350))
    pen.qCurveTo((100, 200), (250, 200))
    pen.qCurveTo((400, 200), (400, 350))
    pen.closePath()
    font["glyf"]["bullet"] = pen.glyph()
    font["hmtx"].metrics["bullet"] = (500, 100)
    font.setGlyphOrder(order + ["bullet"])
    for table in font["cmap"].tables:
        if table.isUnicode():
            table.cmap[0x2022] = "bullet"


def visible_markers(font):
    # Diagnostic outlines authored here: seven-segment digits, not a substitute
    # for publication-font comparison. hmtx and preexisting glyph IDs stay fixed.
    segments=[(80,650,380,700),(380,370,430,650),(380,70,430,350),
              (80,20,380,70),(30,70,80,350),(30,370,80,650),(80,335,380,385)]
    digits=["012345","12","01643","01236","1256","02536","023456","012","0123456","012356"]
    cmap=font.getBestCmap()
    for char,parts in [(str(i),[segments[int(j)] for j in pattern]) for i,pattern in enumerate(digits)]+[(".",[(80,0,150,70)])]:
        pen=TTGlyphPen(None)
        for left,bottom,right,top in parts:
            pen.moveTo((left,bottom));pen.lineTo((right,bottom));pen.lineTo((right,top));pen.lineTo((left,top));pen.closePath()
        font["glyf"][cmap[ord(char)]]=pen.glyph()


def main():
    assert version == "4.51.0", "fixture generator requires pinned fontTools 4.51.0"
    OUTPUT.mkdir(parents=True, exist_ok=True)
    records = []
    for source, destination in [("body.ttf", "body-list-no-math.ttf"), ("collection.ttc", "collection-list-no-math.ttc"), ("body.ttf", "body-list-visible.ttf")]:
        original = SOURCE / source
        if source.endswith(".ttc"):
            font = TTCollection(original)
            for face in font.fonts:
                add_bullet(face)
        else:
            font = TTFont(original, recalcTimestamp=False)
            add_bullet(font)
        if destination == "body-list-visible.ttf":
            visible_markers(font)
        target = OUTPUT / destination
        font.save(target)
        records.append({"source":str(original.relative_to(ROOT)),"source_sha256":hashlib.sha256(original.read_bytes()).hexdigest(),
                        "file":destination,"sha256":hashlib.sha256(target.read_bytes()).hexdigest(),
                        "visible_decimal_markers":destination == "body-list-visible.ttf"})
    (OUTPUT/"fixture-index.json").write_text(json.dumps({"algorithm":"typaxis.production-list-font-fixture/1", "fonttools":version,
        "changes":"Append U+2022 bullet glyph and cmap entry; preserve original ASCII glyph IDs and advances; no MATH table",
        "cases":records},indent=2)+"\n")


if __name__ == "__main__":
    main()
