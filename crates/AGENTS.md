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
- Keep `init` guidance aligned across interactive and `--non-interactive --config-template ...` flows, including `paper` `--config-profile`, prose `--include-introduction` / `--include-afterword`, and `series` `--initial-book-id`.
- Keep `init` follow-up guidance aligned with the implemented post-scaffold flow: interactive init can optionally run `shosei doctor`, and otherwise prints the `toolchain hint: run shosei doctor` reminder.
- `explain` is the supported way to inspect resolved config and origin data before running output commands.
- Kindle-capable `init` scaffolds are expected to wire `cover.ebook_image` and placeholder cover assets so a fresh scaffold does not start with a missing-cover warning.
- `preview` supports both one-shot output checks and a longer-running `--watch` loop. Describe `--watch` only for iterative local preview workflows.
- `shosei chapter <subcommand>` is for prose books only and updates `manuscript.chapters`; it does not manage manga page order.
- `shosei chapter renumber` is the explicit filename-prefix rewrite flow. Do not imply that chapter add/move/remove rewrites prose filenames automatically.
- `shosei series sync` is the canonical `series.yml`-driven refresh for `shared/metadata/series-catalog.{yml,md}` and prose `manuscript.backmatter`; do not describe it as rewriting handwritten manuscript body files.
- `shosei reference <subcommand>` and `shosei story <subcommand>` are explicit opt-in workspace flows. Start from `scaffold`, then use `map` / `check` before `drift` / `sync`.
- `shosei story seed --template <name>` is the supported book-scoped flow for turning `structures/*.md` `scene_seeds` frontmatter into `scenes.yml` plus `scene-notes/*.md` drafts.
- For `series` repos, keep `reference scaffold` / `story scaffold` examples explicit about scope: use `--shared` for shared metadata workspaces and `--book <book-id>` for book-scoped workspaces.
- Keep `story seed` examples book-scoped only. Do not imply that shared story workspaces or plain `story scaffold` populate `scenes.yml` or scene notes automatically.
- `shosei story sync --report <path> --force` and `shosei reference sync --report <path> --force` are the batch replay flows; do not describe report-driven sync without the explicit reviewed report and `--force`.
- `shosei page check` is for manga books only and inspects page order and spread-related issues without mutating prose chapter config.
- `shosei validate --json` is supported. Keep the stdout JSON payload aligned with the written report schema, and reserve the human issue preview for non-JSON output.
- `shosei handoff kindle`, `shosei handoff print`, and `shosei handoff proof` are the supported handoff destinations. Keep package contents and docs aligned with the implemented destination behavior.

## Validation

Do not claim that Cargo commands have been run unless they were actually executed.

Use these exact commands when validating Rust changes:

- formatting: `cargo fmt`
- CI formatting gate: `cargo fmt --check`
- linting: `cargo clippy --workspace --all-targets -- -D warnings`
- tests: `cargo test --workspace`
- CLI smoke tests:
  - `cargo test -p shosei-cli --test cli_smoke init_cli_interactive_shows_summary_and_writes_after_confirmation -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_prints_issue_preview -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_can_emit_json_report -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_json_includes_epubcheck_runs -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_json_records_missing_epubcheck_without_failing -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_json_fails_when_epubcheck_reports_errors -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_json_includes_print_validator_runs -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_json_records_missing_print_validator_without_failing -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_json_fails_when_print_validator_reports_errors -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke validate_cli_json_includes_kindle_previewer_runs -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke build_cli_prints_tools_and_writes_artifact -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke preview_cli_prints_summary_and_writes_artifact -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke page_check_cli_prints_summary_and_issue_preview -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke chapter_add_cli_updates_config_and_creates_stub_file -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke reference_scaffold_cli_creates_workspace -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke reference_check_cli_prints_issue_preview_and_fails_on_errors -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke reference_drift_cli_writes_report_and_fails_on_drift -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke reference_sync_cli_copies_shared_entry_into_book_scope -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke story_seed_cli_creates_scenes_and_notes_from_template -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke series_sync_cli_generates_catalog_and_updates_books -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke handoff_proof_cli_packages_review_packet -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke handoff_kindle_cli_packages_manifest_with_artifact_details -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke handoff_print_cli_packages_manga_pdf -- --exact`
  - `cargo test -p shosei-cli --test cli_smoke doctor_json_cli_includes_detected_project_context -- --exact`
- focused repo discovery checks: `cargo test -p shosei-core --test repo_discovery`
- focused build / validate / handoff checks: `cargo test -p shosei-core --test book_commands`
- focused chapter workflow checks:
  - `cargo test -p shosei-core --test chapter_commands`
  - `cargo test -p shosei-core --test chapter_renumber`
- focused reference/story workflow checks:
  - `cargo test -p shosei-core --test reference_commands`
  - `cargo test -p shosei-core --test story_commands`
- smoke checks: `cargo run -p shosei-cli --bin shosei -- --help`
- CI runs the formatting gate, linting, workspace tests, repo discovery, and the listed Rust smoke checks on `ubuntu-latest`, `macos-latest`, and `windows-latest`.

## Safety Checks

- Before changing repository structure, verify whether the change affects `single-book` / `series` behavior.
- Before renaming config files, CLI commands, or repo model concepts, update the related spec and ADR first.
