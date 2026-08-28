# ADR-0033: Math, safe-vector, and alternative binding

## Status

Accepted on 2026-08-28 as the math/vector/accessibility decision gate for M4.

This ADR extends only the non-current contract-1.4 target reserved by
[ADR-0032](ADR-0032-semantic-container-and-declared-media.md). It does not
change the current `typaxis.contract/1.3`, publish a contract-1.4 decoder or
Schema alias, register `typaxis.machine-pdf/production-book-1`, add a public
CLI selector, or claim layout, PDF, accessibility, or release support. MI4-04
and MI4-05 may implement this decision through private staging. MI4-13 remains
the sole publication gate.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: the math source/binding and safe-vector subset are closed here |
| implemented | No: the wire/domain/parser/layout/PDF additions are assigned to MI4-04 and MI4-05 |
| public CLI E2E | No: public commands still reject contract 1.4 and the target profile |
| release-supported | No: tagged structure, combined evidence, and publication remain later gates |

## Context

The document model has no lossless inline or display math node. Flattening an
expression to text discards its visual structure; replacing it with PNG
discards scalable paint, source identity, and semantic alternative; and
carrying only rendered paths loses the authored source and extraction value.
The source, layout input, vector paint, speech alternative, SourceSpan, and
selected PDF observation therefore need one tamper-evident identity.

The resource model also needs a scalable image format. Accepting general SVG
would import browser XML, CSS, scripting, animation, font lookup, external
resource, and filter semantics into the deterministic resource boundary.
Accepting caller-authored vector commands as already trusted would merely move
the parser boundary upstream. This ADR instead adopts a deliberately small
SVG-shaped byte language, validates it from stable resource bytes, and lowers
it to a backend-neutral canonical vector IR before layout or PDF work.

The design inputs are
[docs/25](../docs/25-machine-input-pdf-improvements.md) sections 7 and 13.4,
ADR-0002, ADR-0003, ADR-0008, ADR-0009, ADR-0012, ADR-0013, ADR-0022,
ADR-0027, and ADR-0032, plus invariants I-009, I-019, I-025, I-032, I-034,
I-053, I-063, I-065, I-067, I-068, I-073, I-074, and I-078. Existing
source, resource, selected-state, Display, PDF, limit, diagnostic, and
atomic-publication rules remain normative unless this ADR narrows them.

## Adopted identities

The following algorithm and vocabulary identities are fixed:

| Item | Identifier |
| --- | --- |
| math source language/version | `typaxis-math` / `1` |
| combined source dialect identity | `typaxis.math-source/1` |
| in-tree math parser | `typaxis.math-parser/1` |
| in-tree canonical math formatter | `typaxis.math-formatter/1` |
| parsed math fingerprint | `typaxis.math-ast-fingerprint/1` |
| validated source/visual/alternative binding | `typaxis.math-binding/1` |
| display-math flow owner | `typaxis.math-flow/1` |
| fixed-point math layout | `typaxis.math-layout/1` |
| math layout work accounting | `typaxis.math-layout-work/1` |
| safe-vector wire media value | `svg-safe-1` |
| in-tree safe-SVG parser | `typaxis.safe-svg-parser/1` |
| canonical vector IR | `typaxis.safe-vector-ir/1` |
| vector IR fingerprint | `typaxis.safe-vector-ir-fingerprint/1` |
| deterministic vector allocation charge | `typaxis.safe-vector-allocation-charge/1` |

Every fingerprint named here is SHA-256 over an RFC 8785 JCS record whose
required `algorithm` member is the corresponding identifier. The math-AST
record uses tagged nodes in semantic preorder, ordered row children, explicit
nullable subscript/superscript members, and exact identifier/number/symbol
token bytes. The vector-IR record uses definition preorder for dense ClipIds,
paint preorder for draws, ordered path/clip arrays, fixed enum strings, and
only the checked fixed-point integers defined below. JCS does not sort arrays,
erase source spellings, or normalize allocation order on either path.

Changing the accepted math grammar, source normalization, formatter output,
safe-SVG element/attribute set, numeric conversion, path lowering, paint
defaults, or allocation charge requires the corresponding `/2` identity and
a contract/profile compatibility review. An optimization may retain `/1`
only when all receipt, canonical-format, IR, Display, and PDF goldens remain
identical.

## Contract-1.4 math nodes

Contract 1.4 gains two distinct closed nodes. Inline math is an `inline`
alternative:

```json
{
  "kind": "inline_math",
  "math_source": {
    "language": "typaxis-math",
    "text_span": {"end_byte": 5, "start_byte": 0, "text_id": 0},
    "version": "1"
  },
  "node_id": 12,
  "span": {"end_byte": 5, "source_id": 0, "start_byte": 0},
  "speech": "x squared"
}
```

Display math is a styleable general `block` alternative:

```json
{
  "classes": [],
  "kind": "display_math",
  "math_source": {
    "language": "typaxis-math",
    "text_span": {"end_byte": 5, "start_byte": 0, "text_id": 0},
    "version": "1"
  },
  "node_id": 12,
  "span": {"end_byte": 5, "source_id": 0, "start_byte": 0},
  "speech": "x squared"
}
```

In both examples, the package TextMap maps the shown byte ranges to the exact
five UTF-8 bytes `x^{2}`. The examples use canonical JCS member order. Every
shown member is required, `additionalProperties` is false, and `math_source`
is also closed. There is no generic `math` node plus caller boolean, no
delimiter inference, and no automatic promotion from a text inline or
paragraph. The owning node kind is the sole inline/display distinction;
source bytes never include `$`, `$$`, `\(`, `\)`, `\[`, or `\]` delimiters.

