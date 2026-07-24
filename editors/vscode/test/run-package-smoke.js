"use strict";

const cp = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const AdmZip = require("adm-zip");

function npmCommand() {
  return process.platform === "win32" ? "npm.cmd" : "npm";
}

function runNpm(args, options) {
  cp.execFileSync(npmCommand(), args, {
    ...options,
    shell: process.platform === "win32"
  });
}

function main() {
  const extensionRoot = path.resolve(__dirname, "..");
  const packageJson = JSON.parse(
    fs.readFileSync(path.join(extensionRoot, "package.json"), "utf8")
  );
  const target = process.env.SHOSEI_VSCE_TARGET;
  const bundledCli = process.env.SHOSEI_BUNDLED_CLI;
  if (Boolean(target) !== Boolean(bundledCli)) {
    throw new Error("SHOSEI_VSCE_TARGET and SHOSEI_BUNDLED_CLI must be set together");
  }
  const vsixPath = process.env.SHOSEI_VSIX_PATH
    ? path.resolve(process.env.SHOSEI_VSIX_PATH)
    : path.join(
        extensionRoot,
        target
          ? `shosei-vscode-${packageJson.version}-${target}.vsix`
          : `shosei-vscode-${packageJson.version}.vsix`
      );
  const extractRoot = fs.mkdtempSync(path.join(os.tmpdir(), "shosei-vscode-vsix-"));

  try {
    fs.rmSync(vsixPath, { force: true });
    if (target) {
      runNpm(
        [
          "run",
          "package:platform",
          "--",
          "--target",
          target,
          "--binary",
          path.resolve(bundledCli),
          "--out",
          vsixPath
        ],
        {
          cwd: extensionRoot,
          stdio: "inherit"
        }
      );
    } else {
      runNpm(["run", "package", "--", "--out", vsixPath], {
        cwd: extensionRoot,
        stdio: "inherit"
      });
    }

    const archive = new AdmZip(vsixPath);
    verifyPackagedCliMode(archive, target);
    archive.extractAllTo(extractRoot, true);
    verifyPackagedCli(
      path.join(extractRoot, "extension"),
      packageJson.version,
      target
    );

    cp.execFileSync("node", [path.join(__dirname, "run-host-tests.js")], {
      cwd: extensionRoot,
      stdio: "inherit",
      env: {
        ...process.env,
        SHOSEI_EXTENSION_PATH: path.join(extractRoot, "extension"),
        SHOSEI_HOST_TEST_SCOPE: "package",
        SHOSEI_EXPECT_BUNDLED_CLI: target ? "1" : "0"
      }
    });
  } finally {
    fs.rmSync(extractRoot, { recursive: true, force: true });
  }
}

function verifyPackagedCli(extensionPath, version, target) {
  const core = require(path.join(extensionPath, "src", "core.js"));
  const runtime = targetRuntime(target);
  const tooling = core.resolveCliTooling({
    cliCommand: "",
    cliArgs: [],
    extensionPath,
    platform: runtime.platform,
    arch: runtime.arch
  });

  if (!target) {
    if (tooling.source !== "path") {
      throw new Error(`Universal VSIX unexpectedly resolved ${tooling.source} CLI`);
    }
    return;
  }

  if (tooling.source !== "bundled") {
    throw new Error(`Platform VSIX resolved ${tooling.source} CLI instead of bundled CLI`);
  }
  if (runtime.platform !== "win32") {
    fs.chmodSync(tooling.command, 0o755);
  }
  const actualVersion = cp.execFileSync(tooling.command, ["--version"], {
    encoding: "utf8"
  }).trim();
  if (actualVersion !== `shosei ${version}`) {
    throw new Error(
      `Packaged CLI version mismatch: expected shosei ${version}, received ${actualVersion}`
    );
  }
}

function verifyPackagedCliMode(archive, target) {
  if (!target || target.startsWith("win32-")) {
    return;
  }
  const entry = archive.getEntry("extension/bin/shosei");
  if (!entry) {
    throw new Error(`Platform VSIX does not include extension/bin/shosei for ${target}`);
  }
  const unixMode = entry.attr >>> 16;
  if ((unixMode & 0o111) === 0) {
    throw new Error(`Bundled CLI does not have an executable mode in the VSIX: ${target}`);
  }
}

function targetRuntime(target) {
  const runtimes = {
    "linux-x64": { platform: "linux", arch: "x64" },
    "darwin-x64": { platform: "darwin", arch: "x64" },
    "darwin-arm64": { platform: "darwin", arch: "arm64" },
    "win32-x64": { platform: "win32", arch: "x64" }
  };
  if (!target) {
    return { platform: process.platform, arch: process.arch };
  }
  const runtime = runtimes[target];
  if (!runtime) {
    throw new Error(`Unsupported package smoke target: ${target}`);
  }
  return runtime;
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
