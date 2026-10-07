# ADR-0125: Reserve command bounds before native math construction

## Status

Implemented and locally verified under design 28 on 2026-09-30.
Overall design acceptance remains open.

## Decision

Split Book /2 native storage preflight from font-instance construction, MATH
parsing, computation, receipt verification and display preparation.
`BookV2NativeMathPreflight` holds private immutable bindings/resource borrows,
the verified limits fingerprint and the accepted work/record/spool bounds. It
cannot be constructed or altered by a caller and is consumed by the executor.
It issues no computation receipt or public-profile authority. The executor
rejects a different limits fingerprint before native allocation or parsing.
A native-free source returns `None` with the existing caller history.
The old counted and uncounted constructors delegate to this split path.
Successful computation fingerprints, paints and charges stay unchanged.

The PDF driver accepts each local preflight bound into command history before
entering native construction. Each dimension remains independently atomic and
accepted reservations survive another rejected bound or a later failure. No
dimension exceeds its ceiling and retries cannot reset history. A rejected
local preflight retains its original typed cause and observed prefix. After a
complete local preflight, command errors keep work, records, spool order and
stop construction. A font error can therefore be reported only when command
reservations allow font parsing to begin. This deliberately closes ADR-0124's
execution-before-command-check behavior; an insufficient command no longer
executes native math to discover and prioritize its font error.

The reservation helper owns the actual construction continuation used by the
driver. Boundary tests reuse that helper with an observed continuation calling
the real executor, and compare its history/errors with the whole PDF driver.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20260930/native-command-preflight/`.
Three new CLI tests exercise preserved computation/paint identity, foreign
resources/limits, a native-free source, fresh work rejection, separate and
combined command record/spool/work rejection, accepted prefixes and repeated
calls. Controlled TrueType and unchanged original Harano fixtures reject MATH
at both the first and a later formula. Continuation entry counts prove that
rejected command reservations do not enter construction. The previous three
constructor-observation tests retain their local-preflight and font-failure
coverage and now expect an early command limit after a complete preflight.

The first focused compilation used the admission crate name directly in the
CLI, which exposes that type through its existing resources dependency. The
type reference was corrected without adding a dependency. The constructor
budget filter passed eighteen CLI tests, including the six native tests, with
no failures or ignored tests (build 42.92 seconds, tests 39.14 seconds). Final
workspace checking, 70 layout tests, one layout doc-test and 294 Book /2 CLI
tests passed. The CLI filter ran 294 of the binary's 713 tests, with no failures
or ignored tests. Independent checks accepted 759 PDFs / 2,896 pages, 58 font
subsets and their embedded programs, running regions and uniform table pages.
All 759 PDFs and both saved VMB table outputs are byte-identical to stage 261.
The correspondence report binds 155 frozen sources, five evidence runners,
three original inputs, ten successful logs, the command environment and all
command exits. The three acceptance-documentation updates are recorded with
separate hashes. No new shaping/pagination/font or public/legacy diagnostic
test run is claimed. Timings are observations, not controlled performance proof.

## Remaining scope

Prepaid native layout units do not count every operation or prove the complete
command allocation budget. The existing typed-AST receipt revalidation used by
preflight can allocate canonical strings before these native reservations.
Source collection, admission, navigation/policy/vector preparation, shaping,
backend/font internals and simultaneously held temporary storage remain part
of the full allocation/work requirement. Legacy outer native contexts remain
separate. Named pages/columns, public Book /2, original whole-book, managed
hosts, controlled performance and author/visual acceptance remain open.
Speech and SemanticRef remain user-confirmed uncreated.