Inline math is allowed wherever an ordinary inline is allowed. Display math
is allowed in the document body and every general block slot already admitted
by the selected production feature, including a semantic container, list
item, table cell, figure caption, or footnote definition. It remains forbidden
in the restricted page-region grammar. Display math is one atomic block and
does not split internally across lines, columns, or pages. If it cannot fit an
empty full frame under the adopted computed style, the normal terminal
oversize failure is `L5100`; it is never scaled, clipped, rasterized, or split
into unrelated semantic nodes.

The `math_source.text_span` identifies the exact admitted UTF-8 bytes parsed by
the math owner. The node SourceSpan must map through the package TextMap to
that same byte range and SourceId. A caller cannot provide source bytes outside
the node span, point two nodes at an ambiguously mapped range, or substitute a
different text buffer after validation. Dense NodeId preorder and ordinary
source containment continue to apply.

`speech` is a direct producer-authored UTF-8 string rather than a SourceSpan.
It must contain at least one scalar that is not Unicode 16.0 `White_Space`,
must not contain U+0000 through U+001F or U+007F through U+009F, and is charged
as described below. It is not trimmed, case-folded, locale-transformed,
Unicode-normalized, or inferred from visible source. The future
document/language receipt applies language to the math structure; this ADR
does not synthesize a language tag.

Contract 1.4 also adds the exact style selector block type `display_math`;
inline math has no classes or independent selector and inherits its owning
inline run's resolved text style. Existing typed properties have these closed
display meanings:

| Property | Display-math use |
| --- | --- |
| `font_family`, `font_size` | select the one admitted MATH-table face and checked math size |
| `line_height` | minimum block strut; intrinsic ascent + descent may enlarge it but is never clipped |
| `text_align` | place the atomic box at start, at center with any odd residual fixed-point unit on the logical end side, or at end of the indented residual inline size |
| `space_before`, `space_after`, `start_indent`, `end_indent` | existing typed outer glue and residual-frame geometry |
| `keep_with_next`, `page` | existing typed keep and page-selection behavior |

For positive extra strut leading, `extra / 2` is rounded half to even and
assigned to block-start; the exact residual goes to block-end. `width` and
`keep_caption` are known but inapplicable `L5101`; no math-only raw style,
source command, or delimiter can override the typed result.

## `typaxis.math-source/1` grammar and normalization

The adopted source is a small TeX-shaped expression language, not TeX,
LaTeX, MathML, AsciiMath, or an extensible macro language. Its lexical rules
are closed:

- input is exact UTF-8 without BOM, NUL, CR, tab, or C0/C1 controls;
- ASCII space and LF are the only insignificant whitespace and are ignored
  outside tokens;
- identifiers are one or more ASCII `A`-`Z` or `a`-`z` bytes;
- numbers are `(0|[1-9][0-9]*)(\.[0-9]+)?`; leading zeroes are forbidden,
  digit-starting tokens consume the longest valid number, `.` alone remains a
  literal symbol, and signs are separate operator tokens. A leading `0`
  followed immediately by another digit is a lexical error rather than two
  adjacent number tokens;
- literal operators/delimiters are the closed set
  `+ - ± ∓ × ÷ · = ≠ < > ≤ ≥ ≈ ≡ ∼ ∝ ∈ ∉ ∋ ⊂ ⊆ ⊃ ⊇ ∪ ∩ ∧ ∨ ¬ ∀ ∃ ∅ ∞ ∂ ∇ ∑ ∏ ∫ → ← ↔ ↦ ( ) [ ] | ‖ , . : ; !`;
- literal Greek symbols are exactly
  `α β γ δ ε ζ η θ ι κ λ μ ν ξ ο π ρ σ τ υ φ χ ψ ω`,
  `Α Β Γ Δ Ε Ζ Η Θ Ι Κ Λ Μ Ν Ξ Ο Π Ρ Σ Τ Υ Φ Χ Ψ Ω`, and the variant forms
  `ϑ ϕ ϖ ϱ ϵ`;
- `{` and `}` create a nonempty group, `_` and `^` attach at most one
  subscript and one superscript to the immediately preceding atom, and either
  script order parses to the same typed pair;
- the only backslash commands are `\frac{row}{row}`, `\sqrt{row}`, and
  `\operator{ascii identifier}`; and
- an expression is one nonempty row of atoms. A group, fraction argument,
  radical argument, script argument, or operator name cannot be empty.

The recursive grammar, after lexical tokenization, is:

```text
source    = row EOF
row       = term { term }
term      = atom [ scripts ]
scripts   = subscript [ superscript ]
          | superscript [ subscript ]
subscript = "_" argument
superscript = "^" argument
argument  = atom
atom      = identifier | number | literal-symbol | "{" row "}"
          | "\frac" "{" row "}" "{" row "}"
          | "\sqrt" "{" row "}"
          | "\operator" "{" identifier "}"
```

An `atom` used as an unbraced script argument is unscripted; nested scripts
require braces. The parser rejects a third script, duplicate subscript or
superscript, missing delimiter, trailing token, or empty `row`.

Unknown control sequences, macros, macro definitions, environments, alignment
tokens, comments, catcodes, conditionals, file inclusion, shell escape, URI
syntax, package loading, implicit multiplication rewrites, and error-recovery
insertion are rejected. A source producer must lower any richer dialect to
this explicit contract before package creation and must fail rather than omit
an unsupported construct.

There is no pre-parse Unicode normalization. The parser binds the exact source
bytes and produces a typed AST. Equivalent spellings may produce the same AST
only where this ADR says so, such as script order; the source hash still makes
their receipts different. The canonical formatter emits:

- no insignificant whitespace except exactly one ASCII space between two
  adjacent identifier nodes, between two adjacent number nodes, or between a
  number node and an adjacent literal `.` symbol; these are exactly the cases
  where omission could change longest-match tokenization;
- braces for every group and every command argument;
- subscript before superscript when both are present;
- the exact ASCII token bytes stored in every identifier and number node;
- the exact literal Unicode scalar stored in a symbol node; and
- the exact `\frac`, `\sqrt`, and `\operator` command spellings above.

