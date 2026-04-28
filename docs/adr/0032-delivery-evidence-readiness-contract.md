# ADR-0032: delivery evidence exposes submission readiness

- Status: Accepted
- Date: 2026-04-29

## Context

`shosei validate` already records local checks, external validator runs, and a `delivery_evidence` summary. `shosei handoff` already carries that report into the package manifest. That is enough to prove that checks ran, but it leaves a release operator to infer whether a Kindle, print, or proof handoff is ready.

Some evidence cannot be required in CI. Kindle Previewer is proprietary and host-dependent, and store/device validators beyond Kindle Previewer are not built into `shosei`. Treating those as plain future work makes the handoff contract less useful than the underlying report data.

## Decision

`delivery_evidence` includes a submission-readiness contract.

- `summary.ready_for_handoff` gives the aggregate go/no-go signal.
- `submission_readiness[]` gives target-level readiness for enabled output channels.
- `manual_checks[]` records actionable evidence that must be supplied outside required CI, such as a real Kindle Previewer conversion or equivalent manual device review.
- `unsupported_checks[]` remains advisory future work unless a check is marked `blocking`.
- each evidence check carries `blocking` so warning/advisory checks do not make a package look unready.

External validator runs stay in `validators[]` and are mirrored into `delivery_evidence.release_checks[]`. Required CI continues to prove portable structural behavior and report contracts, while real proprietary-tool evidence remains opt-in.

## Consequences

Handoff packages can be inspected without rerunning `validate` to see whether delivery evidence is complete. Missing optional tools can still avoid failing validation, but they produce an explicit incomplete readiness signal when their target needs that evidence before handoff.

This changes the delivery evidence schema to version `2`.
