# ADR-0035: Tagged PDF structure and accessibility validation

## Status

Accepted on 2026-08-29 as the tagged-structure and accessibility-validation
decision gate for M4.

This ADR extends only the non-current contract-1.4 target reserved by
[ADR-0032](ADR-0032-semantic-container-and-declared-media.md). It does not
change current `typaxis.contract/1.3`, add a public contract-1.4 decoder or
Schema alias, register `typaxis.machine-pdf/production-book-1`, or claim an
implemented tagged-PDF, PDF/UA, CLI, or release path. MI4-09 may implement
this decision through private staging. MI4-13 remains the sole publication
gate.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: the logical structure, marked-content, artifact, conformance, and validation policy is closed here |
| implemented | No: structure registry, selected binding, PDF graph, manifest, and validator work belongs to MI4-09 |
| public CLI E2E | No: public commands still reject contract 1.4 and the target profile |
| release-supported | No: complete Matterhorn evidence and atomic publication remain later gates |

## Context

The existing DocumentPackage already distinguishes paragraphs, headings,
lists, tables, figures, links, references, footnotes, semantic containers,
and math. ADR-0033 binds math source, visual paint, producer speech, and PDF
`/ActualText`; ADR-0034 binds document and node language, outline source
owners, and PDF navigation. None of those receipts alone supplies a PDF
structure tree. Reconstructing roles from Display commands, coordinates,
font size, object order, or visible strings would lose the source ownership
that makes those distinctions trustworthy.

