const cp = require("child_process");
const fs = require("fs");
const path = require("path");

const core = require("./src/core");
const { ShoseiViewProvider } = require("./src/view");

const DEFAULT_INIT_SERIES_BOOK_ID = "vol-01";
const SERIES_BOOK_SELECTIONS_KEY = "shosei.series.selectedBooks";

function activate(context) {
  const vscode = require("vscode");
  const output = vscode.window.createOutputChannel("Shosei");
  const validateDiagnostics = vscode.languages.createDiagnosticCollection("shosei-validate");
  const pageCheckDiagnostics = vscode.languages.createDiagnosticCollection("shosei-page-check");
  const referenceCheckDiagnostics = vscode.languages.createDiagnosticCollection(
    "shosei-reference-check"
  );
  const referenceDriftDiagnostics = vscode.languages.createDiagnosticCollection(
    "shosei-reference-drift"
  );
  const storyCheckDiagnostics = vscode.languages.createDiagnosticCollection("shosei-story-check");
  const storyDriftDiagnostics = vscode.languages.createDiagnosticCollection("shosei-story-drift");
  const viewProvider = new ShoseiViewProvider(vscode, {
    getSnapshot: () => resolveViewSnapshot(vscode, context)
  });
  const treeView = vscode.window.createTreeView("shosei.sidebar", {
    treeDataProvider: viewProvider,
    showCollapseAll: false
  });

  context.subscriptions.push(
    output,
    validateDiagnostics,
    pageCheckDiagnostics,
    referenceCheckDiagnostics,
    referenceDriftDiagnostics,
    storyCheckDiagnostics,
    storyDriftDiagnostics,
    treeView
  );
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(() => viewProvider.refresh()),
    vscode.workspace.onDidChangeWorkspaceFolders(() => viewProvider.refresh()),
    vscode.workspace.onDidSaveTextDocument(() => viewProvider.refresh())
  );

  registerCommand(context, "shosei.init", () =>
    runInitCommand(vscode, output, context, viewProvider)
  );

  registerCommand(context, "shosei.chapterAdd", (item) =>
    runChapterAddCommand(vscode, output, context, viewProvider, item)
  );
  registerCommand(context, "shosei.chapterMove", (item) =>
    runChapterMoveCommand(vscode, output, context, viewProvider, item)
  );
  registerCommand(context, "shosei.chapterRemove", (item) =>
    runChapterRemoveCommand(vscode, output, context, viewProvider, item)
  );
  registerCommand(context, "shosei.chapterRenumber", () =>
    runChapterRenumberCommand(vscode, output, context, viewProvider)
  );

  registerCommand(context, "shosei.refreshView", () => viewProvider.refresh());

  registerCommand(context, "shosei.selectBook", async () => {
    const selection = await promptSeriesBookSelection(vscode, context);
    if (selection === undefined) {
      return;
    }
    viewProvider.refresh();
  });

  registerCommand(context, "shosei.explain", () =>
    runTextCommand(vscode, output, {
      title: "explain",
      commandParts: ["explain"],
      requireBook: true,
      extensionContext: context
    })
  );

  registerCommand(context, "shosei.validate", async () => {
    validateDiagnostics.clear();
    await runManagedCommand(vscode, output, {
      title: "validate",
      commandParts: ["validate"],
      requireBook: true,
      extensionContext: context,
      acceptedExitCodes: [0, 1],
      onComplete: (result, resolved) =>
        applyDiagnosticsFromReport(
          vscode,
          output,
          validateDiagnostics,
          "shosei validate",
          result,
          resolved
        )
    });
  });

  registerCommand(context, "shosei.build", () =>
    runManagedCommand(vscode, output, {
      title: "build",
      commandParts: ["build"],
      requireBook: true,
      extensionContext: context
    })
  );

  registerCommand(context, "shosei.preview", () =>
    runManagedCommand(vscode, output, {
      title: "preview",
      commandParts: ["preview"],
      requireBook: true,
      extensionContext: context
    })
  );

  registerCommand(context, "shosei.previewWatch", () =>
    runPreviewWatchTask(vscode, output, context)
  );

  registerCommand(context, "shosei.referenceScaffold", () =>
    runReferenceScaffoldCommand(vscode, output, context, viewProvider)
  );
  registerCommand(context, "shosei.referenceMap", () =>
    runReferenceMapCommand(vscode, output, context, viewProvider)
  );
  registerCommand(context, "shosei.referenceCheck", async () => {
    referenceCheckDiagnostics.clear();
    await runReferenceCheckCommand(
      vscode,
      output,
      context,
      referenceCheckDiagnostics,
      viewProvider
    );
  });
  registerCommand(context, "shosei.referenceDrift", async () => {
    referenceDriftDiagnostics.clear();
    await runReferenceDriftCommand(vscode, output, context, referenceDriftDiagnostics);
  });
  registerCommand(context, "shosei.referenceSync", () =>
    runReferenceSyncCommand(vscode, output, context, viewProvider, referenceDriftDiagnostics)
  );
  registerCommand(context, "shosei.storyScaffold", () =>
    runStoryScaffoldCommand(vscode, output, context, viewProvider)
  );
  registerCommand(context, "shosei.storySeed", () =>
    runStorySeedCommand(vscode, output, context, viewProvider)
  );
  registerCommand(context, "shosei.storyMap", () =>
    runStoryMapCommand(vscode, output, context, viewProvider)
  );
  registerCommand(context, "shosei.storyRevealScene", (item) =>
    runStoryRevealSceneCommand(vscode, item)
  );
  registerCommand(context, "shosei.storyCheck", async () => {
    storyCheckDiagnostics.clear();
    await runStoryCheckCommand(vscode, output, context, storyCheckDiagnostics, viewProvider);
  });
  registerCommand(context, "shosei.storyDrift", async () => {
    storyDriftDiagnostics.clear();
    await runStoryDriftCommand(vscode, output, context, storyDriftDiagnostics);
  });
  registerCommand(context, "shosei.storySync", () =>
    runStorySyncCommand(vscode, output, context, viewProvider, storyDriftDiagnostics)
  );

  registerCommand(context, "shosei.doctor", () =>
    runTextCommand(vscode, output, {
      title: "doctor",
      commandParts: ["doctor"],
      requireBook: false,
      includePath: false,
      allowOutsideRepo: true,
      extensionContext: context
    })
  );

  registerCommand(context, "shosei.pageCheck", async () => {
    pageCheckDiagnostics.clear();
    await runManagedCommand(vscode, output, {
      title: "page check",
      commandParts: ["page", "check"],
      requireBook: true,
      extensionContext: context,
      acceptedExitCodes: [0, 1],
      onComplete: (result, resolved) =>
        applyDiagnosticsFromReport(
          vscode,
          output,
          pageCheckDiagnostics,
          "shosei page check",
          result,
          resolved
        )
    });
  });

  registerCommand(context, "shosei.seriesSync", () =>
    runManagedCommand(vscode, output, {
      title: "series sync",
      commandParts: ["series", "sync"],
      requireBook: false,
      requireSeriesRepo: true,
      extensionContext: context
    })
  );
}

function deactivate() {}

function registerCommand(context, name, handler) {
  const vscode = require("vscode");
  context.subscriptions.push(vscode.commands.registerCommand(name, handler));
}

async function runTextCommand(vscode, output, descriptor) {
  const resolved = await resolveExecutionContext(vscode, descriptor);
  if (!resolved) {
    return;
  }

  await runTextCommandWithResolved(vscode, output, descriptor, resolved);
}

async function runTextCommandWithResolved(vscode, output, descriptor, resolved) {
  if (!resolved) {
    return null;
  }

  const result = await runProcess(vscode, output, descriptor.title, resolved, descriptor);
  if (!result) {
    return null;
  }

  const contents = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
  if (!contents) {
    return result;
  }

  const document = await vscode.workspace.openTextDocument({
    language: "plaintext",
    content: contents
  });
  await vscode.window.showTextDocument(document, { preview: false });
  return result;
}

async function runManagedCommand(vscode, output, descriptor) {
  const resolved = await resolveExecutionContext(vscode, descriptor);
  if (!resolved) {
    return;
  }

  return runManagedCommandWithResolved(vscode, output, descriptor, resolved);
}

async function runManagedCommandWithResolved(vscode, output, descriptor, resolved) {
  if (!resolved) {
    return null;
  }

  const result = await runProcess(vscode, output, descriptor.title, resolved, descriptor);
  if (!result) {
    return null;
  }

  const outcome = core.classifyCommandResult(result, {
    acceptedExitCodes: descriptor.acceptedExitCodes || [0],
    fallbackMessage: `${descriptor.title} completed`
  });
  if (outcome.level === "error") {
    vscode.window.showErrorMessage(outcome.message);
    return null;
  }

  if (typeof descriptor.onComplete === "function") {
    await descriptor.onComplete(result, resolved);
  }

  if (outcome.level === "warning") {
    vscode.window.showWarningMessage(outcome.message);
  } else {
    vscode.window.showInformationMessage(outcome.message);
  }

  return result;
}

async function runPreviewWatchTask(vscode, output, extensionContext) {
  const descriptor = {
    title: "preview watch",
    commandParts: ["preview", "--watch"],
    requireBook: true,
    extensionContext
  };
  const resolved = await resolveExecutionContext(vscode, descriptor);
  if (!resolved) {
    return;
  }

  const invocation = buildInvocation(vscode, resolved, descriptor);
  if (!invocation) {
    return;
  }

  output.show(true);
  output.appendLine(`[shosei] starting preview watch in terminal`);
  output.appendLine(`[shosei] command: ${invocation.command} ${invocation.args.join(" ")}`);

  const execution = new vscode.ProcessExecution(invocation.command, invocation.args, {
    cwd: invocation.cwd
  });
  const workspaceFolder = getWorkspaceFolder(vscode, resolved.repoRoot);
  const task = new vscode.Task(
    { type: "process", task: "shosei.previewWatch" },
    workspaceFolder || vscode.TaskScope.Workspace,
    "Shosei: Preview (Watch)",
    "shosei",
    execution
  );

  task.presentationOptions = {
    reveal: vscode.TaskRevealKind.Always,
    panel: vscode.TaskPanelKind.Dedicated,
    focus: false,
    clear: false
  };

  await vscode.tasks.executeTask(task);
}

