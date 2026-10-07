# ADR-0099: Retain running-region candidate work across failures and retries

## Status

Implemented under design 28 §14.236. The user-run full regression passed:
249 CLI tests, 743 byte-identical prior PDFs, and independent verification of
three running-region PDFs. Broader command accounting and book acceptance
remain open.

## Problem

Running-region convergence held candidate and reshape allowances in local
variables. A failed selection or height check returned an error without exposing
consumption to its caller. Reusing the numeric allowance for a retry refunded
completed work, and the private driver counted only successful reshape passes.

## Decision

Expose a caller-owned `BookV2PageRegionLineBudget` and a budgeted convergence
entry point. Retain actual candidate visits and begun reshape attempts over
successful and failed calls. Reserve each pass before shaping it. Reject an
exhausted pass allowance before starting another initial shape. Keep the existing
entry point as a one-call wrapper with identical successful receipts and counts.

The shared inline projection has a counted internal entry point. Its candidate
budget reports consumption on every return path, including errors during the
selector and later geometry projection. Preflight failures before candidate
visits cost no candidate steps. Region measurement forwards the observed count
before propagating an error. The budget therefore retains work on infeasible
lines, candidate exhaustion, later geometry/height failure and consumer errors.

Use the budgeted entry point in the private PDF driver and propagate begun
reshape attempts into its shared line-pass counter before returning either a
layout or consumer error. No automatic retries or limit increases are added.

This is candidate/pass accounting, not a claim that shaping backend operations,
all allocations or the command's complete failure ledger have been integrated.
Those broader accounting requirements remain open.

## Verification

Controlled TrueType and original Harano tests compare successful receipt hashes
and cumulative counts over repeated calls, exact/one-short work and pass limits,
u64::MAX allowances, and consumer failure. Infeasible lines and height overflow
retain their actual visits, and retries add to prior consumption. Invalid source
bindings before candidate work preserve the previous counts. Existing region
flow, shaping, display, PDF and layout regressions remain applicable. Record
commands and evidence in progress §236, without borrowing older full-regression
results as proof for the new code.
