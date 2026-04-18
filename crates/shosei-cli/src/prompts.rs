use std::{
    io::{self, Write},
    path::Path,
};

#[derive(Debug, Clone)]
pub struct InitWizardAnswers {
    pub config_template: String,
    pub config_profile: Option<String>,
    pub repo_mode: String,
    pub initial_series_book_id: Option<String>,
    pub title: String,
    pub author: String,
    pub language: String,
    pub output_preset: String,
    pub writing_mode: String,
    pub binding: String,
    pub print_target: Option<String>,
    pub print_trim_size: Option<String>,
    pub print_bleed: Option<String>,
    pub print_crop_marks: Option<bool>,
    pub print_sides: Option<String>,
    pub print_max_pages: Option<u64>,
    pub manga_spread_policy_for_kindle: Option<String>,
    pub manga_front_color_pages: Option<u64>,
    pub manga_body_mode: Option<String>,
    pub include_introduction: Option<bool>,
    pub include_afterword: Option<bool>,
    pub initialize_git: bool,
    pub git_lfs: bool,
    pub generate_sample: bool,
    pub run_doctor: bool,
}

pub fn init_mode_banner() -> &'static str {
    "init: answer a few questions or use --non-interactive for scaffold defaults"
}

pub fn prompt_init_wizard(
    config_template_override: Option<&str>,
    config_profile_override: Option<&str>,
) -> io::Result<InitWizardAnswers> {
    let config_template = if let Some(config_template) = config_template_override {
        config_template.to_string()
    } else {
        prompt_choice_with_default(
            "作品カテゴリ [business|paper|novel|light-novel|manga]",
            "novel",
            &["business", "paper", "novel", "light-novel", "manga"],
        )?
    };
    let config_profile = if config_template == "paper" {
        if let Some(config_profile) = config_profile_override {
            Some(config_profile.to_string())
        } else {
            Some(prompt_choice_with_default(
                "paper profile [paper|conference-preprint]",
                "paper",
                &["paper", "conference-preprint"],
            )?)
        }
    } else {
        None
    };

    let default_repo_mode = if config_template == "manga" {
        "series"
    } else {
        "single-book"
    };
    let repo_mode = prompt_choice_with_default(
        "リポジトリ管理単位 [single-book|series]",
        default_repo_mode,
        &["single-book", "series"],
    )?;
    let initial_series_book_id = if repo_mode == "series" {
        Some(prompt_series_book_id("初期 book id", "vol-01")?)
    } else {
        None
    };

    let default_title = match config_profile
        .as_deref()
        .unwrap_or(config_template.as_str())
    {
        "business" => "Untitled Business Book",
        "paper" => "Untitled Paper",
        "conference-preprint" => "Untitled Conference Preprint",
        "light-novel" => "Untitled Light Novel",
        "manga" => "Untitled Manga Volume",
        _ => "Untitled Novel",
    };
    let title = prompt_with_default("タイトル", default_title)?;
    let author = prompt_with_default("著者名", "Author Name")?;
    let language = prompt_with_default("言語コード", "ja")?;

    let output_preset = prompt_choice_with_default(
        "出力先 [kindle|print|both]",
        if config_template == "paper" {
            "print"
        } else {
            "kindle"
        },
        &["kindle", "print", "both"],
    )?;

    let default_writing_mode = match config_template.as_str() {
        "business" | "paper" => "horizontal-ltr",
        _ => "vertical-rl",
    };
    let writing_mode = prompt_choice_with_default(
        "本文方向 [horizontal-ltr|vertical-rl]",
        default_writing_mode,
        &["horizontal-ltr", "vertical-rl"],
    )?;
    let binding = prompt_choice_with_default(
        "綴じ方向 [left|right]",
        if writing_mode == "horizontal-ltr" {
            "left"
        } else {
            "right"
        },
        &["left", "right"],
    )?;

    let includes_print = matches!(output_preset.as_str(), "print" | "both");
    let (
        print_target,
        print_trim_size,
        print_bleed,
        print_crop_marks,
        print_sides,
        print_max_pages,
    ) = if includes_print && config_template != "manga" {
        let profile = config_profile
            .as_deref()
            .unwrap_or(config_template.as_str());
        let default_print_target = if matches!(profile, "paper" | "conference-preprint") {
            "print-jp-pdfx4"
        } else {
            "print-jp-pdfx1a"
        };
        let target = prompt_choice_with_default(
            "print target [print-jp-pdfx1a|print-jp-pdfx4]",
            default_print_target,
            &["print-jp-pdfx1a", "print-jp-pdfx4"],
        )?;
        let trim_size = prompt_choice_with_default(
            "仕上がりサイズ [A4|A5|B6|bunko]",
            if matches!(profile, "paper" | "conference-preprint") {
                "A4"
            } else {
                "bunko"
            },
            &["A4", "A5", "B6", "bunko"],
        )?;
        let bleed = prompt_with_default(
            "bleed",
            if matches!(profile, "paper" | "conference-preprint") {
                "0mm"
            } else {
                "3mm"
            },
        )?;
        let crop_marks = prompt_yes_no(
            "crop marks を有効にしますか",
            !matches!(profile, "paper" | "conference-preprint"),
        )?;
        let (sides, max_pages) = if profile == "conference-preprint" {
            (
                Some(prompt_choice_with_default(
                    "印刷面 [simplex|duplex]",
                    "duplex",
                    &["simplex", "duplex"],
                )?),
                Some(prompt_u64_with_default("最大ページ数", 2)?),
            )
        } else {
            (None, None)
        };
        (
            Some(target),
            Some(trim_size),
            Some(bleed),
            Some(crop_marks),
            sides,
            max_pages,
        )
    } else {
        (None, None, None, None, None, None)
    };

    let (manga_spread_policy_for_kindle, manga_front_color_pages, manga_body_mode) =
        if config_template == "manga" {
            (
                Some(prompt_choice_with_default(
                    "Kindle 見開きポリシー [split|single-page|skip]",
                    "split",
                    &["split", "single-page", "skip"],
                )?),
                Some(prompt_u64_with_default("巻頭カラー枚数", 0)?),
                Some(prompt_choice_with_default(
                    "本文ページモード [monochrome|mixed|color]",
                    "monochrome",
                    &["monochrome", "mixed", "color"],
                )?),
            )
        } else {
            (None, None, None)
        };

    let (include_introduction, include_afterword) = if config_template != "manga" {
        (
            Some(prompt_yes_no("前付きを追加しますか（はじめに等）", false)?),
            Some(prompt_yes_no("後付きを追加しますか（おわりに等）", false)?),
        )
    } else {
        (None, None)
    };

    let initialize_git = prompt_yes_no("Git リポジトリを初期化しますか", false)?;
    let git_lfs = prompt_yes_no("Git LFS を前提にしますか", true)?;
    let generate_sample = prompt_yes_no("サンプル原稿を生成しますか", true)?;
    let run_doctor = prompt_yes_no("生成後に shosei doctor を実行しますか", false)?;

    Ok(InitWizardAnswers {
        config_template,
        config_profile,
        repo_mode,
        initial_series_book_id,
        title,
        author,
        language,
        output_preset,
        writing_mode,
        binding,
        print_target,
        print_trim_size,
        print_bleed,
        print_crop_marks,
        print_sides,
        print_max_pages,
        manga_spread_policy_for_kindle,
        manga_front_color_pages,
        manga_body_mode,
        include_introduction,
        include_afterword,
        initialize_git,
        git_lfs,
        generate_sample,
        run_doctor,
    })
}

