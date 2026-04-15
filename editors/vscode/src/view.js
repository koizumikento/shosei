const path = require("path");

class ShoseiViewProvider {
  constructor(vscode, options) {
    this.vscode = vscode;
    this.options = options;
    this._onDidChangeTreeData = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    this._snapshotPromise = null;
  }

  refresh() {
    this._snapshotPromise = null;
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(item) {
    return item;
  }

  async getChildren(element) {
    const snapshot = await this.getSnapshot();
    if (!element) {
      if (!snapshot.repoRoot) {
        return [
          createInfoItem(
            this.vscode,
            "No shosei repo found",
            "Open a folder with book.yml or series.yml",
            "warning"
          ),
          createGroupItem(this.vscode, "Toolchain", "toolchain", "tools"),
          createActionItem(this.vscode, "Init", "shosei.init", "new-folder"),
          createActionItem(this.vscode, "Doctor", "shosei.doctor", "tools")
        ];
      }

      return [
        createGroupItem(this.vscode, "Context", "context", "info"),
        createGroupItem(this.vscode, "Toolchain", "toolchain", "tools"),
        createGroupItem(this.vscode, "Resolved Config", "config", "settings-gear"),
        createGroupItem(this.vscode, "Structure", "structure", "list-tree"),
        createGroupItem(this.vscode, "Actions", "actions", "play")
      ];
    }

    if (Array.isArray(element.children)) {
      return element.children;
    }

    if (element.group === "context") {
      return buildContextItems(this.vscode, snapshot);
    }
    if (element.group === "toolchain") {
      return buildToolchainItems(this.vscode, snapshot);
    }
    if (element.group === "config") {
      return buildConfigItems(this.vscode, snapshot);
    }
    if (element.group === "structure") {
      return buildStructureItems(this.vscode, snapshot);
    }
    if (element.group === "actions") {
      return buildActionItems(this.vscode, snapshot);
    }

    return [];
  }

  async getSnapshot() {
    if (!this._snapshotPromise) {
      this._snapshotPromise = Promise.resolve(this.options.getSnapshot());
    }
    return this._snapshotPromise;
  }
}

function createGroupItem(vscode, label, group, icon) {
  const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.Expanded);
  item.group = group;
  item.iconPath = new vscode.ThemeIcon(icon);
  item.contextValue = `shosei.group.${group}`;
  return item;
}

function createNestedGroupItem(vscode, label, description, icon, children) {
  const item = new vscode.TreeItem(
    label,
    children.length > 0
      ? vscode.TreeItemCollapsibleState.Expanded
      : vscode.TreeItemCollapsibleState.None
  );
  item.description = description || "";
  item.iconPath = new vscode.ThemeIcon(icon || "list-tree");
  item.contextValue = "shosei.nestedGroup";
  item.children = children;
  return item;
}

function createInfoItem(vscode, label, description, icon, command) {
  const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.None);
  item.description = description || "";
  item.iconPath = new vscode.ThemeIcon(icon || "circle-large-outline");
  item.contextValue = "shosei.info";
  if (command) {
    item.command =
      typeof command === "string"
        ? { command, title: label }
        : command;
  }
  return item;
}

function createPathItem(vscode, label, description, absolutePath, icon) {
  return createInfoItem(
    vscode,
    label,
    description,
    icon,
    absolutePath
      ? {
          command: "vscode.open",
          title: label,
          arguments: [vscode.Uri.file(absolutePath)]
        }
      : undefined
  );
}

function createChapterItem(vscode, repoPath, absolutePath) {
  const item = createPathItem(
    vscode,
    path.basename(repoPath),
    repoPath,
    absolutePath,
    "markdown"
  );
  item.contextValue = "shosei.chapter";
  item.chapterPath = repoPath;
  return item;
}

function createActionItem(vscode, label, command, icon) {
  const item = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.None);
  item.command = { command, title: label };
  item.iconPath = new vscode.ThemeIcon(icon || "play");
  item.contextValue = "shosei.action";
  return item;
}

