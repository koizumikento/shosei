use std::fs;

use shosei_core::{app, cli_api::CommandContext, repo::RepoError};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "shosei-story-commands-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_single_book(root: &std::path::Path) {
    fs::write(
        root.join("book.yml"),
        r#"
project:
  type: novel
  vcs: git
book:
  title: "Sample"
  authors:
    - "Author"
outputs:
  kindle:
    enabled: true
    target: kindle-ja
"#,
    )
    .unwrap();
}

fn write_manuscript_file(root: &std::path::Path, relative: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, "# Chapter\n").unwrap();
}

fn write_story_file(root: &std::path::Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn write_series_repo(root: &std::path::Path) {
    fs::create_dir_all(root.join("books/vol-01")).unwrap();
    fs::write(
        root.join("series.yml"),
        r#"
series:
  id: sample
  title: Sample Series
  type: novel
books:
  - id: vol-01
    path: books/vol-01
"#,
    )
    .unwrap();
    fs::write(
        root.join("books/vol-01/book.yml"),
        r#"
project:
  type: novel
book:
  title: "Vol 1"
  authors:
    - "Author"
manuscript:
  chapters: []
"#,
    )
    .unwrap();
}

#[test]
fn story_scaffold_creates_single_book_workspace() {
    let root = temp_dir("single-book");
    write_single_book(&root);

    let result = app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();

    assert!(result.summary.contains("single-book story workspace"));
    assert!(root.join("story/README.md").is_file());
    assert!(root.join("story/scenes.yml").is_file());
    assert!(root.join("story/characters/README.md").is_file());
    assert!(root.join("story/characters/_template.md").is_file());
    assert!(root.join("story/scene-template.md").is_file());
    assert!(root.join("story/structures/README.md").is_file());
    assert!(root.join("story/structures/kishotenketsu.md").is_file());
    assert!(root.join("story/structures/three-act.md").is_file());
    assert!(root.join("story/structures/save-the-cat.md").is_file());
    assert!(root.join("story/structures/heroes-journey.md").is_file());
    let template = fs::read_to_string(root.join("story/characters/_template.md")).unwrap();
    assert!(template.contains("id: 主人公"));
    let structure = fs::read_to_string(root.join("story/structures/kishotenketsu.md")).unwrap();
    assert!(structure.contains("# 起承転結テンプレート"));
}

#[test]
fn story_scaffold_creates_series_book_workspace() {
    let root = temp_dir("series-book");
    write_series_repo(&root);

    let result = app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();

    assert!(result.summary.contains("story workspace for vol-01"));
    assert!(root.join("books/vol-01/story/README.md").is_file());
    assert!(root.join("books/vol-01/story/scenes.yml").is_file());
    assert!(root.join("books/vol-01/story/scene-template.md").is_file());
    assert!(
        root.join("books/vol-01/story/structures/README.md")
            .is_file()
    );
    assert!(
        root.join("books/vol-01/story/structures/save-the-cat.md")
            .is_file()
    );
}

#[test]
fn story_scaffold_creates_shared_series_workspace() {
    let root = temp_dir("series-shared");
    write_series_repo(&root);

    let result = app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();

    assert!(result.summary.contains("shared series canon workspace"));
    assert!(root.join("shared/metadata/story/README.md").is_file());
    assert!(
        root.join("shared/metadata/story/characters/_template.md")
            .is_file()
    );
    assert!(!root.join("shared/metadata/story/scenes.yml").exists());
    assert!(
        !root
            .join("shared/metadata/story/scene-template.md")
            .exists()
    );
    assert!(!root.join("shared/metadata/story/structures").exists());
}

#[test]
fn story_scaffold_requires_book_for_series_book_scope() {
    let root = temp_dir("series-book-required");
    write_series_repo(&root);

    let error = app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::StoryScaffoldError::Repo(RepoError::BookSelectionRequired { .. })
    ));
}

#[test]
fn story_scaffold_rejects_shared_scope_in_single_book_repo() {
    let root = temp_dir("single-shared");
    write_single_book(&root);

    let error = app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::StoryScaffoldError::SharedRequiresSeries
    ));
}

#[test]
fn story_scaffold_rejects_explicit_book_with_shared_scope() {
    let root = temp_dir("series-conflicting");
    write_series_repo(&root);

    let error = app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(error, app::StoryScaffoldError::ConflictingScope));
}