The formatter is not a source rewriter and its output does not replace the
authored TextSpan. Validation requires
`parse(format(parse(source)))` to produce the same typed AST fingerprint.
Formatter disagreement, parser recovery, or a second parser interpretation is
an internal `I9190`, not a reason to accept the input under another dialect.

## Producer alternative and `ActualText` policy

Version 1 selects **producer-supplied alternative only**. Typaxis does not
generate speech, compare two generated readings, use an ambient screen-reader
service, or derive an alternative from TeX-like tokens. Missing, empty, or
control-containing `speech` is `P1102` during package validation. Typaxis can
prove that the supplied alternative stayed attached to the intended math; it
does not claim to prove linguistic quality.

The PDF paint for each math node is one typed marked-content group whose
`/ActualText` Unicode scalar sequence is exactly `speech`. The backend may use
the required PDF UTF-16BE string encoding, escaping, and object placement, but
decoding that PDF string must recover the same scalar sequence. It cannot use
the math source, canonical formatter output, glyph names, or a lossy ASCII
transliteration instead.

ADR-0035 retains this mapping, and MI4-09 must create one `/Formula` structure
element for the same math owner with the same producer alternative as `/Alt`.
The structure policy consumes ADR-0034's
computed language and may not redefine inheritance, substitute a new
alternative, split one atomic math node into unrelated formula owners, or
treat `ActualText` as a replacement for tagged structure. Inline and display
math share this alternative policy.

## Validated math receipt and layout

`typaxis-math` first issues an opaque parsed-source receipt and, after explicit
font metrics are supplied, an opaque math-computation receipt. The layout
binding owner verifies those receipts against the sealed package/profile/font
chain and alone issues `ValidatedMathReceipt`. Its `MathReceiptKey` is the
SHA-256 of canonical JCS under `typaxis.math-binding/1`. The key covers at
least:

- contract ID, package/document fingerprint, target profile receipt, admission
  session and resource-ledger fingerprint, effective-limit fingerprint, and
  math parser/formatter/layout algorithm identities;
- NodeId, inline/display kind, SourceId, SourceSpan, TextSpan, exact source
  byte SHA-256, source language/version, and parsed AST fingerprint;
- exact `speech` UTF-8 byte SHA-256;
- computed-style receipt/fingerprint, selected admitted math font face,
  admitted font bytes SHA-256, face index, MATH-table fingerprint, resolved
  size/style, and LayoutEpoch;
- checked advance, ascent, descent, baseline/axis, and bounding rectangle;
- canonical ordered glyph/rule/path paint IR and its vector fingerprint; and
- actual math-layout work consumed.

The key is owner-issued and cannot be constructed from those public-looking
fields. A raw SHA-256, NodeId, source string, speech string, font ID, or vector
IR cannot stand in for the receipt.

Math font selection uses the ordinary computed `font_family` and explicit
admitted-resource chain. The ordinary family resolver first selects exactly
one FontFaceId; MATH validation neither skips that face nor searches a later
family. The selected face must contain a valid bounded OpenType MATH table and
every required glyph; there is no platform font, fallback family, glyph-name
recovery, remote font, or source-command font switch. The in-tree
`typaxis-font` parser owns MATH-table validation and metric extraction.
`typaxis.math-layout/1` maps source scalars through that face's cmap, uses its
MATH constants/italic corrections/variants for rows, scripts, fractions,
radicals, and display operators, and fails when a required metric, variant, or
glyph is absent; it does not invoke the general text shaper or a heuristic
metric fallback. Glyph positions and rules use checked fixed-point arithmetic.
The paint IR retains original glyph IDs plus logical
Unicode/source ownership and contains no PDF CID, object number, resource
name, or backend handle.

The explicit node kind is also a required layout input: inline math starts in
the `/1` text math style, while display math starts in its display math style
and applies the admitted MATH table's display-operator/variant thresholds.
Scripts use the table's script-scale constants under the same `/1` algorithm;
no source delimiter or command can change the starting style.

Version 1 math layout is horizontal and left-to-right internally. It performs
no source-driven bidi reordering, vertical-glyph substitution, or writing-mode
rotation. An inline math box participates in the surrounding item stream as
one atomic LTR-isolated object; its internal glyph order cannot merge with or
be split by adjacent text runs. Display math uses the target profile's adopted
horizontal-tb/LTR block progression.

Inline math is one atomic inline item with one baseline and cannot be broken
inside the expression. The ordinary line breaker may place a legal break before
the item and move it intact to the next line. If its checked advance exceeds a
complete empty line's inline size, or its ascent/descent cannot fit an empty
full frame, the outcome is terminal `L5100`; the item is not overpainted,
scaled, clipped, split, or retried at the same position. Every display-math
node owns one independent
`MathFlowId` under `typaxis.math-flow/1`, bound to its NodeId, parent FlowId and
position, package/profile/style receipts, MathReceiptKey, LayoutEpoch, and one
atomic terminal. The registry scans display-math owners in dense NodeId
preorder and allocates dense MathFlowIds before worker execution; caller order,
hash-map order, and worker completion cannot assign them. The parent consumes
one typed display-math entry and advances past it only after that terminal. The
display flow uses computed
`space_before`, `space_after`, `start_indent`, `end_indent`, page, and keep
policy; inherited font properties select the math font. No math source command
overrides block geometry or selects a resource.

The selected-state owner extends the same receipt key with the exact parent
FlowId, display MathFlowId when present, page, frame, line/block fragment and
paint ordinals, origin, and transform. Display reopens the receipt and emits
the canonical glyph/rule/path sequence plus its `ActualText` group. PDF
reopens the Display receipt and actual serialized page observation. Manifest
and future structure projection cover the same base key and selected extension.
Missing, extra, reordered, wrong-source, wrong-alternative, wrong-font,
wrong-vector, wrong-page, or foreign-session facts are `I9190` before
publication.

