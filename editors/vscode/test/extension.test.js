const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("path");

const extension = require("../extension");

test("buildReferenceScopedCommandParts appends --shared and --force when requested", () => {
  assert.deepEqual(
    extension.__test.buildReferenceScopedCommandParts("check", {
      shared: true,
      force: true
    }),
    ["reference", "check", "--shared", "--force"]
  );
});

test("buildReferenceSyncCommandParts builds single-id sync commands", () => {
  assert.deepEqual(
    extension.__test.buildReferenceSyncCommandParts({
      direction: { flag: "--from" },
      id: "market",
      force: false
    }),
    ["reference", "sync", "--from", "shared", "--id", "market"]
  );
});

test("buildReferenceSyncCommandParts builds report sync commands", () => {
  assert.deepEqual(
    extension.__test.buildReferenceSyncCommandParts({
      direction: { flag: "--to" },
      report: "/tmp/vol-01-reference-drift.json",
      force: true
    }),
    [
      "reference",
      "sync",
      "--to",
      "shared",
      "--report",
      "/tmp/vol-01-reference-drift.json",
      "--force"
    ]
  );
});

test("buildStoryScopedCommandParts appends --shared and --force when requested", () => {
  assert.deepEqual(
    extension.__test.buildStoryScopedCommandParts("scaffold", {
      shared: true,
      force: true
    }),
    ["story", "scaffold", "--shared", "--force"]
  );
});

test("buildStorySeedCommandParts builds story seed commands", () => {
  assert.deepEqual(
    extension.__test.buildStorySeedCommandParts({
      template: "save-the-cat",
      force: true
    }),
    ["story", "seed", "--template", "save-the-cat", "--force"]
  );
});

test("findStorySceneLine finds matching scene file lines in scenes.yml", () => {
  const contents = [
    "scenes:",
    "  - file: story/scene-notes/01-opening.md",
    "    title: Opening",
    "  - file: 'story/scene-notes/02-turn.md'",
    "    title: Turn"
  ].join("\n");

  assert.equal(
    extension.__test.findStorySceneLine(contents, "story/scene-notes/01-opening.md"),
    1
  );
  assert.equal(
    extension.__test.findStorySceneLine(contents, "story/scene-notes/02-turn.md"),
    3
  );
  assert.equal(
    extension.__test.findStorySceneLine(contents, "story/scene-notes/missing.md"),
    null
  );
});

test("buildStorySyncCommandParts builds single-entity sync commands", () => {
  assert.deepEqual(
    extension.__test.buildStorySyncCommandParts({
      direction: { flag: "--from" },
      kind: "character",
      id: "lead",
      force: false
    }),
    ["story", "sync", "--from", "shared", "--kind", "character", "--id", "lead"]
  );
});

test("buildStorySyncCommandParts builds report sync commands", () => {
  assert.deepEqual(
    extension.__test.buildStorySyncCommandParts({
      direction: { flag: "--to" },
      report: "/tmp/vol-01-story-drift.json",
      force: true
    }),
    ["story", "sync", "--to", "shared", "--report", "/tmp/vol-01-story-drift.json", "--force"]
  );
});

test("referenceWorkspaceRoot resolves single-book, shared, and series-book paths", () => {
  assert.equal(
    extension.__test.referenceWorkspaceRoot(
      { mode: "single-book", repoRoot: "/tmp/book", bookId: null },
      false
    ),
    path.join("/tmp/book", "references")
  );
  assert.equal(
    extension.__test.referenceWorkspaceRoot(
      { mode: "series", repoRoot: "/tmp/series", bookId: null },
      true
    ),
    path.join("/tmp/series", "shared", "metadata", "references")
  );
  assert.equal(
    extension.__test.referenceWorkspaceRoot(
      { mode: "series", repoRoot: "/tmp/series", bookId: "vol-01" },
      false
    ),
    path.join("/tmp/series", "books", "vol-01", "references")
  );
});

test("storyWorkspaceRoot resolves single-book, shared, and series-book paths", () => {
  assert.equal(
    extension.__test.storyWorkspaceRoot(
      { mode: "single-book", repoRoot: "/tmp/book", bookId: null },
      false
    ),
    path.join("/tmp/book", "story")
  );
  assert.equal(
    extension.__test.storyWorkspaceRoot(
      { mode: "series", repoRoot: "/tmp/series", bookId: null },
      true
    ),
    path.join("/tmp/series", "shared", "metadata", "story")
  );
  assert.equal(
    extension.__test.storyWorkspaceRoot(
      { mode: "series", repoRoot: "/tmp/series", bookId: "vol-01" },
      false
    ),
    path.join("/tmp/series", "books", "vol-01", "story")
  );
});

test("referenceEntriesRoot appends entries to the selected workspace root", () => {
  assert.equal(
    extension.__test.referenceEntriesRoot(
      { mode: "single-book", repoRoot: "/tmp/book", bookId: null },
      false
    ),
    path.join("/tmp/book", "references", "entries")
  );
  assert.equal(
    extension.__test.referenceEntriesRoot(
      { mode: "series", repoRoot: "/tmp/series", bookId: null },
      true
    ),
    path.join("/tmp/series", "shared", "metadata", "references", "entries")
  );
});

test("storyScenesPath appends scenes.yml to the selected book workspace root", () => {
  assert.equal(
    extension.__test.storyScenesPath(
      { mode: "single-book", repoRoot: "/tmp/book", bookId: null }
    ),
    path.join("/tmp/book", "story", "scenes.yml")
  );
  assert.equal(
    extension.__test.storyScenesPath(
      { mode: "series", repoRoot: "/tmp/series", bookId: "vol-01" }
    ),
    path.join("/tmp/series", "books", "vol-01", "story", "scenes.yml")
  );
});

test("validateSeriesBookIdInput matches init CLI constraints", () => {
  assert.equal(extension.__test.validateSeriesBookIdInput("pilot"), null);
  assert.equal(
    extension.__test.validateSeriesBookIdInput(""),
    "Book id is required"
  );
  assert.equal(
    extension.__test.validateSeriesBookIdInput("bad/id"),
    "Book id must be a single path segment"
  );
  assert.equal(
    extension.__test.validateSeriesBookIdInput("bad id"),
    "Book id must not contain whitespace"
  );
});

test("formatManuscriptStatsSummary renders validate report character counts", () => {
  assert.equal(
    extension.__test.formatManuscriptStatsSummary({
      total_characters: 12345,
      chapter_characters: 12000,
      frontmatter_characters: 200,
      backmatter_characters: 145
    }),
    "manuscript characters: 12,345 total (chapters 12,000, frontmatter 200, backmatter 145)"
  );
  assert.equal(extension.__test.formatManuscriptStatsSummary(null), null);
});
