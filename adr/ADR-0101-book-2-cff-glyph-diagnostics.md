# ADR-0101: Preserve CFF glyph positions through resource and PDF errors

## Status

Accepted incrementally under design 28 on 2026-09-20. Focused execution, original
Harano failure propagation and independent diagnostic checks pass. User-run full
regression passes 253 CLI tests and 65 font tests, including all 23,060 original
Harano glyphs. Independent checks pass for 759 PDFs, all byte-identical to the
preceding stage, and for the new diagnostic probes. Full-design gates remain open.

## Decision

Correct the shared Type2 observation hook without changing execution, work charging
or accepted programs. Numeric operands have no operator number. A truncated escape
retains the escape prefix position without inventing a complete operator. Failure
at a validated program end records that end cursor, including inside a local or
global subroutine, rather than retaining the last observed token. Distinguish that
cursor from an exact offending field in bounded diagnostic context.

Standalone CFF /2 inspection retains GID, active FD, table-relative position and
optional operator, but has no SFNT context. The admitted selection session attaches
the actual CFF table base, standalone face index, OS/2 embedding permission and
typed charstring/width/budget reason. This yields both original file-relative and
table-relative positions. Internal operation/outline exhaustion includes the actual
limit and next attempted count. Caller-supplied errors retain their kind and position
but do not inherit an internal limit, even when the caller reuses the same error enum.

Expose existing nested errors through `std::error::Error::source` at the selection,
CFF program/subset, font-program and PDF-pipeline boundaries. A caller of the private
source-to-PDF driver can reach the original typed glyph failure without parsing Debug
strings. No input bytes or resource path strings are copied into the fixed-size context.
Success receipts, evaluator identities, font subsets and PDF authorization are unchanged.

## Evidence and remaining scope

Design 28 §14.238 records truncated operands/escapes, empty programs, missing local
returns, FD isolation, initial allocation failure, actual Harano operation/segment
exhaustion, and caller errors that must not invent internal allowances. The driver
test independently reads the SFNT directory and original operator bytes. A separate
FontTools check verifies the font hash, FDSelect, OS/2 permission and emitted notes.

This closes these /2 glyph-context and error-chain gaps. It does not complete legacy
/1 charstring diagnostics, detailed subset-encoding locations, every unsupported
operator classification, public CLI resource notes, full command-budget accounting,
full-book/public/managed-host, performance or author/human acceptance.
