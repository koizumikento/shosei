# AGENTS.md

## Purpose

This repository is a `shosei` publishing project initialized as `{{REPO_MODE}}` with the `{{PROJECT_TYPE}}` template. Use the `shosei` CLI as the source of truth for resolved publishing behavior.

## Project Shape

- Config entrypoint: {{PRIMARY_CONFIG}}
- Primary content paths: {{PRIMARY_CONTENT_PATHS}}
- {{REPO_MODE_RULES}}

## CLI Workflow

1. Inspect resolved state before guessing.
   - Start with `{{EXPLAIN_COMMAND}}` when config defaults, inherited values, output paths, or repo scope matter.
2. Edit the smallest relevant files.
   - Keep config in `book.yml` or `series.yml`.
   - Keep serialized paths repo-relative and `/`-separated.
   - Do not invent config keys, directory conventions, or command behavior that the installed `shosei` does not support.
3. Validate after config or content changes.
   - Run `{{VALIDATE_COMMAND}}`.
   - Narrow with `--target kindle|print` only when the change affects one output path.
   - {{PAGE_CHECK_RULE}}
4. Build, preview, or package only when the task calls for it.
   - Build artifacts with `{{BUILD_COMMAND}}`.
   - Generate local previews with `{{PREVIEW_COMMAND}}`.
   - Prepare handoff packages with `{{HANDOFF_COMMAND}}`.

## Optional Workspaces

- Reference support is opt-in. {{REFERENCE_SCAFFOLD_RULE}}
- Story support is opt-in. {{STORY_SCAFFOLD_RULE}}
- {{OPTIONAL_MAP_RULE}}

## Guardrails

- Preserve the initialized repository model instead of mixing `single-book` and `series` assumptions.
- Keep `book.yml` and `series.yml` as stable config filenames.
- Do not treat generated `dist/` output as source unless a project rule explicitly says otherwise.
- If the CLI behavior and this file disagree, trust the current `shosei` CLI and update this file.
- Repo-scoped agent skills live under `.agents/skills/`; customize those for repeated task-specific workflows.
