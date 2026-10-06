# ADR-0134: Reserve initial Book /2 navigation records

## Status

Implemented with local record-budget and PDF compatibility verification on
2026-10-06. Overall design 28 acceptance remains open.

## Decision

Before cloning navigation source strings or allocating its registries, walk the
exact styled body's immutable wire by borrowing its fields. Reserve logical
record upper bounds against the smaller of the caller ceiling and the body's
max_fragments. Check additions for overflow, preserve accepted reservation
prefixes on rejection, and retain the whole reservation on later syntax errors.
Repeated calls consume new reservations rather than resetting failed history.

The census includes these retained and temporary collections:

| Carrier | Reservation |
| --- | ---: |
| Navigation owner, metadata carrier | 2 |
| Metadata keyword | 1 |
| Language site, stable-sort scratch, prepared language | 3 per site |
| Explicit language spelling, including the document's | 1 possible intern entry |
| Authored anchor | 1 raw map entry + 1 prepared entry |
| Heading or semantic-container outline owner | 1 |
| Equation-number language child | 1 |
| Internal link | 1 raw entry + 1 sort slot + 1 prepared entry |
| Anchor/footnote reference | 1 raw entry + 1 prepared entry |
| Footnote definition lookup | 1 |
| Outline entry | 1 output + 1 stack + 2 validation-set entries |
| Number binding | 3 wanted + 3 found + 1 seen + 1 output + 2 possible virtual-anchor entries |

Walk list items, description items/terms, table captions/rows/cells, figure
captions, footnotes, nested inline containers, and all header/footer masters,
including unselected masters. Region wrappers and break controls do not become
ordinary language sites. Bounds deliberately remain conservative when language
strings, wanted number owners, or anchors can be shared. Counting does not grant
source validation or receipt authority: the original full metadata, language,
span, vector, number, link, reference and outline checks still run on acceptance.

Expose a counted staging API with a typed RecordLimit carrying the source owner
without allocating an error path. The existing convenience API enforces its
body ceiling and maps quota rejection to NavigationRecordLimit. The driver
copies both successful and failed observations into its command ledger before
policy, bindings, native math or initial text-flow construction. Quota rejection
there produces the driver's records limit directly; syntax errors retain their
original kind and JSON Pointer.

Retain an intrinsic source_record_charge on prepared navigation, separately
from caller history and existing fingerprint material. Do not replace the
immutable-owner check with a same-hash test. Legacy navigation, public schemas,
profiles, canonical projections and publication gates are unchanged.

## Verification

Evidence is under
workspace/target/vmb-design/20261006/initial-navigation-record-budget/.
Tests inspect retained projections independently of the census, exercise every
quota prefix and retry, caller/body ceilings, overflow, number/virtual-anchor
indexes, late source errors and foreign styled owners. Driver boundary and
failure-history tests explicitly exercise TrueType and original Harano inputs.
Existing table/description and native/source retry tests inherit the navigation
reservation instead of weakening their bounds.

The initial syntax draft exposed three old source-only allowance fixtures and
an incorrect new label expectation. The fixtures now test the combined
navigation/source boundary; the label uses the actual selected source bytes.
The initial native drafts exposed old command observations that omitted
navigation. Their assertions now include its reservation and prove the earlier
record rejection after exhaustion. These drafts and their logs are preserved.

The full regression first passed 302 CLI cases and exposed four remaining
source-only allowance fixtures. Keep shaping's exact output boundary after
navigation/source reservations; increase the actual Japanese glyph input for
its exact local shaping ceiling. The repaired source tests check cumulative
reservations and navigation's earlier rejection on exhausted retries.

The next full CLI run passed 301 cases; five stopped only on create_new for
existing region/table PDF outputs. Recover those five with fresh output paths,
preserving the original logs and binary. Its SHA-256 remains unchanged and the
unique success union contains all 306 Book /2 cases, with no ignored Book /2
case. The 419 filtered cases are not claimed as run.

Legacy syntax passes 76 tests and six doc-tests; staging syntax passes 150 and
twelve. Layout passes 70 and one doc-test. Their production/dependency inputs
remain unchanged after the CLI fixture adjustments. Fresh workspace all-feature
test checking and the three navigation record tests pass. The four regular
legacy precomposed cases pass; their external host case remains ignored.

Five independent Book /2 validators and source-bound byte comparison pass for
759 PDFs, embedded font programs, running regions and table pages. Every PDF
and both saved VMB table outputs match stage 270 bytes. The legacy independent
verifier passes and all 21 legacy artifacts match the saved old binary's output.
Correspondence binds 202 frozen sources, all 17 source/test changes, original
inputs, the unchanged CLI binary, its 301 + 5 success union, eighteen successful
commands and all artifact pairs. The inherited catalog omitted navigation's
owner module; add its hash before fresh checking/source tests and verify binary
identity across recovery. Record later documentation separately.

## Remaining scope

This bounds logical navigation record collections. It does not account for
Vec capacity, tree-node bytes, Unicode/string temporaries, full source admission,
or all command byte/spool/work and simultaneously retained graphs. Other
owners/codecs, named pages/columns, public Book /2, the original whole-book PDF,
managed hosts, controlled performance and author/visual acceptance remain open.
Speech/SemanticRef remain user-confirmed uncreated. V19 external-host acceptance
with its pinned veraPDF payload remains unverified.
