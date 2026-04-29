# shosei

`shosei` is a VS Code-compatible extension for working with shosei publishing repositories.

The extension is a thin adapter over the real `shosei` CLI. It adds command palette actions, a project sidebar, output channel integration, and Problems reporting while keeping publishing logic in the CLI.

## Requirements

- A VS Code-compatible editor such as VS Code or Cursor
- The `shosei` CLI installed and available on `PATH`
- A shosei repository with either `book.yml` or `series.yml`

If you have not installed the CLI yet, see the shosei install guide in the project repository.

## What You Can Do

- Initialize a shosei repository from the editor
- Inspect the current repository model, selected book, resolved config, structure, and toolchain state
- Run `explain`, `doctor`, `validate`, `build`, `preview`, and `page check`
- Manage prose chapters with add, move, remove, and renumber commands
- Use reference workspace commands: scaffold, map, check, drift, and sync
- Use story workspace commands: scaffold, seed, map, check, drift, and sync
- Run `series sync` for series repositories
- Open validation and drift findings in the Problems panel

## Sidebar

The `Shosei` activity bar view shows the current project context:

- `Context`: repository mode, root, and selected series book
- `Structure`: config files, chapters, reference files, story files, structure templates, and editorial sidecars
- `Actions`: project, chapter, reference, story, and series commands
- `Resolved Config`: title, project type, language, outputs, writing mode, binding, and editorial summary
- `Toolchain`: required and optional tool status from `shosei doctor --json`

For series repositories, use `Shosei: Select Book` when the active file is outside `books/<book-id>/`.

## Commands

- `Shosei: Init`
- `Shosei: Chapter Add`
- `Shosei: Chapter Move`
- `Shosei: Chapter Remove`
- `Shosei: Chapter Renumber`
- `Shosei: Explain`
- `Shosei: Validate`
- `Shosei: Build`
- `Shosei: Preview`
- `Shosei: Preview (Watch)`
- `Shosei: Reference Scaffold`
- `Shosei: Reference Map`
- `Shosei: Reference Check`
- `Shosei: Reference Drift`
- `Shosei: Reference Sync`
- `Shosei: Story Scaffold`
- `Shosei: Story Seed`
- `Shosei: Story Map`
- `Shosei: Reveal Scene In Index`
- `Shosei: Story Check`
- `Shosei: Story Drift`
- `Shosei: Story Sync`
- `Shosei: Doctor`
- `Shosei: Page Check`
- `Shosei: Series Sync`
- `Shosei: Select Book`
- `Shosei: Refresh View`

## Settings

By default, the extension runs `shosei` from `PATH`.

```json
{
  "shosei.cli.command": "shosei",
  "shosei.cli.args": []
}
```

To run a local source checkout of the CLI, set `shosei.cli.command` to `cargo` and pass the CLI crate with `--manifest-path`.

```json
{
  "shosei.cli.command": "cargo",
  "shosei.cli.args": [
    "run",
    "--manifest-path",
    "/path/to/shosei/crates/shosei-cli/Cargo.toml",
    "--bin",
    "shosei",
    "--"
  ]
}
```

For series repositories, set `shosei.series.defaultBookId` when commands should use a specific book and the active file is not under `books/<book-id>/`.

## Manual VSIX Install

Open VSX-compatible editors can update from Open VSX. For manual installs, use the `shosei-vscode-<version>.vsix` asset from the GitHub Release page.

- VS Code: run `Extensions: Install from VSIX...`
- Cursor: run `Extensions: Install from VSIX...`

After installing the extension, the actual publishing work is still performed by the configured `shosei` CLI.
