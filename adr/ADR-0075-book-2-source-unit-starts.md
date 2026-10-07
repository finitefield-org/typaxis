# ADR-0075: Bind source-unit origins through shaping and page placement

## Status

Accepted incrementally under design 28 on 2026-09-10. Connects source-position
origins to actual inline selection, body collection and provisional page placement.
Heterogeneous table driver convergence and repeated-header variants remain open.

## Decision

Extend exact-flow source-width assignments with optional body-relative horizontal
starts indexed by original logical-unit position. A supplied origin profile requires
a complete width profile for the same paragraph and unit count; empty paragraphs
use one slot. Reject foreign flows, malformed counts, missing paired widths and
origins whose width extends outside the original body or note-content envelope.
Frameless shaping rejects origin assignments instead of silently discarding them.

Rebind these candidates on each actual shaping pass. Preserve source-unit identity
through the existing text/scalar/atomic/break validation, retain owned origin copies
in the exact new frame owner, and include presence, owner and all origins/widths in
a separate frame digest. The original paragraph envelopes and source hierarchy
remain available. Ordinary callers without origin assignments keep their previous
frame and PDF encoding.

Preflight retained origin/paragraph records before allocation. Charge source-event
and unit visits to the same cumulative shaping work as table/block projection and
inline selection. Copy each profile once. Footnote bounds use the original generated
marker's content frame, with starts relative to the measurement body's x origin.
Retain complete source profiles, including currently unreachable line starts.

The body collector uses the selected line's original start unit to obtain its
candidate horizontal origin. Apply alignment slack from that line's selected width,
then preserve the origin through existing body/note physical-page translation.
Original glyphs, spans, source order and source closure remain required.

Finalization independently checks explicit source origins even when the page plan
has no width reflow. After the existing physical width checks, compare selected
origins with the projected source paragraph starts. A displaced candidate cannot
become PDF-authorized merely because its width is correct. The current root-table
continuation guard remains until per-occurrence frame convergence is integrated.

## Evidence and remaining work

Design 28 §14.209 and the progress ledger record controlled and unchanged Harano
body/note occurrence frames rebound through real shaping, selected widths, collector
positions, physical page placement and source closure. Start/center/end alignment
and original glyph identity are checked. Changed width does not require changed
line count: the Harano body case retains its count while using narrower lines.

Separate fixed-width table tests accept original origins and reject a one-raw-unit
displacement with unchanged widths at finalization. Missing, short, long, foreign
or out-of-envelope candidates fail. Actual convergence candidate/frame work succeeds at the exact
observed budget and fails one short; retained-origin record deltas are checked.

The private driver still needs joint width/origin feedback and repeated-header
layout variants for heterogeneous continuations. This is not a new heterogeneous
PDF, public profile, full-book result, scaling or author/human acceptance.
