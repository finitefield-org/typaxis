# ADR-0089: Assemble PDFs from verified actual header variants

## Status

Accepted incrementally under design 28 on 2026-09-10. Verified variant displays
can traverse the complete resource/PDF pipeline. Automatic private-driver header
catalogs and varying-width convergence remain open.

## Decision

Remove PendingHeaderVariants after actual display ownership and resource closure
have been implemented and tested. Both PDF and resource-selection entry checks
require the immutable display's exact admitted ledger and limits. All actual
header flows were checked by the display constructor; selection validates each
actual font-use table before closure and encoding. No resource or source check is
relaxed. Remove the obsolete error variant.

Audit PDF consumers: source structure, table/note relations, navigation metadata
and page masters refer to the shared original semantic source. Actual paints,
physical geometry, anchors and font uses come from the verified display. These
consumers must not re-index variant line ordinals into the base measurement.
The independent nonpainting-line probe now resolves its global fragment's actual
flow. Original anchors own destinations; repeated headers remain artifacts and
cannot replace original source destinations or create duplicate semantic content.

Reuse the existing complete body/resource/structure/navigation/assembly test
chain for variant displays, including stateful pipeline byte equality, exact
work/record/spool/output limits, one-short rejection, prior-work propagation and
non-refunding failed retries. Keep the dedicated actual-flow draw/source oracle.
This exercises assembled PDFs, not just isolated font objects.

Add raster and anchor-only nonpainting header leaves in body and note fixtures.
Assert one original plus one copy per continuation for both raster and empty-line
geometry. Raster fixtures use a 160pt region so the fixed image and header fit;
other fixtures retain their original 100pt region and constant valid 240pt width.
No fixed object is shrunk to make pagination succeed.

## Independent evidence

Twenty variant PDFs cover controlled native/vector, original Harano CFF,
contextual GSUB, raster and nonpainting cases in both split/join directions and
body/note regions. The generic independent PDF verifier checks source structure,
marked content, artifacts, navigation, references and complete object graphs.
Resource probes independently check original versus subset outlines and metrics.

`tools/verify_book_v2_header_pdfs.py` joins each assembled PDF to its actual
resource probe by the display fingerprint. It verifies embedded program hashes,
CID widths/maps, ToUnicode ambiguity decisions and the complete multiset of
actual page CID paints. A glyph present only in a repeated header must occur
only inside Artifact scopes. Exclude only the separately verified BMA Type3
anchor font. Serialized font-program and CID mutations must be rejected;
contextual cases also reject removal of Artifact tags.

The mutation harness initially replaced a flattened reader page's Contents and
then cloned the original page tree, losing that mutation. The self-test exposed
the ineffective edit. It now mutates the cloned writer tree and reparses serialized
bytes; the actual mutated PDFs are rejected. Do not report the initial ineffective
mutation as a successful tamper rejection.

Render original Harano body/note PDFs' first and repeated pages with Poppler and
inspect the images. Confirm changed line partitions, preserved vector/number
placement and original-only footnote definition markers. These controlled Latin
fixtures are not full Japanese-book or human/PDF-UA acceptance.

See design 28 §14.223 and its progress ledger for commands and completed results.
The private driver still needs automatic catalogs and varying-width convergence;
table_repeated_frame_reflow, remaining named scopes/running content/columns,
full-book/public/manifests/managed-host/scaling and actual author/human acceptance
remain open. Do not count test-constructed variant catalogs as private-driver
convergence callbacks or VMB producer/Go acceptance.
