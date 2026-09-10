# ADR-0062: Select physical masters from authored body page scopes

## Status

Accepted incrementally under design 28 on 2026-09-10. Same-width named body scopes
now participate in private page selection and PDF convergence. ADR-0063 extends
this to explicit named body-break nodes. Horizontal reflow, conflicting names within
parallel tables, named definition content, running regions, columns and full-book/
public/managed-host/author/human gates remain open.

## Decision

Resolve used page names from the original source-flow Begin/End nesting and typed
computed page properties. An auto child retains its enclosing named region; a nested
explicit name overrides it until its End, then the enclosing name resumes. Record
body-region owner/name associations and a first/even/odd master cache for each used
name. Definition streams do not choose body page names. The plan borrows its exact
styled source and owns bounded name/cache storage; line passes borrow one immutable
plan rather than copying independent caches. Charge source visits, name lookups,
master selection and sorting to cumulative work, and reserve name records/bytes
against remaining command allowances before allocation.

A successor-only style validation/cascade path accepts named table pages. Keep the
old table-1 validator's auto-only rule and its consumers unchanged. The private
source-version-bound rule owner selects the appropriate path; a standalone style
calculation grants no legacy or successor profile authority.

Map actual body items and root tables to the source plan. Stop page enumeration at
a name transition and treat it as a terminal policy boundary for ranking. A first
named item chooses its named master on physical page zero without a synthetic blank
page. Keep-with-next across a changed name is diagnosed like a forced-break conflict.
Retain the current name through table continuation and, after body completion, through
pending-note-only pages. Explicit leading, consecutive and trailing source breaks
retain their original progress contract. The existing input decoder continues to
reject empty semantic containers; this change does not broaden that input contract.

Retain the selected name and its source-plan index with each physical page. Stable
comparison includes the name and actual rectangles. Final PDF selection uses that
name with the existing physical first/parity/source-order priority. Per-page boxes,
stream transforms and cross-page navigation coordinates use the resulting master.
Generated page labels still pass through complete source-to-PDF convergence.

One parallel table cannot silently choose between conflicting cell/nested-table
names. Preserve an owner-specific diagnostic for those cases and for named content
inside a footnote definition. An explicit page property on a standalone forced-break
node now has a source diagnostic rather than being silently dropped; named enclosing
scopes containing ordinary forced breaks are supported. These remaining forms are
not reinterpreted as supported by this incremental decision.

## Evidence

Design 28 §14.196 and its progress ledger record ordinary/named transitions, nested
scope restoration, named first pages, leading/consecutive/trailing forced pages, continuing
body tables, note tails and converged forward/back page labels. Exact/one-short work,
record and name-byte allowances are checked, alongside keep and unsupported-source
diagnostics. Original-Harano reference and nested-scope PDFs provide visible-font
evidence. Independent checks derive names from original body owner scopes and compare
source-specific page sequences, boxes, font metrics, indentation, columns, note
bottoms and annotations. Full-design completion is not claimed.
