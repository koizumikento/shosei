"use strict";

const cp = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const TARGETS = {
  "linux-x64": { executable: "shosei", platform: "linux", arch: "x64" },
  "darwin-x64": { executable: "shosei", platform: "darwin", arch: "x64" },
  "darwin-arm64": { executable: "shosei", platform: "darwin", arch: "arm64" },
  "win32-x64": { executable: "shosei.exe", platform: "win32", arch: "x64" }
};

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (!argument.startsWith("--") || index + 1 >= argv.length) {
      throw new Error(`Expected --key value argument, received: ${argument}`);
    }
    options[argument.slice(2)] = argv[index + 1];
    index += 1;
  }
  return options;
}

function run(command, args, options = {}) {
  const result = cp.spawnSync(command, args, {
    ...options,
    encoding: "utf8",
    stdio: options.stdio || "pipe",
    shell: options.shell ?? false
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(
      [
        `${command} ${args.join(" ")} failed with exit code ${result.status}`,
        result.stdout,
        result.stderr
      ]
        .filter(Boolean)
        .join("\n")
    );
  }
  return result;
}

function npxCommand() {
  return process.platform === "win32" ? "npx.cmd" : "npx";
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const target = TARGETS[options.target];
  if (!target) {
    throw new Error(`Unsupported VS Code target: ${options.target || "(missing)"}`);
  }
  if (!options.binary) {
    throw new Error("--binary is required");
  }
  if (!options.out) {
    throw new Error("--out is required");
  }
  if (process.platform !== target.platform || process.arch !== target.arch) {
    throw new Error(
      `Target ${options.target} must be packaged on ${target.platform}-${target.arch}; current runtime is ${process.platform}-${process.arch}`
    );
  }

  const extensionRoot = path.resolve(__dirname, "..");
  const packageJson = JSON.parse(
    fs.readFileSync(path.join(extensionRoot, "package.json"), "utf8")
  );
  const sourceBinary = path.resolve(options.binary);
  if (!fs.statSync(sourceBinary).isFile()) {
    throw new Error(`Bundled CLI is not a file: ${sourceBinary}`);
  }
  if (target.platform !== "win32") {
    fs.chmodSync(sourceBinary, 0o755);
  }

  const versionResult = run(sourceBinary, ["--version"]);
  const actualVersion = versionResult.stdout.trim();
  const expectedVersion = `shosei ${packageJson.version}`;
  if (actualVersion !== expectedVersion) {
    throw new Error(
      `Bundled CLI version mismatch: expected "${expectedVersion}", received "${actualVersion}"`
    );
  }

  const binRoot = path.join(extensionRoot, "bin");
  const stagedBinary = path.join(binRoot, target.executable);
  if (fs.existsSync(stagedBinary)) {
    throw new Error(`Refusing to overwrite existing staged CLI: ${stagedBinary}`);
  }

  let staged = false;
  try {
    fs.mkdirSync(binRoot, { recursive: true });
    fs.copyFileSync(sourceBinary, stagedBinary);
    staged = true;
    if (target.platform !== "win32") {
      fs.chmodSync(stagedBinary, 0o755);
    }

    run(
      npxCommand(),
      [
        "--yes",
        "@vscode/vsce@3.8.0",
        "package",
        "--target",
        options.target,
        "--out",
        path.resolve(options.out)
      ],
      {
        cwd: extensionRoot,
        stdio: "inherit",
        shell: process.platform === "win32"
      }
    );
  } finally {
    if (staged) {
      fs.rmSync(stagedBinary, { force: true });
    }
  }
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
