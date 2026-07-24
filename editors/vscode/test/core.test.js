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
  fs.writeFileSync(
    path.join(root, "books", "vol-02", "book.yml"),
    "book:\n  title: Nested Book\n"
  );
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
    cliSource: "configured",
    commandParts: ["validate"],
    bookId: "vol-01",
    repoRoot: "/tmp/project"
  });

  assert.equal(invocation.command, "cargo");
  assert.equal(invocation.source, "configured");
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

test("buildInitCommandParts maps guided init answers to flags", () => {
  const commandParts = core.buildInitCommandParts({
    path: "/tmp/my-book",
    configTemplate: "novel",
    repoMode: "single-book",
    title: "My Book",
    author: "Ken",
    language: "ja-JP",
    outputPreset: "both",
    includeIntroduction: true,
    includeAfterword: true,
    force: true
  });

  assert.deepEqual(commandParts, [
    "init",
    "/tmp/my-book",
    "--non-interactive",
    "--force",
    "--config-template",
    "novel",
    "--repo-mode",
    "single-book",
    "--title",
    "My Book",
    "--author",
    "Ken",
    "--language",
    "ja-JP",
    "--output-preset",
    "both",
    "--include-introduction",
    "--include-afterword"
  ]);
});

test("buildInitCommandParts includes paper config profile when selected", () => {
  const commandParts = core.buildInitCommandParts({
    path: "/tmp/preprint",
    configTemplate: "paper",
    configProfile: "conference-preprint",
    repoMode: "single-book",
    title: "Preprint",
    author: "Ken",
    language: "ja",
    outputPreset: "print"
  });

  assert.deepEqual(commandParts, [
    "init",
    "/tmp/preprint",
    "--non-interactive",
    "--config-template",
    "paper",
    "--config-profile",
    "conference-preprint",
    "--repo-mode",
    "single-book",
    "--title",
    "Preprint",
    "--author",
    "Ken",
    "--language",
    "ja",
    "--output-preset",
    "print"
  ]);
});

test("buildInitCommandParts includes initial book id for series scaffolds", () => {
  const commandParts = core.buildInitCommandParts({
    path: "/tmp/my-series",
    configTemplate: "manga",
    repoMode: "series",
    initialBookId: "pilot",
    title: "My Series",
    author: "Ken",
    language: "ja",
    outputPreset: "both"
  });

  assert.deepEqual(commandParts, [
    "init",
    "/tmp/my-series",
    "--non-interactive",
    "--config-template",
    "manga",
    "--repo-mode",
    "series",
    "--initial-book-id",
    "pilot",
    "--title",
    "My Series",
    "--author",
    "Ken",
    "--language",
    "ja",
    "--output-preset",
    "both"
  ]);
});

test("resolveCliTooling falls back to repo cargo manifest in development", () => {
  const root = tempDir("dev-cli");
  const extensionPath = path.join(root, "editors", "vscode");
  const manifestPath = path.join(root, "crates", "shosei-cli", "Cargo.toml");
  fs.mkdirSync(extensionPath, { recursive: true });
  fs.mkdirSync(path.dirname(manifestPath), { recursive: true });
  fs.writeFileSync(manifestPath, "[package]\nname = \"shosei-cli\"\n");

  const tooling = core.resolveCliTooling({
    cliCommand: "",
    cliArgs: [],
    extensionPath,
    enableDevelopmentFallback: true
  });

  assert.deepEqual(tooling, {
    command: "cargo",
    args: [
      "run",
      "--manifest-path",
      manifestPath,
      "--bin",
      "shosei",
      "--"
    ],
    source: "development"
  });
});

test("resolveCliTooling keeps explicit CLI settings over development fallback", () => {
  const root = tempDir("explicit-cli");
  const extensionPath = path.join(root, "editors", "vscode");
  const manifestPath = path.join(root, "crates", "shosei-cli", "Cargo.toml");
  fs.mkdirSync(extensionPath, { recursive: true });
  fs.mkdirSync(path.dirname(manifestPath), { recursive: true });
  fs.writeFileSync(manifestPath, "[package]\nname = \"shosei-cli\"\n");

  const tooling = core.resolveCliTooling({
    cliCommand: "custom-shosei",
    cliArgs: ["--flag"],
    extensionPath,
    enableDevelopmentFallback: true
  });

  assert.deepEqual(tooling, {
    command: "custom-shosei",
    args: ["--flag"],
    source: "configured"
  });
});

