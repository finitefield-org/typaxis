# ADR-0034: Document metadata, language, and outline binding

## Status

Accepted on 2026-08-28 as the metadata and book-navigation decision gate for
M4.

This ADR extends only the non-current contract-1.4 target reserved by
[ADR-0032](ADR-0032-semantic-container-and-declared-media.md). It does not
change current `typaxis.contract/1.3`, add a public contract-1.4 decoder or
Schema alias, register `typaxis.machine-pdf/production-book-1`, or claim an
implemented metadata, language, outline, PDF, CLI, or release path. MI4-07 may
implement this decision through private staging. MI4-13 remains the sole
publication gate.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: the metadata, language, outline, and PDF mapping are closed here |
| implemented | No: Wire, domain, preflight, selected-state, PDF, manifest, and validator work belongs to MI4-07 |
| public CLI E2E | No: public commands still reject contract 1.4 and the target profile |
| release-supported | No: tagged structure, combined evidence, and publication remain later gates |

## Context

The current machine contract preserves heading levels and package anchors, but
it does not carry document metadata or language and does not promise a PDF
outline. Inferring a title from the first heading, a language from host locale,
or dates from a build clock would make identical input produce different or
unverifiable document facts. Building outline objects directly from selected
coordinates would also detach navigation from the source heading/container
and the package's existing named-destination registry.

The target therefore needs three explicit, related inputs:

- a closed document-metadata record whose absent facts remain explicitly
  absent;
- one required document language plus typed node overrides with deterministic
  inheritance; and
- one explicit outline hierarchy whose source owners and named destinations
  are validated before layout and extended, rather than reconstructed, by
  selected state and PDF observations.