#[test]
fn story_seed_creates_scenes_yml_and_scene_notes_from_template() {
    let root = temp_dir("seed-single");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();

    let result = app::story_seed(
        &CommandContext::new(&root, None, None),
        app::StorySeedOptions {
            template: "kishotenketsu".to_string(),
            force: false,
        },
    )
    .unwrap();

    assert_eq!(result.scene_count, 4);
    assert_eq!(result.created_note_count, 4);
    assert!(result.summary.contains("story seed: applied 4 seed(s)"));
    let scenes = fs::read_to_string(root.join("story/scenes.yml")).unwrap();
    assert!(scenes.contains("file: story/scene-notes/01-scene.md"));
    assert!(scenes.contains("起: 日常の提示"));
    let note = fs::read_to_string(root.join("story/scene-notes/01-scene.md")).unwrap();
    assert!(note.contains("structure_template: kishotenketsu"));
    assert!(note.contains("structure_beat: 起"));
    assert!(note.contains("# 起: 日常の提示"));
}

#[test]
fn story_seed_keeps_existing_scene_notes_without_force() {
    let root = temp_dir("seed-keep-notes");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "story/structures/custom.md",
        r#"---
scene_seeds:
  - title: 導入
    file: story/scene-notes/opening.md
    characters:
      - 主人公
---
# Custom
"#,
    );
    write_story_file(
        &root,
        "story/scene-notes/opening.md",
        "---\ncharacters:\n  - 既存\n---\n# Keep me\n",
    );

    let result = app::story_seed(
        &CommandContext::new(&root, None, None),
        app::StorySeedOptions {
            template: "custom".to_string(),
            force: false,
        },
    )
    .unwrap();

    assert_eq!(result.created_note_count, 0);
    let scenes = fs::read_to_string(root.join("story/scenes.yml")).unwrap();
    assert!(scenes.contains("file: story/scene-notes/opening.md"));
    let note = fs::read_to_string(root.join("story/scene-notes/opening.md")).unwrap();
    assert!(note.contains("# Keep me"));
    assert!(
        result
            .summary
            .contains("kept scene note story/scene-notes/opening.md")
    );
}

#[test]
fn story_seed_requires_force_to_replace_nonempty_scenes_yml() {
    let root = temp_dir("seed-requires-force");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: manuscript/01.md
    title: Existing
"#,
    )
    .unwrap();

    let error = app::story_seed(
        &CommandContext::new(&root, None, None),
        app::StorySeedOptions {
            template: "kishotenketsu".to_string(),
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::StorySeedError::ScenesRequireForce { .. }
    ));
}

#[test]
fn story_seed_force_overwrites_existing_scene_notes() {
    let root = temp_dir("seed-force");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    fs::write(
        root.join("books/vol-01/story/scenes.yml"),
        r#"
scenes:
  - file: books/vol-01/story/scene-notes/01-scene.md
    title: Existing
"#,
    )
    .unwrap();
    write_story_file(
        &root,
        "books/vol-01/story/scene-notes/01-scene.md",
        "---\ncharacters:\n  - 既存\n---\n# Old\n",
    );

    let result = app::story_seed(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySeedOptions {
            template: "three-act".to_string(),
            force: true,
        },
    )
    .unwrap();

    assert_eq!(result.scene_count, 4);
    let scenes = fs::read_to_string(root.join("books/vol-01/story/scenes.yml")).unwrap();
    assert!(scenes.contains("books/vol-01/story/scene-notes/01-scene.md"));
    let note = fs::read_to_string(root.join("books/vol-01/story/scene-notes/01-scene.md")).unwrap();
    assert!(note.contains("structure_template: three-act"));
    assert!(note.contains("# 第一幕: Setup"));
}

#[test]
fn story_map_writes_report_for_single_book_story_workspace() {
    let root = temp_dir("map-single");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: manuscript/01.md
    title: Opening
  - file: manuscript/02.md
"#,
    )
    .unwrap();

    let result = app::story_map(
        &CommandContext::new(&root, None, None),
        app::StoryMapOptions::default(),
    )
    .unwrap();

    assert!(result.summary.contains("story map: 2 scene(s)"));
    assert!(result.summary.contains("manuscript/01.md - Opening"));
    assert!(root.join("dist/reports/default-story-map.json").is_file());
}

