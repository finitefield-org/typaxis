# ADR-0037: Producer-composed math-vector placement

## Status

Accepted on 2026-09-03 as the producer-composed vector decision gate for M4.

This ADR extends only the non-current `typaxis.contract/1.4` and
`typaxis.machine-pdf/production-book-1` target reserved by
[ADR-0032](ADR-0032-semantic-container-and-declared-media.md). It adds a
versioned path beside, rather than inside, ADR-0033's native math and
`svg-safe-1` paths. It does not revise any closed decision in ADR-0033 through
ADR-0036, change current `typaxis.contract/1.3`, publish a contract-1.4 decoder
or Schema alias, register the production profile, or change the seven public
profile descriptors or `paragraph-1` default. MI4-V03 through MI4-V18 may
implement private slices, MI4-V19 may establish feature-local publication
readiness, and MI4-13 remains the sole atomic publication owner.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: wire kinds, media, metrics, layout, vector grammar, receipts, PDF mapping, accessibility, limits, diagnostics, and capability projection are closed here |
| implemented | Corpus/interface evidence only in MI4-V01; product implementation starts at MI4-V03 |
| public CLI E2E | No: public input/output remains contract 1.3 with seven profiles |
| release-supported | No: MI4-V19 and MI4-13 remain required |

### Adoption-branch evidence

The contract branch was resolved from repository state, not from a planned
version number:

| Evidence on 2026-09-03 | Observed state |
| --- | --- |
| `/usr/bin/git status --short --branch` | clean `main...origin/main` before this decision change set |
| `typaxis_core::CONTRACT` and `contracts/contract-version.md` | current canonical output is `typaxis.contract/1.3` |
| top-level Schema aliases and `samples/conformance/machine-capabilities.json` | current 1.3; accepted contracts end at 1.3 |
| public capability artifact | exactly seven profiles; default is `typaxis.machine-pdf/paragraph-1` |
| public CLI isolation tests | help/capabilities omit `production-book-1`; explicit selection is rejected as unknown |
| `docs/25` master milestone | MI4-13 is Pending |

Contract 1.4 is therefore unpublished and may receive this additive private
staging shape under ADR-0032's pre-publication rule. If MI4-13 publishes 1.4
before this feature reaches MI4-V19, no remaining shape may be added to 1.4;
a new contract/profile decision is required instead.

## Context and decision boundary

VMB can use `texToSvg` to produce final math outlines, exact source TeX,
speech/alternative text, a content hash, and fixed-point metrics. Requiring
Typaxis to parse or typeset that TeX would duplicate the producer and would not
preserve the producer's chosen visual result. Treating the SVG as an ordinary
Figure would lose baseline, advance, source, Formula semantics, line-breaking,
and equation-number ownership. Extending ADR-0033's native `inline_math` /
`display_math` nodes or `svg-safe-1` parser in place would change frozen `/1`
canonical records and public compatibility expectations.

The adopted path therefore consumes a precomposed vector resource plus
producer metrics and semantic bindings. Typaxis validates and places it but
does not interpret, normalize, or typeset TeX. The trust boundary still begins
at stable contained resource bytes: a producer hash, provenance record, metric,
or assertion that an SVG is safe is untrusted until the appropriate owner
issues a sealed receipt.

The detailed design input is
[docs/27](../docs/27-vmb-precomposed-math-vector.md). Existing fixed-point,
source/TextMap, style, line-breaking, selected-state, resource admission,
Display, PDF graph, manifest, diagnostic, limit, and atomic-publication rules
remain normative unless this ADR narrows them.

## Adopted identities

The following exact identities are immutable:

