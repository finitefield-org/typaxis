# ADR-0064: Translate measured content to selected horizontal page origins

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0061/0062/0063
from fixed horizontal origins to independently varying body and footnote origins.
Width changes still require source-aware reflow. Conflicting parallel/definition
page names, running regions, columns and full-book/public/managed-host/author/human
acceptance remain open.

## Decision

A page frame plan may retain different X origins when widths are equal. Keep the
measurement origins from the first unnamed page selection and retain every selected physical
origin separately. Do not reshape or rescale content merely to move it horizontally.
Width disagreement still produces HorizontalReflow; arbitrary-width layout is not
claimed by removing the origin-equality guard.

During actual mixed-page placement, compute independent deltas between selected and
measured body and footnote regions. Translate the complete body/definition fragments,
including table leaves and repeated headers, and each vector viewport exactly once.
Move list markers according to their bound fragment's region, and move original note
numbers with their definition. Checked arithmetic retains owner-specific failures.
Charge translated fragment/marker visits to the shared work budget, without making a
second source stream or resetting allowances. Zero deltas preserve the previous
placement path and output.

The separator already comes from the selected note bounds. Equation numbers are
placed after translating the parent bounds and viewport. Source closure verifies a
block viewport against its measured source position plus the selected region delta;
it does not waive the position check. Page stability compares the translated bounds,
viewports, marker geometry and selected rectangles. PDF drawing, annotations and
navigation consume those actual placements through the existing common pipeline.

## Evidence

Design 28 §14.198 and its progress ledger retain paired original/shifted fixtures for
ordinary body, footnotes, unequal table cells in body and definitions, named reference
and nested scopes, repeated table headers with lists and notes, and numbered vector
blocks. Original Harano pairs cover notes and cross-page references. Full-driver
work limits are checked exactly and one short. The source-normalized placement
comparison includes original ownership, source/item identity, bounds, viewports,
list/note/equation markers and separator geometry.

Independent PDF comparison derives translation amounts from original master
rectangles, checks actual text and annotation positions and named destinations, and
rejects unchanged or altered geometry. Original fixed-origin frame/name probes are
still checked separately. This is evidence for same-width horizontal translation;
it does not prove width-dependent reflow or full-book completion.
