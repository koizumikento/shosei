const path = require("path");
const test = require("node:test");
const assert = require("node:assert/strict");

const extension = require("../extension");

test("suggestChapterPath stays repo-relative for single-book repos", () => {
  const suggested = extension.__test.suggestChapterPath({
    repo_root: "/tmp/my-book",
    book_root: "/tmp/my-book",
    manuscript: {
      chapters: []
    }
  });

  assert.equal(suggested, "manuscript/01-chapter-1.md");
});

test("suggestChapterPath includes series book prefix when no chapters exist", () => {
  const suggested = extension.__test.suggestChapterPath({
    repo_root: "/tmp/my-series",
    book_root: "/tmp/my-series/books/vol-01",
    manuscript: {
      chapters: []
    }
  });

  assert.equal(suggested, "books/vol-01/manuscript/01-chapter-1.md");
});

test("suggestChapterPath reuses the existing chapter directory", () => {
  const suggested = extension.__test.suggestChapterPath({
    manuscript: {
      chapters: ["books/vol-01/manuscript/01-opening.md"]
    }
  });

  assert.equal(suggested, "books/vol-01/manuscript/02-chapter-2.md");
});

test("buildChapterRenumberCommandParts maps options to CLI flags", () => {
  assert.deepEqual(extension.__test.buildChapterRenumberCommandParts({
    startAt: 3,
    width: 4,
    dryRun: true
  }), [
    "chapter",
    "renumber",
    "--start-at",
    "3",
    "--width",
    "4",
    "--dry-run"
  ]);
});

test("validateChapterPathInput rejects non repo-relative paths", () => {
  assert.equal(
    extension.__test.validateChapterPathInput(path.join("manuscript", "01.md")),
    process.platform === "win32" ? "Use a repo-relative path with '/' separators" : null
  );
  assert.equal(
    extension.__test.validateChapterPathInput("/tmp/01.md"),
    "Use a repo-relative path with '/' separators"
  );
  assert.equal(
    extension.__test.validateChapterPathInput("manuscript/01.md"),
    null
  );
});
