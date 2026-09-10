# ADR-0069: Rebind block parent frames to selected physical widths

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0068 from
paragraphs to fixed-size figures and display math. Width-dependent table columns,
remaining advanced page features and the full-design release gates remain open.

## Decision

Bind block parent-frame width candidates to the exact immutable original flow.
Require one entry for each raster/ordinary SVG figure, vector figure, native
display formula and precomposed display formula, in original source event order.
Reject foreign flows, missing/extra/reordered/duplicate owners and widths exceeding
the original measurement envelope. The frameless paragraph projection rejects
block assignments rather than silently discarding them.

Apply candidates to the original block owner region during each real shaping pass.
Retain the original parent frames separately, with unchanged start positions and
paragraph measurement envelopes. Bind all overrides into the frame fingerprint.
Block geometry then uses the narrowed parent width for indentation, alignment and
equation-number placement. Preserve declared image dimensions, SVG scale, native
math computations, glyphs, source spans and admitted resource identity. Caption
paragraphs use their existing independent source-width feedback.

The current equation-number contract requires a right-hand number and its declared
minimum gap. If the selected parent is too narrow, return the existing geometry
diagnostic; do not move the number below the formula or scale source content.

After source-closed physical placement, derive each block's target parent width
from its original measurement parent and the selected body/note root-width delta.
Retain the targets in source event order. Semantic and repeated occurrences remain
distinct; reject conflicting repeated widths or duplicate semantic consumption.
Unreferenced definitions keep their original widths. Width-changing table flows
remain rejected because a root delta cannot remeasure fractional columns/cells.

Combine paragraph and block targets in the driver's complete assignment comparison
and cycle digest. Rebind block frames before measuring blocks on the next pass.
Reuse the existing source/label/PDF convergence and bounded paragraph refinement;
resource ceilings and infeasible geometry remain errors, not acceptance shortcuts.

Physical math finalization verifies block parent widths independently of the driver.
Apply this check whenever a varying-width plan or explicit block-width candidates
exist, including a component caller without a page plan. Provisional parent frames
cannot acquire PDF authority by bypassing the driver callback.

Charge retained measurement frames and feedback/coverage records before allocation.
Charge source visits, frame lookup/sort, width hashing/comparison and physical checks
to existing work limits. Keep the original paragraph-only and fixed-frame paths
byte-compatible. Candidate widths do not replace source styles or source owners.

## Evidence

Design 28 §14.203 and the progress ledger record three-alignment component checks,
fresh shaping and exact/one-short work, invalid owner/order/envelope rejection,
number-gap rejection, and provisional-finalization rejection. Actual private PDFs
cover vector figures with captions and numbered formulas, centered native display
math, PNG, JPEG and ordinary SVG in body/footnote regions. Unchanged Harano covers
all figure families; native math retains the controlled MATH-capable fixture font.

Native math copies use distinct original input ranges and matching text mappings;
they do not evade the source validator's prohibition on overlapping math domains.
Independent PDF inspection checks original declared physical widths, fixed image
sizes, figure/formula alignment and right-hand equation numbers, with altered and
missing blocks rejected. Native centering uses actual glyph extents and verified
embedded advances; independent mathematical-layout optimality is not claimed.

These remain private staging results, without full-book, public-profile, manifest,
managed-host, PDF/UA or author/human acceptance authority.
