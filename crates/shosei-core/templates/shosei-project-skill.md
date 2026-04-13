---
name: "shosei-project"
description: "{{DESCRIPTION}}"
compatibility: "Requires a repository initialized by `shosei init` and a local `shosei` CLI in the working environment."
---

# Shosei Project

Use this skill for day-to-day work inside this repository. It is repo-scoped on purpose: keep the instructions narrow and customize the project notes below instead of pasting the same rules into every prompt.

## Repo Notes

- This repo was initialized as `{{REPO_MODE}}` with the `{{PROJECT_TYPE}}` template.
- Primary config entrypoint: {{PRIMARY_CONFIG}}
- Primary content paths: {{PRIMARY_CONTENT_PATHS}}
- Customize these notes with printer, distro, naming, or handoff rules before relying on implicit invocation.

## Use For

- inspecting resolved config before editing or building
- updating `book.yml` or `series.yml`
- editing prose under `manuscript/` or manga sources under `manga/`
- validating, building, previewing, or preparing handoff with `shosei`
- tasks phrased like "build this book", "fix the config", "validate the manuscript", "prepare Kindle output", or "update the manga pages"

## Do Not Use For

- changing the `shosei` CLI implementation itself
- generic Rust, CI, or repository-maintenance work that is not about operating a `shosei` project
- inventing new config schema or directory conventions without first confirming they are supported by the installed `shosei`

## Workflow

1. Identify the repo shape before acting.
   - Use `book.yml` for `single-book`.
   - Use `series.yml` plus the target `books/<book-id>/book.yml` for `series`.
   - {{REPO_MODE_RULES}}
2. Inspect resolved state before guessing.
   - Run `{{EXPLAIN_COMMAND}}` when config origin or defaults matter.
   - Read the existing config and generated paths before editing.
3. Edit the smallest relevant surface.
   - Config changes go through `book.yml` / `series.yml`.
   - Prose content lives in `manuscript/`.
   - Manga content lives in `manga/script/`, `manga/storyboard/`, `manga/pages/`, `manga/spreads/`, and `manga/metadata/`.
   - Keep serialized config paths repo-relative and `/`-separated.
4. Validate after changes.
   - Run `{{VALIDATE_COMMAND}}` after config or content edits.
   - Narrow with `--target kindle|print` when the change only affects one output path.
   - {{PAGE_CHECK_RULE}}
5. Build or handoff only when the task calls for it.
   - Use `{{BUILD_COMMAND}}` when the user wants artifacts or output verification.
   - Use `{{PREVIEW_COMMAND}}` for preview generation.
   - Use `{{HANDOFF_COMMAND}}` only for packaging tasks.
6. Report what changed and what remains uncertain.
   - Mention the files you changed.
   - Include the `shosei` commands you ran and whether they succeeded.
   - Call out unsupported schema, missing dependencies, or ambiguous `series` targets instead of papering over them.

## Guardrails

- Keep `book.yml` and `series.yml` as the stable config filenames.
- Preserve the repository model instead of mixing `single-book` and `series` assumptions.
- Prefer `shosei explain` before inferring resolved values from partial config.
- Do not rewrite chapter lists, page sets, or shared asset paths unless the task requires it.
- Do not add `scripts/`, `references/`, or `agents/openai.yaml` to this skill until the instruction-only version proves insufficient.
- After customizing this template, keep the description aligned with the actual trigger phrases your team uses.