Math is never routed through `ImageResourceId`, `AdmittedImageMediaKind`, PNG,
or the safe-SVG resource decoder. Its visual output is vector paint owned by
the math receipt, while safe SVG remains an independently admitted figure
resource. A SafeVector attestation carries no embedded text or alternative;
each Figure use retains its existing use-specific `alt` in the validated Figure
receipt. That alternative cannot attest the resource bytes or enter the Form
plan; ADR-0035 fixes the tagged-Figure policy and MI4-09 owns its implementation.

## Safe-SVG declaration and attestation

Contract 1.4 adds exactly one value to the image declaration reserved by
ADR-0032:

```text
svg-safe-1
```

The trusted domain variant is `ImageMediaType::SvgSafe1`. After stable read and
successful bounded validation/lowering, only `typaxis-resource-admission` may
issue `AdmittedImageMediaKind::SafeVector`. The M4 manifest wire value for both
the declared media and the corresponding nonnull attested kind is
`svg-safe-1`; equality remains a typed same-resource comparison. The
attestation also binds the contract/package/admission session, declared-media
policy receipt, effective-limit fingerprint, `ImageResourceId`, URI, stable
bytes SHA-256, parser and IR identities, intrinsic dimensions, view box,
allocation charge, and IR fingerprint.

`svg-safe-1` describes this exact subset, not any file whose suffix is `.svg`
and not arbitrary `image/svg+xml`. URI suffix, host MIME metadata, caller
assertion, a leading `<svg` byte sequence, selected Figure, Display command,
or PDF Form subtype cannot issue or override the attestation. A reference-mode
contract-1.4 exporter obtains the value only from the same-session SafeVector
attestation under ADR-0032's no-partial-output rule.

The production profile's declared-media policy must allow `svg-safe-1` before
resource open. A missing, unknown, legacy, or profile-disallowed declaration
is rejected before open. Stable bytes that do not validate as this exact
subset fail after read and before canonical vector IR allocation. No parser
failure retries the bytes as PNG, general SVG, XML, or caller-authored vector
commands.

## Safe-SVG lexical and element subset

Safe SVG 1 is a strict UTF-8 SVG-shaped language. It is parsed iteratively and
is intentionally narrower than XML/SVG:

- bytes have no BOM, NUL, CR, C0/C1 control, DOCTYPE, entity, character
  reference, comment, CDATA, processing instruction, or XML declaration;
- only ASCII space, tab, and LF may appear between tokens or elements;
- the root is exactly unprefixed `svg` with required
  `xmlns="http://www.w3.org/2000/svg"`; prefixed names and any other namespace
  declaration are rejected, and that fixed namespace literal is compared as
  grammar bytes and never dereferenced;
- character data other than allowed inter-element whitespace is rejected;
- attributes may use single or double quotes, attribute order has no semantic
  effect, and duplicate or unknown attributes are rejected; and
- unknown elements are rejected rather than ignored.

Element, attribute, command, and keyword names are ASCII case-sensitive.
Numeric attribute lexing is also closed and does not import the broader SVG
number grammar. `wsp` is one ASCII space, tab, or LF byte and `sep` is one or
more `wsp`; a numeric/list value has no leading or trailing `wsp`. A geometry
attribute contains exactly one decimal. `viewBox` contains four decimals,
`points` contains alternating x/y decimals, and path data contains command and
decimal tokens, with every adjacent token separated by `sep`. Transform
functions and adjacent transform functions use the same `sep`; their exact
forms are `matrix(a b c d e f)`, `translate(tx)`, `translate(tx ty)`,
`scale(sx)`, and `scale(sx sy)`. One-argument translate uses `ty = 0`, and
one-argument scale uses `sy = sx`. Commas, sign-as-separator shorthand, leading
or trailing separators, and every other tokenization are rejected.

Markup delimiters are equally exact. `svg`, `defs`, `clipPath`, and `g` use a
start tag and the exact matching `</name>` end tag; geometry elements use only
the self-closing form. A start or self-closing tag separates its name and each
attribute with `sep`, has no whitespace around `=`, and has no whitespace
before `>` or `/>`. An attribute value starts and ends with the same single or
double quote and contains only the value grammar for that attribute. These
rules reject empty-element spellings for containers, paired spellings for
geometry, whitespace inside an end tag, and every XML compatibility spelling
not expressly admitted here.

The complete element set is:

| Element | Allowed children and purpose |
| --- | --- |
| `svg` | optional one leading `defs`, then one or more `g` or paint geometry |
| `defs` | one or more `clipPath`; it must precede every paint element |
| `clipPath` | exactly one `path`, `rect`, `circle`, `ellipse`, or `polygon` geometry |
| `g` | one or more nested `g` or paint geometry; carries inherited paint/transform/clip state |
| `path` | strict path data lowered to canonical segments |
| `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon` | geometry lowered to canonical path segments |

Only `clipPath` may carry `id`. IDs match
`[A-Za-z_][A-Za-z0-9_.-]{0,63}`, are unique by UTF-8 bytes, and are remapped to
dense definition-preorder ClipIds in the IR. `clip-path` is either absent or
exactly `url(#ID)` and may reference only a declared local clip path. Forward,
missing, cyclic, nested-clip, and unused clip definitions are rejected.

The complete non-geometry attribute set is:

- root `xmlns`, `width`, `height`, and `viewBox`;
- `transform` on `g`, paint geometry, and clip geometry;
- `clip-path` on `g` and paint geometry;
- inherited `fill`, `stroke`, `stroke-width`, `fill-rule`, `stroke-linecap`,
  `stroke-linejoin`, and `stroke-miterlimit` on `g` and paint geometry;
- `fill-rule` on clip geometry, with every other paint attribute forbidden
  there; and
- `id` on `clipPath` only.