#[test]
fn story_map_preserves_japanese_titles() {
    let root = temp_dir("map-japanese-keys");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: manuscript/01.md
    title: 導入
"#,
    )
    .unwrap();

    let result = app::story_map(
        &CommandContext::new(&root, None, None),
        app::StoryMapOptions::default(),
    )
    .unwrap();

    assert!(result.summary.contains("story map: 1 scene(s)"));
    assert!(result.summary.contains("manuscript/01.md - 導入"));
    let report = fs::read_to_string(root.join("dist/reports/default-story-map.json")).unwrap();
    assert!(report.contains("\"file\": \"manuscript/01.md\""));
    assert!(report.contains("\"title\": \"導入\""));
}

#[test]
fn story_map_requires_story_scenes_file() {
    let root = temp_dir("map-missing");
    write_single_book(&root);

    let error = app::story_map(
        &CommandContext::new(&root, None, None),
        app::StoryMapOptions::default(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::StoryMapError::MissingScenesFile { .. }
    ));
}

#[test]
fn story_map_requires_book_for_series_root() {
    let root = temp_dir("map-series-book-required");
    write_series_repo(&root);

    let error = app::story_map(
        &CommandContext::new(&root, None, None),
        app::StoryMapOptions::default(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::StoryMapError::Repo(RepoError::BookSelectionRequired { .. })
    ));
}

#[test]
fn story_check_reports_duplicate_and_missing_scene_files() {
    let root = temp_dir("check-single");
    write_single_book(&root);
    write_manuscript_file(&root, "manuscript/01.md");
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: manuscript/01.md
    title: Opening
  - file: manuscript/01.md
    title: Repeat
  - file: manuscript/99.md
    title: Missing
"#,
    )
    .unwrap();

    let result = app::story_check(
        &CommandContext::new(&root, None, None),
        app::StoryCheckOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 2);
    assert!(!result.has_errors);
    assert!(result.summary.contains("issues: 2"));
    assert!(root.join("dist/reports/default-story-check.json").is_file());
    let report = fs::read_to_string(root.join("dist/reports/default-story-check.json")).unwrap();
    assert!(report.contains("duplicate scene file entry"));
    assert!(report.contains("scene file not found"));
}

#[test]
fn story_check_marks_invalid_repo_path_as_error_issue() {
    let root = temp_dir("check-invalid-path");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: ../outside.md
"#,
    )
    .unwrap();

    let result = app::story_check(
        &CommandContext::new(&root, None, None),
        app::StoryCheckOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 1);
    assert!(result.has_errors);
    let report = fs::read_to_string(root.join("dist/reports/default-story-check.json")).unwrap();
    assert!(report.contains("invalid scene file"));
    assert!(report.contains("\"severity\": \"error\""));
}

#[test]
fn story_check_resolves_scene_frontmatter_refs_against_story_entities() {
    let root = temp_dir("check-frontmatter-refs");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "story/characters/hero.md",
        "---\naliases:\n  - Ace\n---\n# Hero\n",
    );
    write_story_file(
        &root,
        "story/locations/city-gate.md",
        "---\nid: gate-town\n---\n# Gate Town\n",
    );
    write_story_file(
        &root,
        "manuscript/01.md",
        "---\ncharacters:\n  - hero\n  - ghost\nlocations: gate-town\n---\n# Opening\n",
    );
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: manuscript/01.md
    title: Opening
"#,
    )
    .unwrap();

    let result = app::story_check(
        &CommandContext::new(&root, None, None),
        app::StoryCheckOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 1);
    assert!(!result.has_errors);
    let report = fs::read_to_string(root.join("dist/reports/default-story-check.json")).unwrap();
    assert!(report.contains("scene references unknown character `ghost`"));
    assert!(!report.contains("unknown location"));
}

#[test]
fn story_check_accepts_japanese_story_values() {
    let root = temp_dir("check-japanese-keys");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "story/characters/hero.md",
        "---\nid: 主人公\nrole: 視点人物\n---\n# 主人公\n",
    );
    write_story_file(
        &root,
        "story/locations/roof.md",
        "---\nid: 学園屋上\n---\n# 学園屋上\n",
    );
    write_story_file(
        &root,
        "manuscript/01.md",
        "---\ncharacters:\n  - 主人公\nlocations:\n  - 学園屋上\n---\n# 導入\n",
    );
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: manuscript/01.md
    title: 導入