| Item | Identifier |
| --- | --- |
| wire media | `svg-safe-2` |
| production SafeVector component | `typaxis.resource-profile/safe-vector/2` |
| complete production resource set | `typaxis.production-book-resource-set/2` |
| safe-SVG parser | `typaxis.safe-svg-parser/2` |
| canonical vector IR | `typaxis.safe-vector-ir/2` |
| vector IR fingerprint | `typaxis.safe-vector-ir-fingerprint/2` |
| vector allocation charge | `typaxis.safe-vector-allocation-charge/2` |
| producer metric validation | `typaxis.precomposed-vector-metrics/1` |
| block vector style registry/cascade | `typaxis.precomposed-vector-style/1` |
| source/vector/alternative binding | `typaxis.precomposed-math-binding/1` |
| atomic inline itemization | `typaxis.atomic-vector-inline/1` |
| producer-composed math block flow | `typaxis.math-vector-flow/1` |
| inline/block selected layout | `typaxis.precomposed-vector-layout/1` |
| vector Display command/receipt | `typaxis.draw-vector-display/2` |
| content-key Form dedupe | `typaxis.vector-form-dedupe/1` |
| per-content Form/ExtGState plan | `typaxis.safe-vector-form-plan/2` |
| deduplicated Form plan set | `typaxis.safe-vector-form-plans/2` |
| vector PDF object/use closure | `typaxis.safe-vector-pdf-closure/2` |
| SafeVector resource/usage manifest | `typaxis.safe-vector-manifest/2` |
| producer-composed math manifest | `typaxis.math-vector-manifest/1` |
| computed language registry | `typaxis.computed-language-registry/2` |
| book-navigation profile view/receipt | `typaxis.book-navigation-profile-view/2`, `typaxis.book-navigation-profile-receipt/2` |
| selected/PDF/manifest navigation | `typaxis.book-navigation-selected/2`, `typaxis.book-navigation-pdf/2`, `typaxis.book-navigation-manifest/2` |
| PDF/UA profile and preflight | `typaxis.pdfua1-profile/2`, `typaxis.production-accessibility-preflight/2` |
| lower authorization and role vocabulary | `typaxis.production-accessibility-authorization/2`, `typaxis.structure-role-vocabulary/2` |
| structure/selected/marked-content | `typaxis.structure-registry/2`, `typaxis.selected-structure-binding/2`, `typaxis.marked-content-plan/2` |
| tagged observation/validator/manifest | `typaxis.tagged-pdf-observation/2`, `typaxis.tagged-pdf-validator/2`, `typaxis.tagged-pdf-manifest/2` |
| validation policy/assessment | `typaxis.pdfua1-validation-policy/2`, `typaxis.matterhorn-assessment/2` |

Every fingerprint under these identities is SHA-256 over an RFC 8785 JCS
record with the exact algorithm identifier. Arrays retain the order fixed in
this ADR; sets use the stated numeric or UTF-8-byte ordering. Changing an
accepted SVG token, metric or spacing meaning, style applicability, flow or
number allocation, alternative/language/structure mapping, dedupe key/order,
limit charge, PDF operator policy, or manifest closure requires the affected
next identity and a compatibility decision.

### Compatibility freeze

| Existing path | Frozen behavior |
| --- | --- |
| `svg-safe-1` | parser/IR/fingerprint/allocation `/1`, accepted bytes, canonical IR, Schema, and goldens are byte-frozen; it is never parsed by `/2` |
| native math/layout registries | `inline_math`, `display_math`, `typaxis.math-flow/1`, native `MathFlowId`, `typaxis.math-manifest/1`, `typaxis.basic-block-style-registry/1`, `typaxis.basic-flow-registry/1`, and `typaxis.semantic-container-flow-registry/1` canonical records/goldens are unchanged; there is no conversion in either direction |
| SafeVector `/1` | `typaxis.safe-vector-selected-layout/1`, `typaxis.draw-vector-display/1`, `typaxis.safe-vector-form-plan/1`, `typaxis.safe-vector-form-plans/1`, `typaxis.safe-vector-pdf-closure/1`, and `typaxis.safe-vector-manifest/1` remain frozen; `svg-safe-2` never falls back to them |
| metadata and XMP | `typaxis.document-metadata/1`, `typaxis.bcp47-language/1`, `typaxis.utc-second/1`, `typaxis.outline-registry/1`, and `typaxis.book-xmp/2` serialization remain frozen; the same metadata/language input produces the same XMP bytes |
| navigation `/1` | computed-language and all book-navigation view/receipt/selected/PDF/manifest `/1` records remain frozen |
| tagged PDF `/1` | role vocabulary, structure, marked-content, validation policy/evidence, and `typaxis.tagged-pdf-manifest/1` remain frozen |
| ADR-0036 resource set | `typaxis.production-book-resource-set/1` and its five component meanings remain frozen; `/2` preserves PNG/JPEG/TrueType/CFF ordering and replaces only SafeVector `/1` with `/2` |
| public capability bytes | the exact seven-profile descriptor, accepted-contract array, profile order, public help, and `paragraph-1` default remain byte-identical until MI4-13 |

The production resource set `/2` has this exact component order:

