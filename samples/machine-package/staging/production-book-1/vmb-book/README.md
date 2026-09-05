# Real VMB book fixtures

`svg/chapter-fraction-equivalence.svg` is an unchanged extraction from the
user-provided VMB chapter package, resource 0. Its supplied bytes include
whitespace before `/>`; do not normalize it. `fixture-index.json` records its
hash and the producer declaration, without inventing an exporter revision.
The TeX comes from text buffer 3 (`source_tex` on node 7).

Source: `vmb-book-fractions-equivalence/v1` in the user-provided VMB repository.
Only this small mathematical expression and its supplied SVG output are included;
no complete chapter/book or font binary is copied into this fixture. This regression
input is not a grant to redistribute the complete book or its fonts.

This is currently parser evidence. The supplied generic alt and original producer
metrics are intentionally not adopted as accessibility/placement goldens. The
full design also requires the remaining real formula cases, validated exporter
outputs, chapter/5,000-resource CLI tests, and independent full-book PDF checks.
