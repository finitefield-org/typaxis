# ADR-0036: JPEG and OpenType/CFF resource profiles

## Status

Accepted on 2026-08-29 as the JPEG and OpenType/CFF resource decision gate
for M4.

This ADR extends only the non-current contract-1.4 target reserved by
[ADR-0032](ADR-0032-semantic-container-and-declared-media.md). It preserves
the existing `png`, `svg-safe-1`, `sfnt-truetype-glyf`, and
`ttc-truetype-glyf` meanings. It does not change current
`typaxis.contract/1.3`, add a public contract-1.4 decoder or Schema alias,
register `typaxis.machine-pdf/production-book-1`, or claim a working JPEG or
OpenType/CFF path. MI4-11 and MI4-12 may implement the two profiles through
private staging. MI4-13 remains the sole publication gate.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: the media values, admitted subsets, resource profiles, limits, transforms, subset, PDF plans, and dependency policy are closed here |
| implemented | No: JPEG belongs to MI4-11 and OpenType/CFF belongs to MI4-12 |
| public CLI E2E | No: public commands still reject contract 1.4 and the target profile |
| release-supported | No: combined fixture, renderer/extractor evidence, and atomic publication remain MI4-13 work |

## Context and decision boundary

The private M4 target already has bytes-attested PNG, SafeVector, standalone
TrueType-sfnt, and TrueType-collection declarations. Those values do not say
what subset of JPEG or OpenType/CFF is safe and deterministic, how metadata
affects color or orientation, whether a font may be subset-embedded, or which
PDF font program and descendant type is correct. Treating either family as a
generic image/font capability would permit a decoder, filename suffix, PDF
writer, or platform library to choose semantics after profile preflight.

