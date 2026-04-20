"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const vscode = require("vscode");

const EXTENSION_ID = "koizumikento.shosei-vscode";
const COMMAND_IDS = [
  "shosei.explain",
  "shosei.validate",
  "shosei.doctor",
  "shosei.previewWatch",
  "shosei.selectBook",
  "shosei.storyRevealScene",
  "shosei.init"
];

const ALL_TESTS = [
  ["registers the published command surface", testRegisteredCommands],
  ["loads validate reports into Problems", testValidateDiagnostics],
  ["starts preview watch as a process task", testPreviewWatchTask],
  ["reuses the selected series book for later commands", testSeriesBookSelection],
  ["reveals the matching scene entry in scenes.yml", testRevealScene],
  ["maps guided init answers to CLI flags", testGuidedInit]
];

const PACKAGE_SCOPE_TESTS = new Set([
  "registers the published command surface",
  "loads validate reports into Problems",
  "starts preview watch as a process task"
]);

async function run() {
  const extension = vscode.extensions.getExtension(EXTENSION_ID);
  assert.ok(extension, `Extension ${EXTENSION_ID} is installed`);
  await extension.activate();

  const tests =
    process.env.SHOSEI_HOST_TEST_SCOPE === "package"
      ? ALL_TESTS.filter(([name]) => PACKAGE_SCOPE_TESTS.has(name))
      : ALL_TESTS;

  for (const [name, testFn] of tests) {
    console.log(`BEGIN ${name}`);
    await testFn();
    console.log(`OK ${name}`);
  }
}

function makeTempDir(name) {
  return fs.mkdtempSync(path.join(os.tmpdir(), `shosei-vscode-${name}-`));
}

function normalizeFsPath(filePath) {
  return path.normalize(filePath).toLowerCase();
}

async function clearEditors() {
  await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  await sleep(50);
}

async function openFile(filePath) {
  const document = await vscode.workspace.openTextDocument(vscode.Uri.file(filePath));
  await vscode.window.showTextDocument(document, { preview: false });
  return document;
}

async function configureStubCli() {
  const supportRoot = makeTempDir("cli");
  const stubPath = path.join(supportRoot, "stub-shosei.js");
  const logPath = path.join(supportRoot, "stub-log.jsonl");
  fs.writeFileSync(stubPath, buildStubCliScript(), "utf8");
  fs.chmodSync(stubPath, 0o755);

  const config = vscode.workspace.getConfiguration("shosei");
  await config.update("cli.command", process.execPath, vscode.ConfigurationTarget.Workspace);
  await config.update("cli.args", [stubPath], vscode.ConfigurationTarget.Workspace);
  return { logPath, dispose: () => fs.rmSync(supportRoot, { recursive: true, force: true }) };
}

