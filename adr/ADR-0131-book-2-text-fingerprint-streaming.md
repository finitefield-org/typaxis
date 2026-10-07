# ADR-0131: Stream body and inline fingerprint preimages

## Status

Implemented with local compatibility verification under design 28 on 2026-10-06.
Overall design acceptance remains open.

## Decision

Feed the existing document, paragraph and generated-marker binary preimages
directly into the incremental core SHA-256 state. Keep their existing field
order, big-endian integer widths, generated-source discriminants, content hashes,
algorithm domains, intrinsic counts and source/owner checks. Preserve the checked
size arithmetic and its typed overflow errors. The temporary receipt-only vectors
and their allocation failure points disappear because those vectors are no
longer allocated. The shared list/footnote helper uses the same streaming path.

Stream prepared inline paragraph and native math inline preimages through private
`fmt::Write` helpers into SHA-256. Decimal integers, quoted lowercase hash bytes,
source-span JCS field order, mode names, boundaries and cluster ranges retain
their existing encoding. `write!` formats directly into the sink without an
intermediate `format!` or `to_string` allocation. Validation and construction of
the retained units, clusters and boundaries still precede the paragraph digest.
Native computation and geometry validation retain their existing order.

Only a fixed SHA-256 state is retained by these digest writers. Empty paragraph
encoding and the selected line result's public `canonical_jcs` remain their
existing representations. No schema, profile, receipt authority, effective
configuration or semantic content is changed. A digest is not a replacement for
full source validation or the immutable owner binding.

## Verification

Evidence lives under
`workspace/target/vmb-design/20261006/text-fingerprint-streaming/`.
Before changing production code, the actual public shape/inline projections
were captured with TrueType and the original Harano font from production commit
83fedd4. A preserved test binary, source hashes and complete projections accompany
the captured hashes. The fixture addition was committed separately as 36d92cd.
Two tests now compare freshly computed complete projections against those frozen
hashes, with capture mode explicitly disabled. All eighteen cases pass after the
streaming change: body, controls, empty paragraphs, generated list/footnote labels,
text/Page references, large paragraphs, line contexts and typed backend limits.

A separate client links the archived, unchanged linebreak Rust sources from
36d92cd and the current library in one debug process. Its baseline package name
and inherited manifest fields are adapted only for side-by-side linking; both
versions use the actual workspace dependency versions. Real parsed and verified
math computation receipts come from the repository math font. Input construction
and output serialization happen outside the observed constructor window.

Thirty-six comparisons pass: thirty-one successful native/vector/text/mixed
constructions across all three Japanese modes and sizes through 65,536 units,
and five typed rejections. Complete public fingerprints and unit/cluster
projections agree. Allocation requests do not increase; each successful case
reduces allocation calls and requested bytes. Native inline construction falls
from six calls/591 requested bytes to zero. The 65,536-unit normal text paragraph
falls from 367,662 calls/24,985,904 requested bytes to twelve/10,420,251; observed
peak Rust-owned bytes above its prepared input fall from 8,847,343 to 7,798,851.
The mixed case falls from 367,703 calls to thirty-nine. These observations are
not foreign allocation, whole-book RSS or controlled production performance
claims. Only the standalone probe forwards standard System allocator operations
with unsafe code; production modules retain their prohibition.

The initial isolated dependency draft could not unpack a newer cached flate2
outside the sandbox. It is preserved; seeding the probe from the actual workspace
lockfile avoids that version drift. A subsequent probe compile exposed an
incorrect private MathFontFace import; that draft is preserved and corrected to
the public typaxis_font owner. The locked comparison then passes in 19.385 seconds.

The final frozen sources pass core 21/math 8 tests, legacy syntax 76 tests and
six doc-tests, Book /2 syntax 147 tests and twelve doc-tests, legacy shaping 27
tests and one doc-test, staging shaping 28 tests and one doc-test, linebreak 52
tests and its Unicode 16 conformance test, workspace all-feature/test checking,
layout 70 tests and one doc-test, and all 302 Book /2 CLI cases with original
fonts and ten saved VMB jobs. Every final test has zero failures and ignored
cases. The CLI's 419 filtered cases are not claimed as run. Its 251.70 seconds
are an observation, not a controlled performance comparison.

Five independent validators pass: 759 source PDFs/2,896 pages, 58 variant font
subsets and embedded PDFs, three running-region PDFs and sixteen uniform-table
PDFs. All 759 pairs and both saved VMB table PDFs are byte-identical to stage 267.
Source/run correspondence binds 191 frozen sources including all eight changed
files, thirteen runner/client artifacts, three original inputs, sixteen success
logs, commands, exit codes, environment, preserved/measured binaries, eighteen
frozen public projections, thirty-six allocation comparisons and all PDF pairs.
Its first draft assumed all six captured fingerprint sources occurred in the
preceding source catalog. That catalog omits production_inline.rs; the failed
draft is preserved, and all six baseline production sources now match their
actual 83fedd4 Git archive. This change required no production/test rerun.
Three documentation updates after verification have separately recorded hashes.

## Remaining scope

This removes five categories of digest-only scratch storage. Retained owner
containers and capacity, other digest codecs, Unicode/backend/font temporary
allocation, complete byte/spool/work accounting and simultaneous graphs,
initial navigation/admission and command accounting remain required work.
Named page changes/columns, public Book /2, the original whole-book PDF, managed
hosts, controlled performance and author/visual acceptance remain open.
Speech/SemanticRef remain user-confirmed uncreated.
