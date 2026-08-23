# ADR-0025: Block selector and inheritance cascade

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Profile 1.0 selectors use only `block_type(.class)*` over the six styleable block kinds. Blocks and selector class components are unique and UTF-8-byte-sorted. The property registry is closed to typed `font_family`, `font_size`, `line_height`, and `page`; unknown names and mismatched value kinds are errors. Text materialization requires a resolved family, positive font size, and positive line height before shaping, without an ambient system-font default. Rule source order is the validated wire array index; declaration order is not a wire field and is derived from the declarations array index, including when one rule repeats a property. A matched rule expands its known acyclic extends chain from root to child, and declarations compete by `(important, matched-rule specificity, matched-rule source order, inheritance depth, origin declaration order)` lexicographic maximum.

## Consequences

Selector parsing, class canonicalization, property/value validation, required computed-text-style construction, style identity/extends validation, cascade evaluation, schemas, fixtures, and validators enforce one deterministic interpretation.
