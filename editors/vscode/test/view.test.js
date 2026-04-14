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
