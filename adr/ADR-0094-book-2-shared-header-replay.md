# ADR-0094: Share compatible header replays and retain prepaid catalog ownership

## Status

Implemented incrementally under design 28 on 2026-09-11. Verification and limits
are recorded in progress section 229. This does not complete design 28.

## Problem

Independent parent/child headers at the same root width were reconstructed in
separate sibling graphs. Every header trial also charged the complete header
projection and measurement again, even when a bound catalog already retained
those exact objects. The 16-paragraph nested footnote fixture exhausted its
10,000,000-record ceiling despite a larger explicit line-pass allowance.

## Decision

Group requests by exact original root owner and parent width within one source
replay. Collect and merge their original header leaf sets, retaining each target
table's request-to-group index. Reproject the union of selected source paths in
one sibling. Root-only groups retain the existing root-header projection path;
groups that combine independent headers use source-scoped projection. Do not
share graphs between different roots, widths, sources or label iterations.

Construct every target header from its group's exact measurement. Keep separate
header source ranges, geometry and identity. Independently validate each header's
physical frames in the catalog. Sharing a graph neither merges semantic tables
nor authorizes unselected body content at a header's width.

A recursive trial may borrow an already charged header when its exact catalog is
bound to the search, the search ledger covers the catalog's charge, the base
measurement is identical, and the catalog contains the exact header reference.
Charge the bounded identity scan and one trial selection owner. Nested paint
copies remain separately charged by their ordinary leaf allocator. Do not
recharge the catalog's retained header projection and measurement per trial.
The public standalone header-selection path, without this ownership proof, keeps
its conservative measurement/projection accounting. Failed attempts retain fees.

Prepay group mappings, source collection, union capacity, sorting and lookups.
Keep the original conservative allocation bounds for the replay ownership
vectors; the number of actual rebuilt siblings may be smaller than the request
count. No production resource ceiling is increased by this change.

## Verification and limits

Assert that independent headers on the same physical root/width use the exact
same measurement reference in parent/child, parallel and deep fixtures. Restore
the 16-paragraph stress input without reducing its contents or raising its
record ceiling. It produces 16 body pages and 32 footnote pages under the existing
explicit 512 line / 128 page pass allowances and 1,000,000,000 work ceiling.

Larger accepted inputs exposed test-oracle budgets that incorrectly imposed a
fixed total work ceiling below the already consumed input work. PDF assembly and
pipeline verification now add their 100,000,000-work allowance to that input
work. Exact and one-short work/record/spool/output assertions, prior propagation
and rejection after failed builds remain intact. Production driver limits are
unchanged.

Full regression, independent PDF/resource verification, preceding PDF byte
comparisons and measured charges are recorded in the progress ledger. Catalog
discovery still rebuilds after each new request and each source/label iteration;
full-book performance, the remaining nested terminal fixtures, public routes,
manifests, managed hosts and genuine author/human acceptance remain unfinished.
