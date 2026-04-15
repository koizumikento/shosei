# AGENTS.md

## Scope

Rules in this file apply to `crates/shosei-cli/` and `crates/shosei-core/`.

## Implementation Rules

- Implementation language is Rust.
- Keep publishing logic, repo discovery, config merge, validation planning, and toolchain decisions in Rust CLI/core.
- Only add a new CLI flag or subcommand when the workflow needs repeatable automation, scripting, or explicit repo-local control.
- Keep config paths repo-relative and `/`-separated in serialized config.
- Treat `book.yml` and `series.yml` as stable names unless a spec and ADR explicitly change them.

## Repository Model Rules

- Preserve the repository model:
  - `single-book`: root `book.yml`
  - `series`: root `series.yml` plus `books/<book-id>/book.yml`
- For `series` repos, current repo discovery requires either `--book <book-id>` or running the command from inside `books/<book-id>/...`.

## Current CLI Surface

The current CLI surface wired in `shosei-cli` is:

- `init`
- `explain`
- `build`
- `validate`
- `preview`
- `chapter`
- `reference`
- `story`
- `series`
- `page`
- `doctor`
- `handoff`

Keep examples and command handling aligned with the actually implemented surface.

## Workflow Rules

- For current user flows, prefer examples in the order `init` -> `explain` -> `build` / `validate`.
- `explain` is the supported way to inspect resolved config and origin data before running output commands.
- `shosei chapter <subcommand>` is for prose books only and updates `manuscript.chapters`; it does not manage manga page order.
- `shosei reference <subcommand>` and `shosei story <subcommand>` are explicit opt-in workspace flows. Start from `scaffold`, then use `map` / `check` before `drift` / `sync`.
- `shosei story sync --report <path> --force` and `shosei reference sync --report <path> --force` are the batch replay flows; do not describe report-driven sync without the explicit reviewed report and `--force`.
- `shosei page check` is for manga books only and inspects page order and spread-related issues without mutating prose chapter config.
- `shosei handoff print` and `shosei handoff proof` are the supported handoff destinations. Keep package contents and docs aligned with the implemented destination behavior.

## Validation

Do not claim that Cargo commands have been run unless they were actually executed.

Use these exact commands when validating Rust changes:

- formatting: `cargo fmt`
- linting: `cargo clippy --workspace --all-targets -- -D warnings`
- tests: `cargo test --workspace`
- CLI smoke tests: `cargo test -p shosei-cli --test cli_smoke`
- focused repo discovery checks: `cargo test -p shosei-core --test repo_discovery`
- focused build / validate / handoff checks: `cargo test -p shosei-core --test book_commands`
- focused chapter workflow checks:
  - `cargo test -p shosei-core --test chapter_commands`
  - `cargo test -p shosei-core --test chapter_renumber`
- focused reference/story workflow checks:
  - `cargo test -p shosei-core --test reference_commands`
  - `cargo test -p shosei-core --test story_commands`
- smoke checks: `cargo run -p shosei-cli --bin shosei -- --help`

## Safety Checks

- Before changing repository structure, verify whether the change affects `single-book` / `series` behavior.
- Before renaming config files, CLI commands, or repo model concepts, update the related spec and ADR first.
