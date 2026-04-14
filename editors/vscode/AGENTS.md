# AGENTS.md

## Scope

Rules in this file apply to `editors/vscode/`.

## Ownership

`editors/vscode/` is a thin VS Code adapter over `shosei`. It owns editor integration, not publishing logic.

## Adapter Rules

- Shell out to the `shosei` CLI for real work.
- Do not reimplement repo discovery, config merge, validation planning, build planning, or toolchain inspection in the extension host.
- Do not let the extension write scaffold files that the CLI already knows how to generate.
- Gather user input in VS Code, map it to CLI flags, run the command, and render the result.

## Command Integration Rules

- Use `shosei explain --json` for resolved config / structure views.
- Use `shosei doctor --json` for toolchain views.
- Use CLI-generated report paths from `validate` and `page check` to populate Problems.
- Keep command mapping aligned with the implemented CLI surface and current specs.

## Sync Rules

- Keep `README.md` in this directory aligned with `../../docs/specs/vscode-extension.md`.
- If guided flows change, update the corresponding README examples and wording in the same change.
- Keep adapter behavior consistent with ADR-0025 and do not fork logic that belongs in Rust.

## Validation

When changing the extension, use these checks as appropriate:

- `node --check extension.js`
- `node --check src/core.js`
- `node --check src/view.js`
- `node --test test/core.test.js`
- `node --test test/extension.test.js`