function buildStubCliScript() {
  return `#!/usr/bin/env node
"use strict";
const fs = require("node:fs");
const path = require("node:path");

const args = process.argv.slice(2);
const logPath = process.env.SHOSEI_STUB_LOG;

function findOption(flag) {
  const index = args.indexOf(flag);
  if (index === -1 || index + 1 >= args.length) {
    return null;
  }
  return args[index + 1];
}

function appendLog(entry) {
  if (!logPath) {
    return;
  }
  fs.mkdirSync(path.dirname(logPath), { recursive: true });
  fs.appendFileSync(logPath, JSON.stringify(entry) + "\\n");
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2));
}

const repoRoot = findOption("--path") || process.cwd();
const bookId = findOption("--book");
appendLog({ args, cwd: process.cwd(), repoRoot, bookId });

if (args[0] === "validate") {
  const reportPath = path.join(repoRoot, "dist", "reports", "default-validate.json");
  writeJson(reportPath, {
    issues: [
      {
        severity: "warning",
        target: "manuscript",
        cause: "Stub validate warning",
        remedy: "Check the chapter content",
        location: { path: "manuscript/01.md", line: 2 }
      }
    ],
    manuscript_stats: {
      total_characters: 10,
      chapter_characters: 8,
      frontmatter_characters: 1,
      backmatter_characters: 1
    }
  });
  console.log(
    "validation completed for " +
      (bookId || "default") +
      " with outputs: kindle, issues: 1, manuscript characters: 10, report: " +
      reportPath
  );
  process.exit(1);
}

if (args[0] === "doctor") {
  if (args.includes("--json")) {
    console.log(JSON.stringify({ environment: { os: process.platform }, required_tools: [], optional_tools: [] }));
  } else {
    console.log("doctor completed");
  }
  process.exit(0);
}

if (args[0] === "init") {
  const targetRoot = args[1];
  const configName = args.includes("--repo-mode") && findOption("--repo-mode") === "series"
    ? "series.yml"
    : "book.yml";
  fs.mkdirSync(targetRoot, { recursive: true });
  fs.writeFileSync(path.join(targetRoot, configName), "stub: true\\n");
  console.log("init completed");
  process.exit(0);
}

if (args[0] === "explain") {
  console.log("explain completed for " + (bookId || "default"));
  process.exit(0);
}

if (args[0] === "preview" && args.includes("--watch")) {
  console.log("preview watch started");
  const interval = setInterval(() => {}, 1000);
  process.on("SIGTERM", () => {
    clearInterval(interval);
    process.exit(0);
  });
  process.on("SIGINT", () => {
    clearInterval(interval);
    process.exit(0);
  });
} else {
  console.log("stub command completed");
  process.exit(0);
}
`;
}