pub fn render_init_summary(target: &Path, answers: &InitWizardAnswers) -> String {
    let profile = answers
        .config_profile
        .as_deref()
        .unwrap_or(&answers.config_template);
    let run_doctor = if answers.run_doctor { "yes" } else { "no" };
    let initialize_git = if answers.initialize_git { "yes" } else { "no" };
    let git_lfs = if answers.git_lfs { "yes" } else { "no" };
    let generate_sample = if answers.generate_sample { "yes" } else { "no" };

    let mut lines = vec![
        "init plan:".to_string(),
        format!("- path: {}", target.display()),
        format!("- template: {}", answers.config_template),
        format!("- profile: {profile}"),
        format!("- repo mode: {}", answers.repo_mode),
    ];
    if let Some(book_id) = answers.initial_series_book_id.as_ref() {
        lines.push(format!("- initial book id: {book_id}"));
    }
    lines.extend([
        format!("- title: {}", answers.title),
        format!("- author: {}", answers.author),
        format!("- language: {}", answers.language),
        format!("- outputs: {}", answers.output_preset),
        format!("- writing mode: {}", answers.writing_mode),
        format!("- binding: {}", answers.binding),
    ]);
    if let Some(print_target) = answers.print_target.as_ref() {
        lines.push(format!("- print target: {print_target}"));
    }
    if let Some(trim_size) = answers.print_trim_size.as_ref() {
        lines.push(format!("- print trim size: {trim_size}"));
    }
    if let Some(bleed) = answers.print_bleed.as_ref() {
        lines.push(format!("- print bleed: {bleed}"));
    }
    if let Some(crop_marks) = answers.print_crop_marks {
        lines.push(format!(
            "- print crop marks: {}",
            if crop_marks { "yes" } else { "no" }
        ));
    }
    if let Some(sides) = answers.print_sides.as_ref() {
        lines.push(format!("- print sides: {sides}"));
    }
    if let Some(max_pages) = answers.print_max_pages {
        lines.push(format!("- print max pages: {max_pages}"));
    }
    if let Some(spread_policy) = answers.manga_spread_policy_for_kindle.as_ref() {
        lines.push(format!("- manga spread policy: {spread_policy}"));
    }
    if let Some(front_color_pages) = answers.manga_front_color_pages {
        lines.push(format!("- manga front color pages: {front_color_pages}"));
    }
    if let Some(body_mode) = answers.manga_body_mode.as_ref() {
        lines.push(format!("- manga body mode: {body_mode}"));
    }
    if let Some(include_introduction) = answers.include_introduction {
        lines.push(format!(
            "- include introduction: {}",
            if include_introduction { "yes" } else { "no" }
        ));
    }
    if let Some(include_afterword) = answers.include_afterword {
        lines.push(format!(
            "- include afterword: {}",
            if include_afterword { "yes" } else { "no" }
        ));
    }
    lines.extend([
        format!("- initialize git: {initialize_git}"),
        format!("- git lfs: {git_lfs}"),
        format!("- generate sample: {generate_sample}"),
        format!("- run doctor after init: {run_doctor}"),
    ]);
    lines.join("\n")
}