async function resolveExecutionContext(vscode, descriptor) {
  if (descriptor.allowOutsideRepo) {
    const cwd = await pickStartPath(vscode, { promptForWorkspace: true });
    return {
      repoRoot: toWorkingDirectory(cwd) || process.cwd(),
      mode: null,
      bookId: null
    };
  }

  const startPath = await pickStartPath(vscode, { promptForWorkspace: true });
  if (!startPath) {
    vscode.window.showErrorMessage("Open a workspace folder or file before running shosei commands.");
    return null;
  }

  const repo = core.findRepoRoot(startPath);
  if (!repo) {
    vscode.window.showErrorMessage("Could not find book.yml or series.yml from the current workspace context.");
    return null;
  }

  if (descriptor.requireSeriesRepo && repo.mode !== "series") {
    vscode.window.showErrorMessage("This command requires a series repository.");
    return null;
  }

  let bookId = null;
  if (repo.mode === "series" && descriptor.requireBook) {
    bookId = await resolveSeriesBookId(
      vscode,
      descriptor.extensionContext,
      repo.repoRoot,
      startPath
    );
    if (!bookId) {
      return null;
    }
  }

  return {
    repoRoot: repo.repoRoot,
    mode: repo.mode,
    bookId
  };
}

async function resolveSeriesBookId(vscode, extensionContext, repoRoot, startPath) {
  const pinned = getStoredSeriesBookSelection(extensionContext, repoRoot);
  if (pinned) {
    return pinned;
  }

  const inferred = core.inferSeriesBookId(repoRoot, startPath);
  if (inferred) {
    return inferred;
  }

  const configured = vscode.workspace
    .getConfiguration("shosei")
    .get("series.defaultBookId");
  if (typeof configured === "string" && configured.trim()) {
    return configured.trim();
  }

  return promptForSeriesBookId(vscode, extensionContext, repoRoot);
}

function buildInvocation(vscode, resolved, descriptor) {
  const config = vscode.workspace.getConfiguration("shosei");
  const tooling = core.resolveCliTooling({
    cliCommand: config.get("cli.command"),
    cliArgs: config.get("cli.args"),
    extensionPath: descriptor.extensionContext?.extensionPath,
    enableDevelopmentFallback:
      descriptor.extensionContext?.extensionMode === vscode.ExtensionMode.Development
  });

  return core.buildCliInvocation({
    cliCommand: tooling.command,
    cliArgs: tooling.args,
    commandParts: descriptor.commandParts,
    bookId: resolved.bookId,
    repoRoot: resolved.repoRoot,
    cwd: resolved.cwd || resolved.repoRoot,
    includePath: descriptor.includePath !== false
  });
}

async function runProcess(vscode, output, title, resolved, descriptor) {
  const invocation = buildInvocation(vscode, resolved, descriptor);
  const acceptedExitCodes = descriptor.acceptedExitCodes || [0];

  output.show(true);
  output.appendLine(`[shosei] ${title}`);
  output.appendLine(`[shosei] cwd: ${invocation.cwd}`);
  output.appendLine(`[shosei] command: ${invocation.command} ${invocation.args.join(" ")}`);

  try {
    const result = await spawnProcess(invocation, output);
    if (!acceptedExitCodes.includes(result.code)) {
      const detail = result.stderr.trim() || result.stdout.trim() || `${title} failed`;
      vscode.window.showErrorMessage(detail);
      return null;
    }
    return result;
  } catch (error) {
    vscode.window.showErrorMessage(error.message);
    output.appendLine(`[shosei] error: ${error.message}`);
    return null;
  }
}

function spawnProcess(invocation, output) {
  return new Promise((resolve, reject) => {
    const child = cp.spawn(invocation.command, invocation.args, {
      cwd: invocation.cwd,
      env: process.env
    });
    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk) => {
      const text = chunk.toString();
      stdout += text;
      output.append(text);
    });

    child.stderr.on("data", (chunk) => {
      const text = chunk.toString();
      stderr += text;
      output.append(text);
    });

    child.on("error", (error) => {
      reject(new Error(renderSpawnError(invocation.command, error)));
    });

    child.on("close", (code) => {
      resolve({
        code: typeof code === "number" ? code : 1,
        stdout,
        stderr
      });
    });
  });
}

function spawnProcessQuiet(invocation) {
  return new Promise((resolve, reject) => {
    const child = cp.spawn(invocation.command, invocation.args, {
      cwd: invocation.cwd,
      env: process.env
    });
    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });

    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });

    child.on("error", (error) => {
      reject(new Error(renderSpawnError(invocation.command, error)));
    });

    child.on("close", (code) => {
      resolve({
        code: typeof code === "number" ? code : 1,
        stdout,
        stderr
      });
    });
  });
}

async function applyDiagnosticsFromReport(vscode, output, collection, source, result, resolved) {
  const reportPath = core.extractReportPath([result.stdout, result.stderr].join("\n"));
  if (!reportPath) {
    output.appendLine(`[shosei] ${source}: report path not found in command output`);
    return;
  }

  const absolutePath = core.toAbsolutePath(resolved.repoRoot, reportPath);
  let issues;
  try {
    issues = core.readIssuesFromReport(absolutePath);
  } catch (error) {
    output.appendLine(`[shosei] ${source}: failed to read report ${absolutePath}: ${error.message}`);
    return;
  }

  const perFile = new Map();
  for (const issue of issues) {
    const diagnosticLocation = resolveDiagnosticLocation(resolved.repoRoot, issue);
    if (!diagnosticLocation) {
      continue;
    }
    const uri = vscode.Uri.file(path.normalize(diagnosticLocation.filePath)).toString();
    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(diagnosticLocation.line, 0, diagnosticLocation.line, 0),
      [issue.cause, issue.remedy].filter(Boolean).join("\n"),
      issue.severity === "error"
        ? vscode.DiagnosticSeverity.Error
        : vscode.DiagnosticSeverity.Warning
    );
    diagnostic.source = source;
    diagnostic.code = issue.target || source;

    const entry = perFile.get(uri) || [];
    entry.push(diagnostic);
    perFile.set(uri, entry);
  }

  collection.clear();
  for (const [uri, diagnostics] of perFile.entries()) {
    collection.set(vscode.Uri.parse(uri), diagnostics);
  }

  output.appendLine(
    `[shosei] ${source}: loaded ${issues.length} issue(s) from ${absolutePath}`
  );
}

function resolveDiagnosticLocation(repoRoot, issue) {
  if (!issue || typeof issue !== "object") {
    return null;
  }

  const location =
    issue.location && typeof issue.location === "object" ? issue.location : null;
  const candidatePath =
    typeof location?.path === "string" && location.path.trim() ? location.path : null;
  if (!candidatePath) {
    return null;
  }

  const line =
    Number.isFinite(location.line) && location.line > 0 ? Math.trunc(location.line) - 1 : 0;

  return {
    filePath: core.toAbsolutePath(repoRoot, candidatePath),
    line
  };
}

async function pickStartPath(vscode, options = {}) {
  const active = vscode.window.activeTextEditor?.document?.uri;
  if (active && active.scheme === "file") {
    return active.fsPath;
  }

  const folders = vscode.workspace.workspaceFolders || [];
  if (folders.length === 1) {
    return folders[0].uri.fsPath;
  }
  if (folders.length > 1 && options.promptForWorkspace !== false) {
    const picked = await vscode.window.showWorkspaceFolderPick({
      placeHolder: "Select the workspace folder to use for shosei"
    });
    return picked ? picked.uri.fsPath : null;
  }

  return null;
}

async function resolveViewSnapshot(vscode, extensionContext) {
  const startPath = await pickStartPath(vscode, { promptForWorkspace: false });
  const doctorResolved = {
    repoRoot: null,
    cwd: toWorkingDirectory(startPath) || process.cwd(),
    mode: null,
    bookId: null
  };
  const doctorResult = await loadDoctorSnapshot(vscode, extensionContext, doctorResolved);
  if (!startPath) {
    return {
      repoRoot: null,
      mode: null,
      bookId: null,
      bookSource: null,
      explain: null,
      configError: null,
      doctor: doctorResult.doctor,
      doctorError: doctorResult.error
    };
  }

  const repo = core.findRepoRoot(startPath);
  if (!repo) {
    return {
      repoRoot: null,
      mode: null,
      bookId: null,
      bookSource: null,
      explain: null,
      configError: null,
      doctor: doctorResult.doctor,
      doctorError: doctorResult.error
    };
  }

  const pinned = getStoredSeriesBookSelection(extensionContext, repo.repoRoot);
  const inferred = core.inferSeriesBookId(repo.repoRoot, startPath);
  const configured = vscode.workspace
    .getConfiguration("shosei")
    .get("series.defaultBookId");

  let bookId = null;
  let bookSource = null;
  if (repo.mode === "series") {
    if (pinned) {
      bookId = pinned;
      bookSource = "selected";
    } else if (inferred) {
      bookId = inferred;
      bookSource = "active file";
    } else if (typeof configured === "string" && configured.trim()) {
      bookId = configured.trim();
      bookSource = "setting";
    }
  }

  const snapshot = {
    repoRoot: repo.repoRoot,
    mode: repo.mode,
    bookId,
    bookSource,
    explain: null,
    configError: null,
    doctor: doctorResult.doctor,
    doctorError: doctorResult.error
  };

  if (repo.mode === "single-book" || bookId) {
    const explainResult = await loadExplainSnapshot(vscode, extensionContext, {
      repoRoot: repo.repoRoot,
      mode: repo.mode,
      bookId
    });
    snapshot.explain = explainResult.explain;
    snapshot.configError = explainResult.error;
  }

  return snapshot;
}

