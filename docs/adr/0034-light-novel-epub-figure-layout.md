# ADR-0034: light-novel EPUB figure layout uses generated CSS

Status: Accepted

## Context

Light-novel EPUB / Kindle output often needs illustrations to occupy their own page-like unit rather than flow inline with prose. Pandoc converts Markdown images with captions into `figure` / `figcaption`, but existing scaffolded `epub.css` does not force page breaks around figures.

Changing only scaffolded CSS would not help projects already initialized with older `styles/epub.css`. Applying page breaks to `img` alone would also split captions away from images when Pandoc has produced a `figure`.

## Decision

Add `images.epub_figure_layout` with values `auto`, `inline`, and `standalone`.

- `auto` resolves to `standalone` for `book.profile: light-novel`.
- `auto` resolves to `inline` for `business`, `paper`, `conference-preprint`, and `novel`.
- `standalone` can be set explicitly for those non-light-novel prose profiles.

For prose Kindle / EPUB builds that resolve to `standalone`, `shosei build` writes a generated EPUB figure stylesheet and passes it to Pandoc after authored `base.css` and `epub.css`. The generated CSS targets `figure`, not bare `img`, and keeps `figcaption` from being intentionally sent to a separate page. EPUB reader support for CSS page breaks varies, so the contract is to express the standalone-page intent where the reader honors it.

Manga builds keep their fixed-layout page image model and do not use this prose generated stylesheet.

## Consequences

- Existing prose projects can get the new light-novel behavior without editing scaffolded CSS.
- Authored CSS remains responsible for general EPUB look and feel; generated CSS owns the profile/config-driven figure pagination rule.
- Captions should live inside Pandoc's `figure` when the image and caption must stay together. The recommended Markdown form is `![図：キャプション。](assets/images/example.png)`. A following `*図：...*` paragraph is outside the `figure` and is not compatible with standalone figure pagination.
- `series.yml` can provide `defaults.images.epub_figure_layout`, with book-level config able to override it.