The target PDF backend is the PDF 1.7 backend adopted by ADR-0014. The matching
accessibility standard is
[ISO 14289-1:2014 (PDF/UA-1)](https://www.iso.org/standard/64599.html), based
on ISO 32000-1:2008. PDF/UA-2 is based on PDF 2.0 and is not silently selected.
The implementation guidance and validation inventory used by this decision
are the PDF Association's
[Tagged PDF Best Practice Guide: Syntax](https://pdfa.org/resource/tagged-pdf-best-practice-guide-syntax/),
[Matterhorn Protocol 1.1](https://pdfa.org/resource/the-matterhorn-protocol/),
and [PDF/UA-1 Reference Suite 1.1](https://pdfa.org/resource/pdfua-reference-suite/).
Matterhorn 1.1 contains 136 failure conditions: 87 machine checks, 47 checks
normally requiring human judgment, and two without a specific test. A machine
validator success is therefore necessary but is not, by itself, a claim that
the document's semantics or alternatives are appropriate.

The design inputs are
[docs/25](../docs/25-machine-input-pdf-improvements.md) sections 7 and 13.4,
ADR-0003, ADR-0008, ADR-0010 through ADR-0015, ADR-0020, ADR-0023,
ADR-0027 through ADR-0034, plus invariants I-003, I-009, I-014, I-025,
I-030, I-031, I-032, I-034, I-037, I-040, I-053, I-059, I-063, I-065,
I-067, I-068, I-073, and I-076 through I-080. Existing source, selected-state,
Display, PDF, limit, diagnostic, and atomic-publication rules remain normative
unless this ADR narrows them.

## Adopted identities

The following identities are fixed:

| Item | Identifier |
| --- | --- |
| PDF/UA target | `typaxis.pdfua1-profile/1` |
| production accessibility preflight | `typaxis.production-accessibility-preflight/1` |
| profile-bound lower authorization | `typaxis.production-accessibility-authorization/1` |
| structure role vocabulary and generated slots | `typaxis.structure-role-vocabulary/1` |
| validated logical structure registry | `typaxis.structure-registry/1` |
| selected fragment to structure/artifact binding | `typaxis.selected-structure-binding/1` |
| page-local marked-content and object-reference plan | `typaxis.marked-content-plan/1` |
| tagged PDF observation | `typaxis.tagged-pdf-observation/1` |
| PDF/UA XMP serialization | `typaxis.book-xmp/2` |
| in-tree independent validator | `typaxis.tagged-pdf-validator/1` |
| release validation policy | `typaxis.pdfua1-validation-policy/1` |
| Matterhorn human-assessment ledger | `typaxis.matterhorn-assessment/1` |

Every fingerprint under these identities is SHA-256 over an RFC 8785 JCS
record with a required `algorithm` member. Ordered arrays retain the exact
order below. Sets are first sorted by the stated byte or numeric key. A hash
map, thread completion order, page object number, or validator report order
cannot choose semantic order.

Changing a role, generated wrapper, parent rule, reading order, artifact
class, alternative mapping, MCID/ParentTree allocation, XMP conformance
identifier, validation tool/version/profile, warning policy, or limit charge
requires the corresponding `/2` identity and a contract/profile compatibility
review. A security response may make the target unavailable without reusing
an identity for reduced validation.

## Closed accessibility profile

`typaxis.pdfua1-profile/1` is the required accessibility component of the
future `typaxis.machine-pdf/production-book-1` descriptor. It targets an
unencrypted PDF 1.7 file with PDF/UA-1 identification. It does not target
PDF/UA-2, WTPDF, PDF/A, WCAG, Section 508, or another legal accessibility
regime. Passing this profile cannot be advertised as compliance with one of
those other regimes.

The target consumes the complete closed production-book semantic domain. In
addition to the earlier profile checks, accessibility preflight requires:

- non-null `metadata.title`, because PDF/UA-1 requires `dc:title`; the existing
  metadata string rules continue to require a nonempty meaningful scalar
  sequence syntactically, while Matterhorn human review determines whether it
  clearly identifies the document;
- body headings represented only by `/H1` through `/H6`; if any heading is
  present, the first is level 1 and a later level may increase by at most one
  in logical reading order;
- every Figure to be semantic and to have the existing nonempty producer
  `alt`, with at least one non-whitespace Unicode scalar; contract 1.4 has no
  authored decorative-Figure switch;
- every Table to have at least one head row and one body row, every head cell
  to have a non-whitespace accessible-content contribution, and the declared
  head rows to be the complete set of column headers;
- every Paragraph and Heading to have at least one real-content contribution
  after transparent Anchor/break handling;
- every Link to have a closed accessible-name plan that proves at least one
  non-whitespace contribution from exact Text/math speech or a
  guaranteed-nonempty Reference/footnote-reference label;
- every Link and every outline source to have a computed language equal to the
  document language, because PDF 1.7 has no `/Lang` entry on annotation or
  outline-item dictionaries and this `/1` profile does not insert language
  escape sequences into their text strings; other semantic owners may retain
  receipt-bound language overrides;
- every footnote definition to have a body reference, as already required by
  the footnote profile, and every footnote reference used by this target to
  occur in a paragraph inline subtree and outside a Link; and
- every potentially painting semantic variant to have exactly one real-content
  or artifact rule in this ADR's closed mapping.

A structurally valid package outside those conditions fails production profile
preflight with `L5100` before layout/PDF. This does not make a formerly valid
contract-1.4 package malformed and does not broaden or narrow an older public
profile. Semantic claims that require judgment—whether an `alt` is useful,
whether a table really uses only column headers, whether heading levels match
the author's intent, or whether reading order is appropriate—remain subject
to the Matterhorn ledger.

After selection, each Figure must bind exactly one admitted image/vector
placement, each Link must bind one or more annotations and the final nonempty
accessible-name sequence below, and every selected paint must bind exactly one
real-content owner or artifact occurrence. Failure of a preflight-valid owner
to close those selected relations is `I9190`, not a second semantic
interpretation or an `L5100` fallback.

Machine-profile preflight issues a sealed
`ProductionAccessibilityPreflightReceipt`. Its only lower dependency-inversion
projection is the syntax-owned `ProductionAccessibilityProfileAuthorization`,
created while binding the exact nonzero preflight-receipt fingerprint to the
package, descriptor, language/outline/math-source facts, and effective limits.
The `typaxis-layout-contract` structure builder requires that authorization as
well as the underlying syntax receipts. This preserves the existing dependency
direction: neither layout-contract nor Display depends on or reimplements the
machine-profile decision.

## PDF/UA document-level mapping and XMP versioning

The tagged target retains ADR-0034's metadata facts and introduces
`typaxis.book-xmp/2`, as required by that ADR's versioning rule. Version 1
remains the byte identity implemented by the MI4-07 navigation slice. Version
2 preserves every version-1 property, value, omission rule, XML escape, and
relative property order, then adds the standardized PDF/UA identification
namespace and property:

```xml
xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/"
<pdfuaid:part>1</pdfuaid:part>
```

The namespace declaration follows `xmlns:xmp` in the one
`rdf:Description`; `pdfuaid:part` is the final property after `pdf:Producer`.
It is the integer lexical value `1`. `pdfuaid:rev`, `pdfuaid:amd`,
`pdfuaid:corr`, PDF/A identifiers, extension schemas, arbitrary RDF, packet
timestamps, padding, and host-derived values are absent. Version 2 always
contains `dc:title`, `dc:language`, `pdf:Producer`, and `pdfuaid:part`.

The catalog and page graph add exactly the PDF/UA-owned facts below:

- `/MarkInfo << /Marked true >>`;
- `/StructTreeRoot` referencing the one indirect structure-tree root;
- `/ViewerPreferences << /DisplayDocTitle true >>`;
- the existing canonical catalog `/Lang` and version-2 Metadata stream;
- `/Tabs /S` on every page containing an annotation; and
- no `/Suspects true`, encryption dictionary, reference XObject, XFA,
  optional-content, form field, multimedia, JavaScript, or other feature not
  admitted by the production profile.

The PDF header remains `%PDF-1.7`. A producer or tool cannot switch the
profile to PDF/UA-2 by writing a different XMP value, catalog `/Version`, or
validator flag.

## Exhaustive source-to-structure mapping

The registry maps the complete target semantic vocabulary. A source-backed
row means exactly one structure element per source NodeId, even when its paint
is split across lines, columns, or pages. A generated row has no caller ID and
uses the closed generated key described below.

### Block and catalog owners

| Semantic owner | PDF structure type | Fixed children or attributes |
| --- | --- | --- |
| `Document` | `/Document` | sole child of `/StructTreeRoot`; owns body structure in source order |
| `semantic_container(result)` | `/Result` | role-mapped to `/Div`; owns its block children |
| `semantic_container(proof)` | `/Proof` | role-mapped to `/Div`; owns its block children |
| `semantic_container(exercise)` | `/Exercise` | role-mapped to `/Div`; owns its block children |
| `paragraph` | `/P` | owns inline structure in logical source order |
| `heading(level = N)` | `/H1` through `/H6` | exact source level; owns inline structure |
| `list` | `/L` | `/A << /O /List /ListNumbering /Decimal >>` when ordered, with `/Disc` substituted when unordered |
| `list_item` | `/LI` | exactly generated `/Lbl`, then generated `/LBody` |
| `table` | `/Table` | generated `/THead`, then generated `/TBody`; an empty section is forbidden by this profile |
| head/body `table_row` | `/TR` | child of the matching generated section; cells in increasing origin-column order |
| head `table_cell` | `/TH` | table attributes and deterministic `/ID`; owns cell blocks |
| body `table_cell` | `/TD` | table attributes and `/Headers`; owns cell blocks |
| `figure` | `/Figure` | exact `/Alt`; image/vector MCR first, optional generated `/Caption` last |
| `display_math` | `/Formula` | exact producer `/Alt`; owns the one math MCR |
| `footnote_definition` | `/Note` | deterministic unique `/ID`; generated `/Lbl`, then definition blocks |
| `page_break` | no element | no paint and no marked content; affects pagination only |
| table column declaration | no element | sizing input only; cell attributes retain the column relation |

### Inline owners

| Semantic owner | PDF structure type | Fixed behavior |
| --- | --- | --- |
| `text` | `/Span` | owns all selected text MCRs for that NodeId in logical byte order |
| `emphasis` | `/Em` | custom type role-mapped to `/Span`; owns its inline children |
| `strong` | `/Strong` | custom type role-mapped to `/Span`; owns its inline children |
| `link` | `/Link` | owns its inline children followed by its contiguous annotation OBJRs |
| `reference` (`text`, `page`, or `number`) | `/Reference` | generated selected label is real content owned directly by this element |
| `footnote_reference` | `/Reference` | owns one generated `/Lbl`; receipt relates it to exactly one `/Note` |
| `inline_math` | `/Formula` | same `/Alt`, MCR, and language policy as display math |
| `anchor` | no element | named-destination owner only; no paint |
| `soft_break` | no element | retains the existing empty allowed-break site; no paint/MCID |
| `hard_break` | no element | retains the existing empty mandatory-break site; no paint/MCID |

Page-region roots, paragraphs, headings, text, and breaks are not body
semantic duplicates. Every selected header/footer occurrence is one
pagination artifact and creates no structure element. A page-region heading
therefore does not enter the body heading sequence or outline `/SE` mapping.

The exact `/RoleMap` has only these custom entries, in PDF-name byte order:

```text
/Em /Span
/Exercise /Div
/Proof /Div
/Result /Div
/Strong /Span
```

Standard types are never remapped. Unknown source variants, unknown generated
slots, a custom type without the exact mapping, a second mapping target, or a
circular map is not a compatible extension.

The pre-layout registry stores a closed PDF-independent `StructureRole` enum,
not a PDF name, dictionary, object, or MCID. The mapping tables above are the
one-to-one PDF projection of those enum variants. `typaxis-layout-contract`
therefore does not serialize or parse PDF syntax; the downstream PDF-profile
finalizer maps only a receipt-authorized role to the listed `/S` name and
RoleMap entry.

## Generated structure nodes and dense identity

`StructureNodeId` is a dense zero-based `u32`. ID 0 is the source Document's
`/Document` element. The structure owner is either `Source(NodeId)` or
`Generated(GeneratedStructureKey)`. A generated key is the tuple
`(owner NodeId, slot, ordinal)`, where `slot` is exactly one of:

```text
list_label, list_body, table_head, table_body, figure_caption, footnote_label
```

Every slot has ordinal 0 in version 1. A missing required slot, a second slot,
or a caller-supplied generated key is `I9190`. Generated nodes inherit the
source owner's computed language and SourceSpan for diagnostics but do not
pretend to be source NodeIds.

The `typaxis-layout-contract` structure builder consumes the sealed
syntax-owned semantic/node/generated-site, metadata, language, outline, and
math-source receipts, validates the complete semantic tree, and allocates IDs
during one iterative logical traversal before layout. It
allocates a source element on entry, then visits each structural `/K` child in
the mapping-table order; visiting a generated wrapper allocates that wrapper
and exhausts its subtree before the next sibling. Thus `/THead` and all of its
rows/cells precede `/TBody`, and every `/Lbl` precedes its sibling `/LBody`.
MCR/OBJR positions consume no StructureNodeId. The builder does not inspect
selected coordinates or paint.

The sealed `StructureRegistryReceipt` fingerprint covers the exact package,
document and source-index identities; the profile-authorization and every
consumed upstream receipt fingerprint; the effective-limit fingerprint; and
the complete ordered StructureNodeId records with owner, role, parent, child
order, generated key, language, alternative, attributes, IDs, table-header,
footnote, Link, and outline relations. It also covers the one-time generated
node and projected-depth permits. Omitting an upstream identity or retaining
only a caller-provided digest is not a compatible receipt.

Footnote definitions are the deliberate exception to raw catalog position.
For each definition, source preorder identifies its last reference. The
`/Note` is inserted immediately after the direct inline child branch of the
owning `/P` that contains that last reference; notes anchored in the same
branch are inserted in last-reference preorder. The `/Note` is emitted exactly
once, while every reference remains a `/Reference` at its source position and
therefore precedes the matching Note. The profile restriction to paragraph,
non-Link references makes this parent unambiguous. The generated Note and
Reference `/Lbl` values are the exact same globally unique marker, so a PDF
1.7 consumer scanning forward from any reference reaches the one matching
Note before any same-labelled Note. Version 1 does not invent a PDF 2.0 `/Ref`
entry or an interactive link. The registry still records the complete
FootnoteId-to-Note and reference-to-Note relation for independent closure
checking.

Every `/Note` and `/TH` has the ASCII `/ID` value
`typaxis-se-` followed by its StructureNodeId as exactly eight lowercase hex
digits. When at least one such element exists, the structure root emits one
`/IDTree` whose byte-sorted names map those IDs to the exact structure element;
otherwise it omits `/IDTree` and allocates no IDTree object. No other element
has `/ID` in version 1.

## Canonical parentage and logical reading order

Structure `/K` order, not page geometry or MCID number, is the semantic reading
order. The rules are:

1. General body blocks retain validated source ownership and array order.
   Container splitting, columns, float placement, and page selection do not
   move their structure elements.
2. Inline children retain recursive source order. Emphasis, Strong, Link, and
   Reference boundaries are not flattened. Anchor and break nodes are
   transparent only because they have no structure element.
3. One List stays one `/L` across pages. Each `/LI` has `/Lbl` before `/LBody`;
   the label owns the exact generated marker and the body owns item blocks.
4. One Table stays one `/Table` across pages. `/THead` precedes `/TBody`; rows
   retain section order; cells use validated origin-column order. Row/cell
   fragments stay under the same source structure elements.
5. A Figure's selected image or vector is read before its caption, matching
   wire order. Caption blocks are children of one generated `/Caption` and are
   never flattened into `/Figure` marked content.
6. Formula, Figure, and semantic-container elements are never split into
   multiple semantic owners. Multiple MCRs, when a non-atomic owner spans
   pages, remain ordered children of that one element.
7. Footnote Notes use the last-reference insertion rule above. Their generated
   definition label precedes their body blocks. Repeated physical continuation
   fragments do not repeat that label or create another Note.
8. A physically floated Figure, a balanced-column fragment, or a footnote
   region can therefore have a page/paint order different from structure order.
   Both ordinals are recorded; neither is treated as a repair for the other.

Each structure element omits `/K` exactly when it has no children or MCRs;
otherwise `/K` is always an array, including for a single child. `/P` and
heading content that would have no real content after transparent nodes is
rejected by accessibility preflight rather than emitted as an empty semantic
tag. Empty data cells remain explicit `/TD` elements and may omit `/K`.

## Table header associations

Contract 1.4 declares only leading column-header rows. It has no row-header or
arbitrary header-association syntax. The tagged target therefore uses this
closed projection:

- every head cell is `/TH` with `/A << /O /Table /Scope /Column ... >>`;
- every head and body cell emits `/RowSpan` and `/ColSpan` in that same table
  attribute dictionary only when the validated value is greater than 1;
- every `/TH` uses its deterministic structure `/ID`;
- for each `/TD`, the applicable headers are all head cells whose validated
  half-open column interval intersects the data cell's interval, ordered by
  head row and then origin column; and
- `/Headers` on the `/TD` table attribute is the nonempty array of those exact
  TH ID byte strings in that order.

The source grid receipt, not coordinates or painted rules, supplies row,
origin, rowspan, colspan, and section. Repeated head paint after the first
selected occurrence is an artifact; it does not create another `/TH`, ID, or
header association. A producer whose intended table needs row headers or a
different association is outside `/1` and must fail or use a future explicit
contract rather than mislabel cells. Matterhorn human review confirms that
the adopted column association is appropriate for each release fixture.

## Alternatives, language, links, and outlines

All PDF text strings below use the existing canonical UTF-16BE-with-BOM
encoding and decode to the exact accepted Unicode scalar sequence. Version 1
does not insert U+001B PDF text-string language escape sequences; the profile
equality rule above keeps annotation `/Contents` and outline titles under the
catalog language while structure and marked-content `/Lang` handle semantic
content overrides.

### Figure and math alternatives

- `/Figure /Alt` is exactly the producer's Figure `alt`. It is not inferred
  from filename, caption, OCR, resource metadata, or surrounding text. Figure
  paint does not add `/ActualText` in version 1.
- `/Formula /Alt` is exactly ADR-0033's producer `speech`. The Formula's real
  paint MCR uses an outer `/Formula` structural sequence; its paint is wrapped
  by the one property-only `/Span` sequence whose `/ActualText` decodes to
  that same `speech`. Neither math source nor formatter output may replace it.
- Inline and display math have one Formula owner each. Vector implementation
  does not turn either into Figure, and `/ActualText` is not a substitute for
  the structure element or `/Alt`.
- Existing non-math text extraction may put `/ActualText` on its owner-bound
  `/Span` MCR only when the adopted cluster/extraction receipt requires it.
  The decoded value must equal that receipt; structure planning cannot create
  a new normalization.

### Computed language

The source Document element inherits the catalog `/Lang`. Every source or
generated structure element records ADR-0034's computed canonical language.
It emits `/Lang` exactly when that value differs from its nearest structure
parent's effective value. Moving an override to a paint fragment, dropping it
because no glyph is present, or deriving it from a font/validator is forbidden.

ADR-0034's owner-bound marked-content `/Lang` remains on painted leaves whose
language differs from the document language. Math serializes `/MCID` on the
outer `/Formula` dictionary and puts `/ActualText` plus applicable `/Lang` in
one nested property-only `/Span` dictionary. Structure language supplements
that paint rule and never replaces it. Figure
`/Alt`, Formula `/Alt`/`/ActualText`, link `/Contents`, generated labels, and
caption text all use the matching receipt language.

### Link annotation relation and accessible name

One source Link produces one `/Link` element. Its semantic children precede a
contiguous suffix of one or more OBJR dictionaries ordered by selected page,
line, then rectangle ordinal. Every OBJR has exactly `/Type /OBJR`, `/Pg`, and
`/Obj` pointing to one existing Link annotation. Every annotation has one
`/StructParent` ParentTree key and is referenced by exactly one OBJR. Multiple
annotations under one Link retain the same validated action/target.

The annotation `/Contents` is the exact accessible-name sequence obtained by
recursively concatenating the Link's semantic children in source order:
Text contributes exact source text, math contributes producer speech,
Reference and footnote-reference contribute their selected generated label,
soft/hard break contributes one U+0020, Anchor contributes empty, and
Emphasis/Strong contribute their children. No trim, whitespace collapse,
URI substitution, localization, or filename inference occurs. The result must
contain a non-whitespace scalar. The Link's computed language equals the
catalog language by profile preflight; an annotation dictionary never receives
the nonstandard `/Lang` key. Every annotation page has `/Tabs /S`.

### Outline source relation

Every ADR-0034 outline entry in the tagged target adds exactly one
`/SE` indirect reference to the structure element whose source kind/NodeId
already owns that entry. Only `/H1` through `/H6`, `/Result`, `/Proof`, or
`/Exercise` can be referenced. The catalog `/Lang` determines outline-title
language, and profile preflight requires the source element's computed language
to equal it; `/SE` records source identity but is not a language override for
the outline string. Missing, extra, wrong-owner, or duplicate `/SE` is `I9190`;
the relation cannot change label, parent, open state, destination, or
named-destination coordinates.

## Closed artifact policy

Every paint-producing Display record is classified before PDF serialization
as real content or exactly one artifact class. Two structural MCID groups, or
a structural group and an artifact, cannot overlap or nest. The exact
property-only `/Span` wrapper below may nest inside its structural group but
has no MCID or independent structure owner. Artifacts have no StructureNodeId,
MCID, MCR, OBJR, `/Alt`, or `/ActualText`, and never appear in ParentTree or
structure `/K`.

| Selected paint | Required marked-content form |
| --- | --- |
| page-master header occurrence | `/Artifact << /Type /Pagination /Subtype /Header >> BDC ... EMC` |
| page-master footer occurrence | `/Artifact << /Type /Pagination /Subtype /Footer >> BDC ... EMC` |
| standalone generated page number, if a later adopted page master emits one outside those regions | `/Artifact << /Type /Pagination >> BDC ... EMC` |
| table-head occurrence after the first semantic occurrence | `/Artifact << /Type /Pagination >> BDC ... EMC` |
| footnote separator, table rule/background, link decoration, crop-safe layout rule, or other engine-owned decoration | `/Artifact << /Type /Layout >> BDC ... EMC` |

The current table visual policy emits no rules/background, but the class is
reserved for an already adopted future decoration rather than inferred later.
Page breaks and blank pages emit no artifact merely to occupy space. Figure
images/vectors, math paths, list and footnote markers, reference labels, and
link text are real content. A producer cannot send a semantic Figure and ask
the PDF backend to artifact it; a decorative-image input requires a future
wire/profile decision.

Artifact classification never exempts text from the existing Unicode mapping
and extraction receipts. Header, footer, or page-number text must retain its
exact character mapping even though it is outside logical structure.

A reusable Form XObject contains no MCID. Its page-level `Do` invocation is
inside the Figure or Formula MCR, or inside an artifact scope for decoration.
The same Form may be invoked multiple times only under those page-owned
classifications. Marked content never begins in one stream and ends in
another. Thus a reused Form cannot trigger Matterhorn 1.1 failure condition
30-002 by carrying an MCID inside the Form stream.

Every real group uses `BDC`/`EMC`, an inline property dictionary, and the
standard structure type reached after RoleMap as its outer marked-content tag.
It never uses `BMC`, an indirect `/Properties` resource, or a caller tag. The
outer property order is `/MCID`, then `/ActualText`, then `/Lang` when those
later entries are applicable. A `/Span` structure owner puts all applicable
entries in that one outer dictionary. For any other owner needing
`/ActualText` or paint-level `/Lang`, the outer dictionary contains only
`/MCID`; one nested `/Span << /ActualText ... /Lang ... >> BDC` wrapper, with
only applicable entries in that order, covers the identical paint and closes
before the outer `EMC`. Formula always uses this nested form because its
structure owner is `/Formula` and its vector paint requires `/ActualText`.
The nested wrapper is part of the owning record's property tuple, does not
consume another MCID or MCR, and cannot cross or contain an artifact.

## Selected binding and marked-content plan

The logical structure registry exists before layout. Layout and selected state
do not change it; they issue `SelectedStructureBindingReceipt`, which maps
every selected fragment/occurrence to either one StructureNodeId plus semantic
fragment ordinal, or one closed artifact class. The receipt covers at least:

- contract/package/document, source-index, target-profile, effective-limit,
  and computed-language fingerprints;
- StructureNodeId, source/generated owner, parent, role, attributes,
  alternative hash, and logical child ordinal;
- FlowId/terminal, selected fragment key, repetition/carry index, page/frame,
  selected origin/extent, LayoutEpoch, and selected-layout fingerprint;
- admitted image/font/math receipts where their paint is selected; and
- the exact first semantic table-head occurrence, every generated list marker,
  every footnote reference/definition marker, generated reference value, and
  artifact classification.

Display construction consumes that receipt and attaches the binding to exact
ordered `DisplayPaintId` values. It cannot search coordinates, strings,
resource kinds, or style to choose a role. The marked-content planner then
partitions each page's final paint sequence into nonempty maximal contiguous
groups with one identical real owner, semantic-fragment ordinal, and property
tuple, or one identical artifact occurrence and class. It records both page
paint order and structure logical order. A group cannot contain paints from
two semantic owners or two logical fragments merely because their style or
resource matches.

That planner is a private PDF-profile finalizer in `typaxis-display-list`,
downstream of the immutable PDF-independent Display document and upstream of
PDF graph/object allocation. The Display value itself stores no MCID, PDF
name, object number, ParentTree key, or content-stream syntax. The finalizer
consumes the structure receipt re-exported through `typaxis-layout` and issues
the sealed `MarkedContentPlanReceipt`; `typaxis-pdf` consumes that receipt only
through its existing `typaxis-display-list` edge and cannot recreate it from
Display. No new or forbidden direct PDF-to-layout dependency is adopted.

Every real group gets one `MarkedContentRecordId`, page index, content-stream
ordinal 0, page paint ordinal, semantic ordinal within its structure owner,
StructureNodeId, and page-local MCID. Every artifact group gets an
`ArtifactRecordId` and the same page/paint evidence but no MCID. The plan's
bidirectional closure requires:

- every selected paint exactly once in a real or artifact group;
- every real group exactly once in its owning structure `/K` and ParentTree;
- every structure MCR exactly one real group and nonempty ordered paint set;
- every artifact scope exactly one artifact record and no structure relation;
- every selected Link annotation exactly one OBJR/StructParent relation; and
- every unselected semantic/resource record zero paint/MCR/OBJR records.

## MCID, MCR, ParentTree, and object allocation

MCIDs are page-local dense nonnegative integers. For each page, scan the one
canonical content stream in final serialized paint order and assign 0 through
N-1 to real groups only. Artifact groups consume no MCID. A page with no real
group omits `/StructParents` and has no page ParentTree entry.

Structure `/K` refers to every real group with the direct dictionary:

```text
<< /Type /MCR /Pg PAGE_REF /MCID N >>
```

Bare integer kids are not emitted. MCR dictionaries are ordered by semantic
ordinal even when their page-local MCIDs are not increasing. A source owner
split across pages retains one structure element with one MCR per contiguous
real group, each carrying the correct page reference.

ParentTree keys share one dense namespace:

1. pages with at least one real group receive keys from zero in physical page
   order; their values are arrays indexed by dense MCID and containing the
   exact owning structure-element indirect reference;
2. Link annotations then receive one key each in existing selected-Display
   page-encounter `/Annots` order; their values are the owning `/Link`
   structure-element reference, and this physical key order does not redefine
   source Link structure order; and
3. `/ParentTreeNextKey` is the first unused integer.

Each participating page stores its page key as `/StructParents`; each
annotation stores its annotation key as `/StructParent`. The one ParentTree
number-tree root uses a direct, numerically sorted `/Nums` array in version 1;
it does not build shape-dependent intermediate nodes. The one IDTree similarly
uses a direct byte-sorted `/Names` array. Their bounded serialized sizes are
checked before object allocation.

After every object role adopted through ADR-0034, PDF appends roles in this
order: StructTreeRoot, ParentTree, IDTree when nonempty, then one StructElem
per StructureNodeId. Structure elements have exactly `/Type /StructElem`,
`/S`, `/P`, applicable `/K`, `/Lang`, `/Alt`, `/ID`, `/A`, and no other
caller-controlled dictionary entries. The StructTreeRoot has `/Type
/StructTreeRoot`, `/K` as the one-element array containing the Document
reference, `/ParentTree`, `/ParentTreeNextKey`, exact `/RoleMap`, and applicable
`/IDTree`. Forward references are preflighted before dense PDF object
allocation. Worker completion and map iteration never assign an object number.

## Receipt and ownership chain

The private target chain is:

```text
strict 1.4 Wire + sealed semantic/node/generated-site registries
  -> metadata + computed-language + outline + math-source receipts
  -> production accessibility preflight
  -> ProductionAccessibilityProfileAuthorization
  -> StructureRegistryReceipt
  -> resource/media/math layout + selected fragment/artifact binding
  -> SelectedStructureBindingReceipt
  -> Display paint binding + MarkedContentPlanReceipt
  -> frozen PDF graph + VerifiedPdfBytesReceipt
  -> TaggedPdfObservation + versioned manifest facts
  -> independent in-tree validator observation
  -> veraPDF observation + Matterhorn assessment ledger
  -> MI4-13 release evidence
```

The layout-contract structure-registry owner alone owns roles and
source/generated parentage/logical order while consuming—not redefining—the
syntax-owned language, alternative, footnote, grid, Link, and outline facts
plus the dependency-inversion authorization. Machine-profile preflight owns the
accepted accessibility subset and binds that authorization to its exact receipt.
Layout owns selected fragments and artifact occurrence identity. Display owns
paint ordering and its exact binding. The private `typaxis-display-list`
marked-content finalizer alone assigns MCIDs from final Display order and
issues the separate plan receipt. PDF owns only object allocation and
canonical serialization from those receipts. Manifest and validators consume
observations; none can issue an upstream receipt or repair the file.

`TaggedPdfObservation` covers the exact catalog/MarkInfo/ViewerPreferences,
version-2 XMP hash and fields, role map, structure objects/parents/kids,
attributes/IDs/IDTree, MCR/OBJR dictionaries, page and annotation ParentTree
entries, MCIDs, artifacts, language, alternatives, link Contents/Tabs,
outline `/SE`, dense object roles, serializer receipt, and PDF bytes hash.
Closure is bidirectional against both `StructureRegistryReceipt` and
`MarkedContentPlanReceipt`. The PDF writer cannot accept a raw role name,
NodeId-to-MCID map, object number, or caller-authored structure dictionary.

## Limits and one-time accounting

No synonymous `max_structure_*`, `max_mcid_*`, or `max_accessibility_*` config
field is added. Existing inclusive limits cover the same units. Exact maximum
succeeds; max+1 is refused before allocation, ID issuance, or serialization.

| Work | Existing limit and charge | Code |
| --- | --- | --- |
| source-backed structure elements | reuse the source semantic node's existing `max_ast_nodes` charge; never charge it a second time merely for projection | `P1120` at the original node precheck |
| each generated structure node | one additional unit in StructureNodeId allocation order against the same package `max_ast_nodes` aggregate | `P1120` |
| structure nesting | `/Document` is depth 1 and every source/generated structure edge adds 1 against `max_ast_nesting_depth`; this independent projection check may be deeper than raw AST because of `/LI`, table, caption, or Note wrappers | `P1121` |
| each real MCR and artifact record | one additional per-selected-state unit, in page paint order, against `max_fragments` | `L5110` |
| page-local MCID | additionally representable as `0..=2,147,483,647`; the next real marked-content record is refused before issue even if configured `max_fragments` is higher | `L5110` |
| derived Link `/Contents` | one UTF-8 buffer against `max_text_buffer_bytes` and one derived accessible-name charge against `max_text_bytes`; source children retain their prior charges | `T2100` / `T2101` |
| `/Alt`, `/ActualText`, `/Lang`, `/Contents`, `/ID`, marker, and header-ID source values | each source/generated UTF-8 value remains bounded by `max_text_buffer_bytes`; already charged semantic values are referenced, not copied to reset or double-count the logical aggregate | `T2100` |
| StructTreeRoot, ParentTree, optional IDTree, and every StructElem | one actual object each against `max_pdf_objects`, together with all earlier PDF roles | `G6100` |
| MCR/OBJR/property dictionaries, ParentTree/IDTree arrays, UTF-16 strings, XMP, streams, and final PDF | every simultaneously live or serialized byte participates in existing `max_spool_bytes` and `max_output_bytes` accounting | `D8101` |

`StructureNodeId` must fit `u32`; the configured AST maximum and the generated
node preflight are applied before conversion. Fixed role/slot vocabulary bytes
do not create caller-controlled strings, but their serialized bytes still
participate in spool/output limits. A retry, validator rerun, alternative
layout candidate, interning pass, or foreign package receipt cannot reset an
aggregate or reuse a permit.

## Validation phases and diagnostics

Validation fails at the earliest owner with the needed authority:

| Condition | Owner, code, and primary location |
| --- | --- |
| invalid/missing Figure alt, math speech, NodeId, FootnoteId target, table grid, or earlier language/outline fact | existing strict decode/syntax rule and exact package/source location; this ADR does not reclassify it |
| null title, bad heading sequence, table without both sections/header accessible content, footnote reference outside the closed placement, Link whose accessible-name plan has no non-whitespace contribution, or source semantics outside the target accessibility subset | production accessibility preflight, `L5100`, exact owner Pointer/SourceSpan, before layout |
| generated slot, structure node/depth, marked-content, derived string, PDF object, MCID, output, or spool max+1 | owning limit code and the item that would cross the inclusive maximum |
| missing/extra/wrong role, parent, child order, language, alternative, table header, footnote relation, fragment owner, artifact class, page, semantic/paint ordinal, MCID, MCR, OBJR, ParentTree, IDTree, outline `/SE`, or serialized observation | `I9190`; no fallback, retry under another role, or partial PDF success |
| required validator absent, wrong version/profile/config, crash, timeout, malformed/truncated report, noncompliance, warning, failed human check, or incomplete ledger | release-gate failure; no release-supported claim or evidence aggregation |

Canonical registry validation visits structure nodes in StructureNodeId order,
then selected records by page/paint order, then object observations by adopted
role order. When one discrepancy makes later relations untrustworthy, the
owner stops at that safe phase boundary. A validator message cannot replace an
earlier package diagnostic, and an external report is not serialized as a
source error.

There is no text, raster, untagged, artifact, role-map, title, table-header,
or validator-warning fallback. A request for `production-book-1` fails rather
than emitting an untagged PDF or silently removing `pdfuaid:part`.

## Independent validator and release policy

The machine gate has two independent implementations:

1. The repository-owned `typaxis.tagged-pdf-validator/1`, implemented by
   extending `tools/verify_pdf_structure.py` in MI4-09, parses final PDF bytes
   without importing the writer or accepting its object model. It consumes a
   canonical expectation derived from upstream receipts and independently
   checks every fact in `TaggedPdfObservation`, including bidirectional
   paint/MCR/ParentTree closure and logical reading order.
2. The external validator is the official
   [`verapdf-greenfield-1.30.2-installer.zip`](https://software.verapdf.org/rel/1.30/)
   release,
   invoked with the [documented](https://docs.verapdf.org/cli/validation/)
   explicit flavour `ua1` (`verapdf -f ua1 FILE`). The managed tool is
   admitted from the official signed release, whose signing-key fingerprint is
   `13DD 102B 4DD6 9354 D12D E5A8 3184 8632 78B1 7FE7`. Report
   `buildInformation` must identify core and validation-model 1.30.2. Automatic
   flavour detection, development builds, another parser, metadata fixing, or
   a locally modified validation profile cannot satisfy this identity.

The veraPDF configuration records passes, does not stop at a first failure,
shows error messages, performs no metadata fix, and emits a complete
machine-readable report. The release verifier parses the report rather than
trusting process exit alone. It requires the one job to parse successfully and
be compliant, with zero failed checks/jobs, exceptions, out-of-memory events,
or unconsumed report records.

The version-1 warning allowlist is empty. Any veraPDF warning-or-higher log,
in-tree validator finding, unknown/skipped check caused by tool failure,
truncation, parse ambiguity, version mismatch, crash, timeout, or missing
binary fails the gate. A future narrowly justified warning exception requires
a new validation-policy identity naming exact tool/version/rule and an
independent assertion; it cannot be an environment variable or blanket
"warnings allowed" switch.

Matterhorn Protocol 1.1 is the fixed conformance inventory. Release evidence
contains one `typaxis.matterhorn-assessment/1` row for each of its 136 failure
conditions. Machine rows link to the veraPDF/in-tree results and cannot be
manually overridden. Each human or no-specific-test row is `pass` or
`not_applicable` with reviewer identity, rationale, and artifact-bound
evidence; `fail`, `unreviewed`, an empty rationale, or a different PDF hash
fails aggregation. In particular the ledger reviews title fitness, alternative
quality, language appropriateness, semantic role/parentage, logical reading
order, table association, heading intent, color/contrast, link purpose,
navigation, and font embedding permission where applicable.

The PDF/UA identifier describes the candidate file's target. Typaxis may call
an artifact PDF/UA-1 conforming or release-supported only when the in-tree
validator, exact veraPDF gate, and complete Matterhorn ledger all pass for the
same PDF hash and source/profile receipts. A veraPDF-only success may be
reported only as "passes the machine-verifiable PDF/UA-1 checks in veraPDF
1.30.2". No general accessibility, WCAG, legal, or validator-unsupported role
claim follows from it.

MI4-09 must add fixtures covering every adopted role and artifact class plus
missing/extra/wrong-parent/order/MCID/alternative/language/header/relation
tamper. MI4-13 must bind the exact validator binaries/config/reports and human
ledger into the release evidence on explicitly managed hosts. A missing tool
is a failed required gate, never a successful skip.

## Public compatibility and implementation sequence

At ADR adoption, this decision adds no DocumentPackage field and no current or
private Schema bytes. The existing contract-1.4 semantic facts are sufficient;
MI4-09 adds only private versioned manifest/expectation schemas and the
`book-xmp/2`/tagged PDF projection. Current aliases, public seven-profile
capabilities, help, default, frozen 1.0 through 1.3 registries, and old artifact
goldens remain unchanged.

MI4-09 privately implements the structure registry, selected bindings,
marked-content plan, PDF graph, manifest, in-tree validator, and exact external
gate. It must consume ADR-0032's container roles, ADR-0033's math alternative,
and ADR-0034's language/outline receipts without revising them. ADR-0036 and
MI4-11/12 may add the closed JPEG/CFF resource components but cannot change
this role/validator policy. MI4-13 may publish only the complete profile and
validation evidence in ADR-0032's atomic order.

## Rejected alternatives

1. **Infer tags in Display or PDF.** Paint has insufficient authority to
   distinguish a paragraph from a heading, a Figure from Formula, or a table
   header from a styled cell.
2. **Use `/ActualText` without a structure tree.** It improves extraction but
   does not encode Formula, list, table, heading, link, note, or reading-order
   semantics.
3. **Allocate MCIDs in structure order.** MCIDs are page/content-stream local;
   doing so would make physical paint order and page splitting nondeterministic
   or force the structure tree to follow geometry.
4. **Clone tags for page fragments or repeated headers.** That changes one
   source owner into multiple semantics and duplicates table header content.
5. **Treat all generated text as artifact.** List/footnote markers and
   references are meaningful; only pagination/layout repetitions and
   decoration use the closed artifact classes.
6. **Infer table headers from coordinates or typography.** Only the validated
   head/body grid can authorize TH/TD and header association.
7. **Use Figure for vector math or derive math Alt from source.** Both violate
   the producer-bound Formula/alternative contract in ADR-0033.
8. **Keep `typaxis.book-xmp/1` while claiming PDF/UA-1.** It lacks the required
   PDF/UA identifier; silently changing its bytes would violate ADR-0034.
9. **Run veraPDF in auto-detect mode or accept warnings.** Either makes the
   release result depend on metadata/tool behavior outside the fixed policy.
10. **Claim full accessibility from machine validation.** Matterhorn explicitly
    separates machine and human checks; a machine-only claim exceeds the
    adopted evidence.

## Consequences

- Source semantic ownership determines roles and reading order before layout,
  while selected receipts make their relationship to physical paint and pages
  independently checkable.
- Split content retains one semantic element, repeated layout content becomes
  an explicit artifact, and page-local MCIDs remain deterministic without
  defining semantic order.
- Figure alt, Formula Alt/ActualText, language, link annotations, table headers,
  footnotes, and outline `/SE` share their existing owner receipts rather than
  parallel PDF-only interpretations.
- PDF/UA-1 requires a versioned XMP projection, a non-null title, document-level
  catalog facts, complete tagged structure, and both machine and human release
  evidence.
- MI4-09 has a closed implementation target; MI4-13 still owns public profile
  registration and any release-supported conformance statement.