async function loadDoctorSnapshot(vscode, extensionContext, resolved) {
  const descriptor = {
    title: "doctor snapshot",
    commandParts: ["doctor", "--json"],
    extensionContext,
    includePath: false
  };
  const invocation = buildInvocation(vscode, resolved, descriptor);

  try {
    const result = await spawnProcessQuiet(invocation);
    if (result.code !== 0) {
      return {
        doctor: null,
        error: result.stderr.trim() || result.stdout.trim() || "Failed to load doctor status"
      };
    }

    return {
      doctor: JSON.parse(result.stdout),
      error: null
    };
  } catch (error) {
    return {
      doctor: null,
      error: error.message
    };
  }
}

async function loadExplainSnapshot(vscode, extensionContext, resolved) {
  const descriptor = {
    title: "explain snapshot",
    commandParts: ["explain", "--json"],
    requireBook: resolved.mode === "series",
    extensionContext
  };
  const invocation = buildInvocation(vscode, resolved, descriptor);

  try {
    const result = await spawnProcessQuiet(invocation);
    if (result.code !== 0) {
      return {
        explain: null,
        error: result.stderr.trim() || result.stdout.trim() || "Failed to load resolved config"
      };
    }

    return {
      explain: JSON.parse(result.stdout),
      error: null
    };
  } catch (error) {
    return {
      explain: null,
      error: error.message
    };
  }
}

async function runInitCommand(vscode, output, extensionContext, viewProvider) {
  const targetRoot = await promptInitTarget(vscode);
  if (!targetRoot) {
    return;
  }

  const initOptions = await promptInitOptions(vscode, targetRoot);
  if (!initOptions) {
    return;
  }

  const normalizedTarget = path.resolve(targetRoot);
  const initDescriptor = {
    title: "init",
    commandParts: core.buildInitCommandParts({
      path: normalizedTarget,
      configTemplate: initOptions.configTemplate,
      configProfile: initOptions.configProfile,
      repoMode: initOptions.repoMode,
      initialBookId: initOptions.initialBookId,
      title: initOptions.title,
      author: initOptions.author,
      language: initOptions.language,
      outputPreset: initOptions.outputPreset,
      includeIntroduction: initOptions.includeIntroduction,
      includeAfterword: initOptions.includeAfterword,
      force: initOptions.force
    }),
    extensionContext,
    includePath: false
  };
  const initResolved = {
    repoRoot: normalizedTarget,
    cwd: path.dirname(normalizedTarget),
    mode: null,
    bookId: null
  };

  const initResult = await runManagedCommandWithResolved(
    vscode,
    output,
    initDescriptor,
    initResolved
  );
  if (!initResult) {
    return;
  }

  if (initOptions.runDoctor) {
    await runTextCommandWithResolved(
      vscode,
      output,
      {
        title: "doctor",
        commandParts: ["doctor"],
        extensionContext,
        includePath: false
      },
      {
        repoRoot: normalizedTarget,
        cwd: normalizedTarget,
        mode: null,
        bookId: null
      }
    );
  }

  if (viewProvider) {
    viewProvider.refresh();
  }
}

async function runChapterAddCommand(vscode, output, extensionContext, viewProvider, item) {
  const prose = await resolveProseChapterContext(vscode, extensionContext);
  if (!prose) {
    return;
  }

  const chapterPath = await promptNewChapterPath(vscode, prose.explain);
  if (!chapterPath) {
    return;
  }

  const title = await vscode.window.showInputBox({
    title: "Chapter title",
    prompt: "Heading used when creating a new markdown stub",
    value: suggestChapterTitle(prose.explain),
    ignoreFocusOut: true,
    validateInput: (value) => (value.trim() ? null : "Title is required")
  });
  if (title === undefined) {
    return;
  }

  const placement = await promptChapterAddPlacement(
    vscode,
    prose.explain,
    extractChapterItemPath(item)
  );
  if (placement === undefined) {
    return;
  }

  const result = await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "chapter add",
      commandParts: buildChapterAddCommandParts({
        chapterPath,
        title: title.trim(),
        before: placement.before,
        after: placement.after
      }),
      extensionContext,
      requireBook: true
    },
    prose.resolved
  );
  if (!result) {
    return;
  }

  if (viewProvider) {
    viewProvider.refresh();
  }

  const uri = vscode.Uri.file(path.resolve(prose.resolved.repoRoot, chapterPath));
  const document = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(document, { preview: false });
}

async function runChapterMoveCommand(vscode, output, extensionContext, viewProvider, item) {
  const prose = await resolveProseChapterContext(vscode, extensionContext);
  if (!prose) {
    return;
  }

  const chapterPath =
    extractChapterItemPath(item) ||
    (await promptChapterSelection(vscode, prose.explain, {
      title: "Move chapter",
      placeHolder: "Select the chapter to move"
    }));
  if (!chapterPath) {
    return;
  }

  const placement = await promptChapterMovePlacement(vscode, prose.explain, chapterPath);
  if (!placement) {
    return;
  }

  const result = await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "chapter move",
      commandParts: buildChapterMoveCommandParts({
        chapterPath,
        before: placement.before,
        after: placement.after
      }),
      extensionContext,
      requireBook: true
    },
    prose.resolved
  );
  if (!result) {
    return;
  }

  if (viewProvider) {
    viewProvider.refresh();
  }
}

async function runChapterRemoveCommand(vscode, output, extensionContext, viewProvider, item) {
  const prose = await resolveProseChapterContext(vscode, extensionContext);
  if (!prose) {
    return;
  }

  const chapterPath =
    extractChapterItemPath(item) ||
    (await promptChapterSelection(vscode, prose.explain, {
      title: "Remove chapter",
      placeHolder: "Select the chapter to remove"
    }));
  if (!chapterPath) {
    return;
  }

  const deleteFile = await promptBooleanChoice(vscode, {
    title: "Remove chapter file",
    placeHolder: `Delete ${path.basename(chapterPath)} from disk as well?`,
    trueLabel: "Delete file too",
    falseLabel: "Keep file",
    defaultValue: false
  });
  if (deleteFile === undefined) {
    return;
  }

  const confirmed = await promptBooleanChoice(vscode, {
    title: "Confirm chapter removal",
    placeHolder: `Remove ${chapterPath} from manuscript.chapters?`,
    trueLabel: "Remove chapter",
    falseLabel: "Cancel",
    defaultValue: false
  });
  if (confirmed !== true) {
    return;
  }

  const result = await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "chapter remove",
      commandParts: buildChapterRemoveCommandParts({
        chapterPath,
        deleteFile
      }),
      extensionContext,
      requireBook: true
    },
    prose.resolved
  );
  if (!result) {
    return;
  }

  if (viewProvider) {
    viewProvider.refresh();
  }
}

async function runChapterRenumberCommand(vscode, output, extensionContext, viewProvider) {
  const prose = await resolveProseChapterContext(vscode, extensionContext);
  if (!prose) {
    return;
  }

  const startAt = await promptPositiveInteger(vscode, {
    title: "Renumber chapters",
    prompt: "First number to assign",
    value: "1"
  });
  if (startAt === undefined) {
    return;
  }

  const width = await promptPositiveInteger(vscode, {
    title: "Renumber width",
    prompt: "Zero-padding width for chapter prefixes",
    value: "2"
  });
  if (width === undefined) {
    return;
  }

  const dryRun = await promptBooleanChoice(vscode, {
    title: "Renumber mode",
    placeHolder: "Preview file renames before applying?",
    trueLabel: "Dry run",
    falseLabel: "Apply renumber",
    defaultValue: true
  });
  if (dryRun === undefined) {
    return;
  }

  const descriptor = {
    title: dryRun ? "chapter renumber dry-run" : "chapter renumber",
    commandParts: buildChapterRenumberCommandParts({
      startAt,
      width,
      dryRun
    }),
    extensionContext,
    requireBook: true
  };

  const result = dryRun
    ? await runTextCommandWithResolved(vscode, output, descriptor, prose.resolved)
    : await runManagedCommandWithResolved(vscode, output, descriptor, prose.resolved);
  if (!result) {
    return;
  }

  if (!dryRun && viewProvider) {
    viewProvider.refresh();
  }
}

async function runReferenceScaffoldCommand(vscode, output, extensionContext, viewProvider) {
  const referenceContext = await resolveReferenceScopeContext(vscode, extensionContext, {
    title: "reference scaffold"
  });
  if (!referenceContext) {
    return;
  }

  await runReferenceScaffoldWithResolved(
    vscode,
    output,
    extensionContext,
    referenceContext.resolved,
    referenceContext.shared,
    viewProvider
  );
}

async function runStoryScaffoldCommand(vscode, output, extensionContext, viewProvider) {
  const storyContext = await resolveStoryScaffoldContext(vscode, extensionContext, {
    title: "story scaffold"
  });
  if (!storyContext) {
    return;
  }

  await runStoryScaffoldWithResolved(
    vscode,
    output,
    extensionContext,
    storyContext.resolved,
    storyContext.shared,
    viewProvider
  );
}

