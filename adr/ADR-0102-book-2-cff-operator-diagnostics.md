# ADR-0102: Classify Type2 operator rejection and check hflex negation

## Status

Accepted incrementally under design 28 on 2026-09-20. Focused verification and
user-run full regression pass: 253 CLI tests and 70 font tests, including all
23,060 original Harano glyphs. Independent checks pass for 759 PDFs, all byte-identical
to the preceding stage. Source correspondence and detailed evidence are recorded
in §14.239 and the progress log. Overall design acceptance remains open.

## Decision

Preserve the distinction between an implemented operator with invalid operands,
an opcode defined by Type2 but unsupported by this evaluator, and a reserved
encoding. The opcode classification follows [Adobe Type2 #5177, Appendix A/C,
16 March 2000](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5177.Type2.pdf).
The defined arithmetic, storage and conditional operators, plus deprecated
`dotsection`, remain unsupported. Flex operators remain implemented. Reserved
encodings are not interpreted as CFF2 or obsolete Multiple Master instructions.
This classifies the encountered instruction; it does not certify the unevaluated
remainder or operands of an unsupported program as valid.

Record rejection at actual dispatch, after the existing budget callback and byte
decoding succeed. Do not infer a rejection from a token observed before an earlier
budget, callback or truncated-escape failure. The optional observation hook leaves
legacy CFF /1's broad error enum unchanged. CFF /2 adds typed glyph reasons and
projects them into admitted `FontFailureContext` with the original SFNT/table
position, GID, active FD and embedding status. Reuse `unsupported_cff_operator`;
add `reserved_cff_operator` in the malformed-or-invalid-input class. Successful
execution, work counts, receipts, subsets and publication identities are unchanged.

Replace hflex's unchecked signed negation with `checked_neg`. An unrepresentable
return delta now returns `InvalidCharstring` before either flex cubic is emitted,
in both /1 and /2, instead of panicking in debug or wrapping in release. The current
32-bit fixed-point coordinate domain is preserved.

## Verification and remaining scope

Tests cover all 256 escaped encodings, seven reserved single-byte encodings,
FD-local rejection through a global subroutine, earlier internal/caller failures,
and six signed hflex boundaries. A legacy subset-path test covers the overflow.
Checksummed negative copies of the original Harano font exercise admission through
selected-glyph failure, including exact positions, typed notes and empty cache.
These negative copies do not replace the unchanged-original acceptance tests.

Legacy /1 detailed glyph positions, subset-encoding diagnostics, public CLI notes,
complete command budgets and full-book/public/performance/human gates remain open.
