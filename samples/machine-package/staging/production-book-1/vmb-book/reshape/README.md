# Selected-line contextual shaping fixture

`context.ttf` and `generate.py` are original Typaxis test data, distributed under
this repository's MIT OR Apache-2.0 license. No third-party font outlines or
font program were copied. FontTools is the build tool, not a font source.

Regenerate with Python 3.12 and FontTools 4.51.0:

```sh
python3 samples/machine-package/staging/production-book-1/vmb-book/reshape/generate.py
```

SHA-256: `abd89ef5ab470c02abf50090b7063114eae384387c36f339ae2cb85bfd7500d0`.
The font timestamps are fixed; regeneration with the specified toolchain produces
the checked-in bytes. UPEM is 1,000; hhea ascent/descent are 800/-200.

The OpenType `calt` rule substitutes A (GID 2, advance 600) with C (GID 4,
advance 900) when followed by space and B. Space has advance 250 and B has
advance 600. A is a triangle and C is a rectangle, so both glyph identity and
advance reveal whether the shaper received context across the selected line end.

This is deliberately small diagnostic typography, not a substitute for the
unchanged HaranoAji, Japanese bidi, IVS, or whole-book visual gates. Its width
change also gives a real two-state reshape/rebreak oscillation for checking the
finite pass budget; the test never edits glyph advances after shaping.
