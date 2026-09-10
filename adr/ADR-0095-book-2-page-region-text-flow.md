# ADR-0095: Prepare and shape source-bound header/footer text independently

## Status

Implemented incrementally under design 28 on 2026-09-11. This decision covers
source paragraph preparation and font shaping. Page-region layout, repeated
Artifact placement, resource closure and PDF integration remain outstanding.

## Problem

Book-2 retains authored header/footer regions and validates their source spans,
but its body text flow does not include them. Feeding these paragraphs into the
semantic body would select body pages and consume their content as body text.
The private page/PDF drivers currently reject authored page-region content.

## Decision

Prepare a separate `BookV2PageRegionTextFlow` from an actual
`BookV2SelectedPageMaster`, a Header/Footer choice and the exact source navigation
owner. Borrow the original region and text buffers. Preserve paragraph/heading
owners, inline source spans, language, soft breaks, hard breaks and ordinary
computed styles. A region paragraph cannot select another body page.

Use the shared paragraph carrier with a different navigation type parameter;
it cannot be passed to a body consumer as `PreparedBookV2TextFlow`. The wrapper
retains the exact body/navigation owners, master id and original region. Its
fingerprint uses a separate domain and binds the complete canonical input,
master identity, region owner and role. Selecting the same master on another
physical page preserves this source identity. A same-hash reparse or separately
prepared navigation does not substitute for its retained owner.

Precount paragraph, inline and event records before allocating those arrays,
including caller-provided prior records. Retain existing AST, text and ordinary
style limits. This preparation count is not a command-wide work ledger; the
later page-region driver must include preparation, reshaping and failed attempts
in its own cumulative budgets.

Add a closed page-region input to the shared bidi/grapheme/shaping engine. Use
admitted production resource-set /3 font instances and the existing TrueType
and CFF /2 coverage checks. Bind a separate shape owner to the exact region flow,
resource ledger, effective limits, epoch and selected line contexts. Never
invent a font, language, text span or body structure node for a page region.

Keep the page-plan and PDF `UnsupportedPageMaster` guards until region geometry,
repeated Artifact paint and actual font/PDF resource closure are connected.
Source preparation and glyph shaping alone cannot authorize dropping header or
footer content, public profile registration, or a PDF/UA claim.

## Verification

Dedicated tests cover original-source borrowing, body-flow separation, actual
selected-master lookup, absent regions, distinct roles, source/navigation
identity rejection, exact/one-short record limits, paragraph/heading text,
soft/hard breaks and chosen line contexts. The original Harano font shapes
Japanese page-region text and rejects a context boundary inside a UTF-8 scalar.
A compile-fail example checks that the region carrier cannot become a body flow.
Existing body and PDF regressions and precise outcomes are recorded in progress
section 231; no new header/footer PDF is claimed by this increment.
