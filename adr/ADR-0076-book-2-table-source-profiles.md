# ADR-0076: Converge heterogeneous table paragraphs using source profiles

## Status

Accepted incrementally under design 28 on 2026-09-10. Connects heterogeneous
body/note table paragraphs to the private PDF driver. Conflicting repeated
header/caption frames and changing table-local block frames remain open.

## Decision

Translate source-closed physical table occurrences into paragraph width and
horizontal-origin profiles indexed by original logical units. Remeasure each
root occurrence through the original table hierarchy at its actual parent width.
Use original owner/unit ranges to locate selected lines; do not retain transient
line or leaf indexes across shaping passes. Validate root coverage, including
zero-height tables and referenced versus unreferenced definitions. Require every
observed source unit to have exactly one semantic occurrence.

Repeated pieces may share the profile only when their width and origin agree.
Reject conflicting repeated frames with `table_repeated_frame_reflow`; silently
overwriting one occurrence would authorize incorrect columns. Keep changing
block parent frames explicit as `table_block_frame_reflow` until block profiles
can carry both physical width and origin. Fixed block frames remain independently
checked against remeasured source parents.

Expose an explicit occurrence-frame mode on exact-flow source assignments and
retained frames. Require paired origin profiles; frameless shaping rejects the
mode. Include mode and all origins in separate assignment/frame digests. Ordinary
root-table feedback and callers preserve their previous encoding and behavior.

The private driver first uses existing root-wide feedback. Only its exact
`table_continuation_width_reflow` diagnostic switches the same search to source
profiles, preserving charges from the failed attempt. Later passes retain this
mode. Rebind widths and origins to each fresh source flow, preserve maximum source
envelopes rather than imposing one root width, and compare every profile before
acceptance. Generated-label changes invalidate the candidates as before.

Reuse the existing width-cycle detector and retained legal source line ends for
heterogeneous table paragraphs. Subsequent passes may subdivide but cannot merge
across retained boundaries. Preserve shared work, record, reshape and page-pass
limits, plus consecutive identical final PDF candidates. Precharge profile copies,
source lookups, comparisons and occurrence reprojections; no failed attempt refunds
its budget. Per-occurrence full projections and charged linear source-owner
lookups still require full-book performance work.

Final math processing independently remeasures physical occurrences and verifies
selected widths and origins against the original cell/caption hierarchy. The
explicit mode replaces only the root-wide single-width check; ordinary paragraph
and block checks, source closure and resource identity remain mandatory. Fixed
page plans with explicit origins also require final verification. A one-raw origin
displacement at unchanged width must fail in either mode.

## Evidence and remaining work

Design 28 §14.210 and the progress ledger record controlled and unchanged Harano
body/note PDFs for natural continuation, split paragraphs and a forced break in
a caption. The dedicated independent checker obtains original paragraph IDs via
the PDF ParentTree and verifies source coverage, declared physical columns,
start/center/end alignment and original font metrics. Shared budget boundaries and
negative origin checks remain exercised by component and driver tests.

This does not complete conflicting repeated-header variants, varying table-local
block frames, full-book scaling, public profiles/manifests, managed-host runs or
author/human acceptance. No author approvals or source text are fabricated.