async function runReferenceScaffoldWithResolved(
  vscode,
  output,
  extensionContext,
  resolved,
  shared,
  viewProvider
) {
  let force = false;
  if (fs.existsSync(referenceWorkspaceRoot(resolved, shared))) {
    const overwrite = await promptBooleanChoice(vscode, {
      title: "Reference scaffold mode",
      placeHolder: "Overwrite scaffold template files if they already exist?",
      trueLabel: "Overwrite templates",
      falseLabel: "Keep existing files",
      defaultValue: false
    });
    if (overwrite === undefined) {
      return;
    }
    force = overwrite;
  }

  const result = await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "reference scaffold",
      commandParts: buildReferenceScopedCommandParts("scaffold", {
        shared,
        force
      }),
      extensionContext
    },
    resolved
  );
  if (!result) {
    return null;
  }

  if (viewProvider) {
    viewProvider.refresh();
  }

  return result;
}

async function runStoryScaffoldWithResolved(
  vscode,
  output,
  extensionContext,
  resolved,
  shared,
  viewProvider
) {
  let force = false;
  if (fs.existsSync(storyWorkspaceRoot(resolved, shared))) {
    const overwrite = await promptBooleanChoice(vscode, {
      title: "Story scaffold mode",
      placeHolder: "Overwrite scaffold template files if they already exist?",
      trueLabel: "Overwrite templates",
      falseLabel: "Keep existing files",
      defaultValue: false
    });
    if (overwrite === undefined) {
      return;
    }
    force = overwrite;
  }

  const result = await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "story scaffold",
      commandParts: buildStoryScopedCommandParts("scaffold", {
        shared,
        force
      }),
      extensionContext
    },
    resolved
  );
  if (!result) {
    return null;
  }

  if (viewProvider) {
    viewProvider.refresh();
  }

  return result;
}

async function runReferenceMapCommand(vscode, output, extensionContext, viewProvider) {
  const referenceContext = await resolveReferenceScopeContext(vscode, extensionContext, {
    title: "reference map"
  });
  if (!referenceContext) {
    return;
  }

  const ready = await ensureReferenceWorkspaceInitialized(
    vscode,
    output,
    extensionContext,
    referenceContext,
    viewProvider
  );
  if (!ready) {
    return;
  }

  await runTextCommandWithResolved(
    vscode,
    output,
    {
      title: "reference map",
      commandParts: buildReferenceScopedCommandParts("map", {
        shared: referenceContext.shared
      }),
      extensionContext
    },
    referenceContext.resolved
  );
}

async function runStoryMapCommand(vscode, output, extensionContext, viewProvider) {
  const resolved = await resolveExecutionContext(vscode, {
    title: "story map",
    requireBook: true,
    extensionContext
  });
  if (!resolved) {
    return;
  }

  const ready = await ensureStoryWorkspaceInitialized(
    vscode,
    output,
    extensionContext,
    resolved,
    viewProvider
  );
  if (!ready) {
    return;
  }

  await runTextCommandWithResolved(
    vscode,
    output,
    {
      title: "story map",
      commandParts: buildStoryScopedCommandParts("map"),
      extensionContext
    },
    resolved
  );
}

async function runStoryRevealSceneCommand(vscode, item) {
  const sceneContext = extractStorySceneNoteContext(item);
  if (!sceneContext) {
    vscode.window.showErrorMessage("Story scene note context is not available.");
    return;
  }

  const scenesUri = vscode.Uri.file(path.resolve(sceneContext.repoRoot, sceneContext.scenesPath));
  let document;
  try {
    document = await vscode.workspace.openTextDocument(scenesUri);
  } catch {
    vscode.window.showErrorMessage(`Scenes index was not found at ${sceneContext.scenesPath}.`);
    return;
  }

  const editor = await vscode.window.showTextDocument(document, { preview: false });
  const line = findStorySceneLine(document.getText(), sceneContext.sceneFile);

  if (line === null) {
    vscode.window.showWarningMessage(
      `Scene entry for ${sceneContext.sceneFile} was not found in ${sceneContext.scenesPath}.`
    );
    return;
  }

  const start = new vscode.Position(line, 0);
  const end = document.lineAt(line).range.end;
  const range = new vscode.Range(start, end);
  editor.selection = new vscode.Selection(start, end);
  editor.revealRange(range, vscode.TextEditorRevealType.InCenter);
}

async function runStorySeedCommand(vscode, output, extensionContext, viewProvider) {
  const resolved = await resolveExecutionContext(vscode, {
    title: "story seed",
    requireBook: true,
    extensionContext
  });
  if (!resolved) {
    return;
  }

  const ready = await ensureStoryWorkspaceInitialized(
    vscode,
    output,
    extensionContext,
    resolved,
    viewProvider
  );
  if (!ready) {
    return;
  }

  const template = await promptStorySeedTemplate(vscode, storyStructuresRoot(resolved));
  if (!template) {
    return;
  }

  let force = false;
  if (hasNonEmptyStoryScenes(storyScenesPath(resolved))) {
    const overwrite = await promptBooleanChoice(vscode, {
      title: "Story seed mode",
      placeHolder: "Replace the existing scene index and scene notes with the selected structure seeds?",
      trueLabel: "Replace with --force",
      falseLabel: "Cancel",
      defaultValue: false
    });
    if (overwrite !== true) {
      return;
    }
    force = true;
  } else if (fs.existsSync(storySceneNotesRoot(resolved))) {
    const overwrite = await promptBooleanChoice(vscode, {
      title: "Story seed note mode",
      placeHolder: "Overwrite existing scene notes with the selected structure seeds?",
      trueLabel: "Overwrite notes",
      falseLabel: "Keep existing notes",
      defaultValue: false
    });
    if (overwrite === undefined) {
      return;
    }
    force = overwrite;
  }

  const result = await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "story seed",
      commandParts: buildStorySeedCommandParts({ template, force }),
      extensionContext
    },
    resolved
  );
  if (!result) {
    return;
  }

  if (viewProvider) {
    viewProvider.refresh();
  }
}

async function runReferenceCheckCommand(
  vscode,
  output,
  extensionContext,
  diagnostics,
  viewProvider
) {
  const referenceContext = await resolveReferenceScopeContext(vscode, extensionContext, {
    title: "reference check"
  });
  if (!referenceContext) {
    return;
  }

  const ready = await ensureReferenceWorkspaceInitialized(
    vscode,
    output,
    extensionContext,
    referenceContext,
    viewProvider
  );
  if (!ready) {
    return;
  }

  await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "reference check",
      commandParts: buildReferenceScopedCommandParts("check", {
        shared: referenceContext.shared
      }),
      extensionContext,
      acceptedExitCodes: [0, 1],
      onComplete: (result, resolved) =>
        applyDiagnosticsFromReport(
          vscode,
          output,
          diagnostics,
          "shosei reference check",
          result,
          resolved
        )
    },
    referenceContext.resolved
  );
}

async function runStoryCheckCommand(vscode, output, extensionContext, diagnostics, viewProvider) {
  const resolved = await resolveExecutionContext(vscode, {
    title: "story check",
    requireBook: true,
    extensionContext
  });
  if (!resolved) {
    return;
  }

  const ready = await ensureStoryWorkspaceInitialized(
    vscode,
    output,
    extensionContext,
    resolved,
    viewProvider
  );
  if (!ready) {
    return;
  }

  await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "story check",
      commandParts: buildStoryScopedCommandParts("check"),
      extensionContext,
      acceptedExitCodes: [0, 1],
      onComplete: (result, currentResolved) =>
        applyDiagnosticsFromReport(
          vscode,
          output,
          diagnostics,
          "shosei story check",
          result,
          currentResolved
        )
    },
    resolved
  );
}

async function runReferenceDriftCommand(
  vscode,
  output,
  extensionContext,
  diagnostics
) {
  const resolved = await resolveExecutionContext(vscode, {
    title: "reference drift",
    requireBook: true,
    requireSeriesRepo: true,
    extensionContext
  });
  if (!resolved) {
    return;
  }

  await runReferenceDriftWithResolved(
    vscode,
    output,
    extensionContext,
    diagnostics,
    resolved
  );
}

async function runStoryDriftCommand(vscode, output, extensionContext, diagnostics) {
  const resolved = await resolveExecutionContext(vscode, {
    title: "story drift",
    requireBook: true,
    requireSeriesRepo: true,
    extensionContext
  });
  if (!resolved) {
    return;
  }

  await runStoryDriftWithResolved(vscode, output, extensionContext, diagnostics, resolved);
}

async function runReferenceDriftWithResolved(
  vscode,
  output,
  extensionContext,
  diagnostics,
  resolved,
  options = {}
) {
  const result = await runProcess(
    vscode,
    output,
    "reference drift",
    resolved,
    {
      title: "reference drift",
      commandParts: ["reference", "drift"],
      extensionContext,
      acceptedExitCodes: [0, 1]
    }
  );
  if (!result) {
    return null;
  }

  await applyDiagnosticsFromReport(
    vscode,
    output,
    diagnostics,
    "shosei reference drift",
    result,
    resolved
  );

  if (!options.silentOutcome) {
    const outcome = core.classifyCommandResult(result, {
      acceptedExitCodes: [0, 1],
      fallbackMessage: "reference drift completed"
    });
    if (outcome.level === "error") {
      vscode.window.showErrorMessage(outcome.message);
      return null;
    }
    if (outcome.level === "warning") {
      vscode.window.showWarningMessage(outcome.message);
    } else {
      vscode.window.showInformationMessage(outcome.message);
    }
  }

  return result;
}