function readLog(logPath) {
  if (!fs.existsSync(logPath)) {
    return [];
  }
  return fs
    .readFileSync(logPath, "utf8")
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

async function withStubCli(testFn) {
  const config = vscode.workspace.getConfiguration("shosei");
  const previousCommand = config.get("cli.command");
  const previousArgs = config.get("cli.args");
  const stub = await configureStubCli();
  const previousLogPath = process.env.SHOSEI_STUB_LOG;
  process.env.SHOSEI_STUB_LOG = stub.logPath;
  try {
    await testFn(stub.logPath);
  } finally {
    if (previousLogPath === undefined) {
      delete process.env.SHOSEI_STUB_LOG;
    } else {
      process.env.SHOSEI_STUB_LOG = previousLogPath;
    }
    await config.update("cli.command", previousCommand, vscode.ConfigurationTarget.Workspace);
    await config.update("cli.args", previousArgs, vscode.ConfigurationTarget.Workspace);
    stub.dispose();
  }
}

async function waitFor(predicate, description, timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const value = await predicate();
    if (value) {
      return value;
    }
    await sleep(50);
  }
  throw new Error(`Timed out waiting for ${description}`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function patch(object, property, replacement) {
  const original = object[property];
  object[property] = replacement;
  return () => {
    object[property] = original;
  };
}

function pickByLabel(items, label) {
  const picked = items.find((item) => item.label === label);
  assert.ok(picked, `Quick pick item ${label} should exist`);
  return picked;
}

function prepareSingleBookRepo() {
  const repoRoot = makeTempDir("single-book");
  fs.mkdirSync(path.join(repoRoot, "manuscript"), { recursive: true });
  fs.writeFileSync(path.join(repoRoot, "book.yml"), "book:\n  title: Host Test\n");
  fs.writeFileSync(path.join(repoRoot, "manuscript", "01.md"), "# Chapter\n\nText\n");
  return repoRoot;
}

function prepareSeriesRepo() {
  const repoRoot = makeTempDir("series");
  const bookRoot = path.join(repoRoot, "books", "vol-02");
  fs.mkdirSync(path.join(bookRoot, "manuscript"), { recursive: true });
  fs.writeFileSync(path.join(repoRoot, "series.yml"), "series:\n  id: host-test\n");
  fs.writeFileSync(path.join(bookRoot, "book.yml"), "book:\n  title: Volume 2\n");
  fs.writeFileSync(path.join(bookRoot, "manuscript", "01.md"), "# Volume 2\n");
  return repoRoot;
}

function prepareStoryRepo() {
  const repoRoot = prepareSingleBookRepo();
  fs.mkdirSync(path.join(repoRoot, "story", "scene-notes"), { recursive: true });
  fs.writeFileSync(
    path.join(repoRoot, "story", "scenes.yml"),
    [
      "scenes:",
      "  - file: story/scene-notes/01-opening.md",
      "    title: Opening",
      "  - file: story/scene-notes/02-turn.md",
      "    title: Turn"
    ].join("\\n")
  );
  fs.writeFileSync(
    path.join(repoRoot, "story", "scene-notes", "01-opening.md"),
    "# Opening\\n"
  );
  return repoRoot;
}

async function testRegisteredCommands() {
  const commands = await vscode.commands.getCommands(true);
  for (const commandId of COMMAND_IDS) {
    assert.ok(commands.includes(commandId), `${commandId} should be registered`);
  }
}

async function testValidateDiagnostics() {
  await withStubCli(async () => {
    const repoRoot = prepareSingleBookRepo();
    const manuscriptPath = path.join(repoRoot, "manuscript", "01.md");
    await clearEditors();
    await openFile(manuscriptPath);

    await vscode.commands.executeCommand("shosei.validate");

    const diagnostics = await waitFor(
      () => vscode.languages.getDiagnostics(vscode.Uri.file(manuscriptPath)),
      "validate diagnostics"
    );
    assert.equal(diagnostics.length, 1);
    assert.equal(diagnostics[0].source, "shosei validate");
    assert.match(diagnostics[0].message, /Stub validate warning/);
  });
}

async function testPreviewWatchTask() {
  await withStubCli(async () => {
    const repoRoot = prepareSingleBookRepo();
    const manuscriptPath = path.join(repoRoot, "manuscript", "01.md");
    await clearEditors();
    await openFile(manuscriptPath);

    const taskExecutionPromise = new Promise((resolve) => {
      const disposable = vscode.tasks.onDidStartTask((event) => {
        if (event.execution.task.name === "Shosei: Preview (Watch)") {
          disposable.dispose();
          resolve(event.execution);
        }
      });
    });

    await vscode.commands.executeCommand("shosei.previewWatch");

    const taskExecution = await Promise.race([
      taskExecutionPromise,
      sleep(5000).then(() => {
        throw new Error("Timed out waiting for preview watch task start");
      })
    ]);
    const task = taskExecution.task;
    const taskEndedPromise = new Promise((resolve) => {
      const disposable = vscode.tasks.onDidEndTaskProcess((event) => {
        if (event.execution === taskExecution) {
          disposable.dispose();
          resolve();
        }
      });
    });

    try {
      assert.equal(task.name, "Shosei: Preview (Watch)");
      assert.ok(task.execution instanceof vscode.ProcessExecution);
      const previewIndex = task.execution.args.indexOf("preview");
      assert.notEqual(previewIndex, -1, "task args should include preview");
      assert.equal(task.execution.args[previewIndex + 1], "--watch");
      assert.ok(task.execution.args.includes("--path"));
    } finally {
      taskExecution.terminate();
      await Promise.race([
        taskEndedPromise,
        sleep(5000).then(() => {
          throw new Error("Timed out waiting for preview watch task shutdown");
        })
      ]);
    }
  });
}

async function testSeriesBookSelection() {
  await withStubCli(async (logPath) => {
    const repoRoot = prepareSeriesRepo();
    const manuscriptPath = path.join(repoRoot, "books", "vol-02", "manuscript", "01.md");
    await clearEditors();
    await openFile(path.join(repoRoot, "series.yml"));

    const restore = patch(vscode.window, "showQuickPick", async (items, options) => {
      if (options?.title === "Select a series book") {
        return pickByLabel(items, "vol-02");
      }
      throw new Error(`Unexpected quick pick: ${options?.title || "unknown"}`);
    });

    try {
      await vscode.commands.executeCommand("shosei.selectBook");
    } finally {
      restore();
    }

    await openFile(manuscriptPath);
    await vscode.commands.executeCommand("shosei.explain");

    const entries = readLog(logPath);
    assert.ok(entries.length >= 1, "CLI log should contain explain invocation");
    const explainEntry = entries[entries.length - 1];
    assert.ok(explainEntry.args.includes("--book"));
    assert.ok(explainEntry.args.includes("vol-02"));
  });
}

async function testRevealScene() {
  const repoRoot = prepareStoryRepo();
  const scenesPath = path.join(repoRoot, "story", "scenes.yml");
  await clearEditors();

  await vscode.commands.executeCommand("shosei.storyRevealScene", {
    storyRepoRoot: repoRoot,
    storySceneFile: "story/scene-notes/01-opening.md",
    storyScenesPath: "story/scenes.yml"
  });

  await waitFor(
    () => {
      const activePath = vscode.window.activeTextEditor?.document?.uri.fsPath;
      return (
        typeof activePath === "string" &&
        normalizeFsPath(activePath) === normalizeFsPath(scenesPath)
      );
    },
    "scene index editor"
  );
  assert.match(vscode.window.activeTextEditor.document.getText(), /story\/scene-notes\/01-opening\.md/);
}

async function testGuidedInit() {
  await withStubCli(async (logPath) => {
    const repoRoot = makeTempDir("init");
    await clearEditors();

    const quickPickRestore = patch(vscode.window, "showQuickPick", async (items, options) => {
      switch (options?.title) {
        case "Initialize shosei project":
          return pickByLabel(items, "Choose folder...");
        case "Project template":
          return pickByLabel(items, "Paper");
        case "Paper profile":
          return pickByLabel(items, "Conference Preprint");
        case "Repository mode":
          return pickByLabel(items, "Single Book");
        case "Output preset":
          return pickByLabel(items, "Print");
        case "Introduction scaffold":
          return pickByLabel(items, "Skip introduction");
        case "Afterword scaffold":
          return pickByLabel(items, "Skip afterword");
        case "Run doctor after init":
          return pickByLabel(items, "Skip doctor");
        default:
          throw new Error(`Unexpected quick pick: ${options?.title || "unknown"}`);
      }
    });

    const openDialogRestore = patch(vscode.window, "showOpenDialog", async () => [
      vscode.Uri.file(repoRoot)
    ]);
    const answers = new Map([
      ["Book title", "Host Test Book"],
      ["Author", "Test Author"],
      ["Language", "en"]
    ]);
    const inputRestore = patch(vscode.window, "showInputBox", async (options) => {
      if (!answers.has(options?.title)) {
        throw new Error(`Unexpected input box: ${options?.title || "unknown"}`);
      }
      return answers.get(options.title);
    });

    try {
      await vscode.commands.executeCommand("shosei.init");
    } finally {
      inputRestore();
      openDialogRestore();
      quickPickRestore();
    }

    const entries = readLog(logPath);
    assert.equal(entries.length, 1);
    assert.equal(entries[0].args[0], "init");
    assert.equal(normalizeFsPath(entries[0].args[1]), normalizeFsPath(repoRoot));
    assert.deepEqual(entries[0].args.slice(2), [
      "--non-interactive",
      "--config-template",
      "paper",
      "--config-profile",
      "conference-preprint",
      "--repo-mode",
      "single-book",
      "--title",
      "Host Test Book",
      "--author",
      "Test Author",
      "--language",
      "en",
      "--output-preset",
      "print"
    ]);
    assert.ok(fs.existsSync(path.join(repoRoot, "book.yml")));
  });
}

module.exports = { run };
