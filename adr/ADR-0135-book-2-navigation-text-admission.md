# ADR-0135: Check navigation text quotas before owned copies

## Status

Implemented with local source, allocation and PDF compatibility verification on
2026-10-06. Overall design 28 acceptance remains open.

## Decision

Keep the existing Book /2 logical text ledger and check each charged carrier
before making its owned copy. Validate metadata through borrowed fields, accept
all its existing aggregate charges, then copy the metadata. Collect language
sites by borrowing original spellings rather than cloning them. Their lifetime
is tied to the original wire, including footnotes and all page-region masters.

Canonicalize language tags with fixed storage: 255 output bytes, at most 128
borrowed subtags, and 36 singleton slots. Keep the original lexical/buffer-limit
precedence, grammar, grandfathered spellings, casing, duplicate rejection,
private use and extension ordering. The public canonicalization API still
returns its owned String. No algorithm identity or canonical bytes change.

For Book /2, inspect the bounded canonical spelling, parent language, source
span and exact prepared-vector facts before accepting the language charge.
Charge every effective spelling and the raw spelling only when it differs from
the canonical result. Subtract only the already validated vector override's
exact prepaid bytes. Create owned canonical strings and intern entries after
this check, preserving inheritance and all immutable-owner joins.

Validate number-anchor identifiers by borrowing them, retain the existing
owner/label/span/target checks, then charge before constructing the owned
identifier or adding its virtual anchor. Validate each outline entry and charge
its label before cloning the label. A private callback keeps the legacy outline
wrapper's existing policy. Book /2 stops at the first valid entry whose label
exceeds the quota, before inspecting later entries; a later source error no
longer postpones this resource rejection.

Preserve the existing charge quantities, per-carrier pointers, checked addition,
intrinsic retained_text_bytes and source_record_charge. Counted navigation still
retains its entire record reservation on later text/syntax failure, and the
driver returns that history before layout. Legacy public outputs, source
ownership, schemas, profiles and publication gates remain unchanged.

## Verification

Evidence is under
workspace/target/vmb-design/20261006/initial-navigation-text-admission/.
New syntax tests check grandfathered tags, invalid grammar, all 720 permutations
of six extension groups, 255-byte and configured boundaries, borrowed pointer
identity, and every logical charge boundary across seven source fixtures.
They compare retained projections and precise T2101 pointers at exact and
one-below quotas and keep record history through failed retries. New driver
tests explicitly run TrueType and original Harano paths and observe zero
layout work, spool/output, candidate passes and reshapes on text rejection.

The final frozen-source regression passes legacy syntax 79 tests and six
doc-tests, staging syntax 155 and twelve, layout 70 and one, workspace
all-feature test checking, and all 308 Book /2 CLI cases. The 419 filtered CLI
cases are not claimed as run. Legacy precomposed verification passes four
regular cases and keeps its external-host case ignored.

A standalone probe uses the actual fixed canonicalizer source and the saved
pre-change canonicalizer. All 63,945 input/limit comparisons agree, including
9,774 successful results. Seven successful allocation observations have zero
heap calls/requested bytes in the fixed core. The old error stub preserves kind
only; these allocation comparisons do not concern invalid-path diagnostics.

A second probe calls the real counted navigation API. Rejected 50,000-byte
metadata, virtual number-anchor and outline-label carriers make no allocation
of that size or larger. Exact-quota positive controls make one, three and one
such allocations respectively. Other small allocations remain. Input creation
and output formatting are outside the observation window; unsafe is limited to
standard System allocator forwarding in ignored diagnostic probes.

Five independent Book /2 validators and source-bound comparison pass for
759 PDFs, their embedded fonts, running regions and uniform table pages. All
759 PDFs and both saved VMB table outputs match stage 271 bytes. The legacy
independent verifier passes all 21 checks, retains the original 73-resource
ledger, and all 21 artifacts match the saved old binary's output.
Correspondence binds 667 frozen sources, all eight source/test changes,
original inputs, actual commands/logs, binaries, diagnostic observations and
artifact pairs. Later documentation is hashed separately.

The formatting draft accidentally appended a child-module stdout header and
source; compilation rejected it. Preserve that draft and remove the appended
output before freezing the successful source. Diagnostic dependency drafts
first selected a new cached flate2 version, then attempted an unused Linux
dependency while producing full metadata. Preserve the failures, inherit the
built workspace lock, and run the real host probe with locked/offline Cargo.
No package-cache installation or policy relaxation is required.

## Remaining scope

These are the existing logical text charges, not all allocated bytes. Source
paths, other anchor/owner strings, Vec capacity, tree nodes, intern storage,
simultaneously retained graphs and full command byte/spool/work accounting
still need their own bounds. The number-anchor positive control deliberately
shows three physical copies for one logical charge. Other codecs, Unicode,
backend/font temporaries and admission remain open. Named pages/columns, public
Book /2, the original whole-book PDF, managed hosts, controlled performance and
author/visual acceptance remain required. Speech/SemanticRef are user-confirmed
uncreated; pinned veraPDF/V19 host acceptance remains unverified.
