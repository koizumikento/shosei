const fs = require("fs");
const path = require("path");

function isDirectory(candidate) {
  try {
    return fs.statSync(candidate).isDirectory();
  } catch {
    return false;
  }
}

function fileExists(candidate) {
  try {
    return fs.statSync(candidate).isFile();
  } catch {
    return false;
  }
}

function findRepoRoot(startPath) {
  if (!startPath) {
    return null;
  }

  let current = startPath;
  try {
    if (!fs.statSync(current).isDirectory()) {
      current = path.dirname(current);
    }
  } catch {
    return null;
  }

  let singleBookCandidate = null;

  while (true) {
    const seriesConfig = path.join(current, "series.yml");
    if (fileExists(seriesConfig)) {
      return { repoRoot: current, mode: "series" };
    }

    const bookConfig = path.join(current, "book.yml");
    if (!singleBookCandidate && fileExists(bookConfig)) {
      singleBookCandidate = { repoRoot: current, mode: "single-book" };
    }

    const parent = path.dirname(current);
    if (parent === current) {
      return singleBookCandidate;
    }
    current = parent;
  }
}

function inferSeriesBookId(repoRoot, candidatePath) {
  if (!repoRoot || !candidatePath) {
    return null;
  }

  const fullPath = path.resolve(candidatePath);
  const relative = path.relative(repoRoot, fullPath);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    return null;
  }

  const segments = relative.split(path.sep).filter(Boolean);
  if (segments[0] !== "books" || !segments[1]) {
    return null;
  }

  return segments[1];
}

function listSeriesBookIds(repoRoot) {
  const booksRoot = path.join(repoRoot, "books");
  if (!isDirectory(booksRoot)) {
    return [];
  }

  return fs
    .readdirSync(booksRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort((left, right) => left.localeCompare(right));
}

function sanitizeCliArgs(value) {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .filter((entry) => typeof entry === "string")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function resolveCliTooling(options = {}) {
  const cliCommand = typeof options.cliCommand === "string" && options.cliCommand.trim()
    ? options.cliCommand.trim()
    : "shosei";
  const cliArgs = sanitizeCliArgs(options.cliArgs);

  if (
    options.enableDevelopmentFallback &&
    cliCommand === "shosei" &&
    cliArgs.length === 0
  ) {
    const manifestPath = findRepoCliManifest(options.extensionPath);
    if (manifestPath) {
      return {
        command: "cargo",
        args: [
          "run",
          "--manifest-path",
          manifestPath,
          "--bin",
          "shosei",
          "--"
        ]
      };
    }
  }

  return {
    command: cliCommand,
    args: cliArgs
  };
}

function findRepoCliManifest(extensionPath) {
  if (typeof extensionPath !== "string" || !extensionPath.trim()) {
    return null;
  }

  const manifestPath = path.resolve(
    extensionPath,
    "..",
    "..",
    "crates",
    "shosei-cli",
    "Cargo.toml"
  );
  return fileExists(manifestPath) ? manifestPath : null;
}

function buildCliInvocation(options) {
  const cliCommand = options.cliCommand || "shosei";
  const cliArgs = sanitizeCliArgs(options.cliArgs);
  const commandParts = Array.isArray(options.commandParts) ? options.commandParts : [];
  const args = [...cliArgs, ...commandParts];

  if (options.bookId) {
    args.push("--book", options.bookId);
  }
  if (options.repoRoot && options.includePath !== false) {
    args.push("--path", options.repoRoot);
  }

  return {
    command: cliCommand,
    args,
    cwd: options.cwd || options.repoRoot || process.cwd()
  };
}

function buildInitCommandParts(options = {}) {
  const args = ["init"];

  if (typeof options.path === "string" && options.path.trim()) {
    args.push(options.path.trim());
  }

  args.push("--non-interactive");

  if (options.force) {
    args.push("--force");
  }

  appendOptionArg(args, "--config-template", options.configTemplate);
  appendOptionArg(args, "--repo-mode", options.repoMode);
  appendOptionArg(args, "--title", options.title);
  appendOptionArg(args, "--author", options.author);
  appendOptionArg(args, "--language", options.language);
  appendOptionArg(args, "--output-preset", options.outputPreset);

  return args;
}

function appendOptionArg(args, flag, value) {
  if (typeof value !== "string") {
    return;
  }

  const trimmed = value.trim();
  if (!trimmed) {
    return;
  }

  args.push(flag, trimmed);
}

function extractReportPath(output) {
  if (!output) {
    return null;
  }

  const matches = [...output.matchAll(/report:\s+(.+)$/gm)];
  if (matches.length === 0) {
    return null;
  }

  return matches[matches.length - 1][1].trim();
}

function toAbsolutePath(repoRoot, candidatePath) {
  if (!candidatePath) {
    return null;
  }
  if (path.isAbsolute(candidatePath)) {
    return candidatePath;
  }
  return path.resolve(repoRoot || process.cwd(), candidatePath);
}

function readIssuesFromReport(reportPath) {
  const contents = fs.readFileSync(reportPath, "utf8");
  const parsed = JSON.parse(contents);
  return Array.isArray(parsed.issues) ? parsed.issues : [];
}

function classifyCommandResult(result, options = {}) {
  const acceptedExitCodes = Array.isArray(options.acceptedExitCodes)
    ? options.acceptedExitCodes
    : [0];
  const stdout = typeof result?.stdout === "string" ? result.stdout.trim() : "";
  const stderr = typeof result?.stderr === "string" ? result.stderr.trim() : "";
  const fallbackMessage = options.fallbackMessage || "command completed";

  if (!acceptedExitCodes.includes(result?.code)) {
    return {
      level: "error",
      message: stderr || stdout || fallbackMessage
    };
  }

  if (result?.code !== 0) {
    if (stderr) {
      return {
        level: "error",
        message: stderr
      };
    }

    return {
      level: "warning",
      message: stdout || fallbackMessage
    };
  }

  return {
    level: "info",
    message: stdout || stderr || fallbackMessage
  };
}

module.exports = {
  buildCliInvocation,
  buildInitCommandParts,
  classifyCommandResult,
  extractReportPath,
  findRepoCliManifest,
  findRepoRoot,
  inferSeriesBookId,
  listSeriesBookIds,
  readIssuesFromReport,
  resolveCliTooling,
  sanitizeCliArgs,
  toAbsolutePath
};