Geometry elements accept only the attributes needed by their SVG geometry:
`d`; `x`, `y`, `width`, `height`; `cx`, `cy`, `r`; `cx`, `cy`, `rx`, `ry`;
`x1`, `y1`, `x2`, `y2`; or `points`, as applicable. Rounded-rectangle `rx` and
`ry` are not accepted. Missing optional `x`, `y`, `cx`, or `cy` uses exact zero;
required width, height, radius, or radius pair is positive. A shape with empty,
nonfinite, or degenerate required geometry is rejected rather than retained as
invisible content. Degenerate means that no transformed drawable segment has
distinct points; fill-only geometry and clip geometry additionally require
positive checked width and height, while a stroked horizontal or vertical line
is valid. Every paint geometry must have an effective fill or stroke; `line`
requires a non-`none` stroke, every retained path must contain a drawable
segment under that rule, and the complete resource must contain at least one
paint command. Every `g` subtree must contain a paint geometry. Every clip
definition's sole geometry must lower to one closed fillable path with positive
checked width and height; every subpath in that geometry must end in Close.

Path data accepts `M/m`, `L/l`, `H/h`, `V/v`, `Q/q`, `C/c`, and `Z/z`.
It must begin with `M` or `m`; every later subpath also begins with `M` or `m`,
and only another moveto or EOF may follow `Z`/`z`. A command consumes all
decimal tokens up to the next command or EOF. Moveto requires a positive
multiple of two decimals and lowers every pair after the first to Line;
lineto requires a positive multiple of two, horizontal/vertical lineto a
positive multiple of one, Quadratic a positive multiple of four, and Cubic a
positive multiple of six. Close takes no decimals and resets the current point
to that subpath's moveto point. Lowercase coordinates are offsets from the
current point at the start of each parameter group; uppercase coordinates are
absolute. Smooth curves, elliptical arcs, implicit error recovery, and every
other command or arity are rejected. The IR contains only absolute Move, Line,
Quadratic, Cubic, and Close segments.

`rect` lowers as Move at `(x,y)`, three clockwise Lines, then Close; `line`
lowers as Move then Line. `polyline` requires at least two points and lowers as
Move plus Lines; `polygon` requires at least three points and adds Close.
`H`/`V` lower to Line and relative coordinates become absolute. Circle and
ellipse start at their positive-x axis and lower clockwise to four Cubic
quadrants. Each quadrant uses the fixed 16.16 control ratio
`36,195 / 65,536`; every multiply/add follows the checked round-half-to-even
rule below. A retained Quadratic remains a typed IR segment. When PDF
serializes it, the cubic control points are exactly
`P0 + 2/3 * (P1 - P0)` and `P2 + 2/3 * (P1 - P2)`, with one
round-half-to-even conversion after each exact rational control coordinate.
No library-specific circle, curve-flattening, or tolerance setting may change
those commands.

The following are expressly forbidden: `script`, every `on*` event attribute,
animation, `foreignObject`, `image`, `text`, `tspan`, fonts, glyph references,
`use`, links, `href`, XLink, `xml:base`, external or data URI references,
network or filesystem fetch, CSS or `style`, selectors, media queries,
gradients, patterns, markers, masks, filters, blend modes, opacity, embedded
raster data, metadata with active meaning, unbounded recursion, and unknown
extension content. There is no ignored compatibility branch.

## Geometry, units, transforms, clipping, and paint

Root `width` and `height` are required positive decimals with optional `px` or
`pt`; unitless means `px`, one `px` is exactly `3/4` PDF point, and one `pt` is
exactly one PDF point. Percent, `em`, `ex`, physical CSS units, viewport units,
and `calc()` are forbidden. `viewBox` is required and contains four decimals;
its width and height are positive. The intrinsic width/height ratio must equal
the view-box width/height ratio as exact rationals, so Safe SVG 1 has one
uniform root mapping and no `preserveAspectRatio` branch. The root view box is
always a clip boundary.

Intrinsic width and height are first converted independently to positive
`pdf_point_1_65536` Lengths. View-box values become signed integer multiples
of 1/65,536 view-box user unit. From those stored integers the decoder derives
the horizontal and vertical root scales independently as signed 16.16 values,
rounding each exact ratio once; they must be equal and positive. With that
common scale `s`, the canonical top-left/Y-down root mapping is
`x = s * (u - min_x)`, `y = s * (v - min_y)`, with each stored translation
and point operation checked and rounded half to even. The intrinsic rectangle
`[0, 0, width, height]` is the outermost clip and is never replaced by painted
bounds. The frozen Form plan uses `/BBox [0 0 width height]`; PDF applies the
single [docs/24](../docs/24-units-rounding-and-geometry.md) page-root
`(a=1,b=0,c=0,d=-1,e=0,f=page_height)` conversion when the Form is invoked.
Form serialization and DrawVector placement do not add a second Y flip. No
phase re-fits, recenters, or derives a different scale from the path extrema.

A decimal has grammar `-?(0|[1-9][0-9]*)(\.[0-9]{1,6})?`, no exponent or
leading plus, at most twelve integer digits, and absolute value at most
1,000,000 before and after each checked transform. Root physical dimensions
use signed `pdf_point_1_65536`; view-box geometry and transform translations
use signed integer multiples of 1/65,536 view-box user unit; dimensionless
transform components use signed 16.16. Every conversion uses exact rational
arithmetic and round-half-to-even. No binary float participates in IR,
fingerprints, layout, Display, or PDF.

`transform` accepts only the five forms fixed above. Matrix `a`/`d` and scale
arguments are nonzero dimensionless 16.16 values; matrix `b` and `c` must be
exact zero. Matrix `e`/`f` and translate arguments are view-box coordinates.
Operations concatenate left to right using ADR-0009's column vectors and
`CTM := CTM * M`; each checked fixed-point operation has the same
round-half-to-even rule. Axis reflection and nonuniform scale are allowed;
rotation, skew, perspective, zero determinant, overflow, and out-of-range
results are rejected even when expressed through `matrix(...)`.