The design inputs are
[docs/25](../docs/25-machine-input-pdf-improvements.md) sections 7 and 13.4,
[RFC 5646](https://www.rfc-editor.org/rfc/rfc5646.html),
[RFC 3339](https://www.rfc-editor.org/rfc/rfc3339.html), the
[PDF 1.7 reference](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/pdfreference1.7old.pdf),
ADR-0003, ADR-0008, ADR-0011, ADR-0013, ADR-0015, ADR-0016, ADR-0020,
ADR-0027, ADR-0028, ADR-0032, and ADR-0033, plus invariants I-003, I-009,
I-025, I-032, I-034, I-053, I-059, I-063, I-065, I-067, I-068, I-073,
I-074, I-078, and I-079. Existing package, source, selected-state, Display,
PDF, limit, diagnostic, and atomic-publication rules remain normative unless
this ADR narrows them.

## Adopted identities

The following semantic identities are fixed:

| Item | Identifier |
| --- | --- |
| document metadata | `typaxis.document-metadata/1` |
| BCP 47 syntax and canonical form | `typaxis.bcp47-language/1` |
| computed language inheritance | `typaxis.computed-language-registry/1` |
| canonical UTC timestamp | `typaxis.utc-second/1` |
| validated outline hierarchy | `typaxis.outline-registry/1` |
| selected metadata/language/navigation state | `typaxis.book-navigation-selected/1` |
| fixed XMP serialization | `typaxis.book-xmp/1` |
| serialized PDF observation | `typaxis.book-navigation-pdf/1` |

Every fingerprint under these identities is SHA-256 over an RFC 8785 JCS
record with a required `algorithm` member. Ordered arrays retain the order
defined below. Sets are first placed in their required canonical UTF-8 byte
order; JCS itself is not used to repair an unordered input.

Changing the metadata field set, timestamp grammar, accepted BCP 47 syntax or
canonicalization, language inheritance, outline source/parent/destination
rules, Info/XMP mapping, PDF outline object policy, or limit charge requires
the corresponding `/2` identity and a contract/profile compatibility review.

## Contract-1.4 wire extension

Contract 1.4 adds required top-level `metadata` and `outline` members and a
required `document.language`. This abbreviated example uses canonical JCS
member order:

```json
{
  "contract": "typaxis.contract/1.4",
  "coordinate_unit": "pdf_point_1_65536",
  "document": {
    "blocks": [],
    "footnotes": [],
    "language": "en-US",
    "node_id": 0
  },
  "metadata": {
    "author": "Ada Example",
    "created": "2026-08-28T00:00:00Z",
    "identifier": "urn:example:book:1",
    "keywords": ["mathematics", "proof"],
    "modified": "2026-08-28T00:00:00Z",
    "subject": "A deterministic example",
    "title": "Example Book"
  },
  "outline": {
    "entries": []
  }
}
```

The omitted existing package members remain required by contract 1.4. The
metadata object has exactly the seven shown members. All are required;
`author`, `created`, `identifier`, `modified`, `subject`, and `title` are a
string or null, while `keywords` is always an array and may be empty. Missing
and null are not aliases: a required null records explicit absence and a
missing member is `P1102`. `outline` has exactly one required `entries` array,
which may be empty. Additional properties are false at every new object.

The direct values are producer-supplied facts. Version 1 does not infer title,
author, subject, keywords, identifier, or dates from headings, filenames,
source paths, font names, a repository, the build host, or another field. An
all-null metadata record with an empty keyword array is the canonical explicit
"no document metadata facts" value.

Contract 1.4 also permits an optional `language` string on the closed node
types listed below. Absence means inherit; null and an empty string are invalid.
It does not add language to resource, style, page-master, source-catalog, text-
buffer, anchor-only, or break-only records.

Finally, ADR-0034 adds required nullable `anchor_id` to the private 1.4
`semantic_container` shape. Null means that the container is not a named-
destination owner. This addition is permitted by ADR-0032's incomplete private
1.4 assembly rule and does not make a container an implicit outline entry.
Every non-null value participates in the existing package-wide AnchorId
uniqueness and owner registry under that container's NodeId even when no
outline entry references it.
MI4-07 must update every private 1.4 fixture and round-trip encoder together;
no current or frozen shape gains the member.

## Metadata strings and deterministic normalization

For each non-null `title`, `author`, `subject`, or `identifier`, and for every
keyword:

- the value is exact UTF-8 and contains at least one scalar that is not
  Unicode 16.0 `White_Space`;
- U+0000 through U+001F and U+007F through U+009F are forbidden;
- U+FFFE and U+FFFF, which cannot occur in the fixed XML 1.0 projection, are
  forbidden;
- leading, trailing, and internal whitespace is retained;
- no Unicode normalization, case folding, locale transform, line folding,
  smart punctuation, or whitespace collapse occurs; and
- the accepted UTF-8 bytes are the bytes hashed, emitted to the fixed PDF
  mapping, and reported by the manifest observation.

The decoder does not trim a value into validity. Two canonically equivalent
Unicode scalar sequences remain different producer facts. A producer that
needs NFC or another normalization must perform it before creating the
package and preserve any source mapping it claims outside these direct
metadata fields.

`keywords` is a semantic set represented in strictly increasing exact UTF-8
byte order. Duplicate or out-of-order values are `P1102` at the later array
item. The PDF Info projection joins the canonical array with exactly U+003B
SEMICOLON followed by U+0020 SPACE. A semicolon inside a keyword is retained;
the XMP and manifest arrays, not the joined Info display string, retain keyword
boundaries.

`identifier` is an opaque producer identifier. It is not parsed as a URI,
portable path, DOI, ISBN, PDF name, trailer file identifier, or resource ID.
It appears only in the metadata receipt, manifest, and fixed XMP mapping. In
particular, it never supplies or replaces the PDF trailer `/ID` array.

## Creation and modification facts

Non-null `created` and `modified` use `typaxis.utc-second/1`, the exact ASCII
grammar:

```text
timestamp = date "T" time "Z"
date      = 4DIGIT "-" 2DIGIT "-" 2DIGIT
time      = 2DIGIT ":" 2DIGIT ":" 2DIGIT
```

The date must be a real proleptic-Gregorian date with year 0001 through 9999;
hour is 00 through 23, minute and second are 00 through 59. Uppercase `T` and
`Z`, UTC, and whole seconds are required. Numeric offsets, lowercase `t`/`z`,
fractional seconds, leap-second `60`, a missing zone, and expanded or reduced
precision are rejected rather than normalized. When both facts are present,
`modified` must not precede `created`.

The accepted bytes are already canonical RFC-3339-profile values. PDF Info
maps them mechanically to `D:YYYYMMDDHHmmSSZ`; XMP uses the original canonical
string. No phase reads a clock to fill, alter, or compare these fields against
build time. A modification fact does not change merely because Typaxis emits
a new PDF.

Filesystem modification/change times may still participate in stable-read
race detection, but their values are host-admission evidence only and must not
cross into metadata, XMP, Info, manifest document facts, or fingerprints.

## BCP 47 syntax and canonical form

`typaxis.bcp47-language/1` accepts the RFC 5646 `Language-Tag` syntax using
ASCII hyphen separators. It implements the following structural grammar and
the fixed grandfathered list without consulting an ambient or newly fetched
IANA registry:

```text
language-tag = langtag / privateuse / grandfathered
langtag      = language ["-" script] ["-" region]
               *("-" variant) *("-" extension) ["-" privateuse]
language     = 2*3ALPHA ["-" extlang]
             / 4ALPHA
             / 5*8ALPHA
extlang      = 3ALPHA *2("-" 3ALPHA)
script       = 4ALPHA
region       = 2ALPHA / 3DIGIT
variant      = 5*8alphanum / (DIGIT 3alphanum)
extension    = singleton 1*("-" 2*8alphanum)
singleton    = DIGIT / %x41-57 / %x59-5A / %x61-77 / %x79-7A
privateuse   = "x" 1*("-" 1*8alphanum)
alphanum     = ALPHA / DIGIT
```

The fixed grandfathered values, compared case-insensitively, are:

```text
art-lojban  cel-gaulish  en-GB-oed  i-ami  i-bnn  i-default
i-enochian  i-hak  i-klingon  i-lux  i-mingo  i-navajo  i-pwn
i-tao  i-tay  i-tsu  no-bok  no-nyn  sgn-BE-FR  sgn-BE-NL
sgn-CH-DE  zh-guoyu  zh-hakka  zh-min  zh-min-nan  zh-xiang
```

In addition to ABNF shape, variant subtags and extension singletons must each
be unique case-insensitively. A singleton must have at least one following
extension subtag. Empty subtags, underscore separators, non-ASCII bytes, an
overlong subtag, duplicate variants/singletons, and an incomplete extension or
private-use sequence are `P1102`.

Validation is deliberately registry-independent. A normal language, extlang,
script, region, or variant subtag need not appear in a particular dated IANA
Language Subtag Registry. `Preferred-Value`, `Deprecated`, `Suppress-Script`,
variant `Prefix`, and extension-specific semantic validation are therefore not
applied. This is the RFC distinction between a structurally well-formed tag
and registry validity, and it prevents a registry update from changing
accepted package bytes.

The canonical tag is produced as follows:

1. primary language, extlang, variant, extension, singleton, and private-use
   letters become lowercase;
2. a script becomes ASCII titlecase;
3. an alphabetic region becomes uppercase and a numeric region is unchanged;
4. complete extension sequences are sorted by lowercase singleton byte while
   every subtag inside a sequence retains its order;
5. private use remains last; and
6. a grandfathered tag uses exactly the casing shown in the fixed list and is
   not replaced with a preferred value.

Comparison, inheritance, receipts, PDF `/Lang`, XMP `dc:language`, and
manifest facts use that canonical form. The original package JCS and package
hash still bind the producer spelling. A language tag is at most 255 ASCII
bytes before and after canonicalization. An explicitly unknown language is the
valid tag `und`; an empty value is invalid and is never repaired with `und`,
host locale, or an invented default such as `en`.

## Language inheritance and consumers

`document.language` is required and forms the root computed language. An
optional node override is admitted on these semantic owners:

| Owner | Effect |
| --- | --- |
| `semantic_container`, `paragraph`, `heading`, `list`, `table`, `figure`, `display_math` | applies to the owner and descendants in logical ownership order |
| `list_item`, `table_row`, `table_cell`, `footnote_definition` | applies to the owned subflow and descendants |
| `text`, `emphasis`, `strong`, `link`, `reference`, `footnote_reference`, `inline_math` | applies to that inline subtree or generated/alternative content |

`page_break`, `anchor`, `soft_break`, `hard_break`, resource, style, source,
text-map, page-master, and table-column records do not admit `language`. A
`figure`'s effective language applies to its `alt` and is the inheritance
parent of its caption. A math node's effective language applies to its producer
`speech`, PDF `/ActualText`, and future `/Formula` structure. A reference or
generated marker uses the effective language of its owning semantic site; a
footnote definition does not inherit from the reference that happened to
select it.
Paragraphs and headings inside a page-master region inherit directly from the
document language because the master/region records admit no override; every
repeated selected occurrence retains that one source NodeId's computed tag.

Inheritance follows the validated semantic ownership tree, not FlowId order,
selected page order, paint nesting, style cascade, outline parentage, or PDF
object order. The computed-language registry records each language-capable
NodeId's explicit canonical override or null, effective canonical language,
logical parent NodeId, source span where one exists, and package/profile/limit
fingerprints. Reparenting, skipping an override, or applying a sibling's
language is `I9190`.

Language in version 1 is semantic/PDF information. It does not by itself
select a system font, locale, bidi direction, writing mode, hyphenation
dictionary, line-break table, OpenType language system, math speech generator,
or translated label. Any later shaping or line-breaking use needs a separate
closed profile decision and must consume this receipt rather than reparsing a
raw string.

The document canonical language is emitted as the PDF catalog `/Lang`. A
painted leaf whose effective language differs from the document language is
covered by exactly one owner-bound `/Span` `BDC` inline property list carrying
`/Lang`; it does not allocate an indirect property object. Math adds `/Lang`
to the same canonically serialized inline dictionary that owns `/ActualText`.
Nonpainting container overrides remain in the computed registry. MI4-08 must
retain the rule that a differing tag belongs on the corresponding structure
element, and MI4-09 must implement it. Tagged structure may not replace or
reinterpret the computed tag.

## Outline wire and hierarchy

Each `outline.entries` item has this exact closed shape:

```json
{
  "destination": "chapter-1",
  "label": "Chapter 1",
  "level": 1,
  "outline_id": 0,
  "parent_outline_id": null,
  "source_kind": "heading",
  "source_node_id": 12
}
```

Every member is required. `outline_id` is an `id32`; `parent_outline_id` is an
`id32` or null; `level` is an integer from 1 through 6; `source_kind` is
exactly `heading` or `semantic_container`; `source_node_id` is a NodeId;
`destination` is an existing AnchorId string; and `label` follows the metadata
string rules. There is no caller-authored open state, color, font style,
action, coordinate, page number, object ID, or arbitrary PDF dictionary.

The array is the canonical source-owner preorder and obeys all of these rules:

1. `outline_id` equals the zero-based array index.
2. `source_node_id` values are strictly increasing in validated document
   NodeId preorder. One source node can own at most one outline entry.
3. A level-1 entry has null parent. A level greater than 1 has a non-null
   preceding parent whose level is exactly one less.
4. Parent subtrees are contiguous. Stack validation of the level sequence must
   yield exactly the authored `parent_outline_id`; depth cannot increase by
   more than one between consecutive entries, but it may decrease to any open
   ancestor.
5. For a heading source, `source_kind`, NodeId, and `level` must equal the
   validated heading kind, NodeId, and HeadingLevel. Its `anchor_id` must be
   non-null and exactly equal `destination`. A repeated page-region heading is
   not an outline source; sources belong to the general Document semantic tree.
6. For a semantic-container source, the kind and NodeId must match the
   validated container and its new `anchor_id` must be non-null and exactly
   equal `destination`. The explicit outline level supplies its navigation
   depth; no label or level is inferred from `semantic_kind`.
7. Destination AnchorIds are unique across outline entries and resolve to the
   same source owner in the package anchor registry.

Headings and containers do not automatically create entries. Omitted headings
remain visual headings, and a non-null container anchor remains an ordinary
named destination. An explicit entry never changes layout, creates visible
text, localizes its label, or moves the source node. This preserves ADR-0032's
rule that a semantic container has no implicit label or outline entry while
allowing the producer to opt one into book navigation.

An outline label has the computed language of its source NodeId. There is no
separate label-language override and no `/Lang` key on an outline item; a
producer needing another label language must place the corresponding explicit
override on the source owner. The receipt-bound `/SE` relation below carries
that owner into tagged structure without inventing a second language.

The validated outline receipt covers the exact label bytes, level, parent,
source kind/NodeId/SourceSpan, source semantic fingerprint, heading level or
container kind, source anchor, destination, package anchor-owner proof,
computed source language, package/profile/limit fingerprints, and canonical
entry order. A heading/container swap, same-named anchor from another package,
label-only change, parent change, or target substitution cannot reuse the
receipt.

## Selected destinations and PDF outline mapping

Outline destinations extend the existing selected named-destination registry;
they do not create a parallel coordinate system. Each outline destination must
already have exactly one selected `NamedDestination` derived from the same
package anchor owner, selected page/frame, and frame-local point. For a
semantic container, its anchor point is the logical block-start of its first
selected fragment after `space_before`, at the selected child frame's logical
start edge. A split container still has one anchor and one outline entry.

The selected navigation receipt adds the selected page index, destination view
and point, selected-layout fingerprint, and destination-registry fingerprint
to each outline entry. Missing selected anchors, duplicate selected
destinations, a wrong page/view/point, an unselected source owner, or a target
outside the selected page geometry fails before PDF object allocation. An
outline cannot fall back to a direct caller coordinate, page number, first
paint command, or a same-named destination from another package.

PDF uses the document catalog's existing `/Names << /Dests ... >>` name tree.
Each outline item's `/Dest` is the exact AnchorId byte string key in that name
tree; it is not a copied destination array and there is no `/A` action. Thus an
internal link and an outline entry naming the same anchor necessarily consume
one selected destination value.

When entries are nonempty, PDF emits one indirect outline root and one indirect
item per canonical entry. The root dictionary has exactly `/Type /Outlines`,
`/First`, `/Last`, and `/Count`. Item relationships are derived only from the
validated hierarchy:

- root `/First`, `/Last`, and positive `/Count` cover all entries;
- each MI4-07 item has exactly `/Title`, `/Parent`, `/Dest`, and the applicable
  relationship keys below;
- `/Prev` and `/Next` appear exactly for adjacent siblings;
- `/First` and `/Last` appear exactly for nonleaf items; and
- nonleaf `/Count` is the positive number of all visible descendants, so the
  complete tree is initially open. Leaves omit `/Count`.

MI4-07 outline items omit `/SE`. MI4-08 must retain the source-association rule
in its tagged-structure ADR. Once MI4-09 has issued the matching structure
owner receipt, an outline item may add exactly one `/SE` indirect reference to
the source heading/container's structure element. That binding is nullable in
`BookNavigationPdfObservation`, must agree with the same `source_node_id`, and
cannot alter the outline hierarchy, title, or destination. No other outline
item key is profile-authorized.

The catalog points `/Outlines` to the one root when entries are nonempty and
omits `/Outlines` when `entries` is empty. The profile does not set
`/PageMode /UseOutlines`, create remote actions, or attach color/style flags.
Outline labels are always emitted as PDF UTF-16BE text strings with BOM. New
Info, Metadata, outline-root, and outline-item roles are preflighted before
allocation and appended in that role order after every graph role adopted
through ADR-0033; later M4 ADRs may append roles but cannot insert before or
reorder these. Items use canonical outline preorder. Hash-map order, parent
traversal, page order, and worker completion never allocate objects.

## PDF Info and fixed XMP mapping

The production target emits one indirect document Info dictionary referenced
by trailer `/Info` and one Metadata stream referenced by catalog `/Metadata`.
The Info dictionary contains only the present adopted keys below plus required
`/Producer`. Null facts are omitted from both projections; they are never
filled by the engine. The fixed mapping is:

| Wire/computed fact | PDF Info | XMP |
| --- | --- | --- |
| `title` | `/Title` | `dc:title` localized alternative |
| `author` | `/Author` | one-item `dc:creator` sequence |
| `subject` | `/Subject` | `dc:description` localized alternative |
| ordered `keywords` | `/Keywords`, joined by `; ` | ordered `dc:subject` bag and the same joined `pdf:Keywords` |
| `identifier` | absent | `dc:identifier` |
| `created` | `/CreationDate` PDF UTC date | `xmp:CreateDate` |
| `modified` | `/ModDate` PDF UTC date | `xmp:ModifyDate` |
| document language | catalog `/Lang` | one-item `dc:language` bag |
| engine name/version | `/Producer` | `pdf:Producer` |

An empty keyword array omits Info `/Keywords`, `dc:subject`, and
`pdf:Keywords`; it does not emit an empty string or empty RDF container.

`/Creator`, `/Trapped`, custom Info keys, trailer `/ID`, XMP
`xmp:MetadataDate`, thumbnails, and arbitrary extension metadata are absent.
`/Producer` is the only engine-authored metadata value and is exactly
`<EngineIdentity.name> <EngineIdentity.version>` from the same build receipt;
it contains no Rust path, target triple, username, hostname, or toolchain
string. The same engine identity is already a manifest build fact.

Info values other than the ASCII PDF date are always UTF-16BE text strings
with BOM. The catalog `/Lang` uses the same encoding and exact canonical BCP
47 scalar sequence. Where a fact occurs in both, the Info and XMP projections
must decode to the same accepted value; one is not trusted to repair or
populate the other.

`typaxis.book-xmp/1` is XML 1.0 encoded as UTF-8 without BOM, padding,
comments, XML declaration, inter-element whitespace, or `xpacket` processing
instructions. Its exact outer skeleton and namespace-declaration order are:

```xml
<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:pdf="http://ns.adobe.com/pdf/1.3/" xmlns:xmp="http://ns.adobe.com/xap/1.0/">…</rdf:Description></rdf:RDF></x:xmpmeta>
```

The ellipsis marks the property-byte sequence described here and is not
emitted. There are no bytes before the opening `<x:xmpmeta` or after its final
`>`. The description contains only these properties in this exact order,
skipping an absent/empty fact without reordering the remainder:

```text
dc:title, dc:creator, dc:description, dc:subject, pdf:Keywords,
dc:identifier, xmp:CreateDate, xmp:ModifyDate, dc:language, pdf:Producer
```

`dc:identifier`, `pdf:Keywords`, both `xmp` dates, and `pdf:Producer` use the
exact `<prefix:name>TEXT</prefix:name>` scalar form. `dc:creator` uses
`<dc:creator><rdf:Seq><rdf:li>TEXT</rdf:li></rdf:Seq></dc:creator>`.
`dc:subject` and `dc:language` use the corresponding property wrapper around
`<rdf:Bag>` and one `<rdf:li>TEXT</rdf:li>` per canonical array item. Empty
keywords omit both `dc:subject` and `pdf:Keywords`; document language and
producer are always present.

Title and description use their property wrapper around an `rdf:Alt` whose
first item is `<rdf:li xml:lang="x-default">TEXT</rdf:li>`. When the canonical
document language is not `x-default`, a second item has that canonical
`xml:lang`; both items contain identical text. When it is `x-default`, the
array has only the first item. XML text escapes `&`, `<`, and `>` exactly as
`&amp;`, `&lt;`, and `&gt;`, in that order; attribute values additionally escape
`"` as `&quot;`. No empty-element shorthand, character reference for another
scalar, CDATA, namespace alias, or optional property is emitted.

The Metadata stream dictionary is exactly `/Type /Metadata /Subtype /XML`
plus serializer-owned length/compression entries. Configured deterministic
stream compression may wrap the bytes, but decompression must yield the exact
`typaxis.book-xmp/1` serialization. No generic RDF/XML library ordering,
ambient namespace prefix, packet timestamp, random identifier, or serializer
pretty-printing is permitted to affect bytes.

## Limits and one-time accounting

No synonymous M4 config fields are added. Metadata, language, and outline work
maps to the existing limits because its units have the same semantics. All
maxima are inclusive and max+1 is refused before allocation or receipt/object
issuance.

| Work | Existing limit and charge | Code |
| --- | --- | --- |
| metadata object and outline-registry root | one additional `max_ast_nodes` unit each | `P1120` |
| each keyword and each outline entry | one additional `max_ast_nodes` unit in wire order | `P1120` |
| outline hierarchy | Document root is depth 1, outline-registry root depth 2, and a level-N entry depth `2 + N` against `max_ast_nesting_depth`; the effective accepted level is the lesser of 6 and the configured depth minus 2 | `P1121` |
| each non-null metadata field, keyword, outline label, raw language tag, and canonical language tag instance | each stored instance is at most `max_text_buffer_bytes`; a language tag is additionally at most 255 bytes | `T2100` (`P1102` for the fixed BCP 47 cap) |
| copied metadata/keyword/label bytes, each explicit raw language spelling, and every NodeId's computed canonical language | one logical package aggregate against `max_text_bytes`, charged in metadata field order, keyword order, NodeId order, then outline order; interning does not erase a per-NodeId computed-language charge | `T2101` |
| selected outline records | one additional `max_fragments` unit per entry in canonical outline order | `L5110` |
| Info, Metadata, and outline root/items | existing `max_pdf_objects`, preflighted before dense allocation; language property lists remain inline content bytes | `G6100` |
| XMP/content/object serialization | existing `max_output_bytes` and `max_spool_bytes` before publication | `D8101` |

An empty outline still admits and charges its depth-2 registry root. A
configured nesting maximum below 2 is therefore `P1121`; the effective-level
calculation uses checked subtraction and never wraps or saturates.
Each XMP buffer actually held—raw and, when compression is enabled,
compressed—participates exactly once in the PDF owner's simultaneous
spool-payload accounting alongside its other streams; the final serialized PDF
participates once in the ordinary output-byte counter.

The corresponding codes are `P1120`, `P1121`, `T2100`, `T2101`, `L5110`,
`G6100`, and `D8101`. At one explicit language site, identical raw and
computed-canonical bytes are charged once; different bytes are both charged
because both remain in the package/receipt chain. Every inherited descendant still contributes
its computed canonical bytes to the logical aggregate, but does not duplicate
the allocation: the registry stores an interned canonical language ID while
its fingerprint writes the effective tag for each NodeId. Retrying preflight,
pagination, PDF validation, or a foreign receipt cannot reset any aggregate.

The fixed seven metadata members need no caller-configurable field-count
limit. Keyword and outline counts are bounded by their explicit
`max_ast_nodes` charges, language override count by the already-counted
semantic NodeIds, outline depth by `max_ast_nesting_depth`, strings by the text
limits, selected entries by `max_fragments`, and PDF projection by
`max_pdf_objects`. This is a complete count/depth/byte/object bound without an
overlapping `max_metadata_*`, `max_language_*`, or `max_outline_*` option.

## Validation phases and diagnostics

Validation fails at the earliest owner with the authority and location:

| Condition | Owner, code, and primary location |
| --- | --- |
| missing/extra/wrong-typed metadata, document language, outline, entry, or container anchor member | strict contract-1.4 decoder, `P1102`, exact JSON Pointer |
| empty/whitespace-only/forbidden-scalar/overlong metadata or label, noncanonical keyword order, malformed timestamp, or modified-before-created | metadata semantic validation, `P1102`, exact field/item Pointer |
| malformed/overlong BCP 47 tag or duplicate variant/singleton | language validation, `P1102`, exact `/language` Pointer and token byte offset when available |
| non-dense outline ID, bad level/parent/preorder, duplicate source/destination, wrong source kind/level, missing source anchor, or destination/source-owner mismatch | outline preflight, `P1102`, the responsible entry member Pointer; duplicate notes the first Pointer |
| profile that does not admit metadata/language/outline or an unsupported placement of a language override | capability preflight, `L5100`, before resource open/layout/PDF |
| valid source destination absent or different in selected state | selected navigation, `L5100`, destination Pointer with source-node and selected-state notes, before PDF allocation |
| count/depth/string/aggregate/fragment/object/output max+1 | owning limit code and the item that would cross the inclusive maximum; fixed serializer overhead uses a global output location |
| receipt, inherited language, selected point, Info/XMP, outline relation, object, or serialized observation disagreement | `I9190`; never retried as producer input or another profile |

Package locations use RFC 6901 pointers such as `/metadata/title`,
`/document/language`, a node's exact `/language`, or
`/outline/entries/3/destination`. Source notes identify the heading/container
SourceSpan where relevant, but a direct metadata/label value does not invent a
source location. Canonical validation visits metadata members in the JCS order
shown in the example and keyword indexes when `keywords` is reached, followed
by document language and node languages in NodeId order, then outline entries
in array order and their members in the JCS order shown in the entry example.
A later PDF or validator failure cannot replace an earlier package error.

## Receipt and ownership chain

The private target chain is:

```text
strict 1.4 Wire + package JCS/location index
  -> DocumentMetadataReceipt
  -> ComputedLanguageRegistryReceipt
  -> ValidatedOutlineRegistryReceipt
  -> production profile authorization
  -> selected named-destination registry
  -> BookNavigationSelectedReceipt
  -> Info/XMP/catalog/outline PDF graph
  -> VerifiedPdfBytesReceipt
  -> BookNavigationPdfObservation
  -> versioned manifest facts + independent validator observation
```

The decoder owns only untrusted closed Wire values. Syntax owns metadata
validation, language parsing/inheritance, source-owner lookup, anchor equality,
and the three package-bound validated receipts. Machine-profile preflight owns
the accepted target feature set. Layout/selected state owns destination page
and point. PDF alone owns Info/catalog/Metadata/outline objects and exact
serialization. Manifest and the independent validator consume observations;
neither can issue or reconstruct an earlier receipt.

The selected receipt covers metadata, language, outline, package/profile/
limits, selected layout, and destination registry fingerprints. The PDF
observation covers decoded Info values, decompressed exact XMP hash, catalog
language, destination name-tree keys/arrays, outline object relationships and
titles, nullable source structure-element bindings, the EngineIdentity receipt
and exact producer value, dense object roles, PDF bytes hash, and serializer
receipt. Closure is bidirectional: every validated entry has exactly one
selected target and PDF item, and no extra metadata
property, language property, XMP field, outline item, action, destination, or
structure binding is allowed. The only later extension is the receipt-bound
`/SE` relation authorized above.

MI4-08 must consume the computed-language and outline source-owner receipts
when adopting structure language and heading-relation policy. MI4-09 may then
add structure elements and MCIDs but cannot change metadata, canonical tags,
outline labels/parents, destinations, or existing name-tree coordinates.

## Clock, host, and fallback prohibition

Successful target output never derives document facts from:

- wall or monotonic clocks, `SOURCE_DATE_EPOCH`, timezone, locale, or daylight
  saving rules;
- package/source/resource file timestamps, ownership, permissions, inode,
  absolute path, checkout name, current directory, username, or hostname;
- repository metadata, environment variables, build directory, worker order,
  random values, or PDF viewer defaults; or
- the first heading, first painted text, filename, identifier, or source URI.

Those values may be used only by their already adopted host-safety or engine-
identity owners and may not leak into Info, XMP, catalog language, outline,
manifest document facts, PDF trailer, or object allocation. Same package
bytes, source/resource bytes, profile, limits, engine identity, and config
must produce identical metadata/navigation fingerprints and PDF bytes across
time, timezone, host path, and checkout name.

Invalid or unsupported metadata, language, hierarchy, source, destination, or
limit state is terminal. There is no dropping a bad field, replacing language
with `und`, flattening outline levels, retargeting to the first page, using a
direct destination array, omitting a bad outline item, copying a host date,
or falling back to an old profile/contract. Explicit null metadata, `und`, and
an empty outline are accepted only when actually present in valid input or
the neutral source-export rule below.

## Schema, source export, and publication sequence

At ADR adoption, no Schema file or Rust type changes. MI4-07 must atomically
extend only the independent private `schemas/1.4/` DocumentPackage and
versioned metadata/navigation manifest shapes, add semantic invalid fixtures
for rules Schema cannot express, implement the complete receipt/PDF/validator
chain, and update every existing private 1.4 fixture. Top-level current aliases
and independent 1.0 through 1.3 registries remain byte-identical.

The source-mode 1.4 exporter has no metadata or language syntax in the current
reference TSF. Its only lossless neutral population is therefore the required
all-null metadata record, empty keyword array, document language `und`, empty
outline entries, no node language overrides, and null semantic-container
anchors. It must not infer facts from host/source names, standalone anchors, or
headings. A future source syntax may populate richer facts only under a
separately adopted source/parser contract and exact mappings.

MI4-08 must use this ADR's computed-language and outline-owner bindings in its
decision, and MI4-09 must use them in implementation. MI4-13 may publish only
after MI4-07 and the later tagged/resource slices have closed their private
evidence. Publication follows ADR-0032's atomic switch:
complete frozen 1.4 Schemas and semantics first, then version dispatch and
artifact registries, then profile descriptor/dispatch/capabilities/help, then
combined fixtures and release evidence. The default remains `paragraph-1` and
no old profile gains metadata, node language, outline, Info/XMP, or catalog
language behavior.

## Rejected alternatives

1. **Use file times or the build clock.** This makes the same input produce
   different metadata and confuses host race evidence with document facts.
2. **Infer title, author, or language.** Filenames, first headings, locale,
   fonts, and script detection are not producer authority.
3. **Require a live IANA registry.** Registry date and network state would
   change acceptance; version 1 deliberately validates stable RFC syntax.
4. **Accept arbitrary RFC 3339 spellings and normalize them.** Offsets,
   fractions, leap seconds, and unknown offsets add unnecessary equivalent
   encodings; canonical UTC seconds are sufficient for document facts.
5. **Automatically outline every heading.** Producers need an explicit subset,
   label, hierarchy, and destination, and semantic containers have no implicit
   title.
6. **Store caller page numbers or coordinates.** Pagination must extend the
   existing selected anchor receipt or fail.
7. **Use outline actions or copied destination arrays.** Referencing the one
   named-destination key closes links and outlines over the same target.
8. **Use Info only.** It has no standard document-identifier field or exact
   keyword boundaries and does not supply the fixed XMP observation required
   by the production target.
9. **Accept arbitrary XMP.** Extension vocabularies, packet tools, property
   order, and hidden timestamps would reopen an unbounded metadata surface.
10. **Publish contract 1.4 at this gate.** An ADR without MI4-07 PDF/validator
    and later tagged/combined evidence is not a public feature.

## Consequences

- Contract 1.4 has one explicit metadata absence representation, one stable
  BCP 47 syntax/canonicalization, and one logical inheritance tree.
- Outline hierarchy, source heading/container, named destination, selected
  point, PDF item, and validator observation can be compared through one
  receipt chain.
- Info and fixed XMP expose the same producer facts without clock, filesystem,
  host, locale, arbitrary RDF, or trailer-ID inference.
- Existing AST/text/fragment/PDF limits completely bound the new work without
  adding overlapping configuration fields.
- The adopted domain is intentionally narrow: one author string, canonical
  UTC-second dates, explicit outline entries, no arbitrary XMP, and no implicit
  metadata/navigation generation.
- No user-visible behavior changes at ADR adoption; public contract 1.3 and
  its seven profiles remain byte-frozen until MI4-13.
