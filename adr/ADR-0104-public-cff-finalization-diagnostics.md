# ADR-0104: Publish typed CFF finalization diagnostics in contract 1.4

## Status

Accepted incrementally under design 28 on 2026-09-20. User-run regression passes:
253 CLI tests, 73 font tests, one legacy diagnostic test and one public diagnostic
test covering six check/build cases, all with zero failures or ignored tests.
Independent checks pass for 759 PDFs and 58 resource subsets; all 759 PDFs remain
byte-identical to the preceding stage. Public diagnostic verification rejects
60 alterations across six cases. All 54 frozen source hashes match the executed
sources. Overall design acceptance remains open.

## Decision

Keep the FontFaceId and Cff1Failure in the CLI Failure until the public processing
boundary has the validated package. Do not recover resource identity or context
from a formatted error message. Cff1Error exposes its existing closed diagnostic
code, and Display uses the same mapping without changing textual codes.

At the production build diagnostic boundary, resolve the actual font declaration
and its array position, attach a typed FontFace resource subject and a package JSON
location, and emit the existing contract-1.4 fields through the shared diagnostic
budget. Public JSON uses the resource pointer and notes; no subject field or schema
extension is introduced. An absent declaration is an invariant failure, not an
opportunity to invent a resource location.

Use a canonical reason message and three notes: the declared resource URI, bounded
font context, and the existing supported-outline/inspect-font guidance. Font byte
positions belong in the context note. They are never used as package JSON byte
offsets or source-text spans. Name-keyed /1 has no FD index. Program-end positions
remain distinguishable from offending tokens; subset encoding failures without a
known source position retain none. Input/budget/internal classification, stderr
messages, read ledgers and failed-output publication policy remain unchanged.

## Verification and remaining scope

A public fixture-backed test covers operation, segment and subset-byte exhaustion,
reserved and unsupported operators, and a missing endchar. All six pass check
admission and fail during build with the expected code/exit status, the third font's
resource pointer, URI/context/support notes and a failed manifest containing the
admitted font hashes. The force-overwrite case preserves an existing PDF target.
An independent FontTools/source-byte verifier checks all six artifact sets and
rejects 60 mutations. Existing admission diagnostics and positive production
book-1 behavior remain covered.

This closes the structured public /1 finalization projection. Detailed subset
encoding locations/budget facts, public /2 entry points, full command budgets,
full-book/performance/managed-host and author/human acceptance remain open.
