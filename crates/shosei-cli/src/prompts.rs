use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct InitWizardAnswers {
    pub config_template: String,
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
    let config_template =
        prompt_with_default("作品カテゴリ [business|novel|light-novel|manga]", "novel")?;
    let default_repo_mode = if config_template == "manga" {
        "series"
    } else {
        "single-book"
    };
    let repo_mode =
        prompt_with_default("リポジトリ管理単位 [single-book|series]", default_repo_mode)?;
    let default_title = match config_template.as_str() {
        "business" => "Untitled Business Book",
        "light-novel" => "Untitled Light Novel",
        "manga" => "Untitled Manga Volume",
        _ => "Untitled Novel",
    };
    let title = prompt_with_default("タイトル", default_title)?;
    let author = prompt_with_default("著者名", "Author Name")?;
    let language = prompt_with_default("言語コード", "ja")?;
    let output_preset = prompt_with_default("出力先 [kindle|print|both]", "kindle")?;
    let run_doctor = prompt_yes_no("生成後に shosei doctor を実行しますか", false)?;

    Ok(InitWizardAnswers {
        config_template,
        repo_mode,
        title,
        author,
        language,
        output_preset,
        run_doctor,
    })
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