For every element, `element_ctm := parent_ctm * transform_attribute`; geometry
uses that CTM. A `clip-path` on that element pushes its definition geometry
under `element_ctm * clip_geometry_transform` before the element paints or a
group visits children, and pops it after that element or subtree. The root clip
is pushed first. Nested group/geometry clips therefore intersect in source
nesting order. A clip definition has exactly one geometry and one fill rule,
so there is no implementation-dependent multi-shape union, `objectBoundingBox`
mapping, or clip-rule merge.

Paint is solid eight-bit nominal-sRGB producer data, but the `/1` PDF mapping
makes no device-independent color claim. `fill` and `stroke` are `none` or
`#RRGGBB`, where each component pair uses exactly two ASCII hex digits of either
case; the canonical color is exactly three bytes. PDF Form paint uses
`/DeviceRGB`; each byte becomes unsigned 16.16 by exact
`byte * 65,536 / 255` round-half-to-even before the canonical fixed-point
number serializer. No ambient color profile participates. The complete initial
paint state is fill `#000000`, stroke `none`, stroke width `1` view-box unit,
fill rule `nonzero`, line cap `butt`, line join `miter`, and miter limit `4`;
inherited values are resolved into every draw command. `fill-rule` is
`nonzero|evenodd`, line cap is `butt|round|square`, line join is
`miter|round|bevel`, stroke width is a positive view-box-unit decimal, and
miter limit is a dimensionless decimal at least one. There is no alpha, dash,
gradient, pattern, ICC profile, text, or font paint. Clip geometry uses fill
plus its fill rule and never stroke.

All basic shapes lower to the same absolute path vocabulary. Canonical IR
order is source document preorder after `defs`; each draw records its resolved
transform, clip stack, path, fill, and stroke. It contains logical geometry and
paint only: no XML name, caller source ID, host path, PDF operator, object
number, resource name, or backend handle.

## Safe-vector and math limits

All maxima are inclusive. Exact max is accepted and the operation that would
cross max+1 is refused before read, allocation, evaluation, command issuance,
or PDF object creation. MI4-04 and MI4-05 add these four positive fields to a
sealed workspace-internal `M4ResourceLimits` extension and the private 1.4
package-config shape:

| Limit | Default | Hard maximum | Stable code |
| --- | ---: | ---: | --- |
| `max_vector_nodes` | 100,000 | 1,000,000 | `R7120` |
| `max_vector_path_segments` | 1,000,000 | 10,000,000 | `R7121` |
| `max_vector_nesting_depth` | 32 | 64 | `R7122` |
| `max_math_layout_units` | 1,000,000 | 10,000,000 | `L5111` |

The extension is validated together with the existing `ResourceLimits` and
both are covered by one effective-limit fingerprint. Before MI4-13, current
config aliases, the public `ResourceLimits` canonical JCS encoding, its default
hash, and every old profile descriptor omit the extension. A staging owner must
receive the combined sealed limit receipt; it cannot read ambient defaults or
substitute an extension from another package/session.

The three count/work fields are checked `u64`; nesting depth is checked `u32`.
Their wire members are positive JSON integers. Zero, a value above the listed
hard maximum, or a noninteger is versioned config input `P1102` before a limit
receipt or resource session exists; the table's `R`/`L` code instead reports
accepted configuration whose actual work would cross its inclusive maximum.

The complete charge and diagnostic table is:

| Work | Limit and charge | Refusal owner/code |
| --- | --- | --- |
| stable encoded Safe-SVG bytes | existing per-resource `max_image_bytes` and aggregate `max_resource_bytes` | stable read before parser, `R7100` |
| elements including `svg`, `defs`, `clipPath`, `g`, and every geometry element | one `max_vector_nodes` unit each, root depth 1 | lexical preflight before record allocation, `R7120` |
| canonical Move/Line/Quadratic/Cubic/Close work | one `max_vector_path_segments` unit for every segment stored for paint or clip-definition geometry after shape/path expansion, every stored clip segment replayed by each `clip-path` reference, and the synthetic outer clip's Move + three Lines + Close | lexical/reference preflight before IR allocation or command issuance, `R7121` |
| element nesting | root depth 1, every child edge +1 against `max_vector_nesting_depth` | iterative lexical preflight before descent, `R7122` |
| canonical vector allocation | existing `max_decoded_image_bytes`, charged as `64 * nodes + 80 * stored_segments + 32 * paint_or_clip_commands + source_clip_id_bytes`, all checked | permit before each IR allocation, `R7111` |
| math source and producer alternative | exact source-span length and speech length each against `max_text_buffer_bytes`; source bytes retain their existing one-time admitted-buffer charge, while speech adds exactly once to `max_text_bytes` | before parse or speech copy, `T2100` / `T2101` |
| math AST nodes and grammar nesting | existing `max_ast_nodes` and `max_ast_nesting_depth`, with the math node itself already counted once, each math AST node an additional unit, the math AST root at owning semantic-node depth +1, and every typed AST child edge +1 | iterative parser precheck, `P1120` / `P1121` |
| math layout/paint work | one `max_math_layout_units` unit for each typed AST node in the single canonical preorder layout evaluation, each emitted canonical layout box, and each output glyph, rule, or path command | permit before evaluation/allocation/issuance, `L5111` |
| selected math/vector records | one existing `max_fragments` unit per selected atomic math node and per selected SafeVector placement, in selected page/frame paint order | selected-state owner before record issuance, `L5110` |
| PDF Form/resource/page objects | existing `max_pdf_objects` | PDF graph preflight before dense object allocation, `G6100` |

Each selected math/vector occurrence above is one explicit auxiliary record in
addition to its containing ordinary fragment's existing unit. Display commands,
manifest projection, and PDF use reopen that record and do not charge another
`max_fragments` unit.

