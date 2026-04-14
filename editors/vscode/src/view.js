const path = require("path");

class ShoseiViewProvider {
  constructor(vscode, options) {
    this.vscode = vscode;
    this.options = options;
    this._onDidChangeTreeData = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._onDidChangeTreeData.event;
  }

  refresh() {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(item) {
    return item;
  }

  async getChildren(element) {
    const snapshot = await this.options.getSnapshot();
    if (!element) {
      if (!snapshot.repoRoot) {
        return [
          createInfoItem(
            this.vscode,
            "No shosei repo found",
            "Open a folder with book.yml or series.yml",
            "warning"
          ),
          createActionItem(this.vscode, "Doctor", "shosei.doctor", "tools")
        ];
      }

      return [
        createGroupItem(this.vscode, "Context", "context", "info"),
        createGroupItem(this.vscode, "Actions", "actions", "play")
      ];
    }

    if (element.group === "context") {
      return buildContextItems(this.vscode, snapshot);
    }
    if (element.group === "actions") {
      return buildActionItems(this.vscode, snapshot);
    }

    return [];
  }
}

function createGroupItem(vscode, label, group, icon) {
  const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.Expanded);
  item.group = group;
  item.iconPath = new vscode.ThemeIcon(icon);
  item.contextValue = `shosei.group.${group}`;
  return item;
}

function createInfoItem(vscode, label, description, icon, command) {
  const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.None);
  item.description = description || "";
  item.iconPath = new vscode.ThemeIcon(icon || "circle-large-outline");
  item.contextValue = "shosei.info";
  if (command) {
    item.command = { command, title: label };
  }
  return item;
}

function createActionItem(vscode, label, command, icon) {
  const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.None);
  item.command = { command, title: label };
  item.iconPath = new vscode.ThemeIcon(icon || "play");
  item.contextValue = "shosei.action";
  return item;
}

function buildContextItems(vscode, snapshot) {
  const items = [
    createInfoItem(vscode, "Repo Mode", snapshot.mode || "-", "repo"),
    createInfoItem(
      vscode,
      "Repo Root",
      path.basename(snapshot.repoRoot),
      "folder"
    )
  ];

  if (snapshot.mode === "series") {
    const description = snapshot.bookId
      ? `${snapshot.bookId} (${snapshot.bookSource})`
      : "not selected";
    items.push(
      createInfoItem(
        vscode,
        "Target Book",
        description,
        "book",
        "shosei.selectBook"
      )
    );
  } else if (snapshot.mode === "single-book") {
    items.push(createInfoItem(vscode, "Target Book", "current book", "book"));
  }

  return items;
}

function buildActionItems(vscode, snapshot) {
  const items = [];

  if (snapshot.mode === "series") {
    items.push(createActionItem(vscode, "Select Book", "shosei.selectBook", "list-selection"));
  }

  items.push(createActionItem(vscode, "Explain", "shosei.explain", "search"));
  items.push(createActionItem(vscode, "Validate", "shosei.validate", "check"));
  items.push(createActionItem(vscode, "Build", "shosei.build", "gear"));
  items.push(createActionItem(vscode, "Preview", "shosei.preview", "eye"));
  items.push(createActionItem(vscode, "Preview (Watch)", "shosei.previewWatch", "debug-start"));
  items.push(createActionItem(vscode, "Doctor", "shosei.doctor", "tools"));
  items.push(createActionItem(vscode, "Page Check", "shosei.pageCheck", "check"));

  if (snapshot.mode === "series") {
    items.push(createActionItem(vscode, "Series Sync", "shosei.seriesSync", "sync"));
  }

  return items;
}

module.exports = {
  ShoseiViewProvider
};
