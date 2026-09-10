# ADR-0092: Restrict table remeasurement to the sources actually used at that width

## Status

Implemented incrementally under design 28 on 2026-09-11. Verification is recorded
in progress section 227. This does not complete design 28.

## Problem

A 160pt figure in the first body row can fit the 220pt first page. Rebuilding a
header for a later 140pt page nevertheless remeasured that original body row and
rejected its figure before using the header. Fixed-column nested tables exposed
the same issue in the independent physical-width validator: its whole-root
projection visited nested tables that were absent from that physical occurrence.

## Decision

Header width assignments identify their exact root table. Reproject that root's
head at the candidate parent width, preserving the original envelope parents for
its caption and body cells. A nested table inside the head is entirely repeated,
so its caption and body are still reprojected. The complete immutable source,
resources, bindings and computations remain in the provisional sibling graph.
Do not remove semantic content, shrink fixed objects or substitute a foreign flow.

Independently remeasure actual table occurrences and catalog headers using their
source leaf owners. Collect, precharge, sort and deduplicate those owners from
verified selected ranges and actual variant leaves. A source-event traversal
marks every required ancestor before projecting frames. Reproject the selected
paths, including their table columns, cells, list/footnote insets and local styles.
Unobserved branches retain original envelope parents and impose no constraints
on this occurrence. The root table's own columns must still fit its parent.

The scoped frame result borrows the exact frame owner and sorted requested owners.
Its paragraph accessor verifies both source index and owner; its region accessor
rejects owners outside the requested set. Bulk arrays remain provisional storage,
not physical validation. Existing whole-root projection remains available with
its existing budget/encoding. New scoped fingerprints include the root and owner
set, including no-op geometry, and use distinct algorithm identifiers.

Validate source count, strict ordering, root containment and leaf kinds before
exposing a result. Prepay event bitmap/stack storage, traversal, source collection,
sorting, owner lookups and hashing. Retain abandoned work and allocation charges.
Header reconstruction and physical validation remain separate: a header candidate
does not authorize a fixed body object on an infeasible physical page.

## Verification and remaining work

Body and caption figures, directly and inside fixed-column nested tables, are
placed only on the wide first body/footnote page while headers repeat at three
widths. Tests also reject a narrow first page, a requested nested fixed table at
an insufficient width, foreign/non-leaf/duplicate/unordered source owners, and
one-short work/record budgets. Unrequested leaf queries fail.

The complete CLI, independent PDF/font/resource verification and preceding-byte
comparisons are recorded in the progress ledger. Dedicated fixed native/vector
body tests, independently repeated inner body-table headers, remaining named
scope/running content/columns, full-book public CLI/manifests/scaling/managed hosts,
and genuine author/human acceptance remain separate requirements.
