"""Deterministic contextual shaping fixture for repeated-header glyph unions."""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

ROOT = Path(__file__).resolve().parents[6]
SOURCE = ROOT / "samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
font = TTFont(SOURCE, recalcTimestamp=False)
addOpenTypeFeaturesFromString(font, """
languagesystem DFLT dflt;
languagesystem latn dflt;
feature calt { sub f' space i by Z; } calt;
""")
font["head"].created = font["head"].modified = 2082844800
font.save(Path(__file__).with_name("header-context.ttf"))
print({c: font.getGlyphID(c) for c in ["f", "Z"]})
