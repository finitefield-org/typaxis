# Repeated-header contextual font fixture

This deterministic font derives from the repository's semantic-container test
font. `generate.py` uses FontTools and adds a contextual GSUB rule: in `f i`,
substitute `f` (GID 71) with `Z` (GID 59). A retained legal line boundary after
`f ` removes the right-hand context and preserves GID 71. The source text remains
unchanged. This is a test font; original Harano and system fonts are not modified.

The header resource tests use the ordinary and retained-boundary line graphs in
both directions, in body and footnote tables. In the split direction, GID 71 is
absent from every original text paint and appears only in repeated header paint.
The selected glyph union, subset program, CID mapping and exact extraction must
retain it. Run `python3 generate.py` in a Python environment with FontTools.

Generated font: 1,308 bytes, SHA-256
`1b9440f5d251569d2d80854f60de0f7a26a38d7e708ac9c433ade457e3cc5aeb`. Regeneration is byte-identical.
