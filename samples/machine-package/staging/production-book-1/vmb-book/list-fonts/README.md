# Generated-list test fonts

These diagnostic fonts derive from the repository's existing no-MATH ASCII
TrueType/TTC fixtures. The original files are unchanged. They test admitted-font
selection, real glyph advances, generated text, subset embedding and label paint;
they do not establish Japanese-font or full-book typographic acceptance.

- `body-list-no-math.ttf` and `collection-list-no-math.ttc` append a nonempty
  quadratic U+2022 bullet, keeping every original ASCII glyph ID, advance and
  outline. The original ASCII outlines are empty, so these two fonts alone must
  not be used as evidence that decimal labels or body text visibly render.
- `body-list-visible.ttf` also replaces the empty outlines of digits 0–9 and the
  period with diagnostic seven-segment/dot outlines authored in the generator.
  It keeps the original ASCII glyph IDs and advances. The independent PDF probes
  use this font for labels and the existing visible CFF fixture for body A/B.
- All three fonts omit MATH. The TTC has one face, selected with face index 0.
  The original unmodified `body.ttf` remains the missing-bullet negative input.

Reproduce from the repository root with Python and **fontTools 4.51.0**:

```sh
python3 tools/generate_production_list_fonts.py
```

`fixture-index.json` records source/output hashes and whether visible decimal
outlines were added. FontTools normalizes the source fixtures' low head timestamps
when loading; `recalcTimestamp=False` prevents run-time timestamps. Repeated
serialization must reproduce all file hashes. The generator dependency is for
fixture maintenance only; Rust tests consume the checked-in bytes.

The main font fixture uses its repository license. The additional test outlines
are authored in this repository, with no system/publication font copied into the
fixture. The separate VMB engine/font notices continue to apply to the math SVG
fixtures, not to these diagnostic label fonts.
