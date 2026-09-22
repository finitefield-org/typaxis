# ADR-0120: Preserve frame and selected-line reservations on failed construction

## Status

Implemented and locally verified under design 28 on 2026-09-22.
Overall design acceptance remains open.

## Decision

The shared frame kernel exposes each accepted storage preflight before source
scope, frame geometry or allocation can fail. The original entry points delegate
to the same implementation. Book /2 adds a counted body-frame constructor.

Body line measurement observes initial frame reservations, retained original and
reprojected table frames, block-width storage, source-unit origins, width bindings
and selected-line records even when no line owner can be returned. Work and
record observations are committed before propagating the original typed error.
Rejected reservations contribute nothing; a later failure retains earlier
accepted reservations. Binding preflight and selected-line projection describe
the same storage, so their observations combine by maximum rather than addition.

The body-line budget keeps its existing high-water semantics across passes and
attempts. Existing seed capture and ordinary/header drivers inherit these failure
observations. Single/set replay already prepays the complete successful rebuild
bound and does not charge it again. Successful fingerprints and owner charges,
validation order and candidate-work semantics are unchanged.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20260922/frame-line-record-budget/`.
Focused tests use actual TrueType and unchanged original Harano fonts. They cover
frame geometry rejection, zero candidate work, partial width binding, invalid
source starts, foreign assignments, table/block projection failures and retries.
The two new focused tests and three extended body-budget tests passed. Final
workspace checking, 282 Book /2 CLI tests, 69 layout tests plus one doc-test,
104 pagination tests, 76 font tests and public/legacy CFF diagnostics passed.
Independent verification accepted 759 PDFs / 2,896 pages and 58 subsets; all
759 PDFs were byte-identical to stage 256. The correspondence report binds
149 frozen sources and 17 successful logs.

## Remaining scope

This exposes existing reservations; it does not claim complete allocation or
work measurement inside shaping, source preparation or line-break kernels.
Complete command budgeting, remaining named-page/column support, public Book /2,
original whole-book output, managed-host/performance and author/visual acceptance
remain open. Speech/SemanticRef is user-confirmed uncreated.
