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
- Keep `Shosei: Init` aligned with the current post-scaffold choice to run `shosei doctor` immediately after scaffold generation.
- Keep `Shosei: Preview (Watch)` aligned with the implemented `shosei preview --watch` flow; do not document it as a separate rendering path.
- Keep `shosei.cli.command` / `shosei.cli.args` guidance aligned with the configured runner flow and the repo-local `cargo run --manifest-path <repo>/crates/shosei-cli/Cargo.toml --bin shosei --` fallback.
- Keep prose `validate` handling aligned with the current `manuscript_stats` report payload and only show the character summary when the CLI returned it.
- For `series` repos, keep `Shosei: Select Book` and `shosei.series.defaultBookId` aligned with CLI `--book` requirements when the active file is outside `books/<book-id>/`.
- Keep `Shosei: Story Seed` aligned with the implemented `shosei story seed --template <template> [--force]` flow, including template selection from the current book-scoped `story/structures/` workspace.
- Keep `Shosei: Reveal Scene In Index` aligned with the current scene note context action that opens the corresponding `scenes.yml` entry.
- Keep `Shosei: Refresh View` as a view-only reload of CLI-backed repo context; it should refresh `explain --json` / `doctor --json`-driven state instead of introducing adapter-owned cache invalidation rules.
- Keep the Extension Development Host fallback aligned with the current repo-local CLI invocation: `cargo run --manifest-path <repo>/crates/shosei-cli/Cargo.toml --bin shosei --`.
- Keep command mapping aligned with the implemented CLI surface and current specs.

## Sync Rules

- Keep `README.md` in this directory aligned with `../../docs/specs/vscode-extension.md`.
- Treat `README.md` as the packaged extension README shown on Open VSX. Keep it user-facing; put maintainer release, CI, token, and development-host details in `DEVELOPMENT.md` or docs outside the VSIX package.
- If guided flows change, update the corresponding README examples and wording in the same change.
- Keep adapter behavior consistent with ADR-0025 and do not fork logic that belongs in Rust.
- Keep `package.json` scripts, `DEVELOPMENT.md` local packaging steps, and `../../.github/workflows/ci.yml` / `../../.github/workflows/release.yml` aligned when VSIX packaging, Open VSX publish, or release checks change.

## Validation

When changing the extension, use these checks as appropriate:

- `npm ci`
- `npm run check`
- `npm test`
- `npm run test:host`
- `npm run test:package-smoke`
- `npm run package`
- `node --check extension.js`
- `node --check src/core.js`
- `node --check src/view.js`
- `node --test ./test/**/*.test.js`

CI runs `npm run test:host` on Ubuntu, macOS, and Windows. Ubuntu CI wraps host/package smoke with `xvfb-run -a`; use the same wrapper locally on headless Linux.
