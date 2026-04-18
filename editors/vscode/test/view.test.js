const path = require("path");
const test = require("node:test");
const assert = require("node:assert/strict");

const view = require("../src/view");

function createFakeVscode() {
  class TreeItem {
    constructor(label, collapsibleState) {
      this.label = label;
      this.collapsibleState = collapsibleState;
    }
  }

  class ThemeIcon {
    constructor(id) {
      this.id = id;
    }
  }

  return {
    TreeItem,
    ThemeIcon,
    Uri: {
      file(fsPath) {
        return { fsPath };
      }
    },
    TreeItemCollapsibleState: {
      None: 0,
      Collapsed: 1,
      Expanded: 2
    }
  };
}

test("buildRootItems keeps the sidebar groups in the frequent-use order", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildRootItems(vscode, {
    repoRoot: "/tmp/book"
  });

  assert.deepEqual(
    items.map((item) => item.label),
    ["Context", "Structure", "Actions", "Resolved Config", "Toolchain"]
  );
  assert.equal(items[0].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.equal(items[1].collapsibleState, vscode.TreeItemCollapsibleState.Expanded);
  assert.equal(items[2].collapsibleState, vscode.TreeItemCollapsibleState.Expanded);
  assert.equal(items[3].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.equal(items[4].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
});

test("buildStructureItems nests chapters and editorial files under structure groups", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildStructureItems(vscode, {
    explain: {
      repo_root: "/tmp/book",
      manuscript: {
        chapters: ["manuscript/01.md", "manuscript/02.md"],
        frontmatter: [],
        backmatter: []
      },
      editorial: {
        style_path: "editorial/style.yml",
        claims_path: "editorial/claims.yml",
        figures_path: null,
        freshness_path: null,
        style_rule_count: 1,
        claim_count: 2,
        figure_count: 0,
        freshness_count: 0
      }
    }
  });

  assert.equal(items.length, 2);
  assert.equal(items[0].label, "Chapters");
  assert.equal(items[0].collapsibleState, vscode.TreeItemCollapsibleState.Expanded);
  assert.equal(items[0].children.length, 2);
  assert.equal(items[0].children[0].label, "01.md");
  assert.equal(items[0].children[0].description, "manuscript/01.md");

  assert.equal(items[1].label, "Editorial Files");
  assert.equal(items[1].collapsibleState, vscode.TreeItemCollapsibleState.Expanded);
  assert.equal(items[1].children.length, 2);
  assert.equal(items[1].children[0].label, "Style Guide");
  assert.equal(items[1].children[0].description, "style.yml");
});

test("buildStructureItems creates frontmatter and backmatter nested groups when present", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildStructureItems(vscode, {
    explain: {
      repo_root: "/tmp/book",
      manuscript: {
        chapters: ["manuscript/01.md"],
        frontmatter: ["manuscript/frontmatter/title.md"],
        backmatter: ["manuscript/backmatter/afterword.md"]
      },
      editorial: null
    }
  });

  assert.deepEqual(
    items.map((item) => item.label),
    ["Chapters", "Frontmatter", "Backmatter"]
  );
  assert.equal(items[1].children[0].label, "title.md");
  assert.equal(items[2].children[0].label, "afterword.md");
  assert.equal(
    items[1].children[0].command.arguments[0].fsPath,
    path.resolve("/tmp/book", "manuscript/frontmatter/title.md")
  );
});

test("buildStructureItems includes single-book reference files when the workspace is initialized", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildStructureItems(vscode, {
    explain: {
      repo_root: "/tmp/book",
      manuscript: {
        chapters: ["manuscript/01.md"],
        frontmatter: [],
        backmatter: []
      },
      references: {
        current: {
          initialized: true,
          entries_root: "references/entries",
          readme_path: "references/README.md",
          entries: ["references/entries/market.md"]
        },
        shared: null
      },
      editorial: null
    }
  });

  assert.deepEqual(
    items.map((item) => item.label),
    ["Chapters", "Reference Files"]
  );
  assert.equal(items[1].description, "1 entry(s)");
  assert.equal(items[1].children[0].label, "README.md");
  assert.equal(items[1].children[1].label, "market.md");
  assert.equal(items[1].children[1].description, "references/entries/market.md");
  assert.equal(
    items[1].children[1].command.arguments[0].fsPath,
    path.resolve("/tmp/book", "references/entries/market.md")
  );
});

