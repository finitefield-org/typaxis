# ADR-0026: Page selection context and PageName

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Style owns only the computed `page` value. At a page boundary pagination derives physical page number, first, and parity from page index and constructs `PageSelectionContext` before any master is selected. Computed `page` accepts only `auto` or a lexical `PageName`; `PageName` is a distinct semantic type from `StyleId` and `MasterId`. An effective PageName change creates a page boundary, PageMasterSelector matches rules against that same type, and only the winning known master produces a `PageContext` containing `MasterId`. Master definitions are unique; multiple rules may reference the same definition.

## Consequences

Document/style types, page-master schemas, validators, layout APIs, fixtures, and implementation checks enforce the non-circular selection boundary.
