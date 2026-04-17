# AGENTS.md

## Scope

Rules in this file apply to `docs/` and documentation changes that update the published contract for `shosei`.

## Document Ownership

- `specs/` owns normative behavior, workflow, schema, and surface boundaries.
- `adr/` owns durable rationale, tradeoffs, and supersession history.
- `README.md`, `docs/usage.md`, and `site/usage.html` describe current implemented behavior; they are not the place for unresolved design rationale.

## Writing Rules

- Keep docs concise and practical.
- Prefer relative links inside `docs/`.
- Keep command examples aligned with the current CLI name: `shosei`.
- Do not describe commands or features as available unless the current code path actually implements them.
- Prefer examples that reflect currently supported config fields and output behavior. Remove or rewrite stale examples instead of leaving them partially correct.
- When documenting story workspaces, distinguish freeform `structures/` notes from the `scene_seeds` frontmatter contract that `story seed` reads.
- For machine-readable command surfaces, keep `--json` docs aligned with the current CLI behavior instead of older report-file-only guidance.

## Spec-first Rules

- If behavior is still being decided, update the relevant spec before or together with scaffolding code.
- Put normative workflow and contract changes in `docs/specs/`.
- Put durable reasons and tradeoffs in `docs/adr/`.
- Do not duplicate long rationale across spec, README, usage, and AGENTS files. Keep the reason once in spec or ADR, then keep the other surfaces consistent with it.

## Sync Rules

- When command behavior, config fields, or user-visible output changes, update the affected usage docs in the same change.
- Keep `docs/usage.md` and `../site/usage.html` in sync.
- Keep command status labels aligned across `../README.md`, `usage.md`, and `../site/usage.html`.
- Keep `validate --json` and `handoff <kindle|print|proof>` descriptions aligned across `specs/functional-spec.md`, `usage.md`, `../site/usage.html`, and `../README.md`.
- Keep install and release guidance aligned across `../README.md` and `../editors/vscode/README.md` when GitHub Release assets, Homebrew / Scoop distribution, or VSIX packaging flow changes.
- Keep `specs/vscode-extension.md` and `../editors/vscode/README.md` aligned when the extension flow, scope, or ownership changes.

## Review Checklist

Before finishing a docs change, verify:

1. The normative contract lives in `specs/` if behavior changed.
2. The rationale lives in `adr/` if a durable decision changed.
3. Usage examples match the current CLI.
4. Mirror docs that must stay aligned were updated together.
