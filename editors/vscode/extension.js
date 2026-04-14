const cp = require("child_process");
const fs = require("fs");
const path = require("path");

const core = require("./src/core");
const { ShoseiViewProvider } = require("./src/view");

const SERIES_BOOK_SELECTIONS_KEY = "shosei.series.selectedBooks";

function activate(context) {
  const vscode = require("vscode");
  const output = vscode.window.createOutputChannel("Shosei");
  const validateDiagnostics = vscode.languages.createDiagnosticCollection("shosei-validate");
  const pageCheckDiagnostics = vscode.languages.createDiagnosticCollection("shosei-page-check");
  const viewProvider = new ShoseiViewProvider(vscode, {
    getSnapshot: () => resolveViewSnapshot(vscode, context)
  });
  const treeView = vscode.window.createTreeView("shosei.sidebar", {
    treeDataProvider: viewProvider,
    showCollapseAll: false
  });

  context.subscriptions.push(output, validateDiagnostics, pageCheckDiagnostics, treeView);
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(() => viewProvider.refresh()),
    vscode.workspace.onDidChangeWorkspaceFolders(() => viewProvider.refresh()),
    vscode.workspace.onDidSaveTextDocument(() => viewProvider.refresh())
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

  const result = await runProcess(vscode, output, descriptor.title, resolved, descriptor);
  if (!result) {
    return;
  }

  const contents = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
  if (!contents) {
    return;
  }

  const document = await vscode.workspace.openTextDocument({
    language: "plaintext",
    content: contents
  });
  await vscode.window.showTextDocument(document, { preview: false });
}

async function runManagedCommand(vscode, output, descriptor) {
  const resolved = await resolveExecutionContext(vscode, descriptor);
  if (!resolved) {
    return;
  }

  const result = await runProcess(vscode, output, descriptor.title, resolved, descriptor);
  if (!result) {
    return;
  }

  if (typeof descriptor.onComplete === "function") {
    await descriptor.onComplete(result, resolved);
  }

  const message = result.stdout.trim() || `${descriptor.title} completed`;
  if (descriptor.acceptedExitCodes && descriptor.acceptedExitCodes.includes(1) && result.code === 1) {
    vscode.window.showWarningMessage(message);
  } else {
    vscode.window.showInformationMessage(message);
  }
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
  const cliCommand = config.get("cli.command") || "shosei";
  const cliArgs = core.sanitizeCliArgs(config.get("cli.args"));

  return core.buildCliInvocation({
    cliCommand,
    cliArgs,
    commandParts: descriptor.commandParts,
    bookId: resolved.bookId,
    repoRoot: resolved.repoRoot,
    cwd: resolved.repoRoot,
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
    if (!issue || !issue.location) {
      continue;
    }
    const filePath = core.toAbsolutePath(resolved.repoRoot, issue.location);
    const uri = vscode.Uri.file(path.normalize(filePath)).toString();
    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 0),
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
  if (!startPath) {
    return {
      repoRoot: null,
      mode: null,
      bookId: null,
      bookSource: null
    };
  }

  const repo = core.findRepoRoot(startPath);
  if (!repo) {
    return {
      repoRoot: null,
      mode: null,
      bookId: null,
      bookSource: null
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

  return {
    repoRoot: repo.repoRoot,
    mode: repo.mode,
    bookId,
    bookSource
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
  deactivate
};
