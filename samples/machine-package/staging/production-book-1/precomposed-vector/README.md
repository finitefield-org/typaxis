# VMB precomposed-vector corpus

This directory is the checked-in producer-interface corpus for `MI4-V01` and
the canonical private contract-1.4 Wire fixture added by `MI4-V03`. The
`document-package.json` fixture is accepted only by the private 1.4 decoder and
Schema; it must not be exposed by the public CLI or current 1.3 aliases.

The corpus has four canonical ledgers, one private Wire fixture, two canonical
selected-layout traces, and one canonical Display trace:

- `resources.tsv` records dense logical image IDs, contained SVG paths,
  hash-derived production URIs, exact SHA-256 values, and producer provenance.
- `cases.tsv` binds semantic cases to resources, exact TeX files, alternative
  text, nullable/forbidden actual text, language intent, fixed-point metrics,
  spacing, optional equation numbers, and coverage categories.
- `fragments.tsv` binds corpus-only document fragments to ordered case
  occurrences and optional line/block fit context. A fragment placement is
  written as `{case-id}`; this notation and its context fields are test
  evidence and are not a Typaxis source syntax.
- `negative.tsv` binds rejected SVG fixtures under `negative-svg/` to the
  closed Safe-SVG 2 `R7100` reason vocabulary. These files are parser test
  inputs and are never referenced by the document package.
- `document-package.json` and `input.tsf` form the canonical strict-Wire/JCS
  fixture covering all four vector kinds, nullable actual text, source TeX,
  spacing, equation-number child shape, and one `svg-safe-2` provenance record.
  They do not authorize resource admission, layout, or PDF generation.
- `inline-layout-trace.json` records the selected atomic inline layout from
  `MI4-V09`. `block-layout-trace.json` records the selected atomic block layout
  from `MI4-V11`, including page/block/paint ordinals, effective spacing,
  viewport matrices, formula baselines, a separately placed equation number,
  structure-child source order, and the one-time fragment charge.
- `display-v2.json` records the `MI4-V12` DrawVector `/2` closure. Its four
  commands cover all vector kinds, are sorted by `(page_index, paint_ordinal)`,
  use dense usage IDs, and reuse one component-wise `VectorContentKey` across
  inline/block and Figure/Formula placements. The Rust fixture also admits an
  unused, distinct vector resource and proves that it emits no command and
  cannot be substituted for the selected content key.

`resources.tsv` is joined to `cases.tsv` by `image_id`; the repeated
`expected_sha256` must match on both sides. An image ID is a dense logical ID,
while `uri` is derived from the full SVG SHA-256. Distinct IDs may therefore
share one URI and byte sequence. `engine_id`, `engine_version`, and
`rules_version` are producer assertions bound through that join; the corpus
gate records them but does not execute the named engine.

All metric and spacing fields use the package coordinate unit
`pdf_point_1_65536`. `advance`, `ascent`, `descent`, `origin_x`, `baseline`,
`viewport_width`, and `viewport_height` are required for `math_vector`,
`inline_vector`, and `math_vector_block`. `vector_figure` instead carries only
the two viewport fields. Full metric rows must satisfy:

```text
advance > 0
ascent > 0
descent >= 0
viewport_width > 0
viewport_height > 0
0 <= baseline <= viewport_height
ascent >= baseline
descent >= viewport_height - baseline
```

`origin_x + viewport_width` must also fit in a signed 64-bit integer, and the
viewport must be a single-scale round-half-to-even result from the SVG root
geometry. `baseline` is the downward distance from the viewport top to the
formula baseline; `origin_x` is the signed distance from the inline pen origin
to the viewport left. The intended placement equations are therefore:

```text
viewport_left = pen_x + origin_x
viewport_top = line_baseline_y - baseline
line_baseline_y = viewport_top + baseline
```

`advance`, rather than the viewport bounding width, is the atomic inline
occupancy used by line breaking. Inline spacing is nonnegative. Block spacing
is deliberately `-` because its owner is the future typed style contract. An
equation number and its positive `minimum_gap` are either both present on
`math_vector_block` or both `-`.

For math kinds, `source_tex_path` is required and `actual_text = -` means the
authored `alt` is the resolved fallback. Generic vector kinds require both
`source_tex_path` and `actual_text` to be `-`. `language = inherit` records
document-language inheritance; every other value is an explicit override.
The ordered `cases` list in `fragments.tsv` assigns a placement ordinal to
each marker, including repeated markers. Its optional
`inline_remaining_width`, `block_frame_width`, `block_remaining_height`, and
`next_empty_frame_height` fields use the same fixed-point unit. The line-end
fixture binds remaining width to `advance + spacing.before` and excludes the
positive `spacing.after` at the selected line end. The page-end fixture binds
a 450pt block to a 480pt frame, a 16pt current-page remainder, and a 600pt
empty next frame.

All ledgers use UTF-8, LF, a final LF, one exact header, tab-separated fields,
and canonical row order. Comma-separated lists are nonempty, unique, and
UTF-8 byte sorted unless occurrence order is explicitly part of the field.
`-` means null or inapplicable according to the case kind. Every referenced
file is a contained regular file below this directory. TeX files include their
final LF as part of the exact opaque source bytes.

The SVG files are already lowered by the producer boundary: glyph-like
geometry is expanded into the Safe-SVG 2 path/shape subset. They intentionally
contain no `use`, text/font nodes, CSS, script, animation, image, or external
reference. Typaxis must validate and place these bytes; it must not add a VMB
preprocessor or fall back to native math, PNG, or omitted content.

Both selected-layout trace files are canonical JCS and are validated against
the private 1.4 layout-trace schema. The block trace deliberately starts with a
partly consumed page: the Figure and its kept caption fit there, while the
numbered math block moves intact to the next page. Its `pagination_bounds`,
`paint_bounds`, and `structure_bounds` are identical; the producer viewport
remains unchanged; and the `formula` child precedes the independent
`equation_number` child. The Display trace is canonical JCS and is validated
against the private 1.4 display-list schema. It contains no resource URI, raw
SVG, source TeX, PDF object/name, or MCID; those facts remain reachable only
through sealed binding and selected-placement fingerprints.
