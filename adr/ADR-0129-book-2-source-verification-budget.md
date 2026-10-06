# ADR-0129: Retain full source verification reservations through layout

## Status

Implemented and locally verified under design 28 on 2026-10-06.
Focused and broad regression, independent output checks and source/evidence
correspondence pass. Overall design 28 acceptance remains open.

## Decision

Add counted full verification of the exact immutable Book /2 body/navigation.
Initialize caller history before identity checks, reconstruct the complete flow
under caller/body ceilings and compare every existing collection, generated
buffer, local source charge and fingerprint. A stored charge or input hash never
replaces that reconstruction. Accepted reservations survive a construction or
final comparison failure, and a rejected reservation is atomic.

A caller-owned source verification budget accumulates these reservations across
shaping, inline preparation, line passes and retries. Intrinsic shaped/inline
counts and their fingerprint domains remain separate. Previously observed local
output reduces the room for another reconstruction. A sealed variant replay
first prepays its existing independently observed graph bound; prepaid output
is not deducted again. Prepaid credit cannot exceed caller history and this
observation grants no graph/profile/receipt authority.

The body-line budget retains its existing local output high-water observation
and exposes source history separately, including in the converged callback.
Variant seed/capture and single/set replay recover accepted reconstruction
reservations before propagating errors. The actual PDF driver supplies its
current record history and recovers source observations on both success and
failure. Running page regions continue to use their separate prepaid path.
Repeated table-header discovery uses the body variant seed/replay paths.

Source record limits become typed shaping OutputLimit / inline UnitLimit errors;
the source owner is preserved. Source allocation failures remain typed allocation
errors. All original immutable ownership and full flow comparison checks remain.
No public profile, contract, schema, legacy ledger or canonical domain changes.

## Local verification

Evidence lives under
`workspace/target/vmb-design/20261006/source-flow-revalidation-budget/`.
The draft workspace check passed. Book /2 syntax 146 tests and twelve doc-tests
passed, including every verification prefix, exact/body/caller ceilings,
overflow, identity, retries and charge tampering without fingerprint changes.
The two new TT/original-Harano tests passed after correcting a missing test
import, using the established wide-page fixture and distinguishing running
regions from repeated table-header discovery. They cover shared shaping/inline
history, local output compatibility, bad-epoch failure, repeated line attempts,
seed/single/set replay and actual driver failure/retry before PDF construction.
Draft failures are retained and are not acceptance evidence.

Public allocation observations, the complete existing regression, independent
PDF/font checks, deterministic prior-output comparisons and final source/run
correspondence were pending at that draft checkpoint.

The public client subsequently passed: all 38 ordinary allocation observations,
17 projections and seven typed rejections match stage 265. Forty exhausted
verification operations allocate zero heap bytes/calls and twenty exact/short
verification boundaries pass. The complete draft CLI run reports 278 passes
and twenty failures, primarily previous capture/replay/downstream history and
exact-boundary expectations. That draft was not accepted. Follow-up accounting adds the
retained local graph to successful/failing seed history and carries prior replay
observations into every new graph reservation; the follow-up was rechecked below.

The follow-up workspace check passed in 26.71 seconds. All twenty previously
failed cases and both new TT/original-Harano cases then passed individually on
the same source set. Complete regression and independent output checks were
still pending at that targeted checkpoint.

The final frozen set passes Core 21/math 8 tests, legacy syntax 76 tests and six
doc-tests, Book /2 syntax 146 tests and twelve doc-tests, the workspace check,
layout 70 tests and one doc-test, and all 298 Book /2 CLI tests including original
fonts and ten saved VMB jobs. Every suite reports zero failures and zero ignored;
the CLI filter excludes 419 other binary tests. The CLI run took 238.45 seconds,
an observation rather than a controlled performance comparison.

Five independent checks pass: 759 PDFs/2,896 pages, 58 font subsets and embedded
PDFs, three running-region PDFs and sixteen uniform table-page PDFs, including
their tamper rejection suites. All 759 PDFs and the two saved VMB table outputs
are byte-identical to stage 265. Source/run correspondence binds 177 sources,
all 24 changed files, eight clients/runners, three original inputs, twelve
successful logs, commands and environment. Three documentation updates after
verification are recorded separately. A source-union count assertion and the
missing-hash wrapper failure happened before any Cargo invocation; those failed
preflight records are retained separately from the final successful run.

## Remaining scope

This accounts logical source reconstruction reservations. Style-rule internals,
Vec capacity, byte/spool/traversal/hash work and complete simultaneous graphs
are not bounded by this observation. Intrinsic stage output still has its local
ceiling; enforcing the caller's remaining allowance before all local output
construction, and retaining every such failure prefix, remain separate work.
Initial navigation, admission, shaping/backend/font internals, named page
changes/columns, publication of Book /2, the original whole-book PDF, managed
hosts, controlled performance and author/visual acceptance remain open.
Speech/SemanticRef remain user-confirmed uncreated.
