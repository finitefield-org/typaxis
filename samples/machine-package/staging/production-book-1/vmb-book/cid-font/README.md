# Source-font CID extraction fixture

`generate.py` derives `extraction.ttf` from the repository's semantic-container
test font. Its cmap maps both A and B to the original A glyph. GSUB ligates `fi`
to Z and expands X into D and E. The book-2 test sends the original text
`ABA fi X D E f i` through admission, shaping, line/page selection, display,
actual subset writing and CID/extraction planning. It never fabricates glyph
usage records to simulate the shaping result.

The expected cases are a shared ambiguous CID for `ABA`, one-glyph/two-scalar
`fi`, two-glyph/one-scalar X and ordinary single-scalar mappings for D/E/f/i.
Original Harano and installed system font files are unchanged. This asset is
only a font fixture, not a public production-book-2 protocol registration.

Regenerate with `python3 generate.py` (fontTools required).