```text
typaxis.resource-profile/png/1
typaxis.resource-profile/safe-vector/2
typaxis.resource-profile/jpeg-baseline/1
typaxis.resource-profile/truetype-glyf/1
typaxis.resource-profile/sfnt-cff1/1
```

Its exact image-media order is
`png, svg-safe-1, svg-safe-2, jpeg-baseline`; its font-media order remains
`sfnt-truetype-glyf, ttc-truetype-glyf, sfnt-cff1`.
The `/2` SafeVector component dispatches admitted `svg-safe-1` only through
the frozen parser/IR `/1` chain and `svg-safe-2` only through parser/IR `/2`;
neither byte language is reinterpreted by or falls back to the other parser.

## Wire and resource contract

Contract 1.4 gains four explicit kinds. Kind is the sole source of placement
and semantic role; neither TeX nor the resource is inspected to infer it.

| Kind | Placement | Admitted media | Source TeX | PDF role and ActualText |
| --- | --- | --- | --- | --- |
| `inline_vector` | one atomic inline | `svg-safe-1` or `svg-safe-2` | forbidden | Figure; use only nonnull authored `actual_text`, no Alt fallback |
| `math_vector` | one atomic inline | `svg-safe-2` only | required | Formula; nonnull authored `actual_text`, otherwise exact Alt fallback |
| `vector_figure` | atomic block plus existing caption flow | `svg-safe-1` or `svg-safe-2` | forbidden | Figure; no paint-level ActualText |
| `math_vector_block` | atomic one-terminal flow | `svg-safe-2` only | required | Formula; nonnull authored `actual_text`, otherwise exact Alt fallback |

Existing `figure` accepts only its frozen `svg-safe-1` vector branch. Existing
native math accepts no vector resource. Decorative inline vectors are not part
of this production profile.

Both inline kinds have NodeId/SourceSpan, image ID, metrics, spacing, required
Alt, required nullable ActualText, and the existing optional language override.
Only `math_vector` additionally has required `source_tex`; that member is
forbidden on `inline_vector`. `math_vector_block` has NodeId/SourceSpan,
classes, image ID, metrics, required `source_tex`, required Alt, required
nullable ActualText, required nullable `equation_number`, and the existing
language override. `vector_figure` instead has NodeId/SourceSpan, classes,
image ID, positive viewport, required Alt, caption blocks, and the existing
language override; metrics, spacing, source TeX, ActualText, and equation
number are forbidden. Unknown or kind-inapplicable members are not ignored.

An `svg-safe-2` image declaration has required nonnull `expected_sha256` and a
required closed `vector_provenance` containing `engine_id`, `engine_version`,
and `rules_version`. Each string is nonempty printable ASCII and at most 128
bytes. The record is an audit assertion, not parser selection or proof that
Typaxis ran the producer engine. It is forbidden on other media. The node
references the existing typed `ImageResourceId`; raw SVG/XML, a URI, or a
safety boolean is never embedded in the node.

Duplicate image IDs are `P1102` before resource open. Different IDs may name
the same content and retain separate provenance. Stable admission recomputes
SHA-256 over the complete bytes and exact-matches the declaration. A declared
or injected equal digest with different full bytes is `R7100` resource
conflict, not an opportunity to choose one value.

## Metrics, source, and alternative records

`math_vector`, `inline_vector`, and `math_vector_block` carry the closed
`PrecomposedVectorMetrics` fields `advance`, `ascent`, `descent`, `origin_x`,
`baseline`, `viewport.width`, and `viewport.height`. JSON values are canonical
safe integers in the root `pdf_point_1_65536` coordinate unit. `origin_x` alone
is signed; spacing and descent are nonnegative. The following relations are
mandatory:

```text
advance > 0
ascent > 0
descent >= 0
viewport.width > 0
viewport.height > 0
0 <= baseline <= viewport.height
ascent >= baseline
descent >= viewport.height - baseline
```

For admitted intrinsic size `Iw`/`Ih` and node viewport `Vw`/`Vh`, checked
`i128` arithmetic derives exactly one positive unsigned 16.16 scale:

```text
s = round_half_even(Vw * 65536 / Iw)
scale(Iw, s) == Vw
scale(Ih, s) == Vh
```

`origin_x + Vw` must be checked. Nonuniform/x-y scaling, floats, SVG path
bounds, an ambient font size, or a root unit suffix cannot recompute node
metrics. Viewport overhang outside logical advance is permitted, but final
frame feasibility checks the visual bounds separately.

