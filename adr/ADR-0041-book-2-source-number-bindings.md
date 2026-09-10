# ADR-0041: Book-2 source number bindings

## Status

Accepted for implementation on 2026-09-09 under [design 28](../docs/28-vmb-book-production-compatibility.md).
This is an unpublished contract-1.5 decision. Publication and full-book acceptance require separate evidence.

## Decision

The optional document member `number_bindings` is a nonempty array of closed
records with `anchor_id`, `owner_node_id`, `label_node_id`, and `text_span`.
The span has the existing `text_id`, `start_byte`, `end_byte` shape. Missing
metadata means no bindings; a present null or empty array is invalid. Duplicate
anchors and unknown fields are invalid. Bindings are document metadata, not
additional source nodes or PDF structure elements. Their AST charge is one for
the collection plus one per binding, at a minimum nesting depth of three.

Each owner is an actual numbered block or group. Its label must be an actual
Text leaf or equation-number leaf in that owner's subtree. The nonempty selected
range must belong to that leaf's immutable source text buffer and have valid
UTF-8 boundaries. Control-only, whitespace-only, and control-containing labels
are invalid. An existing anchor of the same name must belong to the owner's
subtree; its original location remains the destination. Otherwise the explicit
binding declares a destination at the owner. This supports figure caption
anchors and numbered equations without moving or synthesizing inline nodes.

The producer supplies the exact displayed number range, for example `1.12` in
`定理1.12` or `(1.12)`. Typaxis does not parse a heading to guess its number,
generate a counter from an anchor name, or accept a separate fabricated label.
The VMB producer must bind its resolved LogicalNumber to the corresponding
displayed source projection. Locale-specific digits and alphabetic or Roman
labels remain producer decisions. Rich labels spanning multiple text leaves
require a future explicit carrier and are not inferred by this format.

References retain `format: number` and their original target. Source flow copies
the selected range to the Counter generated-buffer namespace for each reference
owner, applying the same aggregate text, per-buffer and fragment budgets as
other generated labels. Number references with no valid binding fail even if
the target has a heading or outline label. The referring source's style and
language govern shaping. Page references retain their separate candidates and
convergence process. Generated references do not mutate authored buffers.

## Isolation and evidence

The sealed generic carrier enables this field only for the private successor.
Legacy 1.4 typed serialization/deserialization and ordinary public dispatch
reject it even with staging compiled. The temporary frozen validation view
omits only this successor metadata; the retained canonical document preserves
the exact binding. Existing source, text, math and resource validation remains
mandatory.

Required tests include canonical round trips, field closure, exact shared
budgets, source ownership, UTF-8 range failures, duplicate or displaced anchors,
equation number targets, actual Counter provenance, and source immutability.
Actual PDF evidence must additionally verify displayed/extracted numbers and
destination geometry, with independent tampering tests. Producer integration,
public manifests, original full-book evidence and publication remain separate
gates; source acceptance alone does not prove those outcomes.