This decision uses the JPEG/JFIF marker model, the
[OpenType specification](https://learn.microsoft.com/en-us/typography/opentype/spec/),
the [CFF specification](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5176.CFF.pdf),
the [Type 2 charstring specification](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5177.Type2.pdf),
and ISO 32000-1/PDF 1.7 font and DCTDecode mappings. The external documents
describe containers; this ADR deliberately adopts smaller closed subsets.
“JPEG support” below never means every JPEG coding process, and “OpenType/CFF
support” never means every sfnt, CFF, variable, color, collection, or bare-font
form.

The design inputs are [docs/25](../docs/25-machine-input-pdf-improvements.md)
sections 7 and 13.4, ADR-0003, ADR-0008, ADR-0010 through ADR-0015,
ADR-0020, ADR-0027 through ADR-0035, plus invariants I-003, I-009, I-014,
I-025, I-030, I-034, I-037, I-040, I-053, I-059, I-063, I-065, I-067,
I-073, I-076, I-077, I-080, and I-081. Earlier stable-byte, typed-ID,
selected-state, Display, late-finalization, PDF-object, manifest, diagnostic,
limit, and publication rules remain normative unless this ADR narrows them.

## Adopted identities and profile composition

The following immutable identities are fixed:

| Item | Identifier |
| --- | --- |
| preserved PNG behavior component | `typaxis.resource-profile/png/1` |
| preserved SafeVector behavior component | `typaxis.resource-profile/safe-vector/1` |
| baseline JPEG component | `typaxis.resource-profile/jpeg-baseline/1` |
| preserved TrueType `glyf` behavior component | `typaxis.resource-profile/truetype-glyf/1` |
| standalone OpenType/CFF1 component | `typaxis.resource-profile/sfnt-cff1/1` |
| complete production-book resource set | `typaxis.production-book-resource-set/1` |
| JPEG marker preflight | `typaxis.jpeg-marker-preflight/1` |
| JPEG deterministic sanitizer | `typaxis.jpeg-segment-sanitizer/1` |
| JPEG decoded-pixel observation | `typaxis.jpeg-pixel-observation/1` |
| OpenType/CFF1 admission | `typaxis.sfnt-cff1-admission/1` |
| bounded Type 2 evaluator | `typaxis.cff1-charstring-evaluator/1` |
| selected CFF1 glyph closure | `typaxis.cff1-glyph-closure/1` |
| deterministic CFF1 subset | `typaxis.cff1-subset/1` |
| CFF1 embedding-permission decision | `typaxis.cff1-embedding-permission/1` |
| JPEG-to-PDF embedding plan | `typaxis.jpeg-pdf-plan/1` |
| CFF1-to-PDF embedding plan | `typaxis.cff1-pdf-plan/1` |
| dependency supply-chain evidence | `typaxis.dependency-audit/1` |

These component IDs are profile facts, not independently selectable public
machine-PDF profiles. At MI4-13 the production descriptor must advertise
`typaxis.production-book-resource-set/1` with the following exact ordered
component array:

```text
typaxis.resource-profile/png/1
typaxis.resource-profile/safe-vector/1
typaxis.resource-profile/jpeg-baseline/1
typaxis.resource-profile/truetype-glyf/1
typaxis.resource-profile/sfnt-cff1/1
```

Its image-media array is exactly `png`, `svg-safe-1`, `jpeg-baseline`; its
font-media array is exactly `sfnt-truetype-glyf`, `ttc-truetype-glyf`,
`sfnt-cff1`. Order is part of the descriptor. PNG, SafeVector, and TrueType
component descriptors project the already adopted behavior rather than
renaming or revising it. A descriptor must list components and media arrays,
so a consumer can distinguish JPEG from PNG and CFF1 from TrueType without
interpreting a generic `images = true` or `fonts = true` claim.

Each `/1` descriptor, receipt, observation, closure, and plan fingerprint is
SHA-256 over an RFC 8785 JCS record containing its exact `algorithm` identity.
Resource sets are ordered as above; resource records are ordered by dense
typed resource ID; numeric glyph IDs, table tags, PDF object roles, and hashes
use the explicit order in the relevant section below. Host paths, URI suffixes,
hash-map iteration, thread completion, allocator behavior, locale, and PDF
object allocation cannot choose the order.

Changing any admitted marker, coding process, color interpretation, metadata
rule, sfnt/CFF table or operator set, embedding-permission rule, glyph closure,
hint transform, subset serialization, PDF subtype, dependency version/feature,
or limit charge requires the affected `/2` identity plus a contract/profile
compatibility decision. A security response may report the target unavailable;
it may not silently substitute another decoder or reduced profile under `/1`.

## Exact wire values and bytes-derived attestation

Contract 1.4 adds exactly these values to the enums adopted by ADR-0032 and
ADR-0033:

| Resource | Wire and trusted declaration | Decoder-issued attestation |
| --- | --- | --- |
| baseline JPEG image | `jpeg-baseline` / `ImageMediaType::JpegBaseline` | `AdmittedImageMediaKind::JpegBaseline` |
| standalone OpenType/CFF1 font | `sfnt-cff1` / `FontMediaType::SfntCff1` | `AdmittedFontMediaKind::SfntCff1` |

No value named `jpeg`, `jpg`, `image/jpeg`, `otf`, `opentype`, `cff`,
`cff2`, or `font/otf` is an alias. JSON uses the exact case-sensitive strings
above. Missing, null, wrong-typed, or unknown values are `P1102` at the exact
`/resources/images/{index}/media_type` or
`/resources/font_faces/{index}/media_type` JSON Pointer. There is no default.

The `jpeg-baseline` magic is `FF D8` followed immediately by the admitted JFIF
APP0 record and the complete closed marker sequence below. The `sfnt-cff1`
container begins with the four bytes `OTTO`, contains a `CFF ` table, and has
`face_index = 0`. A TrueType scaler, `glyf`, TTC/OTC collection, bare CFF
program, WOFF/WOFF2 wrapper, CFF2 table, or nonzero face index does not attest
as `sfnt-cff1`.

`typaxis-machine-profile` first checks whether the requested profile admits the
declared value. A disallowed known value is `R7100` before any resource is
opened. After the stable contained read, `typaxis-resource-admission` delegates
only bounded typed inspection to the owners fixed below, issues the attested
kind, and exact-matches it to the declaration before pixel decode or outline
evaluation. Bytes that happen to be another supported type are a terminal
media mismatch; the implementation does not retry the resource as that type.

URI suffix, caller MIME, host metadata, source spelling, expected hash,
selected Figure, font family, parser return type, PDF filter/subtype, or a
manifest field cannot issue or override attestation. The logical resource ID,
portable URI, stable byte length, source SHA-256, declared type, attested kind,
profile/component fingerprints, effective-limit fingerprint, parser identity,
and package/session identity are carried by the admission receipt.

Source-mode contract-1.4 `dump-ast` obtains `jpeg-baseline` or `sfnt-cff1`
only by consuming this same successful stable-byte attestation. It cannot infer
`.jpg`, `.jpeg`, `.otf`, a MIME label, or `OTTO` alone. If the source resource
cannot be stably opened, fully attested, permission-checked in the font case,
or exact-matched to an admitted production component, export fails before any
DocumentPackage bytes reach stdout. It does not emit legacy absence, a partial
package, or a contract-1.3 fallback.

## Closed baseline JPEG component

### Container, frame, scan, and sample rules

`typaxis.resource-profile/jpeg-baseline/1` accepts exactly one nonhierarchical,
Huffman-coded, baseline-sequential DCT frame and one scan containing every
frame component (single-component for Gray, interleaved for YCbCr):

- `SOI` is byte zero and a single APP0 JFIF segment immediately follows it.
  The identifier is `JFIF\0`, version is 1.00, 1.01, or 1.02, density unit is
  0, 1, or 2, X/Y densities are equal positive 16-bit values, and thumbnail
  width and height are both zero. JFXX is not admitted.
- There is exactly one `SOF0`. Precision is 8 bits and both unsigned 16-bit
  dimensions are positive. There is no DNL dimension replacement.
- A grayscale frame has component ID 1, sampling 1x1, and one scan component.
  A color frame has component IDs 1/2/3 in Y/Cb/Cr order; Cb and Cr are 1x1;
  Y is exactly 1x1, 2x1, 1x2, or 2x2, corresponding to 4:4:4, 4:2:2,
  4:4:0, or 4:2:0. Components, scan order, and selectors may not be
  duplicated or omitted.
- The single `SOS` names every frame component in frame order and uses
  `Ss = 0`, `Se = 63`, and `Ah = Al = 0`. A second scan or data after the
  terminal `EOI` is invalid.
- Every marker has exactly one `FF` introducer byte. Marker fill bytes are not
  admitted, entropy `FF` data bytes use exactly one following `00`, and unused
  entropy bits before a restart marker or EOI are all one.
- DQT uses only 8-bit precision and table IDs 0 through 3. DHT uses only
  DC/AC classes and table IDs 0 through 3. Every referenced table is defined
  exactly once before the scan and no unused table is present. Each DQT has 64
  nonzero values. Each DHT has 16 checked code-length counts, no oversubscribed
  code space or all-ones code, a nonempty symbol list with no duplicate, DC
  symbols only in 0 through 11, and AC symbols only EOB, ZRL, or run 0 through
  15 with nonzero size 1 through 10. The entropy walk enforces 64 coefficient
  positions per block and checked signed-`i16` DC-predictor addition, rejecting
  the wraparound behavior of the external decoder before that decoder runs.
- At most one DRI is allowed before the scan and, if present, its interval is
  positive. Entropy bytes honor byte stuffing and restart markers occur only
  when enabled, in exact RST0 through RST7 cyclic order between complete MCU
  intervals; there is no marker after a final exact or partial interval.
  Truncation, the wrong MCU count, an early/extra restart, an unstuffed marker,
  or missing EOI is invalid.

SOF1/2/3 and every differential, hierarchical, arithmetic, lossless, extended,
or progressive process are rejected. Twelve-bit samples, four components,
CMYK, YCCK, alpha, palette, non-JFIF RGB interpretation, and non-square sample
aspect are not admitted. Unknown/reserved markers, TEM, DAC, DNL, DHP, EXP,
multiple frames, multiple scans, and concatenated JPEG streams are rejected
rather than ignored.

The in-tree marker preflight is a checked, iterative scan over the already
bounded stable bytes. It keeps fixed-size marker/table/component state, counts
segment lengths before skipping payload, validates entropy/MCU structure with
a bounded Huffman-symbol walk without materializing coefficient blocks, and
computes width, height, channel count, pixel count, decoded output bytes,
sampling, normalized-stream length, each padded component plane, MCU-row
coefficient buffers including the replacement duplicate, each output-row
upsample buffer, and both possible decoder peak-live sets with checked `u64`
arithmetic. The peak is the larger of component planes plus live coefficient
rows and component planes plus the canonical output plus live upsample rows;
fixed parser/worker state is separately bounded by the closed
marker/table/component counts. It acquires all applicable byte, pixel, and
complete decode-workspace permits
before the external decoder can allocate an image buffer. The external
decoder's own maximum-output-buffer setting is an additional check, not the
accounting boundary.

### Color, orientation, metadata, and ICC policy

The single JFIF APP0 segment is container identification, not retained PDF
metadata. Every other APP0 through APP15 segment and every COM segment is
rejected. In particular, EXIF/TIFF orientation and camera metadata, XMP,
Photoshop resources, Adobe APP14 color transforms, ICC APP2 chunks, embedded
thumbnails, and gain maps are not accepted. The engine never rotates or flips
pixels, reads resolution into layout, applies an ICC transform, or guesses RGB,
CMYK, or YCCK. Accepted pixels have top-left raster origin, square pixels, and
the explicit Gray or YCbCr meaning above; Figure geometry continues to come
from the typed layout contract, not JFIF density.

Color JPEG decodes with explicit `jpeg_decoder::ColorTransform::YCbCr` to
row-major RGB8 solely for bounded validity and the decoded observation.
Grayscale uses explicit `ColorTransform::Grayscale` and row-major Gray8. The
observation records exact width, height, channel/color kind, sampling factors,
output byte count, and SHA-256 of that canonical pixel buffer. The buffer is
discarded before PDF stream construction; it is not a source for a lossless/
raster fallback. No nominal DeviceRGB result is promoted to calibrated color.
The decoded pixel hash is validity evidence and is never compared directly to
a PDF renderer's raster hash: each pinned renderer must be exact across repeat
builds of the same artifact, while cross-renderer evidence compares page and
placement geometry plus fixture color sentinels without requiring byte-equal
inverse-DCT rounding.

### Deterministic sanitizer and PDF plan

After successful preflight and full decode, `typaxis.jpeg-segment-sanitizer/1`
emits the exact source byte sequence from SOI through EOI except for removal of
the one JFIF APP0 segment. It preserves the original order and bytes of every
DQT, DHT, DRI, SOF0, SOS, entropy/stuffing/restart, and EOI record. It inserts,
removes, or rewrites nothing else. Its output length is therefore exactly
`source_length - (2 + APP0_length_field)` and its SHA-256 is part of the
receipt. This is the only adopted JPEG transform: there is no coefficient
requantization, pixel re-encoding, quality choice, chroma resampling, metadata
retention, orientation transform, or PNG transcode.

Sanitization uses a validation pass followed by one exact-size allocation.
The decoded pixel buffer is released first. While source and normalized bytes
coexist, their checked sum is charged once to the existing `max_spool_bytes`;
removing APP0 proves the normalized stream is smaller than the already admitted
source, so `max_image_bytes` is not consumed a second time. A short write,
length/hash difference, source mutation, or sanitizer replay difference is
terminal and leaves no frozen resource plan.

The frozen PDF image plan is exact:

| JPEG fact | PDF image XObject value |
| --- | --- |
| grayscale | `/ColorSpace /DeviceGray`, `/BitsPerComponent 8`, `/Filter /DCTDecode`, `/DecodeParms << /ColorTransform 0 >>` |
| YCbCr | `/ColorSpace /DeviceRGB`, `/BitsPerComponent 8`, `/Filter /DCTDecode`, `/DecodeParms << /ColorTransform 1 >>` |
| dimensions | `/Width` and `/Height` equal the attested positive dimensions |
| stream | exact sanitized bytes and direct `/Length` |

There is no `/SMask`, `/Mask`, `/Decode` override, ICCBased color space,
metadata stream, interpolation flag inferred from input, alternate image, or
filter chain. The late finalizer creates one plan per used ImageResourceId and
binds its source, normalized, and pixel-observation hashes plus profile and
limits. The common collection sort key remains `(image, source hash,
ImageResourceId)`; downstream hashes do not reorder or merge resources. The
PDF allocator may choose only the canonical object and resource names already
governed by the common backend contract.

## Closed standalone OpenType/CFF1 component

### Sfnt and table admission

`typaxis.resource-profile/sfnt-cff1/1` accepts one standalone sfnt whose first
four bytes are `OTTO`, whose declaration has `face_index = 0`, and whose
directory contains exactly the following required/optional table vocabulary:

| Class | Exact table tags |
| --- | --- |
| required | `CFF `, `OS/2`, `cmap`, `head`, `hhea`, `hmtx`, `maxp`, `name`, `post` |
| optional shaping/semantics | `BASE`, `GDEF`, `GPOS`, `GSUB`, `JSTF`, `MATH`, `kern` |

Every other tag is rejected, even if an external parser could ignore it.
This rejects `CFF2`, `glyf`/`loca`, TrueType instruction tables, TTC/OTC,
WOFF/WOFF2, bare CFF, EOT, Type 1, SVG/color/bitmap tables, DSIG, vertical
metrics, and variation tables including `avar`, `fvar`, `gvar`, `HVAR`,
`MVAR`, `STAT`, and `VVAR`. The profile does not adopt variable instances,
COLR/CPAL, CBDT/CBLC, sbix, SVG glyphs, bitmap strikes, or vertical writing.

The in-tree sfnt preflight reads the 12-byte header and fixed-size directory
before `read-fonts` receives a view. It checks the exact `OTTO` scaler,
`numTables`, formula-derived search fields, unique ascending unsigned tag-byte
order, checked offset/length endpoints outside the header/directory,
four-byte alignment, table nonoverlap, and zero-filled padding/gaps. The padded
end of the last table is the source length, so no nonzero unparsed prefix or
trailing payload exists. It verifies
per-table checksums with `head.checkSumAdjustment` treated as zero and the
whole-font `0xB1B0AFBA` checksum. Directory or table allocations are
forbidden until `max_font_tables` permits the count. No repair, tag
deduplication, last-table-wins rule, or checksum warning exists.

Required typed cross-checks include:

- `head` is exactly 54 bytes, has version 1.0, exact magic `0x5F0F3CF5`,
  `unitsPerEm = 1000`, ordered signed-16-bit bounds,
  `indexToLocFormat = 0`, and `glyphDataFormat = 0`; this CFF1 component is
  narrower than the unchanged TrueType 16-through-16,384 rule, and variation
  is rejected by the closed table set;
- `maxp` is exactly the six-byte CFF 0.5 form and its positive `numGlyphs`
  agrees with CFF CharStrings; `hhea` is exactly the 36-byte version-1.0 form
  with zero reserved fields and metricDataFormat, and its positive
  `numberOfHMetrics` plus the remaining bearings make `hmtx` exactly the
  formula-derived length for that glyph count;
- `cmap` contains only well-formed Unicode format 4 and/or 12 mappings from
  platform 0 or Microsoft 3/1 or 3/10, rejects surrogate/out-of-range scalars,
  maps only in-range glyph IDs, and requires overlapping subtables to agree;
- `name` yields one canonical nonempty family, subfamily, and PostScript name
  by preferring Windows Unicode language `0x0409` (encoding 10 then 1), then
  the lowest platform-0 encoding/language, then the lowest other Windows
  Unicode encoding/language; every candidate is fully decoded before choice.
  Family/subfamily must be BMP scalar strings; the PostScript name is printable
  ASCII without whitespace, delimiters, or a caller-provided six-letter subset
  prefix. The single CFF Name INDEX byte string must exactly equal that selected
  PostScript name;
- `post` is exactly the 32-byte version-3.0 form; `OS/2` is an exactly sized
  supported version 0 through 5 record; horizontal metrics and global bounding
  boxes are internally valid;
- every optional layout/MATH table is fully parsed and cross-references only
  known glyphs before shaping can consume the face.

The admitted CFF program is version 1.0, contains one Name INDEX entry, one
Top DICT, one CharStrings INDEX, and is name-keyed. Its Top DICT has no ROS,
FDArray, FDSelect, Multiple Master, synthetic-base, CID-keyed, or CFF2 state.
`CharstringType` is explicitly 2, `PaintType` is absent or 0, and StrokeWidth
is absent or zero; stroked glyph programs are not admitted. FontMatrix is
absent or exactly `[0.001 0 0 0.001 0 0]`, and CFF FontBBox must exactly agree
with the admitted `head` bounds before subsetting.
It has one Private DICT and at most one Local Subrs INDEX plus one Global Subrs
INDEX. Charset, Encoding, String INDEX, INDEX offsets, DICT operands, private
bounds, subroutine bias, and glyph count are checked. Duplicate, reserved, or
unknown Top/Private DICT operators are rejected. Known informational Top DICT
and hint Private DICT operators may be parsed but are not copied; PostScript,
BaseFontName/BaseFontBlend, SyntheticBase, Multiple Master, and every CID-keyed
operator are rejected. A bare or CID-keyed CFF1 source is intentionally outside
`/1`; accepted name-keyed input is converted to one canonical CID-keyed subset
for PDF. Header/index/dict/string/charset/encoding/private/subroutine/charstring
ranges must account for the complete CFF table without overlap or unreferenced
gap/trailing bytes.

### Bounded Type 2 evaluation and hint policy

The in-tree `typaxis.cff1-charstring-evaluator/1` consumes only preflighted
CFF byte slices. It uses explicit fixed/limit-checked operand and call stacks;
Rust recursion is forbidden. The call depth may not exceed the Type 2 maximum
of 10, with the top-level charstring at depth zero and each subroutine entry
adding one. Every decoded operand, operator, hint-mask byte, call-frame push,
return, stem, flex expansion, and endchar event consumes one checked operation
before execution; Move/Line/Cubic/Close output is charged separately as an
outline segment. Subroutine indexes are bias-adjusted with checked arithmetic
and must be in range. The operand stack may contain at most 48 values and the
font may declare at most 96 stems; max+1 is rejected before the push/stem.

The closed operator vocabulary is numeric operands plus `hstem`, `vstem`,
`vmoveto`, `rlineto`, `hlineto`, `vlineto`, `rrcurveto`, `callsubr`, `return`,
`endchar`, `hstemhm`, `hintmask`, `cntrmask`, `rmoveto`, `hmoveto`, `vstemhm`,
`rcurveline`, `rlinecurve`, `vvcurveto`, `hhcurveto`, `callgsubr`,
`vhcurveto`, `hvcurveto`, `hflex`, `flex`, `hflex1`, and `flex1`. Reserved,
deprecated, escaped arithmetic/transient-array/storage, `random`, and unknown
operators are rejected. An `endchar` carrying seac operands is rejected, so a
glyph never names another glyph program implicitly. Every path coordinate and
bounds accumulation uses signed `i64` arithmetic in units of 1/65,536 font
unit and must remain in the signed `i32` 16.16 raw range after each operator;
there is no rounding. An integral result uses the shortest integer encoding,
and a fractional result uses the exact Type 2 16.16 encoding.

The optional source width operand is accepted only at the first operator and
with the exact Type 2 arity rule. It is checked with source `nominalWidthX` /
`defaultWidthX` but does not supply OpenType or PDF advance authority: the
admitted `hmtx` value does. Thus a source CFF width disagreement is treated as
non-authoritative input and is not propagated into the subset or `/W`.

Stem and mask syntax is validated and charged but is not retained. The
canonical subset deterministically strips all source hints and subroutines,
flattens flex to ordinary cubic curves, and emits only absolute-move-derived
relative `rmoveto`, `rlineto`, `rrcurveto`, and `endchar` programs with the
shortest legal Type 2 number encodings. A new move or endchar adds one synthetic
Close to the canonical outline IR when a contour is open; Type 2 output relies
on the format's implicit contour close and has no invented close operator. The
output Private DICT fixes `nominalWidthX = 32768` and `defaultWidthX = 0`;
every glyph encodes the exact signed integer `hmtx.advanceWidth - 32768` as the
optional first width operand before its first move, or before endchar for an
empty glyph. This covers the complete admitted unsigned-16-bit advance domain
with the Type 2 signed 16.16 number representation; the reconstructed width is
checked back against `hmtx`. It never autohints, consults a rasterizer, rounds
through floating point, or preserves an unvalidated unreachable subroutine.
This exact hint-stripping policy is part of the subset identity; keeping hints
later requires a new identity and differential review.

### Glyph closure and deterministic subset

The selected-glyph owner unions glyph IDs from every sealed shaping run,
generated-text run, math glyph/variant, and other receipt-authorized paint for
one admitted `(FontFaceId, FontInstanceId)`. It rejects a foreign face, hash,
feature set, instance, package, session, or LayoutEpoch. Each instance's
canonical set is source glyph 0 (`.notdef`) followed by the distinct selected
nonzero source glyph IDs in ascending numeric order. CFF seac, color layers,
variations, and glyph-to-glyph outline references are absent, so no hidden
glyph expansion is allowed.

`max_cids_per_font` limits selected nonzero glyphs in each instance. Because
PDF CIDs and output GIDs are 16-bit and output 0 is `.notdef`, the accepted
selected count is at most `min(max_cids_per_font, 65,534)`. The source glyph
count separately obeys `max_font_glyphs`. The evaluator first unions instance
closures per face and evaluates each distinct `(FontFaceId, source GID)` once
in ascending order; a sealed outline can then be reused by that face's
instance subsets without another operation/segment charge. Retries, pagination
passes, text occurrences, or multiple Unicode mappings do not reset or
multiply the evaluator budget.

The output mapping is dense: `.notdef -> CID/GID 0`, then the ascending source
GIDs map to CID/GID 1 through N. The subsetter emits a standalone `OTTO` sfnt
with exactly these tables in unsigned tag-byte order:

```text
CFF (tag bytes 43 46 46 20)
OS/2
cmap
head
hhea
hmtx
maxp
name
post
```

Rebuilt tables use the deterministic encodings below; all tables use four-byte
zero padding, checked checksums, and a recomputed whole-font
`checkSumAdjustment`. The output CFF1 has header bytes `01 00 04 04`, one Name
INDEX entry equal to the subset name,
one CID-keyed Top DICT with Registry `Adobe`, Ordering `Identity`, Supplement
0, `CIDFontType = 0`, explicit `CharstringType = 2`, and `CIDCount = N + 1`;
the default FontMatrix is omitted. It has an empty Global Subrs INDEX, one Font
DICT/Private DICT selected by format-0 FDSelect value zero for every glyph, no
Encoding, subrs, or hints, and a format-0 dense charset whose CIDs equal output
GIDs. INDEX objects use the smallest legal `offSize`; DICT operators use
ascending encoded operator number and the shortest legal exact operand; the
String INDEX contains only required non-standard strings in first-reference
order.
Global and per-glyph outline minima are rounded toward negative infinity and
maxima toward positive infinity from exact 16.16 values; an empty glyph has
zero bounds for horizontal-metric calculations. The global result must be
nondegenerate and fit signed 16-bit; it is the one shared CFF, `head`, and PDF
FontBBox. The output `head` is the admitted 54-byte version-1.0 record with
`checkSumAdjustment`, created, and modified zeroed during table construction;
it retains the admitted flags, revision, magic, unitsPerEm, macStyle,
lowestRecPPEM, and fontDirectionHint, replaces the four bounds, and fixes
`indexToLocFormat = 0` and `glyphDataFormat = 0`. The final sfnt checksum pass
then writes the sole nonzero `checkSumAdjustment`. The output `hhea` is the
admitted 36-byte version-1.0 record with ascent, descent, lineGap, caret slope,
and caret offset retained; its four reserved fields and metricDataFormat are
zero. It recomputes `advanceWidthMax`, `minLeftSideBearing`,
`minRightSideBearing`, and `xMaxExtent` from the outward-rounded per-glyph bbox,
copied advance, and copied left side bearing. Per glyph, width is `xMax - xMin`,
right side bearing is `advance - leftSideBearing - width`, and extent is
`leftSideBearing + width`; those four fields are respectively the maximum,
minimum, minimum, and maximum over the dense glyph order. It sets
`numberOfHMetrics = N + 1`; every subtraction, addition, and derived signed
field must fit. `maxp` is the six-byte version-0.5
table with `numGlyphs = N + 1`. Admitted `OS/2` and version-3 `post` bytes are
copied exactly. Horizontal advances and left side bearings are copied through
the dense map, with one full `hmtx` record per output glyph. Output `cmap` has
table version 0 and exactly one platform-3, encoding-10, format-12 subtable.
The source subtables are first merged into one scalar-sorted map, with agreeing
duplicates collapsed; only entries mapped to a selected nonzero source glyph
are retained and remapped to its output GID. They are encoded as maximal
ascending constant-delta groups. The name table is format 0 and contains
exactly three
UTF-16BE records, in name-ID order, for platform 3, encoding 10, language
0x0409, and name IDs 1, 2, and 6 using the selected source family/subfamily
and the subset PostScript name; strings are concatenated in record order
without sharing. Optional shaping tables are not copied because shaping is
already sealed.

The existing FontInstanceId-derived six-uppercase-letter subset-name algorithm
is unchanged: the exact output form is `AAAAAA+Typaxis`, where `AAAAAA`
denotes the derived six-letter tag, not a literal constant or the source
PostScript name. That same per-instance value is written to CFF FontName, the
canonical name table's name ID 6, Type0 `/BaseFont`, descendant `/BaseFont`,
and FontDescriptor `/FontName`, then re-extracted from the subset bytes by the
sealed encoder owner. The admitted source PostScript name remains a separate
attested manifest fact. The accepted raw embedding bits remain in the copied
`OS/2` table. No source table order, path, file time, host library, random
value, or hash-map order enters the bytes. The exact subset byte SHA-256 and
the JCS subset-plan fingerprint are separate
recorded facts.

### Embedding permission and PDF plan

The admission owner reads `OS/2.fsType` before shaping, outline evaluation,
subset allocation, or PDF object allocation. The only accepted raw values are:

| `fsType` | Decision |
| ---: | --- |
| `0x0000` | installable embedding: subset embedding admitted |
| `0x0004` | preview-and-print embedding: subset embedding admitted |
| `0x0008` | editable embedding: subset embedding admitted |

Restricted-license embedding (`0x0002`), no-subsetting (`0x0100`), bitmap-only
embedding (`0x0200`), reserved bits, or multiple mutually exclusive base bits
are terminal `R7100`. There is no full-font fallback for no-subsetting, outline
conversion, raster-font fallback, user confirmation bypass, or “embed anyway”
warning. The decision and raw accepted value are sealed into the font admission
receipt and manifest. This technical check is not a legal opinion or license
grant; the producer remains responsible for having rights to use and embed the
font. The `/1` permission identity is scoped to `sfnt-cff1`; this ADR neither
changes nor claims a new permission policy for the preserved TrueType
components.

The frozen PDF plan embeds the exact subset sfnt in FontDescriptor
`/FontFile3` with stream `/Subtype /OpenType`. The composite font is Type0 with
`/Encoding /Identity-H`; its descendant is `/Subtype /CIDFontType0` with
`/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>`.
Apart from serializer-owned `/Length` and the existing configured stream-filter
policy, the font stream adds only `/Subtype /OpenType`; it has no `/Length1`,
`/Length2`, `/Length3`, or `/Metadata` entry.
There is no `/CIDToGIDMap`: the canonical CID-keyed CFF charset maps CID to the
same dense output GID. `/DW` is exactly 1000 and `/W` is one array beginning at
CID 0 with every dense glyph's unsigned `hmtx` advance in CID order. CIDSet is
the shortest high-bit-first bit vector with bits 0 through N set and unused
low bits of the last byte zero. FontDescriptor metrics, bbox, and ToUnicode are
derived from the same glyph-closure and selected text-cluster receipts. A
TrueType `/FontFile2` or CIDFontType2 plan cannot satisfy this component.

Because unitsPerEm is 1000, FontDescriptor Ascent and Descent are the admitted
`hhea` signed values without rescaling. CapHeight is positive `OS/2.sCapHeight`
for version 2 or later and otherwise the positive `hhea` ascender; StemV is the
profile constant 80. ItalicAngle is the exact signed 16.16 `post` value. Flags
always contain Nonsymbolic, add FixedPitch exactly when `post.isFixedPitch` is
nonzero, add Italic exactly when ItalicAngle is nonzero, and contain no other
bits. FontBBox is the shared outward-rounded subset bbox above. Failure of any
range, positivity, or nondegeneracy check prevents the plan.

The CFF plan declares exactly six indirect roles in this order before PDF
allocation: Type0 font, CIDFontType0 descendant, FontDescriptor, OpenType
FontFile3 stream, ToUnicode stream, and CIDSet stream. The JPEG plan declares
exactly one image-XObject role. The PDF backend verifies the complete typed
blueprint and object budget before assigning dense object IDs; neither plan
may gain an inferred metadata, mask, CIDToGIDMap, or auxiliary font object.

The late finalizer issues one plan per used `(FontFaceId, FontInstanceId)` and
binds face index zero, source hash, permission decision, glyph-closure
fingerprint, subset hash, profile, and limits. The common collection sort key
remains `(font, source hash, FontInstanceId)`; the additional facts validate
the plan but cannot reorder or merge it. PDF object allocation consumes that
plan and records exact object roles, stream hash, dictionary facts, CID
mapping, widths, and ToUnicode mapping. An unselected admitted font has
admission and permission evidence but no subset plan or PDF font object.

## Inclusive resource and work limits

All limits are positive and inclusive. Exact max succeeds; an operation that
would produce max+1 is refused before the corresponding read, allocation,
decode, evaluation, subset write, plan, or PDF object. Checked overflow is the
same refusal, never wrapping or saturation.

JPEG reuses the existing base limits:

| Work | Limit and charge | Refusal owner/code |
| --- | --- | --- |
| stable source bytes | per-image `max_image_bytes` and session `max_resource_bytes` | stable reader before marker scan, `R7100` |
| dimensions | checked `width * height` against `max_image_pixels` | marker preflight before decoder construction, `R7110` |
| decoded pixels/workspace | canonical Gray `pixels` or RGB `3 * pixels`; separately, the exact larger of padded component planes plus live MCU-row coefficients (including replacement duplication) and padded component planes plus canonical output plus live upsample rows, all against `max_decoded_image_bytes` | marker preflight plus external output-buffer maximum before allocation, `R7111` |
| source plus sanitized bytes | checked simultaneous byte total against `max_spool_bytes`; the smaller normalized stream inherits the source's `max_image_bytes` proof without a second charge | sanitizer permit before exact allocation, `R7100` |
| selected placement and PDF objects | existing `max_fragments`, `max_pdf_objects`, `max_spool_bytes`, and `max_output_bytes` | selected/PDF/output owners and existing codes |

MI4-12 extends the already private `M4ResourceLimits` from ADR-0033 with these
six distinct fields. They do not change the public/current configuration,
base `ResourceLimits` JCS, or old-profile fingerprints before MI4-13. Under
`/1` all six are consumed only by `sfnt-cff1`; they do not re-interpret the
preserved TrueType components:

| Limit | Default | Hard maximum | Stable code |
| --- | ---: | ---: | --- |
| `max_font_tables` | 64 | 256 | `R7130` |
| `max_font_glyphs` | 65,535 | 65,535 | `R7131` |
| `max_cff_subroutines` | 100,000 | 131,070 | `R7132` |
| `max_cff_charstring_operations` | 10,000,000 | 100,000,000 | `R7133` |
| `max_cff_outline_segments` | 5,000,000 | 50,000,000 | `R7134` |
| `max_font_subset_bytes` | 134,217,728 | 536,870,912 | `R7135` |

Zero, a value above its hard maximum, a noninteger, or
`max_font_subset_bytes > max_spool_bytes` is `P1102` in the private versioned
config before an effective-limit receipt exists. The work codes apply only
after valid configuration:

| Work | Charge and scope | Refusal point |
| --- | --- | --- |
| sfnt directory | one per directory entry, including a later disallowed tag; per face | before directory allocation, `R7130` |
| source glyphs | `maxp.numGlyphs`; per face | before cmap/CFF glyph-index allocation, `R7131` |
| subroutines | Global Subrs count plus Local Subrs count; per face | before subroutine-offset allocation, `R7132` |
| Type 2 work | every decoded operand, operator, hint-mask byte, call-frame push, return, stem, flex expansion, and endchar event; one session-wide aggregate over distinct selected `(FontFaceId, source GID)` in ascending face/GID order | before the event, `R7133` |
| outline output | one unit for each emitted Move, Line, Cubic, or synthetic Close after flex expansion; one session-wide aggregate over the same ordered face/GID union | before appending the segment, `R7134` |
| subset bytes | exact canonical sfnt length per subset and aggregate against existing spool bytes | sizing pass before output allocation, `R7135` |

Existing `max_font_bytes`, `max_fonts`, and `max_resource_bytes` remain
additional admission bounds. Existing `max_cids_per_font` bounds selected
nonzero glyphs; it is not synonymous with `max_font_glyphs`, which bounds
untrusted source indexes before selection. `max_pdf_objects`,
`max_spool_bytes`, and `max_output_bytes` remain additional finalization
bounds. A successful evaluator/subset permit is session-bound and reusable;
another layout pass, resource, parser retry, or foreign receipt cannot reset
or substitute its aggregate budget.

The six new codes and fields remain private contract-1.4 registry values until
MI4-13. MI4-11 adds no new configurable JPEG limit. Current/frozen diagnostic
and config Schemas, public help, defaults, and seven profile descriptors do not
gain any of them early.

## Parser/decoder ownership and dependency audit

JPEG uses a defense-in-depth split. `typaxis-resource-admission` owns the
in-tree marker preflight and sanitizer and is the only stable-byte attestation
issuer. MI4-11 adds this exact direct dependency to that crate only:

| Fact | Adopted value |
| --- | --- |
| crate | `jpeg-decoder = "=0.3.2"` |
| crates.io archive SHA-256 | `00810f1d8b74be64b13dbf3db89ac67740615d6c891f0e7b6179326533011a07` |
| features | `default-features = false`, `features = ["platform_independent"]` |
| transitive parallel/SIMD policy | no `rayon`; platform-specific `arch` module disabled |
| compiled unsafe/native/build policy | compiled crate path forbids unsafe; no build script, native object, dynamic/system library, or executable |
| license | `MIT OR Apache-2.0` |
| declared MSRV | 1.61; must compile at the workspace Rust 1.75 floor |

The crate is in maintenance mode; that is an explicit risk acceptance for the
narrow, preflighted baseline subset, not permission to float to a successor.
The external decoder may validate/decode only after the in-tree marker receipt,
must receive the precomputed maximum buffer and explicit Gray/YCbCr transform,
and cannot issue the media kind or sanitizer bytes. A panic, inconsistent
metadata, unexpected output format/length, or decoder acceptance of a marker
outside the in-tree subset is terminal. PDF, layout, resources, manifest, and
CLI may not depend directly on `jpeg-decoder`.

OpenType/CFF uses the existing exact parser dependency as a typed table view,
with security/policy work kept in tree:

| Fact | Adopted value |
| --- | --- |
| crate | `read-fonts = "=0.31.3"` |
| crates.io archive SHA-256 | `5b8250b8f09ed4b9ba9271e06f10e7b1f03e8f8e3619e2368a991ecb25efa204` |
| new direct-edge request | `default-features = false`, `features = ["std"]` |
| workspace-resolved features | exactly `default`, `libm`, `std`; `default` comes from existing direct users and aliases `std`, while `libm` is required by exact-pinned `harfrust = 0.1.1`; neither adds a CFF evaluator/subsetter API |
| forbidden features | no `experimental_traverse`, `ift`, `serde`, `spec_next`, `codegen_test`, `scaler_test`, or other test feature |
| compiled unsafe/native/build policy | `read-fonts` forbids unsafe and has no build script, native object, dynamic/system library, or executable |
| license | `MIT OR Apache-2.0` |
| declared MSRV | 1.75, equal to the workspace floor |

MI4-12 adds the direct edge `typaxis-font -> read-fonts`; the existing
`typaxis-resource-admission -> typaxis-font` edge consumes its sealed
admission result. The existing normal `typaxis-shaping -> read-fonts` and
dev-only `typaxis-resources -> read-fonts` edges remain; those three are the
exact direct `read-fonts` requesters after MI4-12, and no other workspace crate
may add one under `/1`. `typaxis-font` owns the in-tree sfnt policy, cross-table
checks, Type 2 evaluator, embedding decision, glyph closure, and subset writer.
It uses neither `read-fonts` floating/libm helpers nor a recursive outline
evaluator or general font writer; Cargo's feature union nevertheless remains
recorded exactly rather than being misreported as a per-edge feature set. No
new `skrifa`, `write-fonts`, FreeType, HarfBuzz system library,
fontconfig, image crate, libjpeg, native/dynamic library, executable, network,
or platform font service is adopted. Shaping may continue using its already
exact-pinned stack only after this admission receipt; PDF never reparses the
source font.

`typaxis-testkit` must make the two package versions, archive checksums,
licenses, feature sets, MSRV declarations, direct edges, and forbidden edges
executable assertions. MI4-11/12 verification records locked `cargo metadata`,
`cargo tree -e features` for each package, Rust-1.75 check/test, a source review
of allocation/unsafe boundaries, and a checked-in RFC 8785 JCS
`typaxis.dependency-audit/1` record. That record contains an exact UTC review
date, advisory-database URL fixed to
`https://github.com/RustSec/advisory-db` and its 40-lowercase-hex commit,
the two sorted package/version/archive/license/MSRV records, the resolved
feature/direct-edge sets, unsafe/native-code review result, and advisory IDs
sorted by ASCII bytes. Success requires no unresolved applicable advisory;
ignored, withdrawn, or target-inapplicable entries require an explicit package,
advisory, reason, and reviewer record rather than disappearing. MI4-11,
MI4-12, and MI4-13 each refresh and bind this evidence; builds never query a
live database. An unexpected lock package, feature, native build, license,
checksum, advisory, or edge fails the task/release gate. Dependency upgrades,
the suggested JPEG successor, or a security-driven decoder change require a
new identity/ADR review rather than an unlocked resolution.

## Diagnostics, locations, and terminal publication

The stable classification and primary location are:

| Failure | Code and primary location |
| --- | --- |
| missing/null/unknown/wrong-typed new wire value | `P1102` at the exact `media_type` JSON Pointer |
| known media disallowed by selected profile or legacy declaration under production | `R7100` at the declaration before resource open |
| stable read/hash failure | existing resource code at typed logical resource and portable URI |
| magic/container/face/declaration mismatch | `R7100` at typed logical resource after stable read, before decode/evaluation |
| malformed or unsupported JPEG/sfnt/CFF metadata/program | `R7100` at typed logical resource |
| restricted/no-subset/bitmap-only/reserved font embedding state | `R7100` at the font resource before shaping/subset/PDF |
| pixel/decode or new font work limit | the exact `R7110`, `R7111`, or `R7130` through `R7135` owner above |
| wrong ID/hash/profile/limit/glyph/subset/permission/PDF-plan closure | `I9190` at the first canonical typed owner |

Resource diagnostics serialize the logical `ImageResourceId` or `FontFaceId`
and portable URI subject allowed by the existing diagnostic Schema, never a
host path, decoder debug string, table pointer, allocator address, or inferred
suffix. If several independent resources fail at the same safe phase boundary,
primary order is dense typed resource ID; within one font it is table tag,
source GID, then operator byte offset. Limit failure wins before the work it
guards; a structural error already proven by an earlier bounded byte read wins
over facts that would require later decode/evaluation.

All failures are terminal for the requested resource/profile. There is no
warning-only degradation, generic-image/font success, alternate decoder,
progressive-to-baseline conversion, ICC discard followed by guessed color,
JPEG-to-PNG conversion, CFF-to-path/raster conversion, full-font fallback,
license override, TrueType-plan substitution, or old-profile retry. Before the
terminal closure succeeds there is no PDF, subset, normalized resource stream,
trusted manifest record, or stdout package to publish. Existing output-session
rules govern a later filesystem failure; they do not turn a resource error into
a partial success.

## Same-resource manifest and PDF closure

An admitted JPEG follows exactly this chain:

```text
declared jpeg-baseline + JPEG component/profile receipt
  -> stable ImageResourceId bytes/length/source hash
  -> marker preflight attestation + dimensions/color/sampling/limits
  -> exact external decoded-pixel observation
  -> deterministic sanitized-stream length/hash
  -> selected Figure + DrawImage usage
  -> frozen DCTDecode plan
  -> serialized image XObject observation
  -> M4 resource manifest fact
```

An admitted OpenType/CFF1 font follows exactly this chain:

```text
declared sfnt-cff1 + CFF component/profile receipt
  -> stable FontFaceId/face-index bytes/length/source hash
  -> sfnt/CFF1 table attestation + embedding-permission decision
  -> shaping/math selected source GIDs
  -> canonical glyph closure + bounded outline evaluation
  -> deterministic CID-keyed subset bytes/hash
  -> frozen FontFile3/OpenType + CIDFontType0 plan
  -> serialized font/descriptor/ToUnicode/CIDSet observations
  -> M4 resource manifest fact
```

Every arrow consumes the preceding opaque receipt. The M4 JPEG record includes
declaration, attested kind, component/profile/parser/decoder/sanitizer
identities, source length/hash, dimensions, color, sampling, decoded length and
pixel hash, normalized length/hash, every selected usage, PDF plan fingerprint,
and object/stream observation. The M4 CFF record includes declaration,
attested kind, component/profile/parser/evaluator/subsetter/permission
identities, source length/hash, face index, table/glyph/subroutine facts,
canonical family/PostScript name, and raw accepted `fsType`. Its ordered
instance records carry FontInstanceId, selected source-GID set, dense mapping,
closure fingerprint, subset length/hash/name, PDF plan, ToUnicode/CID facts,
and exact object/stream observations.

Declaration and attestation remain separate typed members and must be equal for
the same logical ID. Hashes and plan/object facts may not be copied between IDs
even when bytes are equal. Each selected use has exactly one admitted owner.
Each used image has exactly one frozen plan and matching serialized object set;
each used `(FontFaceId, FontInstanceId)` has exactly one glyph closure, subset
plan, and matching composite-font object set. Every plan/object points back to
exactly one manifest resource and, for fonts, one ordered instance record.
Unused admitted resources retain declaration/admission evidence but have no
use, plan, subset, or PDF object. Missing, extra, duplicate, wrong-ID,
wrong-face/instance, wrong-hash, wrong-glyph, wrong-permission, wrong-subtype,
or stale-epoch facts are `I9190`, never repaired from the PDF or JSON.

## Implementation and publication sequence

MI4-11 privately adds `jpeg-baseline` to the independent 1.4 Wire/domain/Schema
and production descriptor staging, implements the marker/decoder/sanitizer,
Figure/Display/DCTDecode/manifest chain, exact/max+1 and tamper fixtures, and
old-profile rejection. It must not add `sfnt-cff1` as an implemented component
or expose a public selector.

MI4-12 separately adds `sfnt-cff1`, the six private limits, parser/evaluator/
permission/subsetter chain, shaping/selected/PDF/manifest closure, exact/max+1
and restricted-font fixtures, and old-profile rejection. It must not revise
JPEG, PNG, SafeVector, or TrueType semantics. A security-disabled component
makes the complete production target unavailable; it is not omitted from the
resource-set identity.

MI4-13 validates the complete independent 1.4 registry and combined production
fixture, then atomically publishes the contract, current aliases, descriptor,
resource-set components/media arrays, normal CLI dispatch/help, exporter,
artifacts, renderer/extractor/accessibility evidence, and release records in
ADR-0032's order. Before that gate, current contract/Schema aliases/capability
bytes remain 1.3, frozen registries remain byte-identical, all seven public
profile domains and artifact encoders remain unchanged, the default remains
`paragraph-1`, and public commands reject contract 1.4 and
`production-book-1`.

## Rejected alternatives

1. **Use generic `jpeg` or `otf` declarations.** They conceal coding process,
   container, CFF generation, and PDF-plan differences and cannot form a
   closed capability claim.
2. **Sniff from suffix or MIME and retry decoders.** It gives host/caller data
   authority over typed admission and makes mismatch behavior order-dependent.
3. **Accept progressive, EXIF orientation, ICC, Adobe RGB/CMYK, or arbitrary
   APP metadata.** Each requires additional transform, metadata, color, and
   reproducibility policy not adopted by `/1`.
4. **Decode and re-encode JPEG.** A quality/subsampling/quantization encoder
   would add lossy nondeterminism; the exact segment sanitizer preserves DCT
   data while removing the sole admitted metadata segment.
5. **Pass unvalidated JPEG directly to DCTDecode.** PDF parser acceptance is
   not resource attestation and cannot supply bounded dimensions or decoded
   evidence.
6. **Treat every `OTTO` file or bare CFF as `sfnt-cff1`.** Collections, CFF2,
   CID-keyed source programs, unknown tables, color, and variations have
   different indexing/evaluation/subset obligations.
7. **Keep all CFF hints/subroutines or use a platform font writer.** That
   retains unselected program surface and makes output/tool identity less
   closed; `/1` uses bounded evaluation and canonical hint-stripped programs.
8. **Embed the original font when subsetting is forbidden.** The production
   profile requires selected-glyph closure, so no-subsetting is a rejection,
   not permission to broaden the artifact.
9. **Reuse the TrueType FontFile2/CIDFontType2 plan.** CFF1 in OpenType requires
   FontFile3/OpenType and CIDFontType0 with a different glyph mapping.
10. **Let the PDF or manifest reconstruct declaration, glyphs, permission, or
    color.** Downstream observations verify receipts; they do not create
    resource authority.

## Consequences

- JPEG and OpenType/CFF become independently identifiable closed production
  components without broadening PNG, SafeVector, TrueType, or an old profile.
- JPEG bytes have one deterministic metadata-removal transform, explicit
  Gray/YCbCr meaning, bounded decoded evidence, and an exact DCTDecode plan.
- The initial CFF profile is intentionally standalone, nonvariable,
  noncolor, name-keyed CFF1 input; it produces a deterministic hint-stripped
  CID-keyed subset and rejects incompatible embedding permissions before PDF.
- New parsing dependencies are exact-pinned, feature-closed, MSRV-compatible,
  edge-limited, and auditable; media and policy authority remains in tree.
- MI4-11 and MI4-12 have separate implementation targets, while MI4-13 still
  owns combined advertisement, public migration, and release evidence.