`vector_figure` carries only its positive `viewport`, not advance, ascent,
descent, origin, or baseline. Its admitted intrinsic width/height and viewport
must satisfy the same single-scale equations above. The aligned viewport
rectangle is its block geometry; no independent width property or nonuniform
scale may replace that check.

`source_tex.text_span` identifies nonempty exact UTF-8 without BOM or NUL. Its
TextMap has exactly one identity mapping to a SourceSpan contained by the
owner. Typaxis does not parse, trim, normalize, remove delimiters, or reformat
these bytes. `alt` is required, contains at least one Unicode 16.0 non-
`White_Space` scalar, and contains no C0/C1 control. Nonnull `actual_text` and
equation-number text obey the same meaningful/control rule. No value is
trimmed, normalized, or whitespace-collapsed.

`language` is an optional BCP 47 natural-language override for the alternative,
not a TeX dialect. Existing `typaxis.bcp47-language/1` canonicalization is
reused, while the complete owner registry is `/2`.

## Inline itemization and baseline placement

The only inline placement equations in the top-left/Y-down layout space are:

```text
viewport_left = pen_x + origin_x
viewport_top = line_baseline_y - baseline
line_baseline_y = viewport_top + baseline
```

The SVG bottom edge is never aligned to the text baseline. SafeVector's
intrinsic `viewBox` mapping remains inside the Form plan and cannot apply
`origin_x` or `baseline` a second time.

Each inline vector lowers to one `typaxis.atomic-vector-inline/1` item with no
internal break. In the initial horizontal/LTR profile it contributes one
synthetic source-provenance `AL` line-break unit and an atomic LTR isolate;
U+FFFC or other placeholder text is not inserted. Existing Unicode line-break
rules and the Japanese prohibition table decide each boundary.

Logical line width is `advance`. `spacing.before` and `spacing.after` are exact
nonnegative total gaps, represented by one boundary item carrying the existing
break kind/penalty and a same-line-only width. They do not create break
candidates, add Japanese natural gap/stretch/shrink, or make a prohibited
boundary breakable. A broken boundary has zero pre/post width; line-start
before-space and line-end after-space are zero. Adjacent vectors add the left
after and right before values in the one boundary item.

For every candidate line:

```text
content_ascent  = max(text_ascent, each_vector.ascent)
content_descent = max(text_descent, each_vector.descent)
content_height  = content_ascent + content_descent
extra_leading   = max(0, computed_line_height - content_height)
leading_before  = round_half_even(extra_leading / 2)
leading_after   = extra_leading - leading_before
line_height     = leading_before + content_height + leading_after
line_baseline_y = line_top + leading_before + content_ascent
```

Pagination advances by that line height. Final feasibility checks both logical
advance and `origin_x .. origin_x + viewport.width`. A whole vector/line may
move to the next line or frame. Failure to fit an empty line or empty full
frame is terminal `L5100`; no part is painted separately.

## Block style, numbering, and pagination

`math_vector_block` and `vector_figure` use only
`typaxis.precomposed-vector-style/1`. Both accept `space_before`,
`space_after`, `start_indent`, `end_indent`, `text_align`, `page`, and
`keep_with_next`; only `vector_figure` accepts `keep_caption`.
`math_vector_block`'s `font_family`, `font_size`, and `line_height` affect only
equation-number text. `width` is inapplicable to both, `keep_caption` is
inapplicable to math, and `font_family`/`font_size`/`line_height` are
inapplicable to the Figure owner (its caption resolves its own style). A known
inapplicable property is `L5101`. Existing cascade
precedence/value types are reused, but a frozen basic-style `/1` receipt cannot
authorize either kind.

`MathVectorFlowId` is nominally and numerically separate from native
`MathFlowId`. It is allocated densely from zero in validated
`math_vector_block` NodeId preorder before workers start. Each record binds its
owner, parent FlowId/position, metric/binding/style receipts, LayoutEpoch, and
exact terminal `1`. Deferral re-evaluates the same unconsumed flow; it never
issues an empty or second fragment.

`equation_number` is required nullable on math blocks. Nonnull has one
producer-owned nonwrapping text leaf, the owner-immediately-following dense
NodeId, depth owner+1, a contained identity TextMap, and positive
`minimum_gap`. Formula and number mappings are nonoverlapping and ordered by
`formula_source_span.end_byte <= equation_number.span.start_byte`. Typaxis
does not generate, increment, localize, wrap, or merge the number into SVG or
Formula ActualText. Null allocates no child or NodeId.

