# ADR-0128: Retain main source-flow reservations across command failures

## Status

Implemented and locally verified under design 28 on 2026-10-06.
Overall design acceptance remains open.

## Decision

Book /2 source-flow constructors accept caller record history and a caller
ceiling. They initialize observations before checking immutable source identity,
then reserve logical carriers against the smaller of the caller and body
ceilings before constructing those carriers. Accepted reservations survive later
source, style, allocation and limit failures. A rejected reservation is atomic.
The normal constructors use the same accounting with zero caller history.

The source adapter supplies a reservation hook to the shared collector. Frozen
legacy flows keep their existing behavior. Page-region flows keep their existing
prepaid region accounting and disable this hook to avoid charging twice.

Reservations include the flow owner, temporary footnote ordinals, footnote
definitions, events, paragraphs and inline sites (including container closings),
lists/items, descriptions/items/terms, figures, named-break mappings, the
existing table topology/occupancy bound, each generated input carrier, the
generated overlay's output carriers and retained candidate Page values.
Table occupancy remains a conservative logical charge, not measured memory.
The immutable flow records its local charge excluding caller history. Full
revalidation independently reconstructs and compares it; this field grants no
receipt or profile authority and does not change canonical/fingerprint domains.

The actual private PDF driver supplies its cumulative record history to both
initial source collection and every candidate Page-label collection. It recovers
observations before propagating a typed constructor error, so retries inherit
reservations from earlier successful or failed constructions.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20261006/source-flow-record-budget/`.
Tests count constructed carriers independently, reject every insufficient record
prefix, accept the exact ceiling and test caller history, overflow, identity,
later source/style failures, retries and charge tampering without hash changes.
Description terms/definitions receive the same prefix tests. Existing table and
description boundary tests now bound the complete source flow, preserving their
original topology/text checks.

Actual driver tests cover initial later-paragraph failure and every rejected
candidate constructor boundary, including retries and original Harano resources.
Independent downstream constructor tests inherit both source reservations before
checking line, shape, body, table, page, label, display and PDF failure histories.
The small Harano shaping-bound fixture adds glyphs so its exact local shaping
ceiling can also admit the now-required thirteen source slots; it keeps the
shaping ceiling exact rather than introducing unused allowance.

The current public client preserves the preceding seventeen successful flow
projections and seven typed rejections. All 38 normal prepare/verify allocation
observations are unchanged. Its 36 insufficient/exhausted/overflow reservations
preserve caller history with zero heap allocation; ten successful fixtures also
pass exact and one-below checks. The combined flow charges 250 logical records,
candidate Page values add three, and 1,024 escaped paragraphs charge 5,121.

Core 21 tests, math eight tests, legacy syntax 76 tests plus six doc-tests and
Book /2 syntax 144 tests plus twelve doc-tests passed. Workspace checking, layout
70 tests plus one doc-test and Book /2 CLI 296 tests also passed. The CLI filter
leaves 419 tests outside this run; all executed tests have zero failures/ignored.
Independent checks accept 759 source PDFs / 2,896 pages, 58 font subsets and
their embedded programs, running regions and uniform table pages. All 759 PDFs
and both saved VMB table outputs are byte-identical to stage 264, with no renames.
The correspondence report binds 174 frozen sources including all 24 changed
files, eight clients/runners, three original inputs, twelve successful logs,
actual commands/exits/environment and the measured public-client binary.
Three acceptance-document updates after verification have separate hashes.

Draft logs retain the original local-boundary and incomplete-fixture failures,
then corrected syntax/driver success and twelve old downstream zero-source
history assertions. Those assertions were updated to inherit both source flows;
the complete final regression passed. No new shaping/pagination/font suite,
public/legacy diagnostics or current-VMB whole-book audit is claimed. No
whole-book acceptance or controlled performance proof is claimed.

## Remaining scope

Logical source reservations do not bound style-rule internals, Vec excess
capacity, all byte/spool allocation, traversal/hash work or simultaneously held
revalidation graphs. Flow revalidation still reconstructs a separate flow under
local source limits; its entire reservation must also reach the command budget.
Navigation verification checks immutable body identity only; initial navigation
construction has separate allocation/work gaps. Admission, shaping/backend/font
internals, named page changes/columns, public Book /2 publication, the original
whole-book PDF, managed hosts, controlled performance and author/visual
acceptance remain open. Speech and SemanticRef remain user-confirmed uncreated.
