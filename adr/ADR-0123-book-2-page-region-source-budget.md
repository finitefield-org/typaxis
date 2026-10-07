# ADR-0123: Preserve running-region source construction reservations

## Status

Implemented and locally verified under design 28 on 2026-09-22.
Overall design acceptance remains open.

## Decision

Expose `prepare_book_v2_page_region_text_flow_counted` and retain the original
entry point as a delegate. Initialize the observation with caller history before
identity checks. Publish each accepted owner and paragraph/inline reservation
before subsequent reservations, style lowering and source collection. A rejected
atomic reservation leaves the accepted prefix intact. The returned owner and
fingerprint on success remain unchanged.

The running-region attachment driver recovers this prefix before propagating a
source error. Apply its existing conservative source-record work reservation to
the recovered increment. Accept it only when it fits the work allowance; an
unsuccessful reservation consumes no work. Preserve the original source error
when both source preparation and this work reservation fail. Successful source
preparation retains the existing work-limit error precedence.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20260922/page-region-source-budget/`.
TrueType and unchanged original Harano tests exercise all source reservation
boundaries, exact capacity, overflow, retries, missing style and invalid identity
or absent regions. Attachment tests compare independently reconstructed source
failures, accepted records, work and untouched line-pass counts near the record
ceiling, both with sufficient work and without any remaining source work.

Verification is scoped to this source/attachment change: workspace checking,
syntax tests, running-region CLI tests and independent running-region PDF/source/
font verification with byte comparison against stage 259. Stage 259's broader
759-PDF regression is prior evidence and is not claimed as a new run here.
All 25 running-region tests, 138 syntax tests and 12 syntax doc-tests passed;
workspace checking passed. Independent validation accepted three region PDFs /
ten pages, 126 glyph paints and 31 resource/PDF/source tamper rejections.
All three PDFs are byte-identical to stage 259. The verification report records
140 source hashes and five logs, including both preliminary focused runs.

## Remaining scope

These are existing conservative reservations, not actual source-engine work or a
complete command allocation/spool ledger. Native math, admission, backend
temporary storage, remaining named pages/columns, public Book /2, whole-book and
managed-host/performance/author acceptance remain open. User-confirmed missing
Speech/SemanticRef remains uncreated.
