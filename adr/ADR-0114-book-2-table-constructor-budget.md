# ADR-0114: Retain table constructor charges through nested failures

## Status

Implemented incrementally under design 28 on 2026-09-22. Both focused tests,
Book /2 CLI 268 tests, pagination 104 tests, font 76 tests and workspace type
checks pass without failures or ignored tests. Independent verification covers
759 PDFs / 2,896 pages and 58 resource subsets; all 759 PDFs are byte-identical
to the preceding stage. All 94 frozen source hashes and 24 successful logs are
recorded in run-source-correspondence.json. Overall design acceptance remains open.

## Decision

Add `prepare_book_v2_table_search_counted`, returning accepted constructor work
and cumulative record reservations on both success and failure. The existing
entry point delegates to the same implementation. Work starts at zero for this
public entry point; records include the greater of the measurement and caller
prefixes. Initial receipt/index rejection preserves that prefix without adding
work. Rejected reservations are not counted as accepted work or records.

The shared table kernel retains its local ledger while constructing the search.
It copies the ledger into a successful owner and publishes the observations
before propagating a failure. Nested preparation transfers the ledger into each
child, then restores the child's observed counters to the parent before checking
the result. This retains completed siblings and partial recursive construction;
it cannot substitute the temporary zero ledger for actual consumed counters.

Successful construction, error precedence and fragment fingerprints use the
same algorithms and reservations as before. The legacy production wrapper also
delegates to the shared kernel implementation.

## Verification

Controlled TrueType and unchanged original Harano tests prepare actual lines,
table measurements and nested searches, including sibling children, deeper
nesting, forced breaks and definition queries. They exhaustively reject every
constructor work ceiling below the successful charge, check monotonic bounded
observations, retry with a retained record prefix, exercise exact work and
record ceilings, reject invalid indices, and compare successful fragment
fingerprints with the existing entry point.

## Remaining scope

This provides observations for one table constructor and its recursive children.
Body/joint/definition context constructors and the private PDF driver still need
to propagate these observations. Source admission, shaping internals and full
command allocation accounting remain open. This does not publish Book /2 or
establish whole-book, managed-host, performance or author/human acceptance.
