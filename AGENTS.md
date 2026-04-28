# AGENTS.md

## Purpose

This repository defines and implements `shosei`, a spec-first publishing toolchain centered on a Rust CLI and a thin VS Code adapter.

## Instruction Layout

Keep this root file short and repo-wide. Put specialized rules near the code they govern.

- `AGENTS.md`: repo-wide invariants and cross-surface sync rules
- `docs/AGENTS.md`: specs, ADRs, usage docs, and site sync
- `crates/AGENTS.md`: Rust CLI/core implementation and validation rules
- `editors/vscode/AGENTS.md`: VS Code adapter rules

Add deeper `AGENTS.md` files only when a subtree has materially different rules.

## Source Of Truth

- Product and behavior specs live in `docs/specs/`.
- Decision history and durable rationale live in `docs/adr/`.
- Executable behavior lives in `crates/shosei-cli/` and `crates/shosei-core/`.
- `editors/vscode/` is editor integration only; it does not own publishing logic.

## Repo-wide Invariants

- Use `shosei` as the CLI name in docs and code.
- Keep `book.yml` and `series.yml` as stable config filenames unless a spec and ADR explicitly change them.
- Keep serialized config paths repo-relative and `/`-separated.
- Preserve the repository model:
  - `single-book`: root `book.yml`
  - `series`: root `series.yml` plus `books/<book-id>/book.yml`
- Target platforms are macOS, Windows, and Linux.

## Change Rules

- This repo is spec-first. Do not invent behavior that conflicts with `docs/specs/`.
- If behavior changes, update the relevant spec and ADR in the same change or before implementation.
- When a feature spans multiple surfaces, update them in this order:
  1. spec / ADR
  2. CLI / core
  3. VS Code adapter
  4. usage / README / site docs

## Motivation Check

Before changing a surface, make the motivation explicit in the owning artifact.

- Docs: user-visible contract or workflow clarification
- CLI/core: repeatable, scriptable, cross-platform behavior
- VS Code: editor friction reduction for an existing or concurrently specified CLI workflow

If a surface cannot be justified, do not change it.

## Delegation Preference

- For read-heavy tasks, prefer delegating the repository exploration and summary pass first when subagents are available and allowed.
- Skip delegation for small, targeted reads where spawning a subagent adds more overhead than value.

## Cross-surface Sync

- Do not describe commands or features as available unless the current code path actually implements them.
- Prefer examples that reflect currently supported config fields and output behavior. Remove or rewrite stale examples instead of leaving them partially correct.
- Keep `docs/usage.md` and `site/usage.html` in sync.
- Keep command status labels aligned across `README.md`, `docs/usage.md`, and `site/usage.html`.
- Keep cross-platform smoke coverage aligned across `.github/workflows/ci.yml`, `README.md`, `docs/usage.md`, `site/usage.html`, and `crates/AGENTS.md`.
- Keep `init` scaffold docs aligned across `README.md`, `docs/usage.md`, and `site/usage.html`, including the generated `AGENTS.md`, `.agents/skills/shosei-project/`, and `.agents/skills/shosei-content-review/` templates plus the current `git init` / optional `git lfs install` / optional `shosei doctor` follow-up flow.
- Keep install and release guidance aligned across `README.md`, `site/install.html`, `editors/vscode/README.md`, and `.github/workflows/release.yml` when GitHub Release assets, Homebrew / Scoop distribution, Open VSX publish, or VSIX packaging/update flow changes.
- Keep Homebrew / Scoop manifest guidance aligned with the current release automation in `.github/workflows/release.yml` and the generator scripts in `scripts/generate-homebrew-formula.sh` and `scripts/generate-scoop-manifest.sh`.
- Keep the `story seed` contract aligned across `docs/specs/story-support.md`, `docs/usage.md`, and `site/usage.html` when seeded structure templates or `scene_seeds` behavior changes.
- Keep `docs/specs/vscode-extension.md` and `editors/vscode/README.md` aligned when extension flow, scope, or ownership changes.
- Keep recurring concept-fulfillment reviews aligned with `docs/concept-fulfillment-rubric.md`; do not invent a new scoring standard per run.

## Safety Checks

- Before changing repository structure, verify the effect on `single-book` / `series`.
- Before renaming config files, CLI commands, or repo model concepts, update the related specs and ADRs first.
- Do not commit incidental build output. Treat generated directories such as `dist/` as disposable unless a spec explicitly requires checked-in artifacts.
- If the same mistake happens twice, tighten these instruction files with a concrete rule instead of adding more prompt text elsewhere.
- For recurring concept-fulfillment reviews, use the fixed rubric, cite evidence by file, and compare against the previous run before changing the score.
