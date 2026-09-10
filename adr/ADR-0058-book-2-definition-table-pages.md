# ADR-0058: Place definition tables on stable physical pages

## Status

Accepted incrementally under design 28 on 2026-09-10. Private page selection,
placement, source closure and actual PDF generation now support definition tables.
Remaining body/page forms and full-book/public/managed-host/author/human acceptance
remain required.

## Decision

Prepare body and definition table contexts from one immutable source hierarchy and
one cumulative record/work budget. Scope body searches and completion to the body
preorder prefix. Keep global table identities and definition-local item identities;
never flatten definition tables into the body's completion range. Preserve the
existing body-only preparation and frozen production kernel when no definition
tables exist. A flat-only footnote search still refuses mixed content.

Feed retained mixed selections through physical page convergence and the common
leaf placement routine. Place ordinary parts at their selected offsets and table
leaves at their actual parallel-cell geometry, preserving definition, cell, caption
and repeated-header identities. Nonpainting forced parts consume original source
without creating glyph fragments. Empty roots can make source progress at zero
height. Source closure consumes every referenced original item exactly once,
including forced breaks; repeated header leaves require an original visit.

Physical stability compares full definition continuation, including root ordinal,
marker state and parallel-cell cursor fingerprints, as well as selected source
parts and geometry. Arena allocation identities do not define semantic stability.
Generate the original definition label only from its original marked leaf. Repeated
header fragments retain artifact roles without emitting another note label,
reference annotation or original source occurrence.

## Evidence

Design 28 §14.192 and the progress ledger record joint body/definition tables,
dependency backtracking, the admitted definition matrix, unreferenced definitions,
exact/one-short page budgets and actual private driver PDFs. The original Harano
font produces a four-page combined fixture. An independent PDF checker derives
columns and baselines from declared geometry and font metrics, checks first labels
and annotation counts, and rejects source/position/repetition mutations. Earlier
332 PDFs and both original-Harano VMB table PDFs remain byte-identical. This is
physical fixture evidence, not full-book or public-profile acceptance.
