# ADR-0133: Stream Book /2 vector-binding fingerprints

## Status

Implemented with local compatibility verification on 2026-10-06.
Overall design 28 acceptance and the V19 external host gate remain open.

## Decision

Write Book /2 vector-binding JCS directly into incremental core SHA-256 after
the existing policy, admission, source ordering, resource, placement and math
provenance checks. Remove the per-binding 4096-byte scratch String reservation
and decimal temporary Strings. Keep the receipt vector's fallible reservation,
output limits, epoch, fingerprint domains, source borrows and immutable owner
verification. Removing scratch storage removes only its allocation failure point.

Share private generic writers for resource, placement and source-span fields
with legacy owned receipt construction. Preserve field order, decimal integers,
quoted hashes, geometry, RGB channels and every existing public canonical_jcs.
No schema, profile, source semantics or receipt authority changes.

The legacy regression exposed an existing publication-input defect: a newly
added CFF subroutine diagnostic fixture matched the published resource glob,
increasing its required exact 73-file set to 74. Move that diagnostic fixture
unchanged into cff-media/diagnostics and update its generator and font-test path.
Keep the publication ledger, resource hashes and strict verifier unchanged.

## Verification

Evidence is under
workspace/target/vmb-design/20261006/vector-binding-fingerprint-streaming/.
Eight frozen TrueType/original-Harano binding and mixed-inline projections match
the preserved 526f08c production capture. Both default and staging layout suites
pass 70 tests and one doc-test. Workspace all-feature/test checking and all 304
Book /2 CLI cases pass with original fonts and ten saved VMB jobs; the CLI's 419
filtered cases are not claimed as run.

Font tests pass all 76 cases: 65 default cases and an explicit separate run of
the eleven original-Harano cases. All four regular legacy precomposed-vector
CLI cases pass with their environment isolated from Book /2 probe variables.
The legacy independent verifier passes, and all 21 generated legacy artifacts
are byte-identical to output from the preserved old production binary.

Five Book /2 independent validators pass. All 759 PDF pairs and both saved VMB
table PDFs are byte-identical to stage 269. The diagnostic generator reproduces
the original 1088-byte font, SHA-256
41680ba255150ba71882a9488093032fab4f7918a2bedb7561ee49f1b2fe016f.
The exact published 73-file ledger passes without changing its expected hashes.
Source/run correspondence binds 198 frozen files, all changed and relocated
inputs, baseline provenance, original inputs and fifteen successful gates.
The three later documentation changes have separately recorded hashes.

The first legacy runner failed because it passed Book /2 probe environment keys
to the public legacy configuration parser and included the external host case.
Its logs are preserved. The publication-set failure also occurred with the old
binary. After relocation its readiness receipt was generated; the separately
stored old binary then lacked its expected companion executable location.
This observation is not an external host acceptance result.

The official veraPDF 1.30.2 installer and detached-signature hashes match the
immutable V19 policy. A local installation validates the legacy PDF/UA-1 output
with 172 passed checks, 106 passed rules and no failures. Its installed-tree hash
differs from the pinned payload, so the external case remains explicitly ignored
in the regular suite and no V19 host record is accepted. No allocator, controlled
performance or whole-book acceptance claim is made.

## Remaining scope

Other codecs, retained owners/capacity, Unicode/backend/font temporary
allocation, complete byte/spool/work accounting, simultaneous graphs and initial
navigation/admission remain work. Named pages/columns, public Book /2, the
original whole-book PDF, managed hosts and performance/author/visual acceptance
remain open. Speech/SemanticRef remain user-confirmed uncreated.
