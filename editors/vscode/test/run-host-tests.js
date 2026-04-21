"use strict";

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const { runTests } = require("@vscode/test-electron");

async function main() {
  const extensionDevelopmentPath =
    process.env.SHOSEI_EXTENSION_PATH || path.resolve(__dirname, "..");
  const extensionTestsPath = path.resolve(__dirname, "host", "index.js");
  const workspacePath = fs.mkdtempSync(path.join(os.tmpdir(), "shosei-vscode-host-"));
  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), "shosei-vscode-user-data-"));
  const extensionsDir = fs.mkdtempSync(path.join(os.tmpdir(), "shosei-vscode-extensions-"));
  const userSettingsDir = path.join(userDataDir, "User");
  const userSettingsPath = path.join(userSettingsDir, "settings.json");

  try {
    fs.mkdirSync(userSettingsDir, { recursive: true });
    fs.writeFileSync(
      userSettingsPath,
      JSON.stringify(
        {
          "update.mode": "none",
          "extensions.autoCheckUpdates": false,
          "extensions.autoUpdate": false
        },
        null,
        2
      )
    );

    await runTests({
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs: [
        workspacePath,
        "--disable-extensions",
        "--skip-welcome",
        "--skip-release-notes",
        "--user-data-dir",
        userDataDir,
        "--extensions-dir",
        extensionsDir
      ]
    });
  } finally {
    fs.rmSync(workspacePath, { recursive: true, force: true });
    fs.rmSync(userDataDir, { recursive: true, force: true });
    fs.rmSync(extensionsDir, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
