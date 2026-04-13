# AGENTS.md

## Purpose

This repository defines and implements `shosei`, a Rust CLI for Japanese publishing workflows across EPUB, PDF, Kindle, print handoff, prose books, and manga.

## Source Of Truth

- Product and behavior specs live in `docs/specs/`.
- Decision history lives in `docs/adr/`.
- If behavior changes, update the relevant spec and ADR in the same change or before implementation.
- Use `shosei` as the CLI name in docs and code. The config files remain `book.yml` and `series.yml`.

## Repository Layout

- `docs/specs/`: functional specs, config schema, repository model, migration rules, Rust architecture
- `docs/adr/`: accepted decisions and rationale
- `crates/shosei-cli/`: CLI crate exposing the `shosei` binary
- `crates/shosei-core/`: core library crate for app flows, repo discovery, config, and pipeline planning
- `AGENTS.md`: repo-wide agent instructions

Future directories are expected to include:

- `tests/`: integration and smoke tests
- `fixtures/`: test fixtures and sample books

Add a deeper `AGENTS.md` only when a subdirectory needs rules that differ from this file.

## Current Working Mode

- The repository is currently spec-first.
- Do not invent implementation details that conflict with `docs/specs/`.
- Prefer updating specs before scaffolding code when the behavior is still being decided.

## Delegation Rules

- When a task is read-heavy, delegate the reading pass to a sub-agent first.
- Treat a task as read-heavy when it mainly requires scanning multiple files, specs, ADRs, logs, or diffs before any concrete edit can be made.
- Use the sub-agent for repository exploration, context gathering, and summary preparation; keep final synthesis, decisions, and file edits in the main agent unless the user asks otherwise.
- Skip delegation only for small, targeted reads where spawning a sub-agent would add more overhead than value.

## Implementation Rules

- Implementation language is Rust.
- Target platforms are macOS, Windows, and Linux.
- Keep config paths repo-relative and `/`-separated in serialized config.
- Treat `book.yml` and `series.yml` as stable names unless a spec and ADR explicitly change them.
- Preserve the repository model:
  - `single-book`: root `book.yml`
  - `series`: root `series.yml` plus `books/<book-id>/book.yml`
- The current CLI surface wired in `crates/shosei-cli` is `init`, `explain`, `build`, `validate`, `preview`, `chapter`, `page`, `doctor`, and `handoff`.
- For `series` repos, current repo discovery requires either `--book <book-id>` or running the command from inside `books/<book-id>/...`.
- For current user flows, prefer examples in the order `init` -> `explain` -> `build` / `validate`; `explain` is the supported way to inspect resolved config and origin data before running output commands.
- `shosei chapter <subcommand>` is for prose books only and updates `manuscript.chapters`; it does not manage manga page order.
- `shosei page check` is for manga books only and inspects page order / spread-related issues without mutating prose chapter config.

## Editing Rules

- Keep docs concise and practical. Do not paste large parts of specs into AGENTS files.
- Prefer relative links inside `docs/`.
- When updating docs, keep command examples aligned with the current CLI name: `shosei`.
- Do not describe commands or features as `available` unless the current code path actually implements them.
- When command behavior, config fields, or user-visible output changes, update the affected usage docs in the same change.
- Keep `docs/usage.md` and `site/usage.html` in sync. If one changes, review and update the other in the same change.
- Keep command status labels in `README.md`, `docs/usage.md`, and `site/usage.html` aligned with the current implementation.
- Prefer examples that reflect current supported config fields and output behavior; remove or rewrite stale examples instead of leaving them partially correct.
- Do not add vague instructions like "do the right thing". Write concrete rules or leave them out.

## Validation

Current repo state:

- Rust workspace exists.
- Do not claim that `cargo` commands have been run unless they actually exist and were executed.

Use these exact commands when validating Rust changes:

- formatting: `cargo fmt`
- linting: `cargo clippy --workspace --all-targets -- -D warnings`
- tests: `cargo test --workspace`
- focused repo discovery checks: `cargo test -p shosei-core --test repo_discovery`
- focused chapter workflow checks: `cargo test -p shosei-core --test chapter_commands` and `cargo test -p shosei-core --test chapter_renumber`
- smoke checks: `cargo run -p shosei-cli --bin shosei -- --help`

## Generated Files

- Do not commit incidental build output.
- Treat generated output directories like `dist/` as disposable unless a spec explicitly requires checked-in artifacts.

## Safety Checks

- Before changing repository structure, verify whether the change affects `single-book` / `series` behavior.
- Before renaming config files, CLI commands, or repo model concepts, update the related ADRs and specs first.
- If the same mistake happens twice, tighten this file with a concrete rule instead of adding more prompt text elsewhere.
