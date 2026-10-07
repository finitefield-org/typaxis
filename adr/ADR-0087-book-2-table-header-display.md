# ADR-0087: Draw headers and enumerate fonts through their actual fragment flows

## Status

Accepted incrementally under design 28 on 2026-09-10. Variant-bearing terminals
can construct complete body displays and expose actual font uses. Resource closure
and PDF integration remain explicitly pending.

## Decision

Retain a global physical-fragment count on math terminals. For variant-bearing
results, prepay and reserve one exact-flow reference per physical fragment before
building terminals, then fill that array during the already verified traversal.
Expose a bounds-checked constant-time resolver. No-header results retain no array
and resolve valid indexes to the base flow, preserving their records and bytes.
This avoids rescanning prior pages for each glyph/font query.

Display construction verifies each header's actual flow, shape owner, admitted
resource ledger, limits and vector binding using the existing constant-time
identity checks. Charge header/page traversal on the display ledger. Resolve text
preflight and paint through each actual selected line, preserving exact cluster/
font pointers and physical glyph geometry. Resolve image/vector preflight and
paint, positioned anchors and nonpainting lines through the same flow.

List/footnote markers use the actual global fragment's shape, translating their
page-local fragment indexes with a checked prefix count in both passes. Equation
numbers already carry a global fragment index and now resolve their actual block/
number shape through it. Original unpositioned anchors remain semantic source
records; repeated header paint does not add another unpositioned source obligation.

Font-use enumeration selects the shape/native font-instance table belonging to
that paint's global fragment. Retain the existing admitted ledger/font identity
checks and actual glyph/text/native-scalar views. The body merge, semantic versus
repeated roles, source fingerprints and cumulative preflight/allocation/geometry/
hash accounting remain intact.

Move PendingHeaderVariants from display-builder construction to body display
resource verification. This permits concrete display and font-use inspection while
keeping downstream resource-selection/PDF authorities closed until variant glyph/
font/resource closure is tested and integrated. Do not claim that a display or a
font-use view alone is an assembled or accepted PDF.

## Evidence and remaining work

Design 28 §14.221 and the progress ledger record eight controlled native/vector,
body/note and split/join cases. Add a repeated list marker and anchor to the real
header. Independently compare every text draw to its actual selected cluster and
font, original glyph IDs and physical glyph coordinates; compare number, marker,
image and anchor pointers to the actual owning flow. Every font-use instance must
match that flow's exact table fingerprint and admitted font. Verify the global
resolver's count/bounds and display work/record limits, including one short.

Dedicated fixtures use constant valid physical widths with differing retained
line partitions. Raster headers, nonpainting header lines and original Harano
variant displays need dedicated acceptance beyond legacy regressions. Resource
closure/PDF output, automatic driver catalogs and actual width convergence,
remaining named scopes/running content/columns, full-book/public/manifests/
managed-host/scaling and real author/human acceptance remain open.
