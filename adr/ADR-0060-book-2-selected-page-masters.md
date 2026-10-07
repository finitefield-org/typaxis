# ADR-0060: Use each selected physical page master in PDF geometry

## Status

Accepted incrementally under design 28 on 2026-09-10. Source-bound master selection
and private PDF page geometry are connected. ADR-0061 subsequently connects changing
same-width body/footnote heights and vertical origins. Horizontal reflow, authored
named-page transitions, running regions and columns still require integration.
Full-book/public/managed-host/author/human gates remain open.

## Decision

Share the existing page rule priority between typed style selection and the
successor source query: named-page specificity, first-page specificity, parity
specificity, then later source order. Parity uses the one-based physical page
number; first refers to physical page index zero. Select the original base and
advanced records by their canonical master identities. The allocation-free source
query borrows its exact styled body, charges each rule and binary-search visit to
the caller's cumulative work owner, and rejects page/work limits. It grants no
layout or paint authority.

The private driver obtains the initial body rectangle from the first actual
choice. PDF assembly selects and retains a bounded choice for every materialized
physical page. Require its body rectangle to equal the actual line/frame receipt;
do not substitute differently measured content. Running header/footer content and
columns retain their explicit guard. At this stage mixed-note frame preparation requires its single-master contract.
ADR-0061 preserves that component API and adds an explicit plan-aware path whose
layout owners consume the selected body and footnote geometry.

Encode MediaBox, TrimBox and the page-content Y transform using that page's chosen
master. Annotation rectangles use the source page's height. Internal destinations,
name-tree entries and outline targets use the destination page's height, even when
it differs from the annotation's page. Retain original navigation X/Y, target page,
source text and generated page-reference feedback. The normal convergence loop
must still prove complete stable output before invoking the PDF callback.

## Evidence

Design 28 §14.194 records first/even/default choices, source-only named priority,
later-rule ties, exact/one-short selection work, page overflow, unused masters and
refusal to reuse a different body frame. Three-page synthetic and unchanged-Harano
PDFs contain cyclic forward/back page references across different paper sizes and
asymmetric trims. Independent checks derive choices from original rules and verify
page boxes, stream transforms, annotations, destinations, structure and references;
mutations include another page's boxes and an incorrect destination-page height.
All preceding 396 PDFs and both saved original-Harano VMB table outputs remain
byte-identical. This does not certify changing-width reflow or full-book acceptance.