The number is at logical end; the formula continues to align in the entire
inner frame. Their vertical centers coincide and their rectangles must be
separated by `minimum_gap`. With number width/height `Nw`/`Nh`:

```text
Bh = max(viewport.height, Nh)  # numbered
Bh = viewport.height           # unnumbered
child_top = round_half_even((Bh - child_height) / 2)
```

The odd residual goes to block end. A nonpositive shaped number, overlap, or
width failure is `L5100`; the engine does not shift the formula, wrap the
number, shrink, or crop. Paint and structure order is formula then number.

Alignment uses viewport width, not block-math advance. Pre/post spaces and
indents reuse existing checked block rules: previous `space_after` and current
`space_before` are checked-added without collapse, `space_before` is suppressed
at a page/column start, pending glue does not cross that boundary, and trailing
`space_after` alone never creates a page. Formula/number `Bh` or vector-
figure viewport is one indivisible rectangle. If it does not fit the current
remainder but fits an empty next frame, the whole block moves; if it cannot fit
an empty full frame, the build fails `L5100`. `keep_with_next` is not silently
relaxed. Overflow policy is always `error`; there is no split, fit-to-width,
shrink, crop, rasterization, or page rotation. A vector Figure caption remains
the existing independent caption flow and `keep_caption` policy.

All four kinds count as authored semantic-container content because each has a
meaningful Alt and atomic paint. Path count, TeX, caption, or number presence
cannot make it empty.

## Safe-SVG 2 closed grammar

`svg-safe-2` inherits the complete Safe-SVG 1 lexical, geometry, unit,
fixed-point, transform, clip, and path-lowering rules in ADR-0033 and adds only
exact `currentColor`, presentation attributes `fill-opacity` and
`stroke-opacity`, resolved alpha in the IR, and PDF ExtGState planning.

The complete admitted element vocabulary remains `svg`, an optional leading
`defs`, `clipPath`, `g`, `path`, `rect`, `circle`, `ellipse`, `line`,
`polyline`, and `polygon`. Geometry accepts only its ADR-0033 attributes.
Path commands remain `M/m L/l H/h V/v Q/q C/c Z/z`. Root dimensions,
`viewBox`, exact namespace, unitless/px/pt units, decimal grammar, axis-aligned
matrix/translate/scale, local closed clip paths, and canonical
Move/Line/Quadratic/Cubic/Close lowering are unchanged.

The complete non-geometry attribute vocabulary is the Safe-SVG 1 set plus
`fill-opacity` and `stroke-opacity` on `g` and paint geometry only. They are
presentation attributes, inherit through the same nesting as paint, start at
exact one, and a specified child value replaces rather than multiplies the
inherited value. They are forbidden on clip geometry and in CSS/style.

`fill`/`stroke` additionally accept only the entire exact ASCII value
`currentColor`. Case aliases, leading/trailing whitespace, `inherit`, `var()`,
and an SVG `color` property are forbidden. The IR paint enum is
`None | FixedRgb8 | CurrentColor`. The only resolution source is the sealed
placement's resolved text paint; it is exact black while the production style
domain has no authored color property. Resolution changes require a style and
receipt identity decision.

Alpha lexical values are exactly `0`, `1`, `0.` followed by one through six
digits, or `1.` followed by one through six zeroes. Signs, `.5`, `1.`, leading
zeroes, exponents, and surrounding whitespace are forbidden. Values lower by
round-half-to-even to unsigned 16.16 fill/stroke alpha.

The following remain terminal errors and are never ignored: malformed XML or
UTF-8; BOM/NUL/forbidden controls; entity/DOCTYPE/XML declaration/comment/
CDATA/processing instruction; unknown/duplicate/prefixed element or attribute;
unknown path command or arity; forward/missing/cyclic/unused/external clip;
`script`, events, animation, `foreignObject`; `image`, raster data, font,
`text`, `tspan`; CSS, `style`, selectors, media queries; `href`, XLink,
`xml:base`, data/file/network reference; `use`, symbol, marker, gradient,
pattern, mask, filter, blend mode, group/object `opacity`; rotation/skew,
unsupported units, nonfinite/degenerate geometry, or unknown extension.

