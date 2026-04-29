# shosei

`shosei` is a VS Code-compatible extension for working with shosei publishing repositories.

This extension does not include the publishing engine. It is a thin editor adapter over the real `shosei` CLI: command palette actions, a project sidebar, output channel integration, and Problems reporting all shell out to the configured CLI.

## CLI Requirement

The extension runs the `shosei` CLI you already use in your shell. By default it calls `shosei` from `PATH`.

Check that the command is available:

```bash
shosei --help
```

If that command is not available yet, follow the [shosei CLI setup guide](https://github.com/koizumikento/shosei#install). If you use a custom binary path or a source checkout, configure `shosei.cli.command` and `shosei.cli.args` in the extension settings.

## Requirements

- A VS Code-compatible editor such as VS Code or Cursor
- The `shosei` CLI available on `PATH` or configured in extension settings
- A shosei repository with either `book.yml` or `series.yml`

## What You Can Do

- Initialize a shosei repository from the editor
- Inspect the current repository model, selected book, resolved config, structure, and toolchain state
- Run `explain`, `doctor`, `validate`, `build`, `preview`, and `page check`
- Show the latest prose manuscript character count in the status bar after `validate`
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

If the CLI is installed somewhere else, set `shosei.cli.command` to that executable path and keep `shosei.cli.args` empty.

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
