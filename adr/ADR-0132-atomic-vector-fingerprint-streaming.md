# ADR-0132: Stream atomic-vector fingerprint preimages

## Status

Implemented with local compatibility verification under design 28 on 2026-10-06.
Overall design acceptance remains open.

## Decision

Write atomic-vector item and legacy paragraph fingerprint JCS directly into the
incremental core SHA-256 state through a private `fmt::Write` codec. Preserve
algorithm domains, field order, decimal integers, quoted lowercase hashes,
source spans, metrics, paint, spacing, boundaries and Unicode scalar escaping.
Encode each scalar into a four-byte stack buffer instead of a temporary String.

Keep the complete binding-fingerprint and placement comparisons before item
revalidation. Paragraph validation, retained units and boundaries, typed limits,
owner checks and public selected-line `canonical_jcs` remain unchanged. No
schema, profile, semantic content or receipt authority changes. This removes
hash-only temporary strings; it does not establish a whole-command memory bound.

## Verification

Evidence is saved under
`workspace/target/vmb-design/20261006/vector-fingerprint-streaming/`.
Before changing production code, eight complete public binding and mixed-inline
projections were captured from 526f08c using TrueType and the original Harano
font. They cover all four vector binding kinds, provenance changes, geometry
changes, 256 occurrences and rejection of another immutable owner. The archived
production sources, preserved baseline binary and complete projections accompany
the frozen hashes. Both final capture environment variables are explicitly
removed, and newly computed projections match all eight frozen cases.

Final local checks pass: linebreak 52 tests and Unicode 16 conformance, workspace
all-feature/test checking, layout 70 tests and one doc-test, and all 304 Book /2
CLI cases with the original fonts and ten saved VMB jobs. All failures and ignored
cases are zero; the CLI's 419 filtered cases are not claimed as run.

Five independent PDF/font validators pass. All 759 PDF pairs and both saved VMB
table PDFs are byte-identical to stage 268. Source/run correspondence records
196 frozen source files, all five production/test changes, baseline provenance,
original inputs, commands, environment, logs and PDF pairs. The three subsequent
documentation updates have separately recorded hashes. No allocator measurement,
controlled performance comparison or whole-book acceptance is claimed here.

## Remaining scope

The Book /2 vector-binding scratch String and other digest codecs remain work.
Retained owners/capacity, Unicode/backend/font temporary allocation, complete
byte/spool/work accounting, simultaneous graphs and initial navigation/admission
remain open. Named pages/columns, public Book /2, the original whole-book PDF,
managed hosts and performance/author/visual acceptance also remain open.
Speech/SemanticRef remain user-confirmed uncreated.