A producer that emits `defs/path/use` must expand it before packaging under a
versioned producer rule. Typaxis does not add a producer-specific preprocessor
or general SVG/browser fallback.

Each Form has an ExtGState per distinct `(fill_alpha, stroke_alpha)`, including
`(1,1)`, ordered by the numeric pair. Every draw explicitly selects its pair;
the dictionary adds only `/Type /ExtGState`, `/ca`, and `/CA`. CurrentColor is
set in both stroking and nonstroking state immediately before `Do`. Draw and
placement state is isolated by `q`/`Q`. `/BM`, `/SMask`, `/AIS`, an isolated
transparency group, or ambient alpha is not used. A resource without any
enabled positive-alpha fill or stroke is rejected. The root viewport clip is
outermost; admitted local clips serialize deterministically as `W`/`W*` plus
`n`.

## Display, Form, dedupe, and PDF

The closed resource path is:

```text
declaration + stable bytes/hash/provenance
  -> bounded parser/IR attestation
  -> metric/source/alternative binding
  -> selected atomic placement
  -> DrawVector Display /2
  -> deduplicated Form plan set /2
  -> PDF Form XObject + page-local Do
```

Display carries logical usage/owner/kind/image/content key, IR and selected
fingerprints, page/frame/paint ordinal, viewport, uniform matrix, and resolved
paint. Inline/math use additionally binds pen origin and baseline. It carries
no URI, SVG/XML, TeX, PDF name, or object number. PDF never reparses SVG and
emits path/clip/fill/stroke/ExtGState operators, not a raster image XObject.
The admitted intrinsic viewport is Form `/BBox`; node translation/uniform
scale and the one docs/24 page-root Y flip are applied exactly once.

The dedupe key is the typed tuple:

```text
VectorContentKey(
  source_sha256,
  media_type,
  safe_svg_parser_id,
  vector_ir_id,
  vector_ir_fingerprint
)
```

The source hash is computed over admitted full stable bytes. Resource ID,
NodeId, page, provenance, URI, selected color, and first-use order are not key
members. Different source hashes do not dedupe merely because IR matches; the
same bytes under different media/parser semantics produce different keys.

Relative Form roles and resource names are assigned by component-wise lexical
order: 32-byte source hash, media UTF-8, parser ID UTF-8, IR ID UTF-8, then
32-byte IR fingerprint. There is no ambiguous concatenated string key. Form-
local ExtGState names use numeric alpha-pair order. Only the complete final PDF
graph owner merges all vector/nonvector roles and assigns absolute object
numbers. Hash-map insertion, resource declaration, first page use, and worker
completion order have no authority.

All aliases of one key share one Form while retaining per-image provenance and
usage counts. Zero-use resources retain their admitted facts but allocate no
Form/name/object/`Do`. Positive use requires one Form plus the exact number of
placement uses. Resolved currentColor changes placement state, not the Form key.

## Alternative, language, structure, and manifests

Syntax/layout issues `typaxis.precomposed-math-binding/1` over contract,
profile, package/session/limits/epoch, owner/source/language, exact TeX span and
hash, Alt/resolved-ActualText hashes, image/stable hash/media/provenance,
parser/IR, metrics, spacing or computed style, and resolved paint. Selected
layout, Display, Form/PDF, and structure append their own receipts; no upstream
record contains a downstream MCID, object number, or StructureNodeId.

The PDF mapping is closed:

| Kind | Structure and page marked content |
| --- | --- |
| `math_vector` | one Formula; outer Formula MCR and inner MCID-less property-only Span around `Do` |
| `math_vector_block` | one Formula; same vector MCR/Span, followed by optional source-owned number Span child |
| `inline_vector` | one Figure; outer Figure MCR and an inner property-only Span only when nonnull ActualText or paint Lang is needed |
| `vector_figure` | one Figure using existing Figure/caption policy; inner property-only Span only when paint Lang is needed |

Structure `/Alt` is exact authored Alt. Formula paint `/ActualText` is exact
authored nonnull text or exact Alt fallback; inline Figure uses only authored
nonnull text; block Figure emits none. Structure `/Lang` appears only when the
computed owner language differs from its nearest structure parent, and paint
`/Lang` only when it differs from document language. The outer semantic MCR
owns the MCID. The inner Span has no MCID and encloses only `Do`. Reusable Form
streams contain no MCID, Alt, ActualText, or Lang. Math cannot become Artifact.

