# ADR-0057: Reserve mixed footnote fragments with dependency backtracking

## Status

Accepted incrementally under design 28 on 2026-09-10. ADR-0057 connects shared
required/dependency reservation; ADR-0058 subsequently connects physical page
selection, placement, source closure and private driver PDFs. Remaining body/page
forms and full-book/public/managed-host/author/human acceptance remain required.

## Decision

Extend the existing bounded dependency stack with each mixed definition's actual,
ranked alternatives. A frame owns its immutable incoming demand snapshot and the
remaining iterator; selecting a different alternative preserves other notes' full
cursors. Use the same cumulative record/work owner across enumeration, fragment
conversion, state forks, failed descendants and backtracking. Never reset arenas
or refund discarded candidates.

Preserve the distinction between entry reservation and dependency closure.
select_required_region reserves one legal fragment for each note pending at entry;
new demands remain explicit. If retained references are unstarted, the existing
body-fit kernel invokes the same stack with those dependencies included. Each
active branch visits a definition once, handles cycles without recursive calls,
and undoes its visited flag when removing that choice. An authored forced break
cannot precede another required fragment. Check retained reference targets before
accepting a closed branch, and try another alternative if one remains unstarted.

An initial parallel-table fragment must contain the note's actual first marker.
Advancing only another cell cannot satisfy the first reservation. Authored
forced-only boundaries retain their nonpainting progress semantics; consuming one
does not make its unpainted definition satisfy a retained reference. Mixed cursor
marker state, rather than serial item zero, determines whether the note has started.

The common fit includes the separator and declared geometry, but still grants no
page/paint receipt. ADR-0058 subsequently connects scoped body table preparation,
mixed note placement and source closure before issuing private note-table PDFs.

## Evidence

Design 28 §14.191 records required-region replay over the admitted definition
matrix and six focused fit cases: late and early references, repeated child header,
cycle, two body demands, and a forced-only dependency refusal. Tests prove shorter
table selection, removal of discarded demands, source/marker-once continuation,
separator geometry, and exact/one-short record/work accounting. Existing PDFs remain
byte-identical. The forced dependency test exposed and now rejects advancing a
parallel sibling without the first marker.
