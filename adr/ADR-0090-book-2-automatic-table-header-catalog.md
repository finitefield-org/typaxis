# ADR-0090: Discover and converge repeated table header widths in the private driver

## Status

Implemented incrementally under design 28 on 2026-09-11; local acceptance evidence
is recorded in the design progress ledger. This does not complete design 28.

## Decision

An empty sealed header catalog is a discovery input. An attempted continuation
returns TableHeaderWidthRequired with its original table index, source owner and
exact parent width. The width retains the original hierarchy, list inset and
footnote marker gap. Initial headers still use the base measurement. Missing
variants never fall back to an unverified header. Empty catalogs validate the
base flow and limits and consume bounded work and storage.

A converged line seed can prepare a sibling with new exact-source width
assignments while borrowing its original source, policy, admitted resources,
vector bindings, native computations and page plan. It recomputes line contexts;
it cannot substitute a same-content foreign source or overwrite the base seed.

The private catalog builder scans original root-table measurement widths in
source-event order. Each requested variant changes just that root's parent width;
other roots retain their original measurement widths. Header variants do not
inherit semantic source-unit width/start assignments or retained line boundaries
from the base. The complete set of immutable line, number, block, measurement and
header ownership layers coexist while the sealed catalog is consumed. Vector
storage, scans, reconstruction, shaping passes and projections are charged.

Discover widths lazily through actual body/footnote page search. Retain each
failed search's work and records, and charge one page pass per discovery attempt.
Insert requests in strict table/width order with prepaid bounded storage, lookup
and shifting work. Reject a duplicate request instead of looping. The successful
discovery remains provisional and cannot replace stable page/source/PDF proofs.

The existing private PDF driver enters this path when ordinary source-width
feedback reports table_repeated_frame_reflow. It preserves the spent search
budget, then reconverges the base, discovers headers, selects stable pages and
uses paragraph_frame_feedback for actual variant closures. Labels and source
widths continue through the original convergence loop. Each iteration rebuilds
source-bound graphs; no catalog from an earlier label/source context is reused.
The ordinary no-header path retains its existing work/record accounting.

## Shared record ownership

Table measurements expose the historical record base separately from their
retained block/body/table projections. Exact variant-set membership proves that
line graphs are already retained. Header construction preserves the largest
history plus every independent projection; selection charges a new variant's
retained projection rather than its complete historical total. Every attempted
header placement still pays its projection/attempt charge.

A new search constructor takes the sealed catalog itself and prepays its full
ledger before allocating search state. Binding inside that constructor does not
charge the same history again. The existing independent setter retains its
conservative full catalog charge and all identity/state checks. These changes
remove repeated multiplication of history; they do not refund spent candidates,
replays or source-width passes.

## Validation and limits

Tests cover empty/missing catalog widths, same-source sibling reconstruction,
foreign owners, ordering, exact work/record/line-pass boundaries and one-short
rejection. Body and footnote page searches discover both 140pt and 220pt physical
widths. Automatic catalogs also reach native/vector math terminals with exact
original computation/binding pointers, including contextual/raster fixtures.

Actual private-driver PDF tests cover body/footnotes, ordinary/nested headers and
unchanged original Harano Japanese text. They require final variant source
closure and use the complete body/resource/structure/navigation/PDF oracle.
The controlled body case also replays at exactly the observed cumulative work
and rejects one less. Independent PDF verification and preceding-byte comparisons
are recorded separately in the progress ledger.

The fixtures explicitly allow 128 global line/page passes. Discovery currently
rebuilds its catalog on each source-width iteration; its work and pass cost is
reported, not presented as large-book performance acceptance. Full varying-width
native/vector/raster driver coverage, conflicting named scopes, running content,
columns, the original full book, public CLI/manifests/managed hosts/scaling and
actual author/human acceptance remain separate work. In particular, a full
variant still reprojects the entire selected root: fixed non-header body objects
at a width used only by a later header need dedicated reachability coverage.

The normal CLI feature, all-features workspace and default workspace checks pass.
An additional pagination-only feature selection across the entire workspace
exposes seven existing DescriptionList match gaps in machine-profile when its
own staging feature is disabled. The exact failing command and scope are retained
in the progress ledger; this mixed workspace configuration is not accepted here.
