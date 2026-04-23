# ADR-0031: Kindle Previewer validation is opt-in

Date: 2026-04-23

## Status

Accepted

## Context

`shosei validate` already records target/profile checks and can run external validators such as `epubcheck` and `qpdf` when the generated artifact and tool are available.

Kindle Previewer adds useful device-oriented confidence for Kindle handoff, but it is proprietary, OS-dependent, and not consistently available in CI. Making it a required validator would make `validate` less portable and would conflict with the repository goal of keeping the command surface usable on macOS, Windows, and Linux.

## Decision

Add `validation.kindle_previewer`, defaulting to `false`.

When this setting is `true` and Kindle output is enabled, `shosei validate` attempts to run Kindle Previewer against the generated Kindle artifact and records the result in the existing `validators[]` report array.

The validator uses the same reporting semantics as other external validators:

- unavailable tool: `missing-tool`, no validation failure by itself
- skipped prerequisite: `skipped`, no validation failure by itself
- successful check: `passed`
- failed check or launch failure: `failed` plus a validation error

Logs are written to `dist/logs/<book-id>-kindle-previewer-validate.log` and referenced from the report.

CI should not require the real Kindle Previewer binary. It should prove the report contract with a fake executable smoke test.

Maintainers who have the real Kindle Previewer installed may run a local evidence hook before release or Kindle handoff. That hook is intentionally outside required CI and exists to prove real proprietary-tool execution without weakening cross-platform CI portability.

Validator confidence is split as follows:

| Layer | Proof |
|---|---|
| Report contract | Required CI uses a fake executable to prove `validators[]`, `log_path`, and pass/fail semantics |
| Real conversion | Optional local hook uses the real Kindle Previewer binary when available |
| Beyond Kindle Previewer | Additional store/device validators remain future work until a spec and ADR define them |

## Consequences

- Kindle-oriented validation becomes stronger without making the default workflow depend on proprietary tooling.
- Users who need delivery confidence before Kindle handoff can opt in explicitly per book or series defaults.
- Docs must distinguish `epubcheck` from Kindle Previewer: `epubcheck` remains the default EPUB validator, while Kindle Previewer is an optional device-oriented conversion check.
- Release guidance may point to an optional local evidence hook, but required CI must stay portable.