test("buildStructureItems includes single-book story files when the workspace is initialized", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildStructureItems(vscode, {
    explain: {
      repo_root: "/tmp/book",
      manuscript: {
        chapters: ["manuscript/01.md"],
        frontmatter: [],
        backmatter: []
      },
      story: {
        current: {
          initialized: true,
          story_root: "story",
          readme_path: "story/README.md",
          scenes_path: "story/scenes.yml",
          scene_notes: {
            root: "story/scene-notes",
            files: ["story/scene-notes/01-scene.md"]
          },
          structures: {
            root: "story/structures",
            readme_path: "story/structures/README.md",
            files: [
              "story/structures/kishotenketsu.md",
              "story/structures/three-act.md"
            ]
          },
          characters: {
            kind: "characters",
            root: "story/characters",
            readme_path: "story/characters/README.md",
            entries: ["story/characters/hero.md"]
          },
          locations: { kind: "locations", root: "story/locations", readme_path: null, entries: [] },
          terms: { kind: "terms", root: "story/terms", readme_path: null, entries: [] },
          factions: { kind: "factions", root: "story/factions", readme_path: null, entries: [] }
        },
        shared: null
      },
      editorial: null
    }
  });

  assert.deepEqual(items.map((item) => item.label), ["Chapters", "Story Files"]);
  assert.equal(
    items[1].description,
    "1 entity file(s) + scenes + 1 scene note(s) + 2 structure file(s)"
  );
  assert.equal(items[1].children[0].label, "README.md");
  assert.equal(items[1].children[1].label, "scenes.yml");
  assert.equal(items[1].children[2].label, "Scene Notes");
  assert.equal(items[1].children[2].children[0].label, "01-scene.md");
  assert.equal(items[1].children[2].children[0].contextValue, "shosei.storySceneNote");
  assert.equal(items[1].children[3].label, "Structures");
  assert.equal(items[1].children[3].children[1].label, "kishotenketsu.md");
  assert.equal(items[1].children[4].label, "Characters");
  assert.equal(items[1].children[4].children[1].label, "hero.md");
});

test("buildStructureItems includes book and shared reference groups for series repos", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildStructureItems(vscode, {
    explain: {
      repo_root: "/tmp/series",
      manuscript: {
        chapters: ["books/vol-01/manuscript/01.md"],
        frontmatter: [],
        backmatter: []
      },
      references: {
        current: {
          initialized: true,
          entries_root: "books/vol-01/references/entries",
          readme_path: "books/vol-01/references/README.md",
          entries: ["books/vol-01/references/entries/local.md"]
        },
        shared: {
          initialized: true,
          entries_root: "shared/metadata/references/entries",
          readme_path: "shared/metadata/references/README.md",
          entries: ["shared/metadata/references/entries/shared.md"]
        }
      },
      editorial: null
    }
  });

  assert.deepEqual(
    items.map((item) => item.label),
    ["Chapters", "Book References", "Shared References"]
  );
  assert.equal(items[1].children[1].label, "local.md");
  assert.equal(items[2].children[1].label, "shared.md");
});

