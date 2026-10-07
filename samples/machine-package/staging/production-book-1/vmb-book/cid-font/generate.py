"""Deterministic source-font fixture for real shaping and CID extraction.

Run with Python and fontTools. This is a derived repository test font, not a
replacement for original Harano/system fonts or a public document package.
"""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

ROOT = Path(__file__).resolve().parents[6]
SOURCE = ROOT / "samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
font = TTFont(SOURCE, recalcTimestamp=False)
for table in font["cmap"].tables:
    if table.isUnicode():
        table.cmap[ord("B")] = table.cmap[ord("A")]
addOpenTypeFeaturesFromString(font, """
languagesystem DFLT dflt;
languagesystem latn dflt;
feature ccmp { sub X by D E; } ccmp;
feature liga { sub f i by Z; } liga;
""")
font["head"].created = font["head"].modified = 2082844800
font.save(Path(__file__).with_name("extraction.ttf"))