The equation-number Span follows the Formula vector MCR in logical reading
order, inherits the parent's computed language, and uses its own TextSpan and
glyph/extraction receipt. It is never duplicated into Formula ActualText.
Opaque TeX remains in TextStore plus manifest span/hash closure; no custom PDF
dictionary key or attachment stores it.

The production manifest remains acyclic:

```text
SafeVector manifest /2 -> math-vector manifest /1 -> tagged-PDF manifest /2
book-navigation manifest /2 is a sibling projection and is not back-referenced
```

Built production artifacts require complete nonnull SafeVector, math-vector,
and tagged-PDF record/fingerprint pairs even when their arrays are canonically
empty. Failed artifacts permit only both-null or both-nonnull pairs according
to the last completed owner; empty synthetic receipts are forbidden. Resource
facts are content-key ordered, aliases numeric-image-ID ordered, and placement
facts selected-paint ordered. Facts preserve hashes, sizes, media, provenance,
parser/IR/allocation, intrinsic viewport, content key, conditional final Form,
total and alias counts, source/alternative/language, all metrics/style/flow,
page/frame/paint, rectangle/matrix, and Display/PDF/structure fingerprints.

## Limits, diagnostics, and failure side effects

All maxima are inclusive and max+1 is refused before allocation, issuance, or
serialization. Retries and dedupe do not reset unrelated budgets.

| Work | One-time charge and stable failure |
| --- | --- |
| encoded SVG | `max_image_bytes` plus aggregate `max_resource_bytes`, `R7100` |
| elements/path/depth | `max_vector_nodes` / `max_vector_path_segments` / `max_vector_nesting_depth`, `R7120` / `R7121` / `R7122` |
| Safe-SVG 2 IR allocation | checked `64*nodes + 80*stored_segments + 48*paint_or_clip_commands + source_clip_id_bytes` against `max_decoded_image_bytes`, `R7111` |
| TeX TextBuffer/slice | existing admitted-buffer charge once, slice per-buffer recheck only, `T2100`/`T2101` |
| Alt, nonnull ActualText, number text | each authored string once; null math fallback aliases Alt and is not recharged, `T2100`/`T2101` |
| explicit/computed language | ADR-0034 raw/canonical/owner charges once across the `/2` registry, `T2100`/`T2101` |
| semantic vector/number nodes | existing `max_ast_nodes`/`max_ast_nesting_depth`, `P1120`/`P1121`; the one flow does not add a second AST charge |
| selected vector occurrence | one auxiliary `max_fragments` unit per occurrence, `L5110` |
| relative Form/ExtGState role planning | checked count delta only; no global object charge here |
| complete final indirect-object graph | all actual vector/nonvector objects once before absolute allocation, `max_pdf_objects`, `G6100` |
| Form/page spool and output | existing simultaneous-live `max_spool_bytes` and final `max_output_bytes` owners |

Identical content does not waive per-declaration stable-byte/IR admission work.
Dedupe reduces only Form-plan/object work after admission.

| Condition | Phase and code |
| --- | --- |
| missing/unknown/wrong-typed node, metric, source, alternative, hash, or provenance member | strict decode/syntax `P1102` at exact JSON Pointer |
| invalid meaningful text, TextSpan, metric relation, scale, aspect, NodeId/depth/order, or number gap | syntax/profile `P1102` or existing text-map code |
| profile-disallowed media | pre-resource `R7100` |
| malformed/forbidden/external/unsupported SVG, hash mismatch, resource conflict | admission `R7100` with typed reason |
| vector count/path/depth or allocation max+1 | `R7120`/`R7121`/`R7122`/`R7111` |
| text/AST/selected/PDF-object max+1 | `T2100`/`T2101`, `P1120`/`P1121`, `L5110`, `G6100` |
| inline logical/visual, block/page, or equation-number collision/oversize | layout `L5100` with NodeId/SourceSpan |
| known inapplicable block property | layout/style `L5101` |
| flow, selected, Display, Form, PDF, manifest, language, or structure closure mismatch | internal `I9190` |

`R7100` reasons distinguish at least `malformed_svg`, `forbidden_feature`,
`external_reference`, `unsupported_feature`, `hash_mismatch`, and
`resource_conflict`. Unsupported input is never warning-only. A terminal
failure follows the existing atomic publication order: it cannot omit the
element and return PDF success, create a raster/native-math fallback, or invent
an empty downstream receipt.

## Capability descriptor and publication