test("buildStructureItems includes book and shared story groups for series repos", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildStructureItems(vscode, {
    explain: {
      repo_root: "/tmp/series",
      manuscript: {
        chapters: ["books/vol-01/manuscript/01.md"],
        frontmatter: [],
        backmatter: []
      },
      story: {
        current: {
          initialized: true,
          story_root: "books/vol-01/story",
          readme_path: "books/vol-01/story/README.md",
          scenes_path: "books/vol-01/story/scenes.yml",
          scene_notes: {
            root: "books/vol-01/story/scene-notes",
            files: ["books/vol-01/story/scene-notes/01-opening.md"]
          },
          structures: {
            root: "books/vol-01/story/structures",
            readme_path: "books/vol-01/story/structures/README.md",
            files: ["books/vol-01/story/structures/save-the-cat.md"]
          },
          characters: {
            kind: "characters",
            root: "books/vol-01/story/characters",
            readme_path: null,
            entries: ["books/vol-01/story/characters/lead.md"]
          },
          locations: { kind: "locations", root: "books/vol-01/story/locations", readme_path: null, entries: [] },
          terms: { kind: "terms", root: "books/vol-01/story/terms", readme_path: null, entries: [] },
          factions: { kind: "factions", root: "books/vol-01/story/factions", readme_path: null, entries: [] }
        },
        shared: {
          initialized: true,
          story_root: "shared/metadata/story",
          readme_path: "shared/metadata/story/README.md",
          scenes_path: null,
          scene_notes: null,
          structures: null,
          characters: {
            kind: "characters",
            root: "shared/metadata/story/characters",
            readme_path: null,
            entries: ["shared/metadata/story/characters/hero.md"]
          },
          locations: { kind: "locations", root: "shared/metadata/story/locations", readme_path: null, entries: [] },
          terms: { kind: "terms", root: "shared/metadata/story/terms", readme_path: null, entries: [] },
          factions: { kind: "factions", root: "shared/metadata/story/factions", readme_path: null, entries: [] }
        }
      },
      editorial: null
    }
  });

  assert.deepEqual(items.map((item) => item.label), ["Chapters", "Book Story", "Shared Story"]);
  assert.equal(items[1].children[2].label, "Scene Notes");
  assert.equal(items[1].children[2].children[0].label, "01-opening.md");
  assert.equal(items[1].children[2].children[0].contextValue, "shosei.storySceneNote");
  assert.equal(items[1].children[3].label, "Structures");
  assert.equal(items[1].children[3].children[1].label, "save-the-cat.md");
  assert.equal(items[1].children[4].label, "Characters");
  assert.equal(items[1].children[4].children[0].label, "lead.md");
  assert.equal(items[2].children[1].label, "Characters");
  assert.equal(items[2].children[1].children[0].label, "hero.md");
});

test("buildActionItems groups single-book actions by workflow", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildActionItems(vscode, {
    mode: "single-book",
    explain: {
      project_type: "novel",
      manuscript: {
        chapters: ["manuscript/01.md"]
      }
    }
  });

  assert.deepEqual(
    items.map((item) => item.label),
    ["Project", "Chapters", "Reference", "Story"]
  );
  assert.equal(items[0].collapsibleState, vscode.TreeItemCollapsibleState.Expanded);
  assert.equal(items[1].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.equal(items[2].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.equal(items[3].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.deepEqual(
    items[0].children.map((item) => item.label),
    ["Explain", "Validate", "Build", "Preview", "Preview (Watch)", "Doctor"]
  );
  assert.deepEqual(items[1].children.map((item) => item.label), ["Chapter Add", "Chapter Renumber"]);
  assert.deepEqual(
    items[2].children.map((item) => item.label),
    ["Reference Scaffold", "Reference Map", "Reference Check"]
  );
  assert.deepEqual(
    items[3].children.map((item) => item.label),
    ["Story Scaffold", "Story Seed", "Story Map", "Story Check"]
  );
});

test("buildActionItems keeps advanced series actions under collapsed groups", () => {
  const vscode = createFakeVscode();
  const items = view.__test.buildActionItems(vscode, {
    mode: "series",
    explain: {
      project_type: "novel",
      manuscript: {
        chapters: ["books/vol-01/manuscript/01.md", "books/vol-01/manuscript/02.md"]
      }
    }
  });

  assert.deepEqual(items.map((item) => item.label), [
    "Project",
    "Chapters",
    "Reference",
    "Story",
    "Series"
  ]);
  assert(items[2].children.some((item) => item.label === "Reference Drift"));
  assert(items[2].children.some((item) => item.label === "Reference Sync"));
  assert(items[3].children.some((item) => item.label === "Story Drift"));
  assert(items[3].children.some((item) => item.label === "Story Sync"));
  assert(items[4].children.some((item) => item.label === "Select Book"));
  assert(items[4].children.some((item) => item.label === "Series Sync"));
  assert.equal(items[1].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.equal(items[2].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.equal(items[3].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.equal(items[4].collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
});