pub fn confirm_init_plan() -> io::Result<bool> {
    prompt_yes_no("この内容で scaffold を生成しますか", true)
}

fn prompt_choice_with_default(label: &str, default: &str, choices: &[&str]) -> io::Result<String> {
    loop {
        let value = prompt_with_default(label, default)?;
        if choices.iter().any(|choice| *choice == value) {
            return Ok(value);
        }
        println!("error: choose one of: {}", choices.join(", "));
    }
}

fn prompt_with_default(label: &str, default: &str) -> io::Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn prompt_u64_with_default(label: &str, default: u64) -> io::Result<u64> {
    loop {
        let value = prompt_with_default(label, &default.to_string())?;
        match value.parse::<u64>() {
            Ok(number) => return Ok(number),
            Err(_) => println!("error: enter a non-negative integer"),
        }
    }
}

fn prompt_yes_no(label: &str, default: bool) -> io::Result<bool> {
    let suffix = if default { "Y/n" } else { "y/N" };
    print!("{label} [{suffix}]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        Ok(default)
    } else {
        Ok(matches!(trimmed.as_str(), "y" | "yes"))
    }
}

fn prompt_series_book_id(label: &str, default: &str) -> io::Result<String> {
    loop {
        let book_id = prompt_with_default(label, default)?;
        if let Some(reason) = validate_series_book_id(&book_id) {
            println!("error: invalid initial book id: {reason}");
            continue;
        }
        return Ok(book_id);
    }
}

fn validate_series_book_id(book_id: &str) -> Option<&'static str> {
    if book_id.is_empty() {
        Some("book id must not be empty")
    } else if matches!(book_id, "." | "..") {
        Some("book id must not be `.` or `..`")
    } else if book_id.contains('/') || book_id.contains('\\') {
        Some("book id must be a single path segment")
    } else if book_id.chars().any(char::is_whitespace) {
        Some("book id must not contain whitespace")
    } else {
        None
    }
}
