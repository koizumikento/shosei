# shosei VS Code Extension Development

This file is for maintainers working from the source tree. It is not included in the packaged VSIX.

`editors/vscode/` is a thin VS Code adapter over the `shosei` CLI.

Development policy:

- Delegate publishing logic to the `shosei` CLI.
- Keep VS Code-side code focused on command execution, output channels, Problems integration, and sidebar rendering.
- Reuse `validate` and `page check` JSON reports instead of reimplementing report logic.
- Show prose `manuscript_stats` summaries only when the CLI returned them.
- Keep the sidebar backed by `shosei explain --json` and `shosei doctor --json`.

## Local Install / Update

You can package a local VSIX and install it into VS Code or Cursor without publishing to a marketplace. Installing a newer VSIX with the same extension id updates the existing extension.

Build the VSIX:

```bash
cd editors/vscode
npm run package
```

Generated file:

```text
editors/vscode/shosei-vscode-0.2.14.vsix
```

Install options:

- VS Code: run `Extensions: Install from VSIX...` and select the `.vsix`
- `code` CLI: `code --install-extension editors/vscode/shosei-vscode-0.2.14.vsix`
- Cursor: run `Extensions: Install from VSIX...` and select the same `.vsix`

After local install, real work is still delegated to the configured `shosei` CLI. To use the source tree CLI, set `shosei.cli.command` / `shosei.cli.args` as shown in the public README.

## GitHub Release

The workflow that attaches a VSIX to GitHub Releases is `.github/workflows/release.yml`.

- The repo release tag follows the `shosei-cli` version: `v<cli-version>`.
- Pushing that tag packages the VSIX and CLI binary archives, then attaches them to the matching release.
- If `OPEN_VSX_TOKEN` is set, the same VSIX is published to Open VSX.
- `workflow_dispatch` can also run the release; without an explicit tag it uses `v<shosei-cli-version>`.
- The VSIX asset name uses the version from `editors/vscode/package.json`.
- The release workflow runs `npm ci`, `npm run check`, `npm test`, and `npm run test:host` before packaging.
- Ubuntu also runs `npm run test:package-smoke` against the packaged VSIX.

Example:

```bash
git tag v0.2.14
git push origin v0.2.14
```

Release assets:

```text
shosei-vscode-0.2.14.vsix
shosei-v0.2.14-x86_64-unknown-linux-gnu.tar.gz
shosei-v0.2.14-x86_64-apple-darwin.tar.gz
shosei-v0.2.14-aarch64-apple-darwin.tar.gz
shosei-v0.2.14-x86_64-pc-windows-msvc.zip
```

Open VSX publishing uses the repository Actions secret `OPEN_VSX_TOKEN`. The `straydog` namespace from `package.json` must exist in Open VSX before publishing. Release reruns use `ovsx publish --skip-duplicate` to avoid duplicate version failures.

Homebrew / Scoop manifest publishing only happens when the package repository push in the later release job succeeds. The CLI archives and VSIX are still always attached to the GitHub Release. Open VSX-compatible editors update from the registry; VS Code-compatible editors such as Cursor can also use the release VSIX for manual install and update.

## Development Host

Open the repository root in VS Code and run the `.vscode/launch.json` configuration `shosei: Extension Development Host`.

The development host starts with `--disable-extensions` to isolate activation errors from unrelated local extensions.

If `shosei.cli.command` / `shosei.cli.args` are unset and the repository contains `crates/shosei-cli/Cargo.toml`, the extension falls back to:

```text
cargo run --manifest-path <repo>/crates/shosei-cli/Cargo.toml --bin shosei --
```

`Shosei: Init` collects template, profile, repository mode, initial series book id, title, author, language, output preset, and optional prose scaffold choices, then maps them to `shosei init <path> --non-interactive ...`. The CLI owns scaffold file generation.

## Validation

Install dependencies:

```bash
npm ci
```

Syntax check:

```bash
npm run check
```

Tests:

```bash
npm test
npm run test:host
npm run test:package-smoke
```

PR CI runs `npm run check`, `npm test`, and `npm run test:host` on Ubuntu, macOS, and Windows after `npm ci`. Ubuntu CI also runs `npm run test:package-smoke` for the packaged VSIX. The release workflow uses the same install and check flow before packaging.
