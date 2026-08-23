# Display List契約

Display ListはPDF非依存。logical font/image ID、typed paint、dimensioned transform、cluster-level text mappingを持つ。PDF name、CID、object number、raw action dictionaryは禁止。

command:

- save / restore
- concat_transform
- clip_path
- fill_path
- stroke_path
- draw_glyph_run
- draw_image

linkは描画commandではなくpage annotation collection。path verbはmove/line=1 point、curve=3 points、close=0 point。paint/clip対象pathは`move_to`で始まり、少なくとも1本のlineまたはcurveを持つ。save/restoreはpageごとにbalanceする。

GlyphRunはTextSpanとclustersを持ち、clusterがUnicode所有単位。cluster内の複数glyphへ同じUnicode列を複製しない。
