# ADR-0033: prose image paths follow Pandoc book-root resolution

- Status: Accepted
- Date: 2026-04-29

## Context

Prose books use Pandoc for Kindle / EPUB packaging. `shosei build` invokes Pandoc from the current book root, so a Markdown image such as `assets/images/example.png` points at the book's asset directory: `assets/images/...` in `single-book` repos and `books/<book-id>/assets/images/...` in `series` book scopes.

`shosei validate` previously resolved Markdown image destinations only relative to the manuscript file. That meant a path shape that Pandoc could package into EPUB could still fail local image validation and figure-ledger matching.

## Decision

For prose manuscript images, `shosei validate` resolves local Markdown image destinations the same way the EPUB packaging path is expected to work:

- first prefer an existing path resolved from the current book root
- then fall back to an existing path resolved relative to the manuscript file
- compare `editorial/figures.yml` against the normalized repo-relative path

The fallback preserves existing manuscripts that already use paths such as `../assets/images/example.png` from files under `manuscript/`.

`figures.yml.path` remains a repo-relative asset path.

## Consequences

- A single asset at `assets/images/...` can satisfy Markdown image references, `validation.missing_image`, figure-ledger tracking, Pandoc EPUB packaging, and EPUBCheck in `single-book` repos.
- Series books get the same behavior under the selected book root, for example `books/<book-id>/assets/images/...`.
- Source-file-relative image paths continue to work when they point to existing files.
