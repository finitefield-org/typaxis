# ADR-0093: Retain independent header measurements through nested table continuations

## Status

Implemented incrementally under design 28 on 2026-09-11. Verification and limits
are recorded in progress section 228. This does not complete design 28.

## Problem

A child table inside a body cell or caption can continue independently of its
parent. Its repeated header previously used the base line graph, even when a
catalog supplied a different header for the parent. A physical page-width delta
cannot be applied directly to the child: ancestor columns, cells and insets must
first be resolved. A parent without a header still needs child header variants.

## Decision

Key catalog entries by the original target table index and its ancestor root's
parent width. Resolve that root through the original table topology, charging a
bounded lookup. Missing-width errors retain the target owner/index and request
the root parent width. Independently validate each candidate by reprojecting its
header leaf sources from that root.

The driver discovers the target header's original leaf owners and root through
source events. A source-width assignment retains the sorted leaf set; projection
remeasures its ancestors and preserves unselected branches at their original
envelope parents. Keep the complete semantic source and admitted resources in the
sibling graph. A nested table within a selected header remains wholly repeated,
including its caption and body. Root-only header assignments keep their existing
projection path.

Pass the physical region and exact catalog through recursive child searches.
Reserve each selected header's actual height, and restore temporary search state
on success, rejection and failure. Child trials retain the exact header reference
and header-leaf index along with paint position. Checkpoint rollback truncates
these associations with the trial's leaves. Nested fragment fingerprints include
the association when present; ordinary nested fragment encoding is unchanged.

Mixed selections distinguish their own header from any child variants. Legacy
single-owner placement queries reject either kind. Variant-aware placement,
earlier-capacity searches, width feedback, source closure and physical placement
resolve each leaf through its actual measurement owner. Original semantic ranges
remain base-owned and are consumed once. Repeated paint remains an artifact.
Sparse physical fragment owners allow existing math/display/font/PDF consumers
to use the correct graph without interpreting variant indexes in the base graph.

Prepay source collections, scoped projection scratch, traversal/sorting, retained
paint associations, lookup work and fingerprint storage. Failed searches and
rebuilds retain their charges. Do not turn budget exhaustion into PDF acceptance.

## Verification and limits

The private PDF driver is exercised for body and footnote tables with their own
parent header, no parent header, a child in the caption, parallel child tables,
and three levels of tables. Original HaranoAji Japanese text covers the no-parent-
header cases. Validate source closure, actual frames, selected header table
identities, glyph/subset/CID resources and independent PDF structure. The
no-parent-header body case also verifies exact work success and one-short failure.

These fixtures explicitly allow 512 line passes and 128 page passes. Larger
16-paragraph nested fixtures exposed the existing repeated-reconstruction cost:
128 line passes were insufficient, and the footnote stress case exhausted its
record ceiling after allowing more line passes. The accepted body/caption and
parallel/deep fixtures use eight/four paragraphs as recorded in the progress
ledger. This is not a scaling or full-book performance acceptance.

Dedicated nested math/image/rowspan/forced-break variant fixtures, reconstruction
cost reduction, remaining named/running content/columns, public CLI/manifests,
original full-book runs, managed-host measurements and genuine author/human
acceptance remain separate requirements.