async function runStoryDriftWithResolved(
  vscode,
  output,
  extensionContext,
  diagnostics,
  resolved,
  options = {}
) {
  const result = await runProcess(vscode, output, "story drift", resolved, {
    title: "story drift",
    commandParts: ["story", "drift"],
    extensionContext,
    acceptedExitCodes: [0, 1]
  });
  if (!result) {
    return null;
  }

  await applyDiagnosticsFromReport(
    vscode,
    output,
    diagnostics,
    "shosei story drift",
    result,
    resolved
  );

  if (!options.silentOutcome) {
    const outcome = core.classifyCommandResult(result, {
      acceptedExitCodes: [0, 1],
      fallbackMessage: "story drift completed"
    });
    if (outcome.level === "error") {
      vscode.window.showErrorMessage(outcome.message);
      return null;
    }
    if (outcome.level === "warning") {
      vscode.window.showWarningMessage(outcome.message);
    } else {
      vscode.window.showInformationMessage(outcome.message);
    }
  }

  return result;
}

async function runReferenceSyncCommand(
  vscode,
  output,
  extensionContext,
  viewProvider,
  driftDiagnostics
) {
  const resolved = await resolveExecutionContext(vscode, {
    title: "reference sync",
    requireBook: true,
    requireSeriesRepo: true,
    extensionContext
  });
  if (!resolved) {
    return;
  }

  const direction = await promptReferenceSyncDirection(vscode);
  if (!direction) {
    return;
  }

  const mode = await promptReferenceSyncMode(vscode);
  if (!mode) {
    return;
  }

  let commandParts;
  if (mode === "single") {
    const id = await vscode.window.showInputBox({
      title: "Reference id",
      prompt: "Reference entry id to sync",
      ignoreFocusOut: true,
      validateInput: (value) => (value.trim() ? null : "Reference id is required")
    });
    if (id === undefined) {
      return;
    }

    const force = await promptBooleanChoice(vscode, {
      title: "Overwrite diverged destination",
      placeHolder: "Pass --force when the destination entry differs?",
      trueLabel: "Allow overwrite",
      falseLabel: "No overwrite",
      defaultValue: false
    });
    if (force === undefined) {
      return;
    }

    commandParts = buildReferenceSyncCommandParts({
      direction,
      id: id.trim(),
      force
    });
  } else {
    const confirmed = await promptBooleanChoice(vscode, {
      title: "Batch sync from drift report",
      placeHolder: "Generate the latest reference drift report and apply it with --force?",
      trueLabel: "Generate and apply",
      falseLabel: "Cancel",
      defaultValue: false
    });
    if (confirmed !== true) {
      return;
    }

    if (driftDiagnostics) {
      driftDiagnostics.clear();
    }
    const driftResult = await runReferenceDriftWithResolved(
      vscode,
      output,
      extensionContext,
      driftDiagnostics,
      resolved,
      { silentOutcome: true }
    );
    if (!driftResult) {
      return;
    }

    const reportPath = core.extractReportPath([driftResult.stdout, driftResult.stderr].join("\n"));
    if (!reportPath) {
      vscode.window.showErrorMessage("reference drift report path was not found in CLI output.");
      return;
    }

    commandParts = buildReferenceSyncCommandParts({
      direction,
      report: core.toAbsolutePath(resolved.repoRoot, reportPath),
      force: true
    });
  }

  const result = await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "reference sync",
      commandParts,
      extensionContext,
      requireBook: true
    },
    resolved
  );
  if (!result) {
    return;
  }

  if (viewProvider) {
    viewProvider.refresh();
  }
}

async function runStorySyncCommand(
  vscode,
  output,
  extensionContext,
  viewProvider,
  driftDiagnostics
) {
  const resolved = await resolveExecutionContext(vscode, {
    title: "story sync",
    requireBook: true,
    requireSeriesRepo: true,
    extensionContext
  });
  if (!resolved) {
    return;
  }

  const direction = await promptStorySyncDirection(vscode);
  if (!direction) {
    return;
  }

  const mode = await promptStorySyncMode(vscode);
  if (!mode) {
    return;
  }

  let commandParts;
  if (mode === "single") {
    const kind = await promptStorySyncKind(vscode);
    if (!kind) {
      return;
    }

    const id = await vscode.window.showInputBox({
      title: "Story entity id",
      prompt: "Story entity id to sync",
      ignoreFocusOut: true,
      validateInput: (value) => (value.trim() ? null : "Story entity id is required")
    });
    if (id === undefined) {
      return;
    }

    const force = await promptBooleanChoice(vscode, {
      title: "Overwrite diverged destination",
      placeHolder: "Pass --force when the destination entity differs?",
      trueLabel: "Allow overwrite",
      falseLabel: "No overwrite",
      defaultValue: false
    });
    if (force === undefined) {
      return;
    }

    commandParts = buildStorySyncCommandParts({
      direction,
      kind,
      id: id.trim(),
      force
    });
  } else {
    const confirmed = await promptBooleanChoice(vscode, {
      title: "Batch sync from drift report",
      placeHolder: "Generate the latest story drift report and apply it with --force?",
      trueLabel: "Generate and apply",
      falseLabel: "Cancel",
      defaultValue: false
    });
    if (confirmed !== true) {
      return;
    }

    if (driftDiagnostics) {
      driftDiagnostics.clear();
    }
    const driftResult = await runStoryDriftWithResolved(
      vscode,
      output,
      extensionContext,
      driftDiagnostics,
      resolved,
      { silentOutcome: true }
    );
    if (!driftResult) {
      return;
    }

    const reportPath = core.extractReportPath([driftResult.stdout, driftResult.stderr].join("\n"));
    if (!reportPath) {
      vscode.window.showErrorMessage("story drift report path was not found in CLI output.");
      return;
    }

    commandParts = buildStorySyncCommandParts({
      direction,
      report: core.toAbsolutePath(resolved.repoRoot, reportPath),
      force: true
    });
  }

  const result = await runManagedCommandWithResolved(
    vscode,
    output,
    {
      title: "story sync",
      commandParts,
      extensionContext,
      requireBook: true
    },
    resolved
  );
  if (!result) {
    return;
  }

  if (viewProvider) {
    viewProvider.refresh();
  }
}

async function resolveReferenceScopeContext(vscode, extensionContext, options = {}) {
  const startPath = await pickStartPath(vscode, { promptForWorkspace: true });
  if (!startPath) {
    vscode.window.showErrorMessage("Open a workspace folder or file before running shosei commands.");
    return null;
  }

  const repo = core.findRepoRoot(startPath);
  if (!repo) {
    vscode.window.showErrorMessage("Could not find book.yml or series.yml from the current workspace context.");
    return null;
  }

  if (repo.mode === "single-book") {
    return {
      shared: false,
      resolved: {
        repoRoot: repo.repoRoot,
        mode: repo.mode,
        bookId: null
      }
    };
  }

  const shared = await promptReferenceScope(vscode, options.title);
  if (shared === null) {
    return null;
  }
  if (shared) {
    return {
      shared: true,
      resolved: {
        repoRoot: repo.repoRoot,
        mode: repo.mode,
        bookId: null
      }
    };
  }

  const bookId = await resolveSeriesBookId(vscode, extensionContext, repo.repoRoot, startPath);
  if (!bookId) {
    return null;
  }

  return {
    shared: false,
    resolved: {
      repoRoot: repo.repoRoot,
      mode: repo.mode,
      bookId
    }
  };
}

async function resolveStoryScaffoldContext(vscode, extensionContext, options = {}) {
  const startPath = await pickStartPath(vscode, { promptForWorkspace: true });
  if (!startPath) {
    vscode.window.showErrorMessage("Open a workspace folder or file before running shosei commands.");
    return null;
  }

  const repo = core.findRepoRoot(startPath);
  if (!repo) {
    vscode.window.showErrorMessage("Could not find book.yml or series.yml from the current workspace context.");
    return null;
  }

  if (repo.mode === "single-book") {
    return {
      shared: false,
      resolved: {
        repoRoot: repo.repoRoot,
        mode: repo.mode,
        bookId: null
      }
    };
  }

  const shared = await promptStoryScope(vscode, options.title);
  if (shared === null) {
    return null;
  }
  if (shared) {
    return {
      shared: true,
      resolved: {
        repoRoot: repo.repoRoot,
        mode: repo.mode,
        bookId: null
      }
    };
  }

  const bookId = await resolveSeriesBookId(vscode, extensionContext, repo.repoRoot, startPath);
  if (!bookId) {
    return null;
  }

  return {
    shared: false,
    resolved: {
      repoRoot: repo.repoRoot,
      mode: repo.mode,
      bookId
    }
  };
}

async function ensureReferenceWorkspaceInitialized(
  vscode,
  output,
  extensionContext,
  referenceContext,
  viewProvider
) {
  if (fs.existsSync(referenceEntriesRoot(referenceContext.resolved, referenceContext.shared))) {
    return true;
  }

  const selected = await vscode.window.showWarningMessage(
    `Reference workspace is not initialized at ${referenceWorkspaceRoot(
      referenceContext.resolved,
      referenceContext.shared
    )}.`,
    "Run Reference Scaffold",
    "Cancel"
  );
  if (selected !== "Run Reference Scaffold") {
    return false;
  }

  return Boolean(
    await runReferenceScaffoldWithResolved(
      vscode,
      output,
      extensionContext,
      referenceContext.resolved,
      referenceContext.shared,
      viewProvider
    )
  );
}

async function ensureStoryWorkspaceInitialized(
  vscode,
  output,
  extensionContext,
  resolved,
  viewProvider
) {
  if (fs.existsSync(storyScenesPath(resolved))) {
    return true;
  }

  const selected = await vscode.window.showWarningMessage(
    `Story workspace is not initialized at ${storyWorkspaceRoot(resolved)}.`,
    "Run Story Scaffold",
    "Cancel"
  );
  if (selected !== "Run Story Scaffold") {
    return false;
  }

  return Boolean(
    await runStoryScaffoldWithResolved(
      vscode,
      output,
      extensionContext,
      resolved,
      false,
      viewProvider
    )
  );
}

