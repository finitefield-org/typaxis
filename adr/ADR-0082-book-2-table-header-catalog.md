# ADR-0082: Choose repeated headers from physical body and footnote frames

## Status

Accepted incrementally under design 28 on 2026-09-10. Mixed physical-page
selection can choose frame-validated header associations. Variant-aware placement,
source closure, display resources and PDF integration remain open.

## Decision

Add a sealed catalog of original root-table indexes, actual parent widths and
exact header associations from ADR-0080. Require strictly ordered, nonempty,
unique entries and the exact base measurement and effective limits. Derive widths
from actual variant frames, rather than accepting caller-supplied width labels.
Independently reproject each original table hierarchy at that width and check each
header paragraph's selected line width and source-unit start, or each block's
existing region. A matching root width does not authorize a mismatched leaf frame.

Map a physical body or footnote width to the root parent's width by retaining the
original hierarchy's inset and marker/gap contribution. The mixed scheduler uses
the active physical page frame in both body and definition candidate enumeration
and evaluation. After the original header has been consumed, choose the exact
catalog entry and run ADR-0081 selection with its actual header height. A missing
width remains an explicit failure. Initial source headers and tables without
headers retain their existing selection path.

Bind the catalog before demand/source states are issued, validate the exact flow
and limits, and preserve the shared work/record ledger across table evaluation.
Precharge catalog storage and hierarchy reprojections, preserve the larger shared
input history, and add independent measurement/projection charges outside that
maximum. When binding to a live search, conservatively add the entire catalog
charge to the search ledger, including overlapping history; neither graph hides
behind a maximum. Charge binary lookups and hierarchy work. Earlier-capacity
retries temporarily use the chosen header height and restore ordinary mode.

Permit only an internal scheduler conversion of ADR-0081's separate selection.
The converted mixed fragment retains the exact header association and combined
fingerprint. Semantic ranges and cursors continue to use the original base once;
variant paint queries explicitly attach the correct measurement owner to every
item index. Existing single-owner cell/caption/source placement queries reject
`table_header_variant_placement`, so these selections cannot accidentally issue
legacy placed pages or PDF paint authority.

## Evidence and remaining work

Design 28 §14.216 and its progress ledger record controlled and original Harano
body/note tests with ordinary and nested headers, alternating narrow/wide physical
pages, single source consumption, exact paint owners, mismatched leaf-frame
rejection and cumulative budget boundaries. Local full-suite and independent PDF
regressions are recorded there.

Catalog inputs are explicitly rebuilt line variants. The private PDF driver still
needs to construct those candidates from its physical occurrences, preserve variant
owners through placement/source closure, and close actual variant glyphs/resources.
Stable geometry verification currently depends on placement and is not claimed for
these mixed selections. `table_repeated_frame_reflow` remains on the private PDF
driver. Running regions, columns, remaining page-name scopes, full-book/public/
manifest/managed-host/scaling and actual author/human acceptance remain open.
