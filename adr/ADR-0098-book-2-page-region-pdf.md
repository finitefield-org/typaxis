# ADR-0098: Bind running regions to actual page resources and PDF artifacts

## Status

Implemented on the private Book-2 staging path under design 28 §§14.234–14.235.
The §235 user-run regression passed: 246 CLI tests, 743 unchanged prior PDFs,
and independent verification of three running-region PDFs. The empty-join
budget correction is included in that run. Broader book acceptance remains open.

## Decision

Consume the complete page/header/footer ordered region displays before selecting
font resources. Check exact body, admitted ledger, limits, epoch, selected master,
source region owner, role, page and rectangle. Missing, duplicate and foreign
regions fail; an empty authored region still requires its sealed display. Keep
body paint order and include every running glyph in the shared subset/CID path.

Emit Header/Footer Pagination Artifacts on every occurrence. Assign no MCID or
body fragment authority to them. Navigation language records retain the original
region owner from source traversal: PDF source structure requires body language
nodes to exist and region language nodes to be absent. Unselected masters keep
their source language ownership without acquiring paints or semantic nodes.

The private driver builds running regions after physical pages are known, sharing
record/work allowances and remaining line passes. Existing non-region frame-plan
APIs still reject authored region content. PDF assembly requires corresponding
sealed displays; columns remain unsupported. Empty joins retain caller charges
and master-selection work and enforce record limits without changing fingerprints
or PDF content. Shaping-engine work and failed-candidate accounting across the
whole command are not completed by this change.

## Verification

Original Harano and controlled TrueType fixtures cover region-only glyphs,
multiple pages, empty regions, overflow before PDF callback and missing displays.
An unused-master case checks source/structure separation and exact/one-short
join work, retained prior records/work and excessive record rejection.

Probe output borrows the actual pipeline's CID plans and exports original font
bytes, subsets, mappings, source text and positions. Independent FontTools checks
verify original/subset outlines, metrics, mappings and extraction. The PDF checker
compares embedded programs, widths, CID maps, ToUnicode, every painted CID and
coordinate, pagination scopes without MCIDs, and the fixture's exact body
structure. Mutated resources, PDFs and source bindings must be rejected. These
checks are fixture-specific and are not arbitrary-book or PDF/UA acceptance.

Commands and evidence are recorded in progress §§234–235. Public profile,
full-volume, performance, managed-host and human/author approvals remain open.
