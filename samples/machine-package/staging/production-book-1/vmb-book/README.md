# Real VMB book fixtures

`svg/chapter-fraction-equivalence.svg` is an unchanged extraction from the
user-provided VMB chapter package, resource 0. Its supplied bytes include
whitespace before `/>`; do not normalize it. `fixture-index.json` records its
hash and the producer declaration, without inventing an exporter revision.
The TeX comes from text buffer 3 (`source_tex` on node 7).

Source: `vmb-book-fractions-equivalence/v1` in the user-provided VMB repository.
For the supplied book, only this small expression and its SVG output are included;
no complete chapter/book or font binary is copied into this fixture. This regression
input is not a grant to redistribute the complete book or its fonts.

This is currently parser evidence. The supplied generic alt and original producer
metrics are intentionally not adopted as accessibility/placement goldens. The
full design also requires the remaining real formula cases, validated exporter
outputs, chapter/5,000-resource CLI tests, and independent full-book PDF checks.

`engine-v2/` adds 20 conversions of five test expressions (fraction,
parentheses, equivalence, negation and a long sum), rendered by the actual VMB
2.0.0 engine in inline/block mode at two physical font sizes. These are authored
test inputs, not 20 occurrences extracted from the supplied book. The index
retains each original engine SVG, derived Safe-SVG, source/derived hash,
engine/bundle identity, TeX, speech and physical metrics. Font source records and
the bundled license/notice files are retained; no font binary is copied.

Regenerate from the VMB repository's `vmb-core` directory:

```sh
go run ./tools/typaxis-math-fixtures -output NEW_DIRECTORY
```

Two complete generations were byte-identical. The public CLI admission test
checks all 20 conversions and their source/derived hashes. The mixed-text PDF
probe produced 20 corresponding math facts and 20 extracted alternatives, but
its reading order failed the source-order comparison. These files are geometry
and admission fixtures, not approved PDF visual/accessibility goldens. The
formula-only line-start case also remains a required production-layout fix.