"#,
    )
    .unwrap();

    let result = app::story_check(
        &CommandContext::new(&root, None, None),
        app::StoryCheckOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 0);
    assert!(!result.has_errors);
}

#[test]
fn story_check_reports_duplicate_story_entity_ids() {
    let root = temp_dir("check-duplicate-entity");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "story/characters/hero.md",
        "---\nid: lead\n---\n# Hero\n",
    );
    write_story_file(
        &root,
        "story/characters/rival.md",
        "---\nid: lead\n---\n# Rival\n",
    );
    write_story_file(
        &root,
        "manuscript/01.md",
        "---\ncharacters:\n  - lead\n---\n# Opening\n",
    );
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: manuscript/01.md
"#,
    )
    .unwrap();

    let result = app::story_check(
        &CommandContext::new(&root, None, None),
        app::StoryCheckOptions::default(),
    )
    .unwrap();

    assert!(result.has_errors);
    let report = fs::read_to_string(root.join("dist/reports/default-story-check.json")).unwrap();
    assert!(report.contains("duplicate character id `lead`"));
}

#[test]
fn story_check_ignores_scaffold_template_entries() {
    let root = temp_dir("check-ignore-templates");
    write_single_book(&root);
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "story/characters/hero.md",
        "---\nid: 主人公\n---\n# 実データ\n",
    );
    write_story_file(
        &root,
        "manuscript/01.md",
        "---\ncharacters:\n  - 主人公\n---\n# 導入\n",
    );
    fs::write(
        root.join("story/scenes.yml"),
        r#"
scenes:
  - file: manuscript/01.md
"#,
    )
    .unwrap();

    let result = app::story_check(
        &CommandContext::new(&root, None, None),
        app::StoryCheckOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 0);
    assert!(!result.has_errors);
    let report = fs::read_to_string(root.join("dist/reports/default-story-check.json")).unwrap();
    assert!(!report.contains("duplicate character id"));
}

#[test]
fn story_check_resolves_series_scene_refs_against_shared_story_entities() {
    let root = temp_dir("check-series-shared-refs");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: shared-hero\n---\n# Hero\n",
    );
    write_story_file(
        &root,
        "books/vol-01/manuscript/01.md",
        "---\ncharacters:\n  - shared-hero\n---\n# Opening\n",
    );
    fs::write(
        root.join("books/vol-01/story/scenes.yml"),
        r#"
scenes:
  - file: books/vol-01/manuscript/01.md
"#,
    )
    .unwrap();

    let result = app::story_check(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryCheckOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 0);
    assert!(!result.has_errors);
}

#[test]
fn story_check_does_not_report_shared_book_drift() {
    let root = temp_dir("check-series-ignore-drift");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Shared Hero\n",
    );
    write_story_file(
        &root,
        "books/vol-01/story/characters/hero.md",
        "---\nid: lead\n---\n# Local Hero\n",
    );
    write_story_file(
        &root,
        "books/vol-01/manuscript/01.md",
        "---\ncharacters:\n  - lead\n---\n# Opening\n",
    );
    fs::write(
        root.join("books/vol-01/story/scenes.yml"),
        r#"
scenes:
  - file: books/vol-01/manuscript/01.md
"#,
    )
    .unwrap();

    let result = app::story_check(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryCheckOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 0);
    assert!(!result.has_errors);
}

#[test]
fn story_drift_reports_shared_canon_drift_across_shared_and_book_story_data() {
    let root = temp_dir("check-series-shared-duplicate");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Hero\n",
    );
    write_story_file(
        &root,
        "books/vol-01/story/characters/rival.md",
        "---\nid: lead\n---\n# Rival\n",
    );
    write_story_file(
        &root,
        "books/vol-01/manuscript/01.md",
        "---\ncharacters:\n  - lead\n---\n# Opening\n",
    );
    fs::write(
        root.join("books/vol-01/story/scenes.yml"),
        r#"
scenes:
  - file: books/vol-01/manuscript/01.md
"#,
    )
    .unwrap();

    let result = app::story_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryDriftOptions::default(),
    )
    .unwrap();

    assert!(result.has_errors);
    let report = fs::read_to_string(root.join("dist/reports/vol-01-story-drift.json")).unwrap();
    assert!(report.contains("shared canon drift for character `lead`"));
    assert!(report.contains("\"drifts\""));
    assert!(report.contains("\"kind\": \"character\""));
    assert!(report.contains("\"id\": \"lead\""));
    assert!(report.contains("\"status\": \"drift\""));
    assert!(report.contains("shared/metadata/story/characters/hero.md"));
    assert!(report.contains("books/vol-01/story/characters/rival.md"));
}

