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
- `AGENTS.md`: repo-wide agent instructions

Future directories are expected to include:

- `crates/`: Rust workspace crates such as `shosei-cli` and `shosei-core`
- `tests/`: integration and smoke tests
- `fixtures/`: test fixtures and sample books

Add a deeper `AGENTS.md` only when a subdirectory needs rules that differ from this file.

## Current Working Mode

- The repository is currently spec-first.
- Do not invent implementation details that conflict with `docs/specs/`.
- Prefer updating specs before scaffolding code when the behavior is still being decided.

## Implementation Rules

- Implementation language is Rust.
- Target platforms are macOS, Windows, and Linux.
- Keep config paths repo-relative and `/`-separated in serialized config.
- Treat `book.yml` and `series.yml` as stable names unless a spec and ADR explicitly change them.
- Preserve the repository model:
  - `single-book`: root `book.yml`
  - `series`: root `series.yml` plus `books/<book-id>/book.yml`

## Editing Rules

- Keep docs concise and practical. Do not paste large parts of specs into AGENTS files.
- Prefer relative links inside `docs/`.
- When updating docs, keep command examples aligned with the current CLI name: `shosei`.
- Do not add vague instructions like "do the right thing". Write concrete rules or leave them out.

## Validation

Current repo state:

- There is no Rust workspace yet, so do not claim that `cargo` commands have been run unless they actually exist and were executed.

When the Rust workspace is added, update this file with exact commands for:

- formatting
- linting
- tests
- smoke checks

## Generated Files

- Do not commit incidental build output.
- Treat generated output directories like `dist/` as disposable unless a spec explicitly requires checked-in artifacts.

## Safety Checks

- Before changing repository structure, verify whether the change affects `single-book` / `series` behavior.
- Before renaming config files, CLI commands, or repo model concepts, update the related ADRs and specs first.
- If the same mistake happens twice, tighten this file with a concrete rule instead of adding more prompt text elsewhere.
