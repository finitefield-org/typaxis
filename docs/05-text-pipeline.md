# Text pipeline

```text
TextSpan
 -> grapheme boundaries
 -> bidi paragraph/runs
 -> script/language itemization
 -> cluster-safe font fallback
 -> shaping into glyphs + cluster groups
 -> Unicode line-break candidates
 -> Japanese profile adjustment
 -> break selection
 -> final-line reshape
 -> justification
```

break位置はTextBufferのUTF-8境界であり、shaping cluster内部へ置かない。GlyphRunはvisual glyph orderと、logical TextSpanを所有するcluster groupを分離する。RTLでcluster順がvisualに並んでもsource/text mappingを失わない。

shaping cache keyはfont bytes hash、face index、instance、text bytes、direction、script、language、features、pre/post contextを含む。
