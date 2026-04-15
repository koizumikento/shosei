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