function createToolItem(vscode, tool) {
  const item = new vscode.TreeItem(tool.display_name, vscode.TreeItemCollapsibleState.None);
  item.description = toolDescription(tool);
  item.iconPath = new vscode.ThemeIcon(toolStatusIcon(tool.status));
  item.contextValue = "shosei.tool";
  item.tooltip = buildToolTooltip(tool);
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

function buildConfigItems(vscode, snapshot) {
  if (snapshot.configError) {
    return [
      createInfoItem(
        vscode,
        "Config unavailable",
        snapshot.configError,
        "warning"
      )
    ];
  }

  if (!snapshot.explain) {
    const message =
      snapshot.mode === "series" && !snapshot.bookId
        ? "Select a book to load resolved config"
        : "Resolved config is not loaded";
    return [createInfoItem(vscode, "Resolved Config", message, "info")];
  }

  const explain = snapshot.explain;
  const items = [
    createPathItem(vscode, "Config File", path.basename(explain.config_path), explain.config_path, "file-code"),
    createInfoItem(vscode, "Title", formatWithOrigin(explain.title, originFor(explain, "book.title")), "book"),
    createInfoItem(vscode, "Project Type", formatWithOrigin(explain.project_type, originFor(explain, "project.type")), "symbol-class"),
    createInfoItem(vscode, "Language", formatWithOrigin(explain.language, originFor(explain, "book.language")), "globe"),
    createInfoItem(vscode, "Profile", formatWithOrigin(explain.profile, originFor(explain, "book.profile")), "tag"),
    createInfoItem(vscode, "Writing Mode", formatWithOrigin(explain.writing_mode, originFor(explain, "book.writing_mode")), "text-size"),
    createInfoItem(vscode, "Binding", formatWithOrigin(explain.binding, originFor(explain, "layout.binding")), "layout"),
    createInfoItem(
      vscode,
      "Outputs",
      explain.outputs.length > 0 ? explain.outputs.join(", ") : "none",
      "broadcast"
    )
  ];

  if (hasEditorialContent(explain.editorial)) {
    items.push(
      createInfoItem(
        vscode,
        "Editorial",
        editorialSummary(explain.editorial),
        "note"
      )
    );
  }

  if (snapshot.mode === "series" && explain.shared_paths) {
    items.push(
      createInfoItem(
        vscode,
        "Shared Metadata",
        explain.shared_paths.metadata.length > 0
          ? explain.shared_paths.metadata.join(", ")
          : "none",
        "folder-library"
      )
    );
  }

  return items;
}

function buildToolchainItems(vscode, snapshot) {
  if (snapshot.doctorError) {
    return [
      createInfoItem(
        vscode,
        "Toolchain unavailable",
        snapshot.doctorError,
        "warning"
      )
    ];
  }

  if (!snapshot.doctor) {
    return [createInfoItem(vscode, "Toolchain", "Doctor status is not loaded", "info")];
  }

  const doctor = snapshot.doctor;
  const items = [
    createInfoItem(vscode, "Host", doctor.host_os, "device-desktop"),
    createInfoItem(
      vscode,
      "Required",
      `${doctor.required_available} available, ${doctor.required_missing} missing, ${doctor.required_pending} pending`,
      "tools"
    ),
    createInfoItem(
      vscode,
      "Optional",
      `${doctor.optional_available} available, ${doctor.optional_missing} missing, ${doctor.optional_pending} pending`,
      "tools"
    )
  ];

  const requiredTools = (doctor.tools || []).filter((tool) => tool.category === "required");
  const optionalTools = (doctor.tools || []).filter((tool) => tool.category === "optional");

  if (requiredTools.length > 0) {
    items.push(createInfoItem(vscode, "Required Tools", "", "symbol-key"));
  }
  for (const tool of requiredTools) {
    items.push(createToolItem(vscode, tool));
  }

  if (optionalTools.length > 0) {
    items.push(createInfoItem(vscode, "Optional Tools", "", "beaker"));
  }
  for (const tool of optionalTools) {
    items.push(createToolItem(vscode, tool));
  }

  return items;
}

function buildStructureItems(vscode, snapshot) {
  if (snapshot.configError) {
    return [
      createInfoItem(
        vscode,
        "Structure unavailable",
        snapshot.configError,
        "warning"
      )
    ];
  }

  if (!snapshot.explain) {
    const message =
      snapshot.mode === "series" && !snapshot.bookId
        ? "Select a book to inspect structure"
        : "Structure is not loaded";
    return [createInfoItem(vscode, "Structure", message, "info")];
  }

  const explain = snapshot.explain;
  const items = [];

  if (explain.manuscript) {
    const chapterItems = explain.manuscript.chapters.map((chapter) =>
      createChapterItem(vscode, chapter, path.resolve(explain.repo_root, chapter))
    );
    items.push(
      createNestedGroupItem(
        vscode,
        "Chapters",
        `${chapterItems.length} file(s)`,
        "list-ordered",
        chapterItems
      )
    );

    if (explain.manuscript.frontmatter.length > 0) {
      const frontmatterItems = explain.manuscript.frontmatter.map((entry) =>
        createPathItem(
          vscode,
          path.basename(entry),
          entry,
          path.resolve(explain.repo_root, entry),
          "file"
        )
      );
      items.push(
        createNestedGroupItem(
          vscode,
          "Frontmatter",
          `${frontmatterItems.length} file(s)`,
          "list-flat",
          frontmatterItems
        )
      );
    }

    if (explain.manuscript.backmatter.length > 0) {
      const backmatterItems = explain.manuscript.backmatter.map((entry) =>
        createPathItem(
          vscode,
          path.basename(entry),
          entry,
          path.resolve(explain.repo_root, entry),
          "file"
        )
      );
      items.push(
        createNestedGroupItem(
          vscode,
          "Backmatter",
          `${backmatterItems.length} file(s)`,
          "list-flat",
          backmatterItems
        )
      );
    }
  } else {
    items.push(
      createInfoItem(
        vscode,
        "Chapters",
        "No manuscript chapter structure for this project type",
        "info"
      )
    );
  }

  pushReferenceStructureItems(vscode, items, explain);

  if (hasEditorialContent(explain.editorial)) {
    const editorialItems = [];
    pushEditorialPathItem(
      vscode,
      editorialItems,
      explain.repo_root,
      "Style Guide",
      explain.editorial.style_path
    );
    pushEditorialPathItem(
      vscode,
      editorialItems,
      explain.repo_root,
      "Claims",
      explain.editorial.claims_path
    );
    pushEditorialPathItem(
      vscode,
      editorialItems,
      explain.repo_root,
      "Figures",
      explain.editorial.figures_path
    );
    pushEditorialPathItem(
      vscode,
      editorialItems,
      explain.repo_root,
      "Freshness",
      explain.editorial.freshness_path
    );
    items.push(
      createNestedGroupItem(
        vscode,
        "Editorial Files",
        "Open sidecar config",
        "note",
        editorialItems
      )
    );
  }

  return items;
}

function pushReferenceStructureItems(vscode, items, explain) {
  if (!hasReferenceStructure(explain.references)) {
    return;
  }

  if (explain.references.shared) {
    pushReferenceWorkspaceItem(
      vscode,
      items,
      explain.repo_root,
      "Book References",
      explain.references.current
    );
    pushReferenceWorkspaceItem(
      vscode,
      items,
      explain.repo_root,
      "Shared References",
      explain.references.shared
    );
    return;
  }

  pushReferenceWorkspaceItem(
    vscode,
    items,
    explain.repo_root,
    "Reference Files",
    explain.references.current
  );
}

function buildActionItems(vscode, snapshot) {
  const items = [];
  const projectType = snapshot.explain?.project_type || null;
  const chapterCount = snapshot.explain?.manuscript?.chapters?.length || 0;
  const hasManuscript = Boolean(snapshot.explain?.manuscript);

  if (snapshot.mode === "series") {
    items.push(createActionItem(vscode, "Select Book", "shosei.selectBook", "list-selection"));
  }

  if (hasManuscript) {
    items.push(createActionItem(vscode, "Chapter Add", "shosei.chapterAdd", "add"));
    if (chapterCount > 1) {
      items.push(createActionItem(vscode, "Chapter Move", "shosei.chapterMove", "move"));
      items.push(createActionItem(vscode, "Chapter Remove", "shosei.chapterRemove", "trash"));
    }
    if (chapterCount > 0) {
      items.push(
        createActionItem(vscode, "Chapter Renumber", "shosei.chapterRenumber", "symbol-number")
      );
    }
  }

  items.push(createActionItem(vscode, "Explain", "shosei.explain", "search"));
  items.push(createActionItem(vscode, "Validate", "shosei.validate", "check"));
  items.push(createActionItem(vscode, "Build", "shosei.build", "gear"));
  items.push(createActionItem(vscode, "Preview", "shosei.preview", "eye"));
  items.push(createActionItem(vscode, "Preview (Watch)", "shosei.previewWatch", "debug-start"));
  items.push(createActionItem(vscode, "Reference Scaffold", "shosei.referenceScaffold", "new-folder"));
  items.push(createActionItem(vscode, "Reference Map", "shosei.referenceMap", "list-selection"));
  items.push(createActionItem(vscode, "Reference Check", "shosei.referenceCheck", "checklist"));
  if (snapshot.mode === "series") {
    items.push(createActionItem(vscode, "Reference Drift", "shosei.referenceDrift", "compare-changes"));
    items.push(createActionItem(vscode, "Reference Sync", "shosei.referenceSync", "sync"));
  }
  items.push(createActionItem(vscode, "Doctor", "shosei.doctor", "tools"));

  if (projectType === "manga") {
    items.push(createActionItem(vscode, "Page Check", "shosei.pageCheck", "check"));
  }

  if (snapshot.mode === "series") {
    items.push(createActionItem(vscode, "Series Sync", "shosei.seriesSync", "sync"));
  }

  return items;
}

function pushEditorialPathItem(vscode, items, repoRoot, label, repoPath) {
  if (!repoPath) {
    return;
  }

  items.push(
    createPathItem(
      vscode,
      label,
      path.basename(repoPath),
      path.resolve(repoRoot, repoPath),
      "file"
    )
  );
}

function pushReferenceWorkspaceItem(vscode, items, repoRoot, label, workspace) {
  if (!workspace?.initialized) {
    return;
  }

  const children = [];
  if (workspace.readme_path) {
    children.push(
      createPathItem(
        vscode,
        path.basename(workspace.readme_path),
        workspace.readme_path,
        path.resolve(repoRoot, workspace.readme_path),
        "markdown"
      )
    );
  }

  for (const entry of workspace.entries || []) {
    children.push(
      createPathItem(
        vscode,
        path.basename(entry),
        entry,
        path.resolve(repoRoot, entry),
        "markdown"
      )
    );
  }

  if (children.length === 0) {
    children.push(
      createInfoItem(vscode, "No reference entries yet", workspace.entries_root, "info")
    );
  }

  items.push(
    createNestedGroupItem(
      vscode,
      label,
      `${(workspace.entries || []).length} entry(s)`,
      "bookmark",
      children
    )
  );
}

function formatWithOrigin(value, origin) {
  return origin ? `${value} [${origin}]` : value;
}

function originFor(explain, field) {
  const match = (explain.values || []).find((value) => value.field === field);
  return match ? match.origin : null;
}

function hasEditorialContent(editorial) {
  if (!editorial) {
    return false;
  }

  return Boolean(
    editorial.style_path ||
      editorial.claims_path ||
      editorial.figures_path ||
      editorial.freshness_path ||
      editorial.style_rule_count ||
      editorial.claim_count ||
      editorial.figure_count ||
      editorial.freshness_count
  );
}

function hasReferenceStructure(references) {
  if (!references) {
    return false;
  }

  return Boolean(
    references.current?.initialized || references.shared?.initialized
  );
}

function editorialSummary(editorial) {
  return [
    `${editorial.style_rule_count} rules`,
    `${editorial.claim_count} claims`,
    `${editorial.figure_count} figures`,
    `${editorial.freshness_count} freshness`
  ].join(", ");
}

function toolDescription(tool) {
  if (tool.status !== "available") {
    return tool.status;
  }

  if (tool.detected_as && tool.detected_as !== tool.display_name) {
    return tool.detected_as;
  }

  return tool.version || tool.status;
}

function buildToolTooltip(tool) {
  const lines = [`status: ${tool.status}`];
  if (tool.detected_as) {
    lines.push(`detected as: ${tool.detected_as}`);
  }
  if (tool.version) {
    lines.push(`version: ${tool.version}`);
  }
  if (tool.resolved_path) {
    lines.push(`path: ${tool.resolved_path}`);
  }
  if (tool.status !== "available" && tool.install_hint) {
    lines.push("");
    lines.push(tool.install_hint);
  }
  return lines.join("\n");
}

function toolStatusIcon(status) {
  switch (status) {
    case "available":
      return "pass-filled";
    case "not-yet-implemented":
      return "clock";
    case "planned":
      return "circle-large-outline";
    case "missing":
    default:
      return "warning";
  }
}

module.exports = {
  ShoseiViewProvider,
  __test: {
    buildActionItems,
    buildStructureItems
  }
};
