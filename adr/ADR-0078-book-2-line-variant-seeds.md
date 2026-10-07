# ADR-0078: Retain converged line contexts for independent physical variants

## Status

Accepted incrementally under design 28 on 2026-09-10. Enables simultaneous,
independently owned table-header line graphs from one immutable source flow.
Selecting those graphs as repeated physical paint remains open.

## Decision

Different repeated header widths can change line ends, shaping and natural header
height. A single selected paragraph array cannot represent all those occurrences.
Retain a seed only after actual shape/line convergence, and rebuild a separate
shape/frame/line owner for each required physical variant.

`BookV2BodyLineVariantSeed` keeps the original flow, policy, admitted resources,
vector/native bindings, limits, body/page frame plan and width/origin assignments
borrowed and immutable. It owns original paragraph shaping-context ends and the
observed stable line fingerprint. These ends use the existing shaped UTF-8 context
convention, including atomic placeholders and mandatory breaks; they are not
transient line ordinals or a rewritten source flow. Replay accepts only the seed
and budgets, so callers cannot substitute another label, resource or width context.

Seed creation uses the existing actual convergence loop. Preflight context/seed
storage against caller prior records and the observed graph charge before copying.
Charge capture traversal and retained context construction to the same candidate/
frame work allowance as convergence. Initial shape passes retain their existing
stage-local record ceilings; this API does not claim complete multi-variant build
accounting or full-book scaling.

For replay, prepay the original observed graph bound, new input views and result
record against the seed/caller prior charge before allocating or shaping. All
inputs are immutable, so this is the same graph construction with the same source
context. Run actual shaping, inline preparation, frame and line selection, and
footnote binding. Require the rebuilt fingerprint to match the observed stable
fingerprint and its graph charge to stay within the prepaid bound. Charge replay
candidate/frame work and input/comparison visits; exact and one-short limits apply.

Expose the result only within its owning callback. Two rebuilds may coexist with
different selected contexts, line counts and table header heights, while sharing
the same immutable source and native computations. Shape/line/footnote owners
remain distinct; cross-owner receipts fail verification. Do not reuse an old line
index or mark a different geometry as authorized merely because source text agrees.

## Evidence and remaining work

Design 28 §14.212 and the progress ledger record controlled and unchanged Harano
body/note table-header seeds, fresh simultaneous rebuilds, changed paragraph line
counts and actual table measurements. Exact capture/replay work and record budgets,
foreign assignment rejection and cross-owner receipt rejection are exercised.
A native-math test confirms independently shaped line variants retain one shared
original computation and consume its original inline atom once in each graph.

The existing driver and pagination still reject conflicting repeated frames until
per-occurrence graph selection, header reservation/placement, repeated source
closure and PDF assembly are connected. This increment creates no new repeated
heterogeneous PDF. Remaining page names, running regions/columns, other book forms,
public/full-book/manifests/managed-host/scaling and author/human acceptance remain.
