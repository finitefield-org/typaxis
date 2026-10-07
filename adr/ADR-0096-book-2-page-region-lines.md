# ADR-0096: Select and converge running-region lines in the original page rectangle

## Status

Implemented under design 28 §14.232 on 2026-09-12. Repeated Artifact paint,
resource closure, PDF integration and command-wide failed-work accounting remain
outstanding; the existing unsupported page-content guards stay in place.

## Decision

Extend the closed shared inline adapter with `PageRegion`, forwarding only to
its source paragraph carrier. Keep separate `BookV2PageRegionInlines` and
`BookV2PageRegionLines` owners; neither is a body inline or footnote frame.
Validate the exact source/shape/ledger/limits/epoch before itemization, and the
exact original region and body of the selected master before geometry selection.
Bind page index and the original rectangle into the line-layout fingerprint.
The same source can be laid out on another physical page, but its placement
receipt cannot verify as the first page's receipt.

Use the ordinary inline selector at each paragraph's rectangle width minus
start/end indents. Preserve original glyphs, parsed text spans, mandatory breaks
and the shared blank-line behavior for empty paragraphs. Translate line-local
glyph coordinates by a separate page-space origin, retaining the selector's
existing origin compensation. Apply start/center/end alignment against required
line width and use actual selected line metrics for height. Match body spacing:
suppress outer paragraph spacing and add adjacent internal after/before spacing.
All paragraphs stay together in this fixed region; keep-with-next introduces no
additional break opportunity. An exhausted width, unbreakable content or excess
height is an error, never implicit scaling or clipping.

The direct layout function checks vertical fit. The convergence function first
shapes complete paragraphs, selects actual lines, then repeatedly shapes in the
selected source contexts and compares the resulting sealed layout states. Its
callback receives a borrowed graph only after stability and vertical fit. Delay
height rejection until that stable state: a provisional full-paragraph shape
need not have the final line-local geometry. Remaining line passes are bounded
both by the caller's allowance and the effective resource limit.

Retain caller records, shape records, preparation records, width scratch, line
projection, page origins and live previous line contexts under max_fragments.
Candidate work covers the initial line break and every completed reshape. The
caller remains responsible for carrying the remaining allowance across pages
and retries. These APIs still use the shared stage ceilings: they do not claim that shape work,
failed candidate work, source preparation or every later driver allocation has
been integrated into a command-wide budget. Callers must account for other live
source/driver owners in prior_records.

## Verification

Dedicated tests use the original Harano font and controlled TrueType input.
They compare original glyph references, source extraction, line width, physical
origins, actual line height, alignment and inter-paragraph spacing. Narrow
rectangles change line count. Tests cover hard breaks, blank paragraphs, exact
height and one raw-unit overflow, exhausted indents, unbreakable text, exact and
one-short candidate/record allowances, reshape pass exhaustion, foreign prepared
owners and the same master on another physical page. The previous source/shape
checks remain included. Exact commands and completed shared regressions are
recorded in design 28's progress section 232.
