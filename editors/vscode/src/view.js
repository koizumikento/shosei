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
      return buildRootItems(this.vscode, snapshot);
    }

    if (Array.isArray(element.children)) {
      return element.children;
    }

    if (element.group === "context") {
      return buildContextItems(this.vscode, snapshot);
    }
    if (element.group === "structure") {
      return buildStructureItems(this.vscode, snapshot);
    }
    if (element.group === "actions") {
      return buildActionItems(this.vscode, snapshot);
    }
    if (element.group === "config") {
      return buildConfigItems(this.vscode, snapshot);
    }
    if (element.group === "toolchain") {
      return buildToolchainItems(this.vscode, snapshot);
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

function createGroupItem(vscode, label, group, icon, collapsibleState) {
  const item = new vscode.TreeItem(
    label,
    collapsibleState ?? vscode.TreeItemCollapsibleState.Expanded
  );
  item.group = group;
  item.iconPath = new vscode.ThemeIcon(icon);
  item.contextValue = `shosei.group.${group}`;
  return item;
}

function createNestedGroupItem(vscode, label, description, icon, children, collapsibleState) {
  const item = new vscode.TreeItem(
    label,
    children.length > 0
      ? collapsibleState ?? vscode.TreeItemCollapsibleState.Expanded
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

function createStorySceneNoteItem(vscode, repoRoot, repoPath, scenesPath) {
  const item = createPathItem(
    vscode,
    path.basename(repoPath),
    repoPath,
    path.resolve(repoRoot, repoPath),
    "markdown"
  );
  if (scenesPath) {
    item.contextValue = "shosei.storySceneNote";
    item.storyRepoRoot = repoRoot;
    item.storySceneFile = repoPath;
    item.storyScenesPath = scenesPath;
  }
  return item;
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

function createActionGroup(vscode, label, icon, actions, options) {
  if (actions.length === 0) {
    return null;
  }

  return createNestedGroupItem(
    vscode,
    label,
    `${actions.length} action(s)`,
    icon,
    actions,
    options?.collapsed
      ? vscode.TreeItemCollapsibleState.Collapsed
      : vscode.TreeItemCollapsibleState.Expanded
  );
}

function createToolItem(vscode, tool) {
  const item = new vscode.TreeItem(tool.display_name, vscode.TreeItemCollapsibleState.None);
  item.description = toolDescription(tool);
  item.iconPath = new vscode.ThemeIcon(toolStatusIcon(tool.status));
  item.contextValue = "shosei.tool";
  item.tooltip = buildToolTooltip(tool);
  return item;
}

function buildRootItems(vscode, snapshot) {
  if (!snapshot.repoRoot) {
    return [
      createInfoItem(
        vscode,
        "No shosei repo found",
        "Open a folder with book.yml or series.yml",
        "warning"
      ),
      createGroupItem(
        vscode,
        "Toolchain",
        "toolchain",
        "tools",
        vscode.TreeItemCollapsibleState.Collapsed
      ),
      createActionItem(vscode, "Init", "shosei.init", "new-folder"),
      createActionItem(vscode, "Doctor", "shosei.doctor", "tools")
    ];
  }

  return [
    createGroupItem(vscode, "Context", "context", "info", vscode.TreeItemCollapsibleState.Collapsed),
    createGroupItem(vscode, "Structure", "structure", "list-tree"),
    createGroupItem(vscode, "Actions", "actions", "play"),
    createGroupItem(
      vscode,
      "Resolved Config",
      "config",
      "settings-gear",
      vscode.TreeItemCollapsibleState.Collapsed
    ),
    createGroupItem(vscode, "Toolchain", "toolchain", "tools", vscode.TreeItemCollapsibleState.Collapsed)
  ];
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
  pushStoryStructureItems(vscode, items, explain);

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
  const projectType = snapshot.explain?.project_type || null;
  const chapterCount = snapshot.explain?.manuscript?.chapters?.length || 0;
  const hasManuscript = Boolean(snapshot.explain?.manuscript);
  const items = [];
  const projectActions = [
    createActionItem(vscode, "Explain", "shosei.explain", "search"),
    createActionItem(vscode, "Validate", "shosei.validate", "check"),
    createActionItem(vscode, "Build", "shosei.build", "gear"),
    createActionItem(vscode, "Preview", "shosei.preview", "eye"),
    createActionItem(vscode, "Preview (Watch)", "shosei.previewWatch", "debug-start"),
    createActionItem(vscode, "Doctor", "shosei.doctor", "tools")
  ];
  const chapterActions = [];
  const referenceActions = [
    createActionItem(vscode, "Reference Scaffold", "shosei.referenceScaffold", "new-folder"),
    createActionItem(vscode, "Reference Map", "shosei.referenceMap", "list-selection"),
    createActionItem(vscode, "Reference Check", "shosei.referenceCheck", "checklist")
  ];
  const storyActions = [
    createActionItem(vscode, "Story Scaffold", "shosei.storyScaffold", "new-folder"),
    createActionItem(vscode, "Story Seed", "shosei.storySeed", "new-file"),
    createActionItem(vscode, "Story Map", "shosei.storyMap", "list-selection"),
    createActionItem(vscode, "Story Check", "shosei.storyCheck", "checklist")
  ];
  const seriesActions = [];

  if (snapshot.mode === "series") {
    seriesActions.push(createActionItem(vscode, "Select Book", "shosei.selectBook", "list-selection"));
  }

  if (hasManuscript) {
    chapterActions.push(createActionItem(vscode, "Chapter Add", "shosei.chapterAdd", "add"));
    if (chapterCount > 1) {
      chapterActions.push(createActionItem(vscode, "Chapter Move", "shosei.chapterMove", "move"));
      chapterActions.push(createActionItem(vscode, "Chapter Remove", "shosei.chapterRemove", "trash"));
    }
    if (chapterCount > 0) {
      chapterActions.push(
        createActionItem(vscode, "Chapter Renumber", "shosei.chapterRenumber", "symbol-number")
      );
    }
  }

  if (snapshot.mode === "series") {
    referenceActions.push(
      createActionItem(vscode, "Reference Drift", "shosei.referenceDrift", "compare-changes")
    );
    referenceActions.push(createActionItem(vscode, "Reference Sync", "shosei.referenceSync", "sync"));
    storyActions.push(createActionItem(vscode, "Story Drift", "shosei.storyDrift", "compare-changes"));
    storyActions.push(createActionItem(vscode, "Story Sync", "shosei.storySync", "sync"));
  }

  if (projectType === "manga") {
    projectActions.push(createActionItem(vscode, "Page Check", "shosei.pageCheck", "check"));
  }

  if (snapshot.mode === "series") {
    seriesActions.push(createActionItem(vscode, "Series Sync", "shosei.seriesSync", "sync"));
  }

  const groups = [
    createActionGroup(vscode, "Project", "play", projectActions),
    createActionGroup(vscode, "Chapters", "list-ordered", chapterActions, { collapsed: true }),
    createActionGroup(vscode, "Reference", "bookmark", referenceActions, { collapsed: true }),
    createActionGroup(vscode, "Story", "book", storyActions, { collapsed: true }),
    createActionGroup(vscode, "Series", "library", seriesActions, { collapsed: true })
  ];

  for (const group of groups) {
    if (group) {
      items.push(group);
    }
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

function pushStoryStructureItems(vscode, items, explain) {
  if (!hasStoryStructure(explain.story)) {
    return;
  }

  if (explain.story.shared) {
    pushStoryWorkspaceItem(vscode, items, explain.repo_root, "Book Story", explain.story.current);
    pushStoryWorkspaceItem(vscode, items, explain.repo_root, "Shared Story", explain.story.shared);
    return;
  }

  pushStoryWorkspaceItem(vscode, items, explain.repo_root, "Story Files", explain.story.current);
}

function pushStoryWorkspaceItem(vscode, items, repoRoot, label, workspace) {
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

  if (workspace.scenes_path) {
    children.push(
      createPathItem(
        vscode,
        path.basename(workspace.scenes_path),
        workspace.scenes_path,
        path.resolve(repoRoot, workspace.scenes_path),
        "symbol-array"
      )
    );
  }

  const sceneNotesItem = buildStorySceneNotesItem(
    vscode,
    repoRoot,
    workspace.scene_notes,
    workspace.scenes_path
  );
  if (sceneNotesItem) {
    children.push(sceneNotesItem);
  }

  const structuresItem = buildStoryStructuresItem(vscode, repoRoot, workspace.structures);
  if (structuresItem) {
    children.push(structuresItem);
  }

  for (const kind of ["characters", "locations", "terms", "factions"]) {
    const item = buildStoryKindItem(vscode, repoRoot, workspace[kind]);
    if (item) {
      children.push(item);
    }
  }

  if (children.length === 0) {
    children.push(createInfoItem(vscode, "No story files yet", workspace.story_root, "info"));
  }

  items.push(
    createNestedGroupItem(
      vscode,
      label,
      storyWorkspaceDescription(workspace),
      "book",
      children
    )
  );
}

function buildStoryStructuresItem(vscode, repoRoot, structures) {
  if (!structures) {
    return null;
  }

  const children = [];
  if (structures.readme_path) {
    children.push(
      createPathItem(
        vscode,
        path.basename(structures.readme_path),
        structures.readme_path,
        path.resolve(repoRoot, structures.readme_path),
        "markdown"
      )
    );
  }

  for (const file of structures.files || []) {
    children.push(
      createPathItem(
        vscode,
        path.basename(file),
        file,
        path.resolve(repoRoot, file),
        "markdown"
      )
    );
  }

  if (children.length === 0) {
    return null;
  }

  return createNestedGroupItem(
    vscode,
    "Structures",
    `${(structures.files || []).length} file(s)`,
    "symbol-snippet",
    children
  );
}

function buildStorySceneNotesItem(vscode, repoRoot, sceneNotes, scenesPath) {
  if (!sceneNotes || (sceneNotes.files || []).length === 0) {
    return null;
  }

  const children = (sceneNotes.files || []).map((file) =>
    createStorySceneNoteItem(vscode, repoRoot, file, scenesPath)
  );

  return createNestedGroupItem(
    vscode,
    "Scene Notes",
    `${children.length} file(s)`,
    "note",
    children
  );
}

function buildStoryKindItem(vscode, repoRoot, kind) {
  if (!kind) {
    return null;
  }

  const children = [];
  if (kind.readme_path) {
    children.push(
      createPathItem(
        vscode,
        path.basename(kind.readme_path),
        kind.readme_path,
        path.resolve(repoRoot, kind.readme_path),
        "markdown"
      )
    );
  }

  for (const entry of kind.entries || []) {
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
    return null;
  }

  return createNestedGroupItem(
    vscode,
    storyKindLabel(kind.kind),
    `${(kind.entries || []).length} file(s)`,
    "library",
    children
  );
}

function storyKindLabel(kind) {
  switch (kind) {
    case "characters":
      return "Characters";
    case "locations":
      return "Locations";
    case "terms":
      return "Terms";
    case "factions":
      return "Factions";
    default:
      return kind || "Story";
  }
}

function storyWorkspaceDescription(workspace) {
  const entityCount = ["characters", "locations", "terms", "factions"].reduce(
    (total, kind) => total + (workspace[kind]?.entries?.length || 0),
    0
  );
  const sceneNoteCount = workspace.scene_notes?.files?.length || 0;
  const structureCount = workspace.structures?.files?.length || 0;
  const sceneNoteSuffix =
    sceneNoteCount > 0 ? ` + ${sceneNoteCount} scene note(s)` : "";
  const structureSuffix =
    structureCount > 0 ? ` + ${structureCount} structure file(s)` : "";
  if (workspace.scenes_path) {
    return `${entityCount} entity file(s) + scenes${sceneNoteSuffix}${structureSuffix}`;
  }
  return `${entityCount} entity file(s)${sceneNoteSuffix}${structureSuffix}`;
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

function hasStoryStructure(story) {
  if (!story) {
    return false;
  }

  return Boolean(story.current?.initialized || story.shared?.initialized);
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
    buildRootItems,
    buildActionItems,
    buildStructureItems
  }
};
