const fs = require("fs");
const os = require("os");
const path = require("path");
const test = require("node:test");
const assert = require("node:assert/strict");

const core = require("../src/core");

function tempDir(name) {
  return fs.mkdtempSync(path.join(os.tmpdir(), `shosei-vscode-${name}-`));
}

test("findRepoRoot detects single-book repositories", () => {
  const root = tempDir("single");
  fs.writeFileSync(path.join(root, "book.yml"), "book:\n  title: Test\n");
  fs.mkdirSync(path.join(root, "manuscript"));
  const file = path.join(root, "manuscript", "01.md");
  fs.writeFileSync(file, "# Test\n");

  const repo = core.findRepoRoot(file);
  assert.deepEqual(repo, { repoRoot: root, mode: "single-book" });
});

test("findRepoRoot detects series repositories from nested files", () => {
  const root = tempDir("series");
  const nested = path.join(root, "books", "vol-02", "manuscript");
  fs.mkdirSync(nested, { recursive: true });
  fs.writeFileSync(path.join(root, "series.yml"), "series:\n  id: test\n");
  const file = path.join(nested, "01.md");
  fs.writeFileSync(file, "# Test\n");

  const repo = core.findRepoRoot(file);
  assert.deepEqual(repo, { repoRoot: root, mode: "series" });
  assert.equal(core.inferSeriesBookId(root, file), "vol-02");
});

test("listSeriesBookIds returns sorted directory names", () => {
  const root = tempDir("book-list");
  fs.mkdirSync(path.join(root, "books", "vol-10"), { recursive: true });
  fs.mkdirSync(path.join(root, "books", "vol-02"), { recursive: true });
  fs.mkdirSync(path.join(root, "books", "vol-01"), { recursive: true });
  fs.writeFileSync(path.join(root, "books", "README.md"), "ignored\n");

  assert.deepEqual(core.listSeriesBookIds(root), ["vol-01", "vol-02", "vol-10"]);
});

test("buildCliInvocation appends --book and --path", () => {
  const invocation = core.buildCliInvocation({
    cliCommand: "cargo",
    cliArgs: ["run", "-p", "shosei-cli", "--bin", "shosei", "--"],
    commandParts: ["validate"],
    bookId: "vol-01",
    repoRoot: "/tmp/project"
  });

  assert.equal(invocation.command, "cargo");
  assert.deepEqual(invocation.args, [
    "run",
    "-p",
    "shosei-cli",
    "--bin",
    "shosei",
    "--",
    "validate",
    "--book",
    "vol-01",
    "--path",
    "/tmp/project"
  ]);
});

test("extractReportPath picks the final report path from command output", () => {
  const output = [
    "validation completed for vol-01 with outputs: kindle, issues: 2, report: /tmp/a.json",
    "page check completed for vol-01 with 4 page(s), issues: 1, report: /tmp/b.json"
  ].join("\n");

  assert.equal(core.extractReportPath(output), "/tmp/b.json");
});
