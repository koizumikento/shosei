"use strict";

const cp = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const AdmZip = require("adm-zip");

function npmCommand() {
  return process.platform === "win32" ? "npm.cmd" : "npm";
}

function main() {
  const extensionRoot = path.resolve(__dirname, "..");
  const packageJson = JSON.parse(
    fs.readFileSync(path.join(extensionRoot, "package.json"), "utf8")
  );
  const vsixPath = path.join(extensionRoot, `shosei-vscode-${packageJson.version}.vsix`);
  const extractRoot = fs.mkdtempSync(path.join(os.tmpdir(), "shosei-vscode-vsix-"));

  try {
    fs.rmSync(vsixPath, { force: true });
    cp.execFileSync(npmCommand(), ["run", "package"], {
      cwd: extensionRoot,
      stdio: "inherit"
    });

    new AdmZip(vsixPath).extractAllTo(extractRoot, true);

    cp.execFileSync("node", [path.join(__dirname, "run-host-tests.js")], {
      cwd: extensionRoot,
      stdio: "inherit",
      env: {
        ...process.env,
        SHOSEI_EXTENSION_PATH: path.join(extractRoot, "extension"),
        SHOSEI_HOST_TEST_SCOPE: "package"
      }
    });
  } finally {
    fs.rmSync(extractRoot, { recursive: true, force: true });
  }
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
