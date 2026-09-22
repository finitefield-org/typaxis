# ADR-0122: Retain running-region preparation and measurement record prefixes

## Status

Implemented and locally verified under design 28 on 2026-09-22.
Overall design acceptance remains open.

## Decision

Running-region shaping exposes the shared authored kernel's accepted local
record observation. Inline preparation exposes caller history and each accepted
shape, owner, paragraph and logical-unit reservation. The existing independent
retained-shape addition remains intact. Original APIs delegate to counted paths.

Running-region measurement retains accepted width, selected-line and origin
reservations on all return paths. Selected contexts use the shared counted
capture kernel. The region line budget keeps the largest accepted record prefix
from its individual stages, initial/rebreak preparation, context capture and
feedback retention. Work, pass, error and successful fingerprint behavior remains
unchanged. It does not sum released passes or invent charges for failed atomic
reservations; local shape counts remain stage-local until inline preparation
accepts their addition to caller history.

The PDF region attachment driver recovers record observations when stable lines
were not delivered, alongside the existing candidate-work and begun-pass recovery.
Successful callbacks keep their original owner accounting and resource identity.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20260922/page-region-record-budget/`.
TrueType and unchanged original Harano fixtures exercise each inline/convergence
record boundary, exact capacity, zero candidate work, indentation and height
rejection, missing glyphs, invalid identity, context capture and retries. The
first failed running-region attempt is independently reconstructed and compared
with both attachment and the complete PDF driver. All 23 focused region tests
passed. Final workspace checking, 286 Book /2 CLI tests, 27 shaping tests plus
one doc-test, 69 layout tests plus one doc-test, 104 pagination tests, 76 font
tests and public/legacy CFF diagnostics passed. Independent verification
accepted 759 PDFs / 2,896 pages and 58 subsets; all 759 PDFs were byte-identical
to stage 258. The correspondence report binds 163 frozen sources and 18
successful logs.

## Remaining scope

This exposes existing accepted stage reservations, not a complete command-wide
allocation/work ledger. Source-flow construction, admission, font instances,
backend/context temporary storage and unmetered work remain separate work.
Remaining named-page/column support, public Book /2, original whole-book output,
managed-host/performance and author/visual acceptance remain open. Missing
Speech/SemanticRef remains user-confirmed uncreated.
