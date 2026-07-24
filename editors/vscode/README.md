# shosei

`shosei` is a VS Code-compatible extension for working with shosei publishing repositories.

This extension does not reimplement the publishing engine. It is a thin editor adapter over the real `shosei` CLI: command palette actions, a project sidebar, output channel integration, and Problems reporting all shell out to the bundled or explicitly configured CLI.

## Bundled CLI

The `linux-x64`, `darwin-x64`, `darwin-arm64`, and `win32-x64` extension packages include the matching `shosei` CLI. On these platforms, installing the extension is enough to provide the `shosei` executable used by editor commands.

The universal VSIX and unsupported platforms fall back to `shosei` from `PATH`. You can also select a custom binary or source checkout with `shosei.cli.command` and `shosei.cli.args`.

Pandoc, Chromium, epubcheck, qpdf, Kindle Previewer, and other publishing tools are not bundled. Run `Shosei: Doctor` to see which tools are required for the current repository and output targets.

## Requirements

- A VS Code-compatible editor such as VS Code or Cursor
- A platform-specific extension package, or the `shosei` CLI available on `PATH` for the universal package
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

Leave the CLI settings empty to use the bundled CLI. Development hosts use the repo-local Cargo fallback, and packages without a bundled binary fall back to `shosei` from `PATH`.

```json
{
  "shosei.cli.command": "",
  "shosei.cli.args": []
}
```

To override the bundled CLI, set `shosei.cli.command` to an executable path and keep `shosei.cli.args` empty.

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

Open VSX-compatible editors select and update the package matching the current platform. For manual installs, choose `shosei-vscode-<version>-<target>.vsix` from the GitHub Release page. Use the unqualified `shosei-vscode-<version>.vsix` only as the universal fallback that requires a CLI on `PATH`.

- VS Code: run `Extensions: Install from VSIX...`
- Cursor: run `Extensions: Install from VSIX...`

After installing the extension, the actual publishing work is performed by the bundled or explicitly configured `shosei` CLI.
