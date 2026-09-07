"""Authored test font: contextual A -> C only when followed by space and B."""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

root = Path(__file__).resolve().parent
builder = FontBuilder(1000, isTTF=True)
order = ['.notdef', 'space', 'A', 'B', 'C']
builder.setupGlyphOrder(order)
builder.setupCharacterMap({32: 'space', 65: 'A', 66: 'B', 67: 'C'})
glyphs = {}
for name in order:
    pen = TTGlyphPen(None)
    if name != 'space':
        points = [(50, 0), (300, 700), (550, 0)] if name == 'A' else [(50, 0), (50, 700), (550, 700), (550, 0)]
        pen.moveTo(points[0])
        for point in points[1:]:
            pen.lineTo(point)
        pen.closePath()
    glyphs[name] = pen.glyph()
builder.setupGlyf(glyphs)
builder.setupHorizontalMetrics({name: (250 if name == 'space' else 900 if name == 'C' else 600, 0) for name in order})
builder.setupHorizontalHeader(ascent=800, descent=-200)
builder.setupNameTable({'familyName': 'Typaxis Context Fixture', 'styleName': 'Regular',
                       'uniqueFontIdentifier': 'Typaxis Context Fixture 1',
                       'fullName': 'Typaxis Context Fixture Regular', 'psName': 'TypaxisContextFixture-Regular'})
builder.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200, fsType=0)
builder.setupPost()
builder.setupMaxp()
addOpenTypeFeaturesFromString(builder.font, "languagesystem DFLT dflt; languagesystem latn dflt; feature calt { sub A' space B by C; } calt;")
builder.font['head'].created = builder.font['head'].modified = 3861043200
builder.font.recalcTimestamp = False
builder.save(root/'context.ttf')
