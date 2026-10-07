# ADR-0097: Retain running-region glyph draws beyond line convergence

## Status

Implemented under design 28 §14.233 on 2026-09-20. Font subset/CID closure,
repeated page integration and PDF serialization remain outstanding. The existing
unsupported page-content guards stay in place until those consumers are joined.

## Problem

The page-region convergence callback borrows a temporary source/shape/line graph.
Keeping that graph in a document display would require nested callbacks for every
page and master. Treating those paragraphs as body fragments would instead invent
body semantics and make running text eligible for body MCIDs.

## Decision

Introduce `BookV2PageRegionDisplayBuilder` and `BookV2PageRegionDisplay`. Accept
only the sealed converged page-region view. Validate the exact source body,
shape, admitted ledger, effective limits, epoch and font-instance table. Keep the
source body and ledger borrowed, and copy the selected glyphs into owned vectors
with their actual page-space coordinates. Recover each exact UTF-8 slice from
its original parsed text buffer/span and compare it with the selected cluster.
No character-to-glyph reconstruction, implicit font substitution, or copied
source text buffer is introduced.

Build the same dense admitted font-instance table once per builder. Resolve each
selected run through that table and retain its sealed admitted instance alongside
the original face metrics. These instances borrow the ledger, not the temporary
shaping table. A returned display can therefore outlive source flow, navigation,
shape, lines and the builder while still retaining original text and resources.

A region display's role is always a Header/Footer page artifact, including its
first occurrence. It owns no body fragment index, semantic structure node or
MCID. Preserve original region/paragraph/inline owners, line indices, page index,
frame, exact text span, logical bounds and the actual selected glyphs. Empty
regions still retain an owner/fingerprint even when no glyph is drawn. A later
PDF consumer must emit the pagination Artifact semantics and select/encode all
actual glyphs; this display owner alone cannot authorize PDF output.

Extract the existing body cluster geometry into a helper taking translation and
baseline directly. Both body fragments and page regions use the same geometry;
regions never construct fake body fragments. Keep original advances and offsets.
Logical font metrics are not ink bounds and must not become clipping rectangles.

The persistent builder carries caller records/work across successful and failed
builds. Preflight draw/glyph output slots before allocating them; retain reserved
slots if a subsequent projection fails. Charge source-text comparisons, actual
glyph visits and fingerprint hashing. Bind the display to the converged layout,
ledger, limits, epoch, original cluster identity and projected glyphs. Repeated
pages do not reset the builder allowance. Layout/source/shaping work and other
live allocations remain the caller's responsibility; this is not completion of
the command-wide budget or its failed-line-attempt accounting.

## Verification

Tests consume returned displays after the temporary layout/source-flow scopes
end. Controlled TrueType and original Harano Japanese cases cover two pages and
both roles; compare original text pointers, face hashes, instance ledger identity,
selected GIDs, exact page coordinates, logical bounds and page-sensitive hashes.
Exact/one-short output-record and work limits, failed-work retention, wrong epoch,
foreign source/ledger and empty regions are checked. Existing body geometry and
PDF output must remain unchanged; commands and results are in progress §233.
