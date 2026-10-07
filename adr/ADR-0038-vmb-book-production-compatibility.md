# ADR-0038: VMB book production compatibility correction

## Status

Accepted for implementation on 2026-09-05. Implementation and full-book
verification are tracked separately in
[the implementation evidence ledger](../docs/28-vmb-book-production-progress.md).

## Decision

This is the explicit, limited amendment to ADR-0033 / ADR-0037 required by
[the VMB compatibility design](../docs/28-vmb-book-production-compatibility.md).
The supplied VMB chapter uses whitespace immediately before self-closing tag
delimiters. Its multiple paths already belong to the supported geometry subset.

For Safe-SVG 2 only, permit zero or more ASCII SP, TAB, or LF before the `>`
or `/>` delimiter of a start or empty tag. Apply the same lexical policy to
root, group, and geometry tags. Do not permit whitespace inside `/ >` or in
end tags such as `</g >`. Keep Safe-SVG 1 and every other lexical restriction
frozen. In particular this decision does not admit compact path syntax,
entities, DTDs, scripts, external references, or additional SVG commands.

Keep the existing Safe-SVG 2 parser, IR, allocation-charge and resource-profile
identifiers. Previously admitted input retains exactly the same canonical IR,
fingerprint and work charge. A spaced and an unspaced representation of the
same geometry have identical IR/work but different source-byte hashes. Preserve
independent path paint operations, subpaths, fill rules, currentColor inheritance,
clip ownership, and relative/curve geometry. Do not normalize source bytes
before hashing or admission.

Select production-book-1 defaults before configuration merging: 8,192 image
declarations, 262,144 vector nodes, 4,000,000 vector work segments, and depth 32.
Retain legacy/source defaults and existing hard ceilings. Apply file, environment,
and CLI overrides in that order; an explicit old default is still an override.
All aliases retain per-declaration validation/work charging and content-key Form
sharing. Record final effective limits in the existing config/manifest identity.

Keep resource diagnostic context separate from package byte positions. Report
original JSON Pointers for resource-count failures before the frozen carrier is
constructed. Carry typed resource-local reasons, byte spans, element/path/attribute
context and budget counters through the production diagnostic path. Use existing
diagnostic codes and notes; do not add fields to the frozen 1.4 JSON schemas.

Production authorization permits an empty native-math node set: a book can use
precomposed SVG formulas without any typaxis-math source. Retain the nonempty
requirement of the closed native-math slice. Empty production receipts still
bind and recheck the complete package, limits, profile and session. This does
not bypass native-math input validation or font admission. Authorization alone
does not repair the body-font and selected-layout integration described in
section 14 of the design, and is not a successful PDF build claim.

This correction does not extend sfnt-cff1/1 or the 1.4 capability schema. CID-keyed
CFF1 and expanded capabilities require the separate versioned publication in
sections 7 and 9 of the design, with full-book and host verification before release.

## Verification requirements

Test all terminal whitespace forms, V1 rejection, delimiter negatives, root/group
multiple paths, multiple subpaths, decimals and M/L/C/Q/Z at book complexity.
Compare full canonical IR and counters, not just successful parsing. Test profile
isolation, override precedence, inclusive resource limits, precise first-failure
positions, alias charging and out-of-order admission. Run check-package and
build-package on real VMB fixtures, 5,000 placed resources and the single full
book, including independent PDF visual, extraction and structure checks.
Include a production authorization with no native math and preserve rejection
by the closed math slice. The PDF gate additionally requires ordinary text
fonts without MATH, real shaped advances and shared text/vector pagination.

No single parser or config test establishes full-book readiness. See the design
for exporter input requirements and the separate unchanged-Harano font gate.