test("resolveCliTooling uses a bundled CLI when settings are empty", () => {
  const root = tempDir("bundled-cli");
  const executable = path.join(root, "bin", "shosei");
  fs.mkdirSync(path.dirname(executable), { recursive: true });
  fs.writeFileSync(executable, "fixture");

  const tooling = core.resolveCliTooling({
    cliCommand: "",
    cliArgs: [],
    extensionPath: root,
    platform: "darwin",
    arch: "arm64"
  });

  assert.deepEqual(tooling, {
    command: executable,
    args: [],
    source: "bundled"
  });
});

test("resolveCliTooling uses shosei on PATH when no bundle is available", () => {
  const tooling = core.resolveCliTooling({
    cliCommand: "",
    cliArgs: [],
    extensionPath: tempDir("missing-bundle"),
    platform: "linux",
    arch: "arm64"
  });

  assert.deepEqual(tooling, {
    command: "shosei",
    args: [],
    source: "path"
  });
});

test("resolveCliTooling keeps args-only settings compatible with shosei on PATH", () => {
  const tooling = core.resolveCliTooling({
    cliCommand: "",
    cliArgs: ["--flag"],
    extensionPath: tempDir("args-only")
  });

  assert.deepEqual(tooling, {
    command: "shosei",
    args: ["--flag"],
    source: "configured"
  });
});

test("bundledCliExecutableName maps supported VS Code targets", () => {
  assert.equal(core.bundledCliExecutableName("darwin", "arm64"), "shosei");
  assert.equal(core.bundledCliExecutableName("darwin", "x64"), "shosei");
  assert.equal(core.bundledCliExecutableName("linux", "x64"), "shosei");
  assert.equal(core.bundledCliExecutableName("win32", "x64"), "shosei.exe");
  assert.equal(core.bundledCliExecutableName("linux", "arm64"), null);
  assert.equal(core.bundledCliExecutableName("win32", "arm64"), null);
});

test("extractReportPath picks the final report path from command output", () => {
  const output = [
    "validation completed for vol-01 with outputs: kindle, issues: 2, report: /tmp/a.json",
    "page check completed for vol-01 with 4 page(s), issues: 1, report: /tmp/b.json"
  ].join("\n");

  assert.equal(core.extractReportPath(output), "/tmp/b.json");
});

test("readReport returns parsed JSON payload including manuscript stats", () => {
  const root = tempDir("report");
  const reportPath = path.join(root, "validate.json");
  fs.writeFileSync(
    reportPath,
    JSON.stringify({
      issues: [{ severity: "warning", cause: "example", remedy: "fix" }],
      manuscript_stats: {
        total_characters: 1234,
        chapter_characters: 1200,
        frontmatter_characters: 20,
        backmatter_characters: 14
      }
    })
  );

  const report = core.readReport(reportPath);
  assert.equal(report.manuscript_stats.total_characters, 1234);
  assert.equal(core.readIssuesFromReport(reportPath).length, 1);
});

test("classifyCommandResult treats exit code 1 with stderr as fatal", () => {
  const outcome = core.classifyCommandResult(
    {
      code: 1,
      stdout: "",
      stderr: "error: could not find book.yml"
    },
    {
      acceptedExitCodes: [0, 1],
      fallbackMessage: "validate completed"
    }
  );

  assert.deepEqual(outcome, {
    level: "error",
    message: "error: could not find book.yml"
  });
});

test("classifyCommandResult treats exit code 1 without stderr as warning", () => {
  const outcome = core.classifyCommandResult(
    {
      code: 1,
      stdout: "validation completed for vol-01 with outputs: kindle, issues: 2",
      stderr: ""
    },
    {
      acceptedExitCodes: [0, 1],
      fallbackMessage: "validate completed"
    }
  );

  assert.deepEqual(outcome, {
    level: "warning",
    message: "validation completed for vol-01 with outputs: kindle, issues: 2"
  });
});