async function resolveProseChapterContext(vscode, extensionContext) {
  const resolved = await resolveExecutionContext(vscode, {
    title: "chapter",
    requireBook: true,
    extensionContext
  });
  if (!resolved) {
    return null;
  }

  const explainResult = await loadExplainSnapshot(vscode, extensionContext, resolved);
  if (explainResult.error) {
    vscode.window.showErrorMessage(explainResult.error);
    return null;
  }

  if (!explainResult.explain?.manuscript) {
    vscode.window.showErrorMessage("Chapter commands are available only for prose projects.");
    return null;
  }

  return {
    resolved,
    explain: explainResult.explain
  };
}

function getStoredSeriesBookSelection(extensionContext, repoRoot) {
  if (!extensionContext || !repoRoot) {
    return null;
  }

  const selections = extensionContext.workspaceState.get(
    SERIES_BOOK_SELECTIONS_KEY,
    {}
  );
  return selections[repoRoot] || null;
}

function extractChapterItemPath(item) {
  return typeof item?.chapterPath === "string" ? item.chapterPath : null;
}

function extractStorySceneNoteContext(item) {
  if (
    typeof item?.storyRepoRoot !== "string" ||
    typeof item?.storySceneFile !== "string" ||
    typeof item?.storyScenesPath !== "string"
  ) {
    return null;
  }

  return {
    repoRoot: item.storyRepoRoot,
    sceneFile: item.storySceneFile,
    scenesPath: item.storyScenesPath
  };
}

function findStorySceneLine(contents, sceneFile) {
  const escaped = escapeRegExp(sceneFile);
  const matcher = new RegExp(`^\\s*-?\\s*file:\\s*([\"'])?${escaped}\\1\\s*$`);
  const lines = contents.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    if (matcher.test(lines[index])) {
      return index;
    }
  }
  return null;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

async function setStoredSeriesBookSelection(extensionContext, repoRoot, bookId) {
  if (!extensionContext || !repoRoot) {
    return;
  }

  const selections = {
    ...extensionContext.workspaceState.get(SERIES_BOOK_SELECTIONS_KEY, {})
  };

  if (bookId) {
    selections[repoRoot] = bookId;
  } else {
    delete selections[repoRoot];
  }

  await extensionContext.workspaceState.update(SERIES_BOOK_SELECTIONS_KEY, selections);
}

async function promptSeriesBookSelection(vscode, extensionContext) {
  const startPath = await pickStartPath(vscode, { promptForWorkspace: true });
  if (!startPath) {
    vscode.window.showErrorMessage("Open a series repository before selecting a book.");
    return undefined;
  }

  const repo = core.findRepoRoot(startPath);
  if (!repo || repo.mode !== "series") {
    vscode.window.showErrorMessage("Book selection is available only in a series repository.");
    return undefined;
  }

  const selected = await promptForSeriesBookId(vscode, extensionContext, repo.repoRoot, true);
  return selected === undefined ? undefined : selected;
}

async function promptInitTarget(vscode) {
  const workspaceDirectory = await pickWorkspaceDirectory(vscode);
  const items = [];

  if (workspaceDirectory) {
    items.push({
      label: "Use workspace folder",
      description: workspaceDirectory,
      value: workspaceDirectory
    });
  }

  items.push({
    label: "Choose folder...",
    description: "Select the directory to initialize",
    value: "__browse__"
  });

  const picked = await vscode.window.showQuickPick(items, {
    title: "Initialize shosei project",
    placeHolder: "Select the target folder for shosei init"
  });
  if (!picked) {
    return null;
  }

  if (picked.value === "__browse__") {
    const selected = await vscode.window.showOpenDialog({
      canSelectFiles: false,
      canSelectFolders: true,
      canSelectMany: false,
      openLabel: "Use folder for shosei init"
    });
    return selected?.[0]?.fsPath || null;
  }

  return picked.value;
}

async function promptNewChapterPath(vscode, explain) {
  const suggested = suggestChapterPath(explain);
  const value = await vscode.window.showInputBox({
    title: "Chapter path",
    prompt: "Repo-relative markdown path to add to manuscript.chapters",
    value: suggested,
    ignoreFocusOut: true,
    validateInput: validateChapterPathInput
  });
  if (value === undefined) {
    return null;
  }
  return value.trim();
}

async function pickWorkspaceDirectory(vscode) {
  const folders = vscode.workspace.workspaceFolders || [];
  if (folders.length === 1) {
    return folders[0].uri.fsPath;
  }
  if (folders.length > 1) {
    const picked = await vscode.window.showWorkspaceFolderPick({
      placeHolder: "Select the workspace folder to initialize"
    });
    return picked ? picked.uri.fsPath : null;
  }
  return null;
}

async function promptInitOptions(vscode, targetRoot) {
  const configTemplate = await promptQuickPickValue(
    vscode,
    [
      {
        label: "Paper",
        description: "horizontal prose, print-first by default",
        value: "paper"
      },
      {
        label: "Novel",
        description: "vertical prose, single-book by default",
        value: "novel"
      },
      {
        label: "Business",
        description: "horizontal prose, single-book by default",
        value: "business"
      },
      {
        label: "Light Novel",
        description: "vertical prose, single-book by default",
        value: "light-novel"
      },
      {
        label: "Manga",
        description: "image-first, series by default",
        value: "manga"
      }
    ],
    {
      title: "Project template",
      placeHolder: `Select the project type for ${path.basename(targetRoot)}`
    }
  );
  if (!configTemplate) {
    return null;
  }

  const configProfile =
    configTemplate === "paper"
      ? await promptQuickPickValue(
          vscode,
          [
            {
              label: "Paper",
              description: "general paper or report scaffold",
              value: "paper"
            },
            {
              label: "Conference Preprint",
              description: "A4 two-column preprint preset",
              value: "conference-preprint"
            }
          ],
          {
            title: "Paper profile",
            placeHolder: "Select the profile written to book.yml"
          }
        )
      : null;
  if (configTemplate === "paper" && !configProfile) {
    return null;
  }

  const repoMode = await promptQuickPickValue(vscode, buildRepoModeItems(configTemplate), {
    title: "Repository mode",
    placeHolder: "Select the repository layout"
  });
  if (!repoMode) {
    return null;
  }

  const initialBookId =
    repoMode === "series"
      ? await vscode.window.showInputBox({
          title: "Initial book id",
          prompt: "Book id used for books/<book-id>/ and the first --book examples",
          value: DEFAULT_INIT_SERIES_BOOK_ID,
          ignoreFocusOut: true,
          validateInput: validateSeriesBookIdInput
        })
      : null;
  if (repoMode === "series" && initialBookId === undefined) {
    return null;
  }

  const title = await vscode.window.showInputBox({
    title: "Book title",
    prompt: "Title written to book.yml or series.yml",
    value: defaultTitleForTemplate(configProfile || configTemplate),
    ignoreFocusOut: true,
    validateInput: (value) => (value.trim() ? null : "Title is required")
  });
  if (title === undefined) {
    return null;
  }

  const author = await vscode.window.showInputBox({
    title: "Author",
    prompt: "Primary author name written to scaffold config",
    value: "Author Name",
    ignoreFocusOut: true,
    validateInput: (value) => (value.trim() ? null : "Author is required")
  });
  if (author === undefined) {
    return null;
  }

  const language = await vscode.window.showInputBox({
    title: "Language",
    prompt: "Language code written to scaffold config",
    value: "ja",
    ignoreFocusOut: true,
    validateInput: (value) => (value.trim() ? null : "Language is required")
  });
  if (language === undefined) {
    return null;
  }

  const outputPreset = await promptQuickPickValue(
    vscode,
    buildOutputPresetItems(configTemplate),
    {
      title: "Output preset",
      placeHolder: "Select the output preset for the scaffold"
    }
  );
  if (!outputPreset) {
    return null;
  }

  const includeIntroduction =
    configTemplate !== "manga"
      ? await promptBooleanChoice(vscode, {
          title: "Introduction scaffold",
          placeHolder: "Add a frontmatter scaffold such as はじめに?",
          trueLabel: "Add introduction",
          falseLabel: "Skip introduction",
          defaultValue: false
        })
      : null;
  if (configTemplate !== "manga" && includeIntroduction === undefined) {
    return null;
  }

  const includeAfterword =
    configTemplate !== "manga"
      ? await promptBooleanChoice(vscode, {
          title: "Afterword scaffold",
          placeHolder: "Add a backmatter scaffold such as おわりに?",
          trueLabel: "Add afterword",
          falseLabel: "Skip afterword",
          defaultValue: false
        })
      : null;
  if (configTemplate !== "manga" && includeAfterword === undefined) {
    return null;
  }

  let force = false;
  if (hasInitConfig(targetRoot)) {
    const overwrite = await promptBooleanChoice(vscode, {
      title: "Existing shosei config found",
      placeHolder: "Overwrite book.yml or series.yml in the selected folder?",
      trueLabel: "Overwrite existing config",
      falseLabel: "Keep existing config",
      defaultValue: false
    });
    if (overwrite === undefined) {
      return null;
    }
    force = overwrite;
  }

  const runDoctor = await promptBooleanChoice(vscode, {
    title: "Run doctor after init",
    placeHolder: "Run shosei doctor after scaffold generation?",
    trueLabel: "Run doctor",
    falseLabel: "Skip doctor",
    defaultValue: false
  });
  if (runDoctor === undefined) {
    return null;
  }

  return {
    configTemplate,
    configProfile,
    repoMode,
    initialBookId: repoMode === "series" ? initialBookId.trim() : null,
    title: title.trim(),
    author: author.trim(),
    language: language.trim(),
    outputPreset,
    includeIntroduction: Boolean(includeIntroduction),
    includeAfterword: Boolean(includeAfterword),
    force,
    runDoctor
  };
}

