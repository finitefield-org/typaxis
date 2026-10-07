# ADR-0126: Compare math canonicals without rebuilding temporary strings

## Status

Implemented and locally verified under design 28 on 2026-09-30.
Overall design acceptance remains open.

## Decision

Use one math canonical visitor with a caller-owned `fmt::Write` sink. It writes
retained construction strings or compares against existing immutable canonical
bytes. AST field order, JSON escaping, normalized source separators, braces,
script order, operator spellings and hashes remain unchanged. A comparison must
consume the whole expected string; missing, extra or changed bytes are rejected.
`ParsedMathReceipt::verify` still derives AST accounting, AST and formatted-source
content, all metadata and their hashes. It does not trust cached hashes alone.
The constructor's canonical round-trip also compares directly, and formatting
no longer creates a temporary string for every nested subtree.

Core's existing `push_jcs_string` delegates to a shared `write_jcs_string` visitor,
which propagates bounded-sink failure. SHA-256 keeps the same signature/digests,
borrows full input blocks and pads only a fixed 128-byte tail instead of copying
the whole input. Production/workspace code retains its unsafe-code prohibition.
No public profile, receipt authority, schema or fingerprint domain changes.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20260930/math-receipt-streaming/`.
Before changing the implementation, a standalone public-API client recorded
22 successful math cases, six grammar rejections, four JCS cases and 4,101 SHA
cases, plus the two original fonts and preserved whole-book package. After the
change, all behavioral output matches. Python hashlib independently verifies
SHA cases and original inputs. All 4,148 measured after-client operations allocate
zero heap objects: math verification/required units, SHA cases and original
input hashing. Counts of requested allocation bytes are not peak-memory values.
Only the ignored standalone client's standard allocator forwards use unsafe.

Core 20 tests and math eight tests passed, including padding boundaries,
control/Unicode escaping with a fixed-capacity sink, exact UTF-8 prefix checks
and rejection of self-hashed tampered AST/source/receipt caches. Root-library
changes after client measurement add only the two cfg(test) module registrations;
their exact source correspondence is recorded. Final workspace checking, 70
layout tests, one layout doc-test and 294 Book /2 CLI tests passed. The CLI filter
ran 294 of the binary's 713 tests, with no failures or ignored tests. Independent
checks accepted 759 PDFs / 2,896 pages, 58 font subsets and their embedded
programs, running regions and uniform table pages. All 759 PDFs and both saved
VMB table outputs are byte-identical to stage 262.

The correspondence report binds 160 frozen sources (all fifteen changed files),
seven evidence clients/runners, three original inputs, ten successful logs,
actual commands/exits/environment, both measured client binaries and all PDF
pairs. Before running the checker, its over-escaped tab/newline literals were
corrected; the before/after hashes are separately recorded. The production
sources and other six clients/runners stayed fixed. The three acceptance-document
updates after verification are recorded with separate hashes. No new
shaping/pagination/font or public/legacy diagnostic run is claimed. Timings are
observations, not controlled performance proof.

## Remaining scope

This closes the identified canonical/hash temporary allocation, not the entire
command allocation/work budget. Construction still retains its source/AST and
canonical strings and reparses for round-trip proof. Byte/AST/hash work needs
command reservation before traversal. Main source-flow collection and full-flow
reconstruction during verification, admission, navigation/policy/vectors,
shaping/backend/font internals and concurrent retained graphs remain open.
Named pages/columns, public Book /2, original whole-book, managed hosts,
controlled performance and author/visual acceptance remain open. Meaning data
remains user-confirmed uncreated.