#[test]
fn story_drift_warns_for_redundant_shared_and_book_copies() {
    let root = temp_dir("check-series-shared-redundant");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    let contents = "---\nid: lead\naliases:\n  - Ace\n---\n# Hero\n";
    write_story_file(&root, "shared/metadata/story/characters/hero.md", contents);
    write_story_file(&root, "books/vol-01/story/characters/hero.md", contents);

    let result = app::story_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryDriftOptions::default(),
    )
    .unwrap();

    assert_eq!(result.issue_count, 1);
    assert!(!result.has_errors);
    let report = fs::read_to_string(root.join("dist/reports/vol-01-story-drift.json")).unwrap();
    assert!(report.contains("redundant shared/book character copy for `lead`"));
}

#[test]
fn story_sync_copies_missing_shared_entity_into_book_story_workspace() {
    let root = temp_dir("sync-copy-shared");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Shared Hero\n",
    );

    let result = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            kind: Some("character".to_string()),
            id: Some("lead".to_string()),
            report: None,
            force: false,
        },
    )
    .unwrap();

    assert!(result.changed);
    assert!(
        result
            .summary
            .contains("copied shared canon character `lead`")
    );
    let synced = fs::read_to_string(root.join("books/vol-01/story/characters/hero.md")).unwrap();
    assert!(synced.contains("# Shared Hero"));
}

#[test]
fn story_sync_matches_japanese_id_value() {
    let root = temp_dir("sync-japanese-id");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: 主人公\n---\n# 共有主人公\n",
    );

    let result = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            kind: Some("character".to_string()),
            id: Some("主人公".to_string()),
            report: None,
            force: false,
        },
    )
    .unwrap();

    assert!(result.changed);
    let synced = fs::read_to_string(root.join("books/vol-01/story/characters/hero.md")).unwrap();
    assert!(synced.contains("共有主人公"));
}

#[test]
fn story_sync_requires_force_to_overwrite_diverged_book_entity() {
    let root = temp_dir("sync-force-required");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Shared Hero\n",
    );
    write_story_file(
        &root,
        "books/vol-01/story/characters/hero.md",
        "---\nid: lead\n---\n# Local Hero\n",
    );

    let error = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            kind: Some("character".to_string()),
            id: Some("lead".to_string()),
            report: None,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::StorySyncError::BookEntityConflict { .. }
    ));
}

#[test]
fn story_sync_force_overwrites_diverged_book_entity() {
    let root = temp_dir("sync-force-overwrite");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Shared Hero\n",
    );
    write_story_file(
        &root,
        "books/vol-01/story/characters/hero.md",
        "---\nid: lead\n---\n# Local Hero\n",
    );

    let result = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            kind: Some("character".to_string()),
            id: Some("lead".to_string()),
            report: None,
            force: true,
        },
    )
    .unwrap();

    assert!(result.changed);
    let synced = fs::read_to_string(root.join("books/vol-01/story/characters/hero.md")).unwrap();
    assert!(synced.contains("# Shared Hero"));
}

#[test]
fn story_sync_to_shared_copies_missing_book_entity_into_shared_story_workspace() {
    let root = temp_dir("sync-copy-to-shared");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "books/vol-01/story/characters/hero.md",
        "---\nid: lead\n---\n# Local Hero\n",
    );

    let result = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: None,
            destination: Some("shared".to_string()),
            kind: Some("character".to_string()),
            id: Some("lead".to_string()),
            report: None,
            force: false,
        },
    )
    .unwrap();

    assert!(result.changed);
    assert!(
        result
            .summary
            .contains("copied book story data character `lead`")
    );
    let synced = fs::read_to_string(root.join("shared/metadata/story/characters/hero.md")).unwrap();
    assert!(synced.contains("# Local Hero"));
}