MI4-V17 may add the following complete projection to the crate-private
`production-book-1` staging descriptor. Only MI4-13 may merge the same bytes
into the registered public descriptor and publish it. Existing fields merge
these new kinds; new `vector_*` fields are complete values. Set-valued arrays
and their map values use UTF-8 byte order; object keys use JCS UTF-16 order.

```json
{
  "blocks": ["math_vector_block", "vector_figure"],
  "image_formats": ["jpeg", "png", "svg"],
  "inlines": {"kinds": ["inline_vector", "math_vector"]},
  "style_block_types": ["math_vector_block", "vector_figure"],
  "style_selectors": ["math_vector_block", "vector_figure"],
  "vector_features": [
    "clip-path",
    "current-color",
    "paint-opacity",
    "shared-form-xobject"
  ],
  "vector_features_by_profile": {
    "svg-safe-1": ["clip-path", "shared-form-xobject"],
    "svg-safe-2": [
      "clip-path",
      "current-color",
      "paint-opacity",
      "shared-form-xobject"
    ]
  },
  "vector_formats": ["svg"],
  "vector_media_by_kind": {
    "figure": ["svg-safe-1"],
    "inline_vector": ["svg-safe-1", "svg-safe-2"],
    "math_vector": ["svg-safe-2"],
    "math_vector_block": ["svg-safe-2"],
    "vector_figure": ["svg-safe-1", "svg-safe-2"]
  },
  "vector_metrics": [
    "advance",
    "ascent",
    "baseline",
    "descent",
    "origin_x",
    "viewport"
  ],
  "vector_profiles": ["svg-safe-1", "svg-safe-2"]
}
```

`image_formats` is the complete coarse family array, not media-profile names.
Exact media order lives in resource-set `/2` and
`vector_media_by_kind`. At publication the profile array grows from seven to
eight, with `production-book-1` after `paragraph-1` and before `table-1` in
UTF-8-byte order. `default_profile` remains `paragraph-1`. Before MI4-13,
public capabilities advertise none of these additions even if a private
Schema, unit test, or crate-private runner exists.

The master release plan owns only milestone status/dependencies. The detailed
work and acceptance criteria remain in
[docs/27 task plan](../docs/27-vmb-precomposed-math-vector-todo.md):

```text
MI4-V01 -> MI4-V02 -> MI4-V03 ... -> MI4-V18 -> MI4-V19 -> MI4-13
MI4-V18 + MI4-11 + MI4-12 -> MI4-V19
```

MI4-V03 through V18 must remain private. V19 proves feature-local external
evidence and handoff readiness but cannot switch a public alias. MI4-13 must
publish contract, Schema registry/current aliases, resource set `/2`, profile,
capability, language/navigation `/2`, tagged-PDF `/2`, manifest dispatch,
fixtures, and evidence atomically. It cannot publish the producer-vector path
while retaining any `/1` chain or omit JPEG/CFF dependencies. MI4-11 and MI4-12
retain their existing JPEG and CFF scopes.

## Rejected alternatives

- Typaxis-side TeX parsing/typesetting or speech generation: duplicates the
  producer and changes the requested visual/source boundary.
- Reusing native math kinds or `typaxis.math-flow/1`: changes their frozen
  grammar, font, identity, and layout semantics.
- Extending `svg-safe-1` or its `/1` receipts: changes accepted bytes and
  canonical output under a frozen identity.
- General SVG/browser rendering: imports CSS, script, animation, fonts,
  external resources, platform behavior, and silent unsupported-feature risk.
- PNG/raster fallback: loses scalable path paint and makes unsupported input
  appear successful.
- General one-page PDF fragment import: has a much wider object, action,
  filter, font, resource, and security surface; a future closed
  `pdf-form-safe-*` path needs a separate ADR.
- Dedupe by resource ID, first use, provenance, or unverified caller hash:
  either misses equal content or grants untrusted ordering/identity authority.
- Publishing a partial eighth profile: violates descriptor/preflight
  equivalence and ADR-0032's atomic migration.

## Consequences

VMB can hand Typaxis a deterministic, scalable, accessible placement without
asking Typaxis to understand TeX. The cost is a new wire/domain/layout/PDF
vertical path and coordinated `/2` navigation, SafeVector, resource-set, and
tagged-PDF identities. Existing native math, Safe-SVG 1, old profile bytes, and
public CLI behavior remain independently testable and immutable.
