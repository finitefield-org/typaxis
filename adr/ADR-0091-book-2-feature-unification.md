# ADR-0091: Keep shared carrier and error enums stable under Cargo feature unification

## Status

Implemented incrementally under design 28 on 2026-09-11. This does not complete
design 28 or enable its public profile.

## Decision

Cargo may enable book-v2-staging on a dependency without enabling the consuming
crate's own feature. Shared SemanticBlock and WireSemanticBlock description-list
variants, their payload types and common traversals therefore have a stable shape.
Consumers explicitly handle or reject those variants without conditioning their
match arms on an unrelated local feature. The shared production-text CffV2 error
variant follows the same rule. Existing CFF backend availability is unchanged.

Carrier visibility does not grant contract admission. WireSemanticKind remains
sealed. Description-list serialization and deserialization require its existing
DESCRIPTION_LISTS capability, which is false for the frozen contracts. Empty
lists cannot bypass that gate. Book2 vocabulary, entry points, lowering and
layout algorithms remain feature-gated. Frozen lowering/navigation/production
flow explicitly reject description lists; no catch-all arm hides new variants.

## Verification

An external integration test compiles against the library's actual feature set,
without depending on its cfg(test). It rejects empty and populated wire lists
and manually constructed frozen typed carriers, both with and without staging.

Check the entire workspace with each of the 14 individual crate staging features,
not only the CLI feature or --all-features. Default-feature tests also exercise
nine shared consumer crates. Actual PDF regression and frozen CLI checks are
recorded with commands and artifact comparisons in design progress sections 225
and 226. These results supersede the mixed-feature limitation in ADR-0090.
