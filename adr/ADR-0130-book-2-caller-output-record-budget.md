# ADR-0130: Bound retained body output by the caller's remaining records

## Status

Implemented with local compatibility verification under design 28 on 2026-10-06.
Overall design acceptance remains open.

## Decision

Derive an intrinsic output ceiling from the caller/source history after full
source reconstruction. Keep the original effective limits and their fingerprint;
pass the smaller ceiling separately. Previously reserved replay graphs may reuse
their prepaid output allowance. Credit remains bounded by caller history, and
the derived ceiling never exceeds the body's own limit. Reject inconsistent
history or retained observations without subtracting through zero.

The budgeted Book /2 shaping entry passes this ceiling through the common body,
line-context, list-label and footnote-label engines. Count actual glyphs and
clusters from borrowed backend output and accept the run reservation before
allocating retained glyph/cluster vectors. A rejected reservation is atomic;
accepted reservations survive subsequent allocation, coverage or metric errors.
Backend temporary buffers still use their separate reviewed context allowance.
Intrinsic fingerprints and successful output counts retain their existing domains.

Budgeted inline preparation applies the same ceiling before unit/cluster,
vector/native/control/anchor and figure reservations. The prepared immutable
owner carries it into frame selection, source-width projection, line selection,
context capture and footnote projection. It keeps the original document ceiling
separately for later APIs whose caller supplies an absolute ledger including the
retained graph. A local residual ceiling must not subtract that history again.
Body feedback recovers the footnote high-water observation before success or
failure propagation. Seed and replay
use these same budgeted entries and their original ownership/fingerprint guards.

Existing local shaping and inline APIs keep their intrinsic ceiling and ledger.
Legacy and running-region accounting do not acquire command source charges.
No profile, public schema, effective config or receipt authority changes.

## Verification

Evidence lives under
`workspace/target/vmb-design/20261006/caller-output-record-budget/`.
The first workspace check passed before the footnote projection connection and
new tests. Added tests cover remaining/prepaid arithmetic, actual LTR/RTL backend
count reservations, every retained-output prefix, line contexts and generated
labels, exact/short inline and footnote boundaries, and actual driver failures
with TrueType and original Harano fonts. They had not run at that first workspace check.
The first draft shaper reported 26 passes, one doc-test and two original-font cases
still ignored; syntax reported 147 passes and twelve doc-tests. The new CLI cases
initially failed to compile due to missing test imports/incorrect helper scope.
After correcting those test references, both TT/original-Harano cases pass in
6.96 seconds. The first final shaper run exposed an incorrect test expectation
for a reserved run's later metric failure: the precise error is OutOfRange.
After correcting that expectation, all 28 shaper tests and one doc-test pass,
including both original-font cases. The next full CLI run reported 283 passes
and 17 failures. Fifteen exact-boundary cases exposed reuse of the local
residual ceiling for later absolute caller ledgers; two feedback tests still
expected the former local shape prefix without cumulative source reservations.
Those failed runs remain in final-draft-01 and final-draft-02. After separating
the two ceilings and comparing feedback observations with independent budgeted
entries, all seventeen repaired cases and both new CLI cases pass. The original
local exact/one-below boundaries remain intact.

The final frozen sources pass core 21/math 8 tests, legacy syntax 76 tests and
six doc-tests, Book /2 syntax 147 tests and twelve doc-tests, shaping 28 tests
and one doc-test, workspace all-feature/test checking, layout 70 tests and one
doc-test, and all 300 Book /2 CLI cases including original fonts and ten saved
VMB jobs. Every final test reports zero failures and ignored cases. The CLI's
419 filtered cases are not claimed as run. Its 275.89 seconds are an observation,
not a controlled performance comparison.

Five independent validators pass: 759 source PDFs/2,896 pages, 58 variant font
subsets and embedded PDFs, three running-region PDFs, and sixteen uniform-table
PDFs. All 759 PDF pairs and both saved VMB table PDFs are byte-identical to stage
266. The standalone public client preserves seventeen flows, seven typed
rejections and all 38 ordinary allocation observations. Its 36 exhausted source
reservations and 40 exhausted revalidations allocate zero additional heap; both
sets of twenty exact/one-below checks pass. These are not peak-memory claims.

Source/run correspondence binds 184 frozen sources including all 24 changed
files, eight clients/runners, three original inputs, thirteen success logs,
commands, exit codes, environment, measured client binary and all PDF pairs.
Three documentation updates after verification have separate recorded hashes.

## Remaining scope

This bounds the existing logical retained-output reservations. Unpriced owner
containers, vector capacity, backend temporary allocation and byte/spool/work
accounting, complete simultaneously held graphs and initial navigation/admission
remain separate required work. Named page changes/columns, public Book /2,
the original whole-book PDF, managed hosts, controlled performance and author/
visual acceptance remain open. Speech/SemanticRef remain user-confirmed uncreated.
