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
- Keep `Shosei: Init` prompt collection aligned with the implemented `shosei init <path> --non-interactive ...` flags and defaults, including `paper` profile and `series` initial book id handling.
- Keep the Extension Development Host fallback aligned with the current repo-local CLI invocation: `cargo run --manifest-path <repo>/crates/shosei-cli/Cargo.toml --bin shosei --`.
- Keep command mapping aligned with the implemented CLI surface and current specs.

## Sync Rules

- Keep `README.md` in this directory aligned with `../../docs/specs/vscode-extension.md`.
- If guided flows change, update the corresponding README examples and wording in the same change.
- Keep adapter behavior consistent with ADR-0025 and do not fork logic that belongs in Rust.
- Keep `package.json` scripts, `README.md` local packaging steps, and `../../.github/workflows/release.yml` aligned when VSIX packaging or release checks change.

## Validation

When changing the extension, use these checks as appropriate:

- `npm run check`
- `npm test`
- `npm run package`
- `node --check extension.js`
- `node --check src/core.js`
- `node --check src/view.js`
- `node --test ./test/**/*.test.js`