async function promptChapterSelection(vscode, explain, options) {
  const chapters = explain?.manuscript?.chapters || [];
  if (chapters.length === 0) {
    vscode.window.showErrorMessage("No chapters are configured in manuscript.chapters.");
    return null;
  }

  const picked = await vscode.window.showQuickPick(
    chapters.map((chapter) => ({
      label: path.basename(chapter),
      description: chapter,
      value: chapter
    })),
    {
      title: options.title,
      placeHolder: options.placeHolder
    }
  );
  return picked ? picked.value : null;
}

async function promptChapterAddPlacement(vscode, explain, selectedChapterPath) {
  const chapters = explain?.manuscript?.chapters || [];
  if (chapters.length === 0) {
    return {};
  }

  const items = [];
  if (selectedChapterPath) {
    items.push({
      label: "After selected chapter",
      description: selectedChapterPath,
      value: { after: selectedChapterPath }
    });
    items.push({
      label: "Before selected chapter",
      description: selectedChapterPath,
      value: { before: selectedChapterPath }
    });
  }
  items.push({
    label: "Append after last chapter",
    description: chapters[chapters.length - 1],
    value: {}
  });
  items.push({
    label: "Insert before first chapter",
    description: chapters[0],
    value: { before: chapters[0] }
  });

  const picked = await vscode.window.showQuickPick(items, {
    title: "Insert chapter",
    placeHolder: "Choose where to place the new chapter"
  });
  return picked ? picked.value : undefined;
}

async function promptChapterMovePlacement(vscode, explain, targetChapterPath) {
  const chapters = (explain?.manuscript?.chapters || []).filter(
    (chapter) => chapter !== targetChapterPath
  );
  if (chapters.length === 0) {
    vscode.window.showInformationMessage("No alternate chapter position is available.");
    return null;
  }

  const items = [
    {
      label: "Move to beginning",
      description: `before ${chapters[0]}`,
      value: { before: chapters[0] }
    },
    {
      label: "Move to end",
      description: `after ${chapters[chapters.length - 1]}`,
      value: { after: chapters[chapters.length - 1] }
    }
  ];

  for (const chapter of chapters) {
    items.push({
      label: `Before ${path.basename(chapter)}`,
      description: chapter,
      value: { before: chapter }
    });
    items.push({
      label: `After ${path.basename(chapter)}`,
      description: chapter,
      value: { after: chapter }
    });
  }

  const picked = await vscode.window.showQuickPick(items, {
    title: "Move chapter",
    placeHolder: `Choose the new position for ${path.basename(targetChapterPath)}`
  });
  return picked ? picked.value : null;
}

async function promptPositiveInteger(vscode, options) {
  const value = await vscode.window.showInputBox({
    title: options.title,
    prompt: options.prompt,
    value: options.value,
    ignoreFocusOut: true,
    validateInput: (input) => {
      const trimmed = input.trim();
      if (!/^\d+$/.test(trimmed)) {
        return "Enter a positive integer";
      }
      if (Number.parseInt(trimmed, 10) < 1) {
        return "Enter a positive integer";
      }
      return null;
    }
  });
  if (value === undefined) {
    return undefined;
  }
  return Number.parseInt(value.trim(), 10);
}

function buildRepoModeItems(configTemplate) {
  const defaultRepoMode = configTemplate === "manga" ? "series" : "single-book";
  const items = [
    {
      label: "Single Book",
      description: "book.yml at repo root",
      value: "single-book"
    },
    {
      label: "Series",
      description: "series.yml with books/<book-id>/book.yml",
      value: "series"
    }
  ];

  return items.sort((left, right) => {
    if (left.value === defaultRepoMode) {
      return -1;
    }
    if (right.value === defaultRepoMode) {
      return 1;
    }
    return left.label.localeCompare(right.label);
  });
}

function buildOutputPresetItems(configTemplate) {
  const defaultOutputPreset = configTemplate === "paper" ? "print" : "kindle";
  const items = [
    {
      label: "Kindle",
      description: "enable Kindle output only",
      value: "kindle"
    },
    {
      label: "Print",
      description: "enable print output only",
      value: "print"
    },
    {
      label: "Both",
      description: "enable Kindle and print outputs",
      value: "both"
    }
  ];

  return items.sort((left, right) => {
    if (left.value === defaultOutputPreset) {
      return -1;
    }
    if (right.value === defaultOutputPreset) {
      return 1;
    }
    return left.label.localeCompare(right.label);
  });
}

function suggestChapterPath(explain) {
  const chapters = explain?.manuscript?.chapters || [];
  const nextNumber = String(chapters.length + 1).padStart(2, "0");
  if (chapters.length > 0) {
    const chapterDir = path.posix.dirname(chapters[chapters.length - 1]);
    return `${chapterDir}/${nextNumber}-chapter-${chapters.length + 1}.md`;
  }

  const repoRoot = typeof explain?.repo_root === "string" ? explain.repo_root : null;
  const bookRoot = typeof explain?.book_root === "string" ? explain.book_root : null;
  if (repoRoot && bookRoot) {
    const relativeBookRoot = normalizeRepoPath(path.relative(repoRoot, bookRoot));
    const prefix = relativeBookRoot ? `${relativeBookRoot}/` : "";
    return `${prefix}manuscript/${nextNumber}-chapter-${chapters.length + 1}.md`;
  }

  return `manuscript/${nextNumber}-chapter-${chapters.length + 1}.md`;
}

function suggestChapterTitle(explain) {
  const chapters = explain?.manuscript?.chapters || [];
  return `Chapter ${chapters.length + 1}`;
}

function validateChapterPathInput(value) {
  const trimmed = value.trim();
  if (!trimmed) {
    return "Chapter path is required";
  }
  if (!trimmed.endsWith(".md")) {
    return "Chapter path must end with .md";
  }
  if (path.isAbsolute(trimmed) || trimmed.startsWith("./") || trimmed.includes("\\")) {
    return "Use a repo-relative path with '/' separators";
  }
  if (trimmed.split("/").includes("..")) {
    return "Chapter path must not contain '..'";
  }
  return null;
}

function validateSeriesBookIdInput(value) {
  const trimmed = value.trim();
  if (!trimmed) {
    return "Book id is required";
  }
  if (trimmed === "." || trimmed === "..") {
    return "Book id must not be '.' or '..'";
  }
  if (trimmed.includes("/") || trimmed.includes("\\")) {
    return "Book id must be a single path segment";
  }
  if (/\s/.test(trimmed)) {
    return "Book id must not contain whitespace";
  }
  return null;
}

function normalizeRepoPath(value) {
  return value.split(path.sep).join("/");
}

function buildChapterAddCommandParts(options) {
  const commandParts = ["chapter", "add", options.chapterPath];
  if (options.title) {
    commandParts.push("--title", options.title);
  }
  if (options.before) {
    commandParts.push("--before", options.before);
  }
  if (options.after) {
    commandParts.push("--after", options.after);
  }
  return commandParts;
}

function buildChapterMoveCommandParts(options) {
  const commandParts = ["chapter", "move", options.chapterPath];
  if (options.before) {
    commandParts.push("--before", options.before);
  }
  if (options.after) {
    commandParts.push("--after", options.after);
  }
  return commandParts;
}

function buildChapterRemoveCommandParts(options) {
  const commandParts = ["chapter", "remove", options.chapterPath];
  if (options.deleteFile) {
    commandParts.push("--delete-file");
  }
  return commandParts;
}

function buildChapterRenumberCommandParts(options) {
  const commandParts = [
    "chapter",
    "renumber",
    "--start-at",
    String(options.startAt),
    "--width",
    String(options.width)
  ];
  if (options.dryRun) {
    commandParts.push("--dry-run");
  }
  return commandParts;
}

function buildReferenceScopedCommandParts(subcommand, options = {}) {
  const commandParts = ["reference", subcommand];
  if (options.shared) {
    commandParts.push("--shared");
  }
  if (options.force) {
    commandParts.push("--force");
  }
  return commandParts;
}

function buildStoryScopedCommandParts(subcommand, options = {}) {
  const commandParts = ["story", subcommand];
  if (options.shared) {
    commandParts.push("--shared");
  }
  if (options.force) {
    commandParts.push("--force");
  }
  return commandParts;
}

function buildStorySeedCommandParts(options) {
  const commandParts = ["story", "seed", "--template", options.template];
  if (options.force) {
    commandParts.push("--force");
  }
  return commandParts;
}

function buildReferenceSyncCommandParts(options) {
  const commandParts = ["reference", "sync", options.direction.flag, "shared"];
  if (typeof options.id === "string" && options.id.trim()) {
    commandParts.push("--id", options.id.trim());
  }
  if (typeof options.report === "string" && options.report.trim()) {
    commandParts.push("--report", options.report.trim());
  }
  if (options.force) {
    commandParts.push("--force");
  }
  return commandParts;
}

function buildStorySyncCommandParts(options) {
  const commandParts = ["story", "sync", options.direction.flag, "shared"];
  if (typeof options.kind === "string" && options.kind.trim()) {
    commandParts.push("--kind", options.kind.trim());
  }
  if (typeof options.id === "string" && options.id.trim()) {
    commandParts.push("--id", options.id.trim());
  }
  if (typeof options.report === "string" && options.report.trim()) {
    commandParts.push("--report", options.report.trim());
  }
  if (options.force) {
    commandParts.push("--force");
  }
  return commandParts;
}