`max_vector_nodes` and `max_vector_path_segments` are resolver-session
aggregates across every SafeVector resource, consumed in dense ImageResourceId
order; `max_vector_nesting_depth` applies independently to each resource.
Starting another resource does not reset either aggregate. The existing
encoded aggregate `max_resource_bytes`, `max_images`, and PDF-object budget
remain additional bounds.

`max_math_layout_units` is one package/session computation budget consumed in
dense math NodeId order across all inline and display nodes. A successfully
sealed computation receipt is reusable by pagination without another charge;
reparsing, a foreign limit receipt, or a layout-pass retry cannot reset the
budget. Existing aggregate `max_ast_nodes` and `max_text_bytes` remain
additional bounds.

`paint_or_clip_commands` counts one draw for each paint geometry, one push and
one pop for the synthetic outer clip, and one push/pop pair for every
`clip-path` reference. Clip-definition geometry is stored once and referenced
by its dense ClipId. `stored_segments` includes all paint/definition segments
and the synthetic outer clip's five segments but excludes per-reference
replay, while the path-work counter above includes that replay.
`source_clip_id_bytes` is the sum of the UTF-8 byte lengths of every declared
`id` and every referenced ID inside `url(#ID)`, excluding the wrapper bytes.
The allocation formula is a deterministic logical charge, not platform
`size_of`, allocator capacity, compressed bytes, or observed RSS.

The decoder makes three scans over the same stable bytes. A fixed-depth,
no-heap first scan proves lexical shape and exact node, stored-segment,
command, nesting, and source-ID-byte counts; it obtains the corresponding
node/segment/allocation permits. A second scan uses deterministic scratch no
larger than the already permitted allocation charge to prove ID uniqueness,
reference/attribute/geometry validity, and complete paint/clip closure. It
computes the checked clip-segment replay total and obtains that remaining path
work before any IR or command is issued. The scratch is then released. Only a
fully valid input reaches the third scan that allocates and fills the exact-size
canonical IR. Thus malformed input never leaves partial IR, scratch and IR are
not simultaneous, and clip reuse cannot multiply uncharged PDF work.
Limit-fingerprint substitution is `I9190`.

`R7120`, `R7121`, `R7122`, and `L5111` are reserved only for the private 1.4
diagnostic registry until MI4-13. Current/frozen diagnostic registries and
public code tables do not gain them early.

Malformed math syntax is `P1102` at the source byte location. A missing or
unknown math language/version is `P1102` at its exact package JSON Pointer;
unsupported math/profile placement is `L5100` before resource or layout work.
A missing MATH table/glyph or unsupported admitted math font is `R7100` after
font admission but before math layout. Malformed, forbidden, unknown,
out-of-range, or declaration-mismatched Safe SVG is `R7100` at the image
resource subject. The more specific codes above are used only for their exact
limit. Arithmetic overflow never wraps or degrades; the owning input/limit
code is returned before output state changes.

## Crate ownership, dependency edges, and tool identity

MI4-05 creates `typaxis-math`. Version 1 uses an in-tree parser, formatter,
AST, fixed-point layout, and paint-IR builder. Its only workspace dependencies
are `typaxis-core` and `typaxis-font`; it has no third-party runtime dependency.
`typaxis-syntax` may depend on it only for the parsed-source receipt;
`typaxis-layout` supplies sealed font metrics, consumes its computation
receipt, and issues the final package/profile/LayoutEpoch-bound
`ValidatedMathReceipt`; `typaxis-display-list` may consume that final receipt.
`typaxis-math` must not depend on syntax, machine profile, host/resource
admission, shaping, layout, Display, resources/finalization, manifest, PDF, or
CLI, preventing a promotion cycle or backend authority leak.

MI4-04 keeps the Safe-SVG byte parser and canonical IR issuer inside
`typaxis-resource-admission`, the sole stable-byte decoder owner. It is an
in-tree iterative implementation with no XML/SVG/browser dependency.
`typaxis-resources` may consume the sealed IR to create a PDF-ready Form plan;
layout consumes only intrinsic geometry and the ledger binding; Display uses
the logical ImageResourceId plus IR fingerprint. PDF never parses SVG/XML and
resource admission never depends on layout, Display, resources, or PDF.

`typaxis-testkit` must enforce those allowed/forbidden edges and assert that no
new external math, XML, SVG, CSS, browser, speech, or network crate appears in
the closure. Introducing such a dependency, external executable, dynamic
library, platform parser, or speech service is not an implementation detail
under these `/1` identities; it requires a new ADR, exact pin/tool identity,
supply-chain audit, and compatibility decision.

Receipts and M4 evidence record every applicable source/parser/formatter,
layout/work, safe-parser, IR, and charge identity from the adopted-identity
table plus the engine identity. Locale, timezone, filesystem, network,
environment variables, installed fonts, and worker order are absent from
parsing and formatting. Same bytes, profile, limits, admitted font/resource
hashes, and engine identity must produce the same AST/IR fingerprints and PDF
bytes.

## Display, PDF, manifest, and structure closure

An admitted SafeVector follows one closed chain:

```text
declared svg-safe-1 + profile policy
  -> stable bytes/hash + bounded Safe-SVG preflight
  -> SafeVector attestation + canonical IR fingerprint
  -> admitted ImageResourceId ledger
  -> selected Figure placement
  -> Display DrawVector usage
  -> frozen PDF Form plan and serialized Form XObject
  -> M4 manifest declaration/attestation/IR/usage/object facts
```

Every arrow consumes the preceding opaque receipt. The finalization owner
creates exactly one canonical Form plan per `(ImageResourceId, stable bytes
hash, IR fingerprint)` and PDF assigns its object/resource identity. Missing,
extra, wrong-resource, wrong-IR, wrong-placement, a plan/object for an unused
vector resource, use of an unadmitted vector, or unmatched Form observation is
`I9190`. An admitted declaration that is never selected needs no plan and is
not itself an error: its built M4 resource fact retains declaration,
attestation, and IR evidence with an empty usage/object set. The PDF backend
emits path/clip/fill/stroke operators; rasterization is not the normal path and
cannot satisfy the receipt.

