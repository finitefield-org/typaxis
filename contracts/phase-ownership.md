# Phase ownership

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| path admission and root containment | syntax/resource resolver | read validated `PortablePath` |
| source decoding and include policy | syntax | read immutable source catalog |
| normalization and source mapping | text | query owning-buffer-local ranges |
| semantic node/anchor identity | document | retain logical IDs |
| style precedence and source order | style | use computed style |
| bidi levels, glyph selection, positioning | shaping | consume cluster groups and levels |
| paragraph items and legal breaks | linebreak | score/select explicit items |
| block fragmentation and anchor discovery | layout Fragmenter | request with frame state |
| page break, state history, fallback selection | pagination | emit selected page plan |
| page paint, destinations, annotations | display-list builder | serialize/inspect |
| subset, extraction, image/font metrics | resource finalizer | bind finalized plans |
| PDF names, object IDs, stream filters/length | PDF backend | serialize frozen graph |
| build facts and canonical record order | manifest builder | audit reproducibility |

A downstream phase must not reconstruct an upstream decision from presentation data. In particular, PDF must not infer paragraphs from coordinates, pagination must not shape text, and a resource resolver must not accept an arbitrary filesystem path after parsing.