#[test]
fn story_sync_to_shared_requires_force_to_overwrite_diverged_shared_entity() {
    let root = temp_dir("sync-to-shared-force-required");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "books/vol-01/story/characters/hero.md",
        "---\nid: lead\n---\n# Local Hero\n",
    );
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Shared Hero\n",
    );

    let error = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: None,
            destination: Some("shared".to_string()),
            kind: Some("character".to_string()),
            id: Some("lead".to_string()),
            report: None,
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::StorySyncError::SharedEntityConflict { .. }
    ));
}

#[test]
fn story_sync_to_shared_force_overwrites_diverged_shared_entity() {
    let root = temp_dir("sync-to-shared-force-overwrite");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "books/vol-01/story/characters/hero.md",
        "---\nid: lead\n---\n# Local Hero\n",
    );
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Shared Hero\n",
    );

    let result = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: None,
            destination: Some("shared".to_string()),
            kind: Some("character".to_string()),
            id: Some("lead".to_string()),
            report: None,
            force: true,
        },
    )
    .unwrap();

    assert!(result.changed);
    let synced = fs::read_to_string(root.join("shared/metadata/story/characters/hero.md")).unwrap();
    assert!(synced.contains("# Local Hero"));
}

#[test]
fn story_sync_report_requires_force() {
    let root = temp_dir("sync-report-requires-force");
    write_series_repo(&root);

    let error = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            kind: None,
            id: None,
            report: Some(root.join("dist/reports/vol-01-story-drift.json")),
            force: false,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        app::StorySyncError::ReportSyncRequiresForce
    ));
}

#[test]
fn story_sync_report_applies_shared_to_book_batch() {
    let root = temp_dir("sync-report-from-shared");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Shared Hero\n",
    );
    write_story_file(
        &root,
        "books/vol-01/story/characters/hero.md",
        "---\nid: lead\n---\n# Local Hero\n",
    );
    let shared_city = "---\nid: capital\n---\n# Capital\n";
    write_story_file(
        &root,
        "shared/metadata/story/locations/capital.md",
        shared_city,
    );
    write_story_file(
        &root,
        "books/vol-01/story/locations/capital.md",
        shared_city,
    );

    let drift = app::story_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryDriftOptions::default(),
    )
    .unwrap();

    let result = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: Some("shared".to_string()),
            destination: None,
            kind: None,
            id: None,
            report: Some(drift.report_path),
            force: true,
        },
    )
    .unwrap();

    assert!(result.changed);
    assert_eq!(result.changed_count, 1);
    assert_eq!(result.requested_count, 2);
    let synced = fs::read_to_string(root.join("books/vol-01/story/characters/hero.md")).unwrap();
    assert!(synced.contains("# Shared Hero"));
}

#[test]
fn story_sync_report_applies_book_to_shared_batch() {
    let root = temp_dir("sync-report-to-shared");
    write_series_repo(&root);
    app::story_scaffold(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryScaffoldOptions {
            shared: false,
            force: false,
        },
    )
    .unwrap();
    app::story_scaffold(
        &CommandContext::new(&root, None, None),
        app::StoryScaffoldOptions {
            shared: true,
            force: false,
        },
    )
    .unwrap();
    write_story_file(
        &root,
        "books/vol-01/story/characters/hero.md",
        "---\nid: lead\n---\n# Local Hero\n",
    );
    write_story_file(
        &root,
        "shared/metadata/story/characters/hero.md",
        "---\nid: lead\n---\n# Shared Hero\n",
    );
    let local_city = "---\nid: capital\n---\n# Capital\n";
    write_story_file(&root, "books/vol-01/story/locations/capital.md", local_city);
    write_story_file(
        &root,
        "shared/metadata/story/locations/capital.md",
        local_city,
    );

    let drift = app::story_drift(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StoryDriftOptions::default(),
    )
    .unwrap();

    let result = app::story_sync(
        &CommandContext::new(&root, Some("vol-01".to_string()), None),
        app::StorySyncOptions {
            source: None,
            destination: Some("shared".to_string()),
            kind: None,
            id: None,
            report: Some(drift.report_path),
            force: true,
        },
    )
    .unwrap();

    assert!(result.changed);
    assert_eq!(result.changed_count, 1);
    assert_eq!(result.requested_count, 2);
    let synced = fs::read_to_string(root.join("shared/metadata/story/characters/hero.md")).unwrap();
    assert!(synced.contains("# Local Hero"));
}