function defaultTitleForTemplate(configTemplate) {
  switch (configTemplate) {
    case "business":
      return "Untitled Business Book";
    case "paper":
      return "Untitled Paper";
    case "conference-preprint":
      return "Untitled Conference Preprint";
    case "light-novel":
      return "Untitled Light Novel";
    case "manga":
      return "Untitled Manga Volume";
    case "novel":
    default:
      return "Untitled Novel";
  }
}

function hasInitConfig(targetRoot) {
  return (
    fs.existsSync(path.join(targetRoot, "book.yml")) ||
    fs.existsSync(path.join(targetRoot, "series.yml"))
  );
}

async function promptQuickPickValue(vscode, items, options) {
  const picked = await vscode.window.showQuickPick(items, options);
  return picked ? picked.value : null;
}

async function promptReferenceScope(vscode, title) {
  return promptQuickPickValue(
    vscode,
    [
      {
        label: "Book Reference Workspace",
        description: "current book reference entries",
        value: false
      },
      {
        label: "Shared Reference Workspace",
        description: "shared/metadata/references",
        value: true
      }
    ],
    {
      title: title || "Reference scope",
      placeHolder: "Select the reference workspace scope"
    }
  );
}

async function promptStoryScope(vscode, title) {
  return promptQuickPickValue(
    vscode,
    [
      {
        label: "Book Story Workspace",
        description: "current book story data",
        value: false
      },
      {
        label: "Shared Story Workspace",
        description: "shared/metadata/story",
        value: true
      }
    ],
    {
      title: title || "Story scope",
      placeHolder: "Select the story workspace scope"
    }
  );
}

async function promptBooleanChoice(vscode, options) {
  const items = [
    { label: options.trueLabel || "Yes", value: true },
    { label: options.falseLabel || "No", value: false }
  ];
  if (!options.defaultValue) {
    items.reverse();
  }

  const picked = await vscode.window.showQuickPick(items, {
    title: options.title,
    placeHolder: options.placeHolder
  });
  return picked ? picked.value : undefined;
}

async function promptReferenceSyncDirection(vscode) {
  return promptQuickPickValue(
    vscode,
    [
      {
        label: "Shared -> Book",
        description: "copy a shared reference into the selected book",
        value: { flag: "--from", label: "shared" }
      },
      {
        label: "Book -> Shared",
        description: "copy a book reference into shared metadata",
        value: { flag: "--to", label: "shared" }
      }
    ],
    {
      title: "Reference sync direction",
      placeHolder: "Select the direction for reference sync"
    }
  );
}

async function promptStorySyncDirection(vscode) {
  return promptQuickPickValue(
    vscode,
    [
      {
        label: "Shared -> Book",
        description: "copy a shared story entity into the selected book",
        value: { flag: "--from", label: "shared" }
      },
      {
        label: "Book -> Shared",
        description: "copy a book story entity into shared canon",
        value: { flag: "--to", label: "shared" }
      }
    ],
    {
      title: "Story sync direction",
      placeHolder: "Select the direction for story sync"
    }
  );
}

async function promptReferenceSyncMode(vscode) {
  return promptQuickPickValue(
    vscode,
    [
      {
        label: "Single Reference Id",
        description: "copy one reference entry by id",
        value: "single"
      },
      {
        label: "Batch From Drift Report",
        description: "generate the latest drift report and apply it with --force",
        value: "report"
      }
    ],
    {
      title: "Reference sync mode",
      placeHolder: "Select how to choose entries for sync"
    }
  );
}

async function promptStorySyncMode(vscode) {
  return promptQuickPickValue(
    vscode,
    [
      {
        label: "Single Story Entity",
        description: "copy one story entity by kind and id",
        value: "single"
      },
      {
        label: "Batch From Drift Report",
        description: "generate the latest drift report and apply it with --force",
        value: "report"
      }
    ],
    {
      title: "Story sync mode",
      placeHolder: "Select how to choose story entities for sync"
    }
  );
}

async function promptStorySyncKind(vscode) {
  return promptQuickPickValue(
    vscode,
    [
      {
        label: "Character",
        description: "story/characters or shared/metadata/story/characters",
        value: "character"
      },
      {
        label: "Location",
        description: "story/locations or shared/metadata/story/locations",
        value: "location"
      },
      {
        label: "Term",
        description: "story/terms or shared/metadata/story/terms",
        value: "term"
      },
      {
        label: "Faction",
        description: "story/factions or shared/metadata/story/factions",
        value: "faction"
      }
    ],
    {
      title: "Story entity kind",
      placeHolder: "Select the story entity kind to sync"
    }
  );
}

async function promptStorySeedTemplate(vscode, structuresRoot) {
  let templates = [];
  try {
    templates = fs
      .readdirSync(structuresRoot, { withFileTypes: true })
      .filter((entry) => entry.isFile())
      .map((entry) => entry.name)
      .filter((name) => name.toLowerCase().endsWith(".md"))
      .filter((name) => name.toLowerCase() !== "readme.md")
      .sort();
  } catch {
    templates = [];
  }

  if (templates.length === 0) {
    return null;
  }

  return promptQuickPickValue(
    vscode,
    templates.map((name) => ({
      label: name.replace(/\.md$/i, ""),
      description: path.join(path.basename(path.dirname(structuresRoot)), name),
      value: name.replace(/\.md$/i, "")
    })),
    {
      title: "Story structure template",
      placeHolder: "Select the structure template to seed into scenes.yml"
    }
  );
}

function referenceWorkspaceRoot(resolved, shared) {
  if (resolved.mode === "single-book") {
    return path.join(resolved.repoRoot, "references");
  }
  if (shared) {
    return path.join(resolved.repoRoot, "shared", "metadata", "references");
  }
  return path.join(resolved.repoRoot, "books", resolved.bookId, "references");
}

function storyWorkspaceRoot(resolved, shared = false) {
  if (resolved.mode === "single-book") {
    return path.join(resolved.repoRoot, "story");
  }
  if (shared) {
    return path.join(resolved.repoRoot, "shared", "metadata", "story");
  }
  return path.join(resolved.repoRoot, "books", resolved.bookId, "story");
}

function referenceEntriesRoot(resolved, shared) {
  return path.join(referenceWorkspaceRoot(resolved, shared), "entries");
}

function storyScenesPath(resolved) {
  return path.join(storyWorkspaceRoot(resolved), "scenes.yml");
}

function storyStructuresRoot(resolved) {
  return path.join(storyWorkspaceRoot(resolved), "structures");
}

function storySceneNotesRoot(resolved) {
  return path.join(storyWorkspaceRoot(resolved), "scene-notes");
}

function hasNonEmptyStoryScenes(scenesPath) {
  if (!fs.existsSync(scenesPath)) {
    return false;
  }

  try {
    const contents = fs.readFileSync(scenesPath, "utf8");
    return /^(\s*)-\s+file:/m.test(contents);
  } catch {
    return false;
  }
}

async function promptForSeriesBookId(
  vscode,
  extensionContext,
  repoRoot,
  allowClearSelection = false
) {
  const stored = getStoredSeriesBookSelection(extensionContext, repoRoot);
  const bookIds = core.listSeriesBookIds(repoRoot);

  if (bookIds.length > 0) {
    const items = bookIds.map((bookId) => ({
      label: bookId,
      description: stored === bookId ? "selected" : ""
    }));
    if (allowClearSelection && stored) {
      items.unshift({
        label: "Clear selected book",
        description: "fall back to active file or setting",
        pickedValue: null
      });
    }

    const picked = await vscode.window.showQuickPick(items, {
      title: "Select a series book",
      placeHolder: "Choose the book to pass to --book"
    });
    if (!picked) {
      return null;
    }
    if (picked.pickedValue === null) {
      await setStoredSeriesBookSelection(extensionContext, repoRoot, null);
      return null;
    }

    await setStoredSeriesBookSelection(extensionContext, repoRoot, picked.label);
    return picked.label;
  }

  const manual = await vscode.window.showInputBox({
    title: "Series book id",
    prompt: "Enter the book id to pass to shosei --book",
    ignoreFocusOut: true,
    validateInput: (value) => (value.trim() ? null : "Book id is required")
  });
  if (!manual) {
    return null;
  }

  await setStoredSeriesBookSelection(extensionContext, repoRoot, manual.trim());
  return manual.trim();
}

function getWorkspaceFolder(vscode, repoRoot) {
  const uri = vscode.Uri.file(repoRoot);
  return vscode.workspace.getWorkspaceFolder(uri) || null;
}

function renderSpawnError(command, error) {
  if (error && error.code === "ENOENT") {
    return `Failed to launch ${command}. Check shosei.cli.command and shosei.cli.args in VS Code settings.`;
  }
  return `Failed to launch ${command}: ${error.message}`;
}

function toWorkingDirectory(candidate) {
  if (!candidate) {
    return null;
  }

  try {
    return fs.statSync(candidate).isDirectory() ? candidate : path.dirname(candidate);
  } catch {
    return null;
  }
}

module.exports = {
  activate,
  deactivate,
  __test: {
    buildChapterAddCommandParts,
    buildChapterMoveCommandParts,
    buildChapterRemoveCommandParts,
    buildChapterRenumberCommandParts,
    buildReferenceScopedCommandParts,
    buildReferenceSyncCommandParts,
    buildStoryScopedCommandParts,
    buildStorySeedCommandParts,
    buildStorySyncCommandParts,
    findStorySceneLine,
    hasNonEmptyStoryScenes,
    referenceEntriesRoot,
    referenceWorkspaceRoot,
    storySceneNotesRoot,
    storyScenesPath,
    storyStructuresRoot,
    storyWorkspaceRoot,
    resolveDiagnosticLocation,
    suggestChapterPath,
    validateChapterPathInput,
    validateSeriesBookIdInput
  }
};
