use std::{
    io::{self, Write},
    path::Path,
};

#[derive(Debug, Clone)]
pub struct InitWizardAnswers {
    pub config_template: String,
    pub config_profile: Option<String>,
    pub repo_mode: String,
    pub title: String,
    pub author: String,
    pub language: String,
    pub output_preset: String,
    pub run_doctor: bool,
}

pub fn init_mode_banner() -> &'static str {
    "init: answer a few questions or use --non-interactive for scaffold defaults"
}

pub fn prompt_init_wizard() -> io::Result<InitWizardAnswers> {
    let config_template = prompt_with_default(
        "作品カテゴリ [business|paper|novel|light-novel|manga]",
        "novel",
    )?;
    let config_profile = if config_template == "paper" {
        Some(prompt_with_default(
            "paper profile [paper|conference-preprint]",
            "paper",
        )?)
    } else {
        None
    };
    let default_repo_mode = if config_template == "manga" {
        "series"
    } else {
        "single-book"
    };
    let repo_mode =
        prompt_with_default("リポジトリ管理単位 [single-book|series]", default_repo_mode)?;
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
    let output_preset = prompt_with_default(
        "出力先 [kindle|print|both]",
        if config_template == "paper" {
            "print"
        } else {
            "kindle"
        },
    )?;
    let run_doctor = prompt_yes_no("生成後に shosei doctor を実行しますか", false)?;

    Ok(InitWizardAnswers {
        config_template,
        config_profile,
        repo_mode,
        title,
        author,
        language,
        output_preset,
        run_doctor,
    })
}

pub fn render_init_summary(target: &Path, answers: &InitWizardAnswers) -> String {
    let profile = answers
        .config_profile
        .as_deref()
        .unwrap_or(&answers.config_template);
    let run_doctor = if answers.run_doctor { "yes" } else { "no" };

    format!(
        "init plan:\n- path: {}\n- template: {}\n- profile: {}\n- repo mode: {}\n- title: {}\n- author: {}\n- language: {}\n- outputs: {}\n- run doctor after init: {}",
        target.display(),
        answers.config_template,
        profile,
        answers.repo_mode,
        answers.title,
        answers.author,
        answers.language,
        answers.output_preset,
        run_doctor
    )
}

pub fn confirm_init_plan() -> io::Result<bool> {
    prompt_yes_no("この内容で scaffold を生成しますか", true)
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
