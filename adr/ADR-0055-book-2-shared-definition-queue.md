# ADR-0055: Mixed definition continuation in the common demand queue

## Status

Accepted incrementally under design 28 on 2026-09-10. ADR-0057 connects shared
required/dependency reservation; ADR-0058 subsequently connects physical page
selection, placement, source closure and private driver PDFs. Remaining body/page
forms and full-book/public/managed-host/author/human acceptance remain required.

## Decision

Store the definition-local next item, original next root-table ordinal, optional
full table continuation and original marker-consumption state in the actual pending
footnote cursor. A partial table may advance cells while retaining the same serial
item index. An empty table may advance the table ordinal without advancing an item.
Neither action can be represented by the serial item index alone. Leading forced
breaks and empty tables do not consume a definition marker prematurely.

Use one demand/search owner, queue and cumulative record/work budget for every
mixed definition. Prepare definition contexts from the original source table
preorder in one pass; share one immutable hierarchy among them. Preserve each
context's actual mutable table arenas across speculative selection, other notes
and failed attempts. Do not reset a context to restart a continuation.

The definition evaluator borrows this common owner and its selected context.
Resuming a definition copies the immutable incoming demand values under the same
snapshot identity, then each candidate creates its own next-state branch. Selecting
another pending definition retains all other cursors and marker states. Restore
the borrowed context after success and errors without refunding work or records.
Bind a definition-specific candidate to its definition number as well as the
incoming search/state identity; shared snapshot verification remains available
for the common queue owner.

Source marker consumption is not a physical label-placement receipt. ADR-0056
subsequently connects mixed parts to the common sequential region representation.
ADR-0057 then connects required/dependency reservation. ADR-0058 connects actual
physical placement and removes the mixed page preparation guard through scoped
body/definition contexts. The flat-only search continues to reject mixed cursors.

## Evidence

Design 28 §14.189 and its progress entry cover interleaving an unfinished table
with another demanded note, preserving full cursor fingerprints and source-once
coverage, marker state, unreferenced notes, terminal self-reference, exact/one-short
budgets, lookback failure followed by a smaller successful candidate, invalid
capacity recovery and cross-definition receipt refusal. Existing supported PDFs
remain byte-identical. No new physical footnote-table PDF is claimed at this stage.
