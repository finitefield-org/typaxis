# ADR-0088: Close resource uses from actual repeated-header displays

## Status

Accepted incrementally under design 28 on 2026-09-10. Variant-bearing displays
can select glyphs/images, close and write font programs and plan CID extraction.
Assembled variant PDFs and automatic header catalogs remain pending.

## Decision

Separate the resource-selection identity check from the current PDF admission
check. `verify_resource_selection` verifies the immutable display's exact admitted
ledger and limits. The display constructor has already verified every actual
header flow/shape/vector binding against that ledger and those limits. Resource
selection uses this check; the PDF consumer check `verify_resources` retains
PendingHeaderVariants until PDF integration and independent acceptance.

Keep glyph selection driven by actual ordered paint/font-use views. Every use
must have the base table fingerprint; shape and native tables are independently
constructed from the same complete admitted face list, so their dense instance
IDs have the same meaning. Do not merge independently numbered instance tables
by numeric ID. Retain the table identity guard, exact font-capacity bound and all
existing reservation, sorting, hashing and cumulative budget behavior.

Selection unions original glyph IDs across original and repeated occurrences.
Font closure, program writing and CID planning retain each actual use's source,
text, scalar and paint/slot position. Image selection deduplicates admitted
payloads while retaining every physical occurrence. Repetition changes use counts,
not semantic source consumption or the admitted font/image identities.

## Evidence

Four tests cover sixteen native/vector body/note split/join displays: eight
controlled ordinary-font cases, four original Harano CFF cases, and four actual
contextual-GSUB cases. The contextual fixture substitutes f (GID 71) with Z
(GID 59) before space+i; a legal retained line boundary after f+space removes that
context. In the split direction, GID 71 is absent from every original text paint
and occurs only in repeated header text. Selection, closure, subset and CID
extraction must all retain it. The generated repository test font is separate from
unchanged original Harano and system fonts.

Check actual glyph unions, exact instance tables and admitted-font pointers,
closure/program mappings, advances, occurrence-local extraction, image payload
sharing and all physical uses. Exact cumulative work/records/spool limits pass;
one short fails; larger prior work shifts counters without changing fingerprints
or program hashes. PDF resource admission remains explicitly rejected.

Test-only probes export original admitted bytes, generated programs, dense
mappings and actual CID uses. `tools/verify_book_v2_header_resources.py` uses
FontTools to independently compare every mapped outline and horizontal metric,
CFF cmap retention, CID widths and Unicode ambiguity/extraction. Embedded TrueType
programs intentionally omit cmap; their CID mapping and extraction are checked.
The initial checker incorrectly required a TrueType cmap and was corrected to
match the existing embedded-program contract. Hash/selected-glyph/dense-map tamper
cases must be rejected. Detailed evidence and full regression results are in the
progress ledger §14.222.

These probes are resource acceptance, not assembled PDFs or a successful public
profile. Automatic driver catalogs/width convergence, variant PDF consumers,
raster headers, remaining named scopes/running content/columns, full-book/public/
manifests/managed-host/scaling and actual author/human acceptance remain open.
