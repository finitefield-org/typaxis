# ADR-0100: Select a common page scope for parallel table content

## Status

Accepted incrementally under design 28 on 2026-09-20. Focused controlled-font and
unchanged-Harano tests and independent PDF checks pass. User-run full CLI regression
passes all 252 tests; 759 PDFs pass independent checks and all 743 preceding PDFs
remain byte-identical. This does not complete the full-book or public gates.

## Decision

Extend ADR-0062 for a nonempty body table whose actual leaves all resolve to the
same page name. Use that common name for physical table selection and continuation,
even if its enclosing table owner retains a different inherited or explicit name.
Inner explicit scopes already override outer scopes; the table collector must not
reject them solely because its own owner has no content in that inner scope.
Keep the original owner/name registry unchanged. Following body content resumes
its own authored scope through the existing page-boundary and keep rules.

Include captions, original headers, cell leaves and nested-table leaves in the
agreement check. Do not select a first cell's name when other content disagrees.
Empty tables retain their own name; an empty nested table with a different name
still conflicts with the common physical table scope. Report the conflicting leaf
or empty-table owner. Named definition streams remain unsupported.

Only root tables scan complete leaf ranges. Nested nonempty tables read their first
leaf and compare against the parent result, since their entire range was already
checked at the root. This avoids scanning descendants at every nesting depth and
retains the existing bounded name-array allocation and resource accounting.

The resulting selected master goes through existing physical-width feedback,
remeasurement, stable page geometry and PDF assembly. No source declarations,
font data, column widths or page masters are rewritten to obtain acceptance.

## Evidence and limits

Design 28 §14.237 records flat and nested parallel tables, captions, restored body
names, narrower named masters and real line reflow, using controlled TrueType and
the unchanged Harano CFF font. Exact and one-short driver work and original
table-name registry preservation are checked. Conflicting captions and empty
nested tables retain owner-specific errors; previous cell/break/keep/definition
conflict tests still pass.

The dedicated independent verifier checks 16 component/actual-driver PDFs against
original source text, named declarations, physical master selection, font metrics,
column positions and expected line origins. Its scope is these fixtures, not an
independent general paginator or PDF/UA acceptance. Varying names within one table,
named definitions, columns, complete command budgets, full-book/public/managed-host,
performance and author/human acceptance remain open.
