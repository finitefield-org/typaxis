# ADR-0103: Retain original positions in legacy CFF glyph finalization

## Status

Accepted incrementally under design 28 on 2026-09-20. User-run regression passes
253 CLI tests, 73 font tests and the separate legacy CLI diagnostic test. Independent
checks pass for 759 PDFs, all byte-identical to the preceding stage, and both legacy
and original-Harano diagnostic probes. All 50 frozen source files match the run.
Progress §240 records the evidence. Overall design acceptance remains open.

## Decision

Retain table-relative program starts alongside the existing /1 copied charstrings,
local subroutines and global subroutines. Compute starts once from the validated
INDEX data start and contiguous object lengths with checked arithmetic/allocation.
This is bounded by existing glyph/subroutine admission limits and provides constant
time location lookup during execution. Retain the validated SFNT table base and
embedding context without retaining source bytes or resource path strings.

Add `prepare_face_detailed` and `subset_detailed` returning `Cff1Failure`. Existing
methods delegate and return the original `Cff1Error`, preserving broad callers.
Use the shared Type2 observation hooks to keep the selected GID and the actual
root/local/global token or end cursor. Name-keyed /1 has no FDSelect, so FD remains
absent. Width overflow, unsupported/reserved instructions and operation/outline
budgets retain typed reasons. Budgets include the actual limit and attempted count.
Failed outlines never enter the cache; consumed work and previously cached glyphs
remain. Successful selection, work accounting, receipts and subset bytes retain
their existing identities.

Selection and output-encoding failures have phase `subset` without speculative
source positions. Detailed encoding-stage locations and complete budget facts for
those failures remain separate work.

Resource finalization and staging text use detailed methods and carry the actual
FontFaceId alongside the failure. CLI finalization and common-body/footnote mapping
preserve the code and bounded context in the failure message and classify input,
budget and internal failures separately. In particular, selected-glyph budget
errors no longer become generic internal resource-finalization failures. This does
not complete structured public diagnostic subjects, resource URI notes or all
public book-2 entry points.

## Evidence

Tests exercise original SFNT positions for six malformed/unsupported/end cases,
exact internal budgets, invalid selections, failed-cache behavior and cumulative
retry work. A synthetic subroutine font is reproducibly generated from the existing
Typaxis fixture; valid evaluation and four checksummed local/global negative copies
verify original program positions. No production font is modified.

The actual admission/layout/display/resource-finalization harness reaches CLI
mapping with two deliberately tiny budgets. A FontTools verifier independently
checks the unchanged fixture's program bytes, offsets, absent FD, notes, codes and
exit status, with 22 altered-probe rejections. Existing CFF PDF/render/extract/manifest
and resource subset checks pass. Progress §240 records exact commands and results.
