# ADR-0124: Retain native math reservations through constructor failures

## Status

Implemented and locally verified under design 28 on 2026-09-30.
Overall design acceptance remains open.

## Decision

The shared native preflight publishes accepted layout-unit, record and spool
reservations in their existing order. `ProductionNativeMathBudgetObservation`
starts with caller record/spool history and zero layout units. Each indivisible
reservation is published only after its ceiling accepts it. Arithmetic and
identity errors preserve the last accepted prefix. Font-instance prefixes are
accepted as part of the existing combined storage bound.

The legacy computation entry and Book /2 entry delegate to counted constructors.
Book /2 exposes the same observation under its namespace. Font parsing,
computation, receipt verification and display construction failures retain the
preflight observations. Successful native receipts, fingerprints and charges
remain the same; a native-free source still issues no computation owner.

Owned inline preparation exposes native observations separately from its inline
record counter. Owned body-line convergence uses a caller-owned line budget and
a separate native observation. Native layout units are prepaid computation
bounds, not actual completed work or line-candidate work; line reservations retain
their existing maximum-prefix semantics.

The PDF driver accepts each observed local reservation into its cumulative
command history before propagating a native error. Rejected command reservations
consume nothing and cannot exceed their ceilings. The original native cause
takes precedence over later command-limit errors. Successful construction keeps
the existing work, record, spool error order. Retrying cannot erase reservations.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20260930/native-constructor-budget/`.
One shared-preflight test and three Book /2 tests cover work/record/spool limits,
exact capacity, arithmetic overflow, identity, native-free sources, owned inline
and line contexts, font rejection at the first and later formula, unchanged
original Harano rejection, and command retry saturation. Initial CLI compilation
failed on an ambiguous test `sum()` type; explicit `sum::<u64>()` fixed it.
The three focused Book /2 tests passed. Final workspace checking, 70 layout tests,
one layout doc-test and 291 Book /2 CLI tests passed with no failures or ignored
tests. Independent checks accepted 759 PDFs / 2,896 pages, 58 resource subsets
and their embedded font programs, running regions and uniform table pages. All
759 PDFs are byte-identical to stage 259, including both saved VMB table outputs.
The correspondence report binds 151 frozen sources and ten successful logs.
No new shaping/pagination/font or public/legacy diagnostic run is claimed here.

The current VMB source was separately re-audited: its section inventory is
byte-equal to the 2026-09-22 inventory, and its release validation still rejects
the same missing meaning data. That inventory is not a whole-book PDF proof.

## Remaining scope

Observations describe existing conservative reservations. The command driver
still receives them after native construction; reserving all command resources
before computation, internal temporary allocation and complete work accounting
remain required. Legacy context consumers can use the counted computation API
but their outer context observation is not added here. Source collection,
admission, shaping/backend storage and simultaneous held allocation remain open.
Remaining named pages/columns, public Book /2, original whole-book, managed hosts,
controlled performance and author/visual acceptance remain open. Speech and
SemanticRef remain user-confirmed uncreated.