A validated math node follows a distinct chain:

```text
source/span/kind/speech
  -> parsed source receipt
  -> target profile receipt + explicit admitted math font
  -> ValidatedMathReceipt with dimensions/vector fingerprint
  -> atomic inline item or validated display MathFlowId/terminal
  -> selected page/frame/origin
  -> Display math paint + exact ActualText group
  -> serialized PDF observation
  -> M4 manifest math fact and later /Formula structure element
```

The manifest records the base MathReceiptKey, source/alternative hashes,
parser/formatter identities, font/AST/vector fingerprints, selected placement,
Display hash, PDF observation, and structure owner when available. It does not
claim the original source equals the speech or reconstruct either from PDF.
ADR-0035 adds structure-tree/MCID policy around this already fixed formula
owner, and MI4-09 implements it while consuming ADR-0034's computed language
without changing the math source or speech. MI4-13 requires bidirectional
closure and external visual/text/accessibility evidence before publication.

## Closed rejection and fallback policy

The target profile rejects at least:

- a math node flattened to text, source-only text, an image, or unbound paths;
- math delimiters used to infer inline/display kind;
- another math dialect/version, unknown command, macro, environment, parser
  recovery, empty expression, missing/invalid speech, or implicit font lookup;
- engine-generated speech, source text used as automatic `ActualText`, or a
  tagged Formula alternative different from the receipt;
- a math font without the admitted hash/MATH-table/glyph closure;
- math layout or vector paint detached from NodeId, source span, source bytes,
  alternative, kind, font, selected page, or LayoutEpoch;
- generic SVG/XML, unknown or ignored elements/attributes, entity expansion,
  external/local non-clip references, scripts, events, animation, CSS, text,
  font, image, foreign object, filter, mask, pattern, gradient, or network/file
  access;
- suffix/MIME/caller-derived SafeVector attestation, caller-trusted vector IR,
  declaration/bytes mismatch, or a parser retry as another media type;
- float-based coordinates, unbounded numeric tokens, unchecked transform,
  max+1 work, partial IR, or allocation-dependent fingerprint order;
- vector-to-PNG or math-to-PNG as the successful production path; and
- any public 1.4 Schema, descriptor, CLI, capability, or exporter exposure
  before MI4-13.

Every rejection is terminal for that requested profile/input. There is no
plain-text, raster, browser, external-tool, alternate-parser, or old-contract
fallback and no warning-only degradation.

## Publication and implementation sequence

MI4-04 privately adds `svg-safe-1` to the independent 1.4 wire/domain/Schema,
implements the bounded stable-byte decoder and canonical IR, and closes Figure
Display/PDF/manifest observations. MI4-05 privately adds the two math nodes,
`typaxis-math`, producer alternative, font/layout receipt, vector paint, and
`ActualText`. Neither task changes current aliases, public accepted contracts,
help, capabilities, default profile, old profile domains, or frozen artifacts.

ADR-0035 consumes the reserved `/Formula` owner, exact producer alternative,
and ADR-0034 computed-language receipt in its fixed structure policy; MI4-09
must implement that policy rather than revisiting these bindings. ADR-0036
adds separate `jpeg-baseline` and `sfnt-cff1` resource components but cannot
broaden `svg-safe-1`. MI4-13 may publish only the complete contract/profile
and remove all private staging entrances in the atomic order fixed by
ADR-0032.

## Rejected alternatives

1. **Rasterize math.** It loses scalable paint and makes source/alternative
   closure dependent on an unrelated image record.
2. **Use raw TeX or LaTeX.** Macro expansion, packages, catcodes, file access,
   and engine/version behavior are not a closed portable contract.
3. **Generate speech in the engine.** No speech-generation rule/tool identity
   is adopted; ADR-0034's semantic language tag does not authorize generation,
   and silent algorithm changes would alter accessibility semantics.
4. **Use source bytes as `ActualText`.** Extraction would expose formatting
   syntax rather than the producer's intended alternative.
5. **Accept general SVG through a browser/XML stack.** It imports unsupported
   external-resource, CSS, script, font, animation, and platform behavior.
6. **Accept backend-neutral commands directly from the caller.** Without a
   stable-byte decoder receipt, the caller would become the trust owner and
   could bypass complexity and geometry validation.
7. **Allow SVG text or fonts.** That creates a second shaping/font-fallback
   path outside admitted math/text resource closure.
8. **Use floating-point geometry.** Platform/compiler variation would leak
   into fingerprints and PDF bytes.
9. **Infer vector media from `.svg` or MIME.** Neither value proves the safe
   subset or the admitted bytes.
10. **Publish the enum or math decoder early.** A partial public 1.4 surface
    would promise a feature without its lossless PDF/accessibility path.

## Consequences

- MI4-04 has one media value, one strict safe-SVG subset, one parser/IR owner,
  one numeric model, and explicit resource/complexity limits.
- MI4-05 has one source language/version, two explicit node kinds, one
  in-tree parser/formatter pair, and producer-supplied alternative semantics.
- Source, speech, font, visual geometry, selected placement, `ActualText`, and
  future `/Formula` structure can be checked from one receipt chain without
  trusting PDF reconstruction.
- Safe vectors remain scalable and deterministic without importing a browser,
  XML entity engine, CSS cascade, font lookup, filesystem, or network access.
- The adopted subsets are intentionally narrow. Producers must reject or
  explicitly lower richer math/SVG before package creation, and that lowering
  remains outside Typaxis's claimed source semantics.
- No user-visible behavior changes at ADR adoption; public contract 1.3 and
  its seven profiles remain byte-frozen until the MI4-13 gate.
