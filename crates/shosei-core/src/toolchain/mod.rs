use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use crate::config::PdfEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Macos,
    Windows,
    Linux,
    Other,
}

impl HostOs {
    pub fn detect() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macOS",
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Planned,
    Available,
    Missing,
    NotYetImplemented,
}

impl std::fmt::Display for ToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Planned => "planned",
            Self::Available => "available",
            Self::Missing => "missing",
            Self::NotYetImplemented => "not-yet-implemented",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRecord {
    pub key: &'static str,
    pub display_name: &'static str,
    pub status: ToolStatus,
    pub detected_as: Option<String>,
    pub resolved_path: Option<PathBuf>,
    pub version: Option<String>,
    pub install_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainReport {
    pub tools: Vec<ToolRecord>,
}

impl ToolchainReport {
    pub fn tool(&self, key: &str) -> Option<&ToolRecord> {
        self.tools.iter().find(|tool| tool.key == key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRunOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PandocPdfOptions {
    pub pdf_engine: PdfEngine,
    pub table_of_contents: bool,
    pub stylesheets: Vec<PathBuf>,
    pub variables: Vec<(String, String)>,
    pub variable_json: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy)]
pub struct PandocEpubOptions<'a> {
    pub working_dir: &'a Path,
    pub output: &'a Path,
    pub title: &'a str,
    pub language: &'a str,
    pub stylesheets: &'a [PathBuf],
    pub cover_image: Option<&'a Path>,
}

#[derive(Debug, Clone, Copy)]
pub struct PandocHtmlOptions<'a> {
    pub working_dir: &'a Path,
    pub output: &'a Path,
    pub title: &'a str,
    pub language: &'a str,
    pub stylesheets: &'a [PathBuf],
    pub table_of_contents: bool,
}

struct ToolSpec {
    key: &'static str,
    display_name: &'static str,
    candidates: &'static [&'static str],
    version_args: &'static [&'static str],
    install_hint: fn(HostOs) -> String,
}

const TOOL_SPECS: &[ToolSpec] = &[
    ToolSpec {
        key: "pandoc",
        display_name: "pandoc",
        candidates: &["pandoc"],
        version_args: &["--version"],
        install_hint: pandoc_install_hint,
    },
    ToolSpec {
        key: "epubcheck",
        display_name: "epubcheck",
        candidates: &["epubcheck", "epubcheck.cmd", "epubcheck.bat"],
        version_args: &["--version"],
        install_hint: epubcheck_install_hint,
    },
    ToolSpec {
        key: "qpdf",
        display_name: "qpdf",
        candidates: &["qpdf"],
        version_args: &["--version"],
        install_hint: qpdf_install_hint,
    },
    ToolSpec {
        key: "git",
        display_name: "git",
        candidates: &["git"],
        version_args: &["--version"],
        install_hint: git_install_hint,
    },
    ToolSpec {
        key: "git-lfs",
        display_name: "git-lfs",
        candidates: &["git-lfs"],
        version_args: &["version"],
        install_hint: git_lfs_install_hint,
    },
    ToolSpec {
        key: "weasyprint",
        display_name: "weasyprint",
        candidates: &["weasyprint"],
        version_args: &["--version"],
        install_hint: weasyprint_install_hint,
    },
    ToolSpec {
        key: "chromium",
        display_name: "Chromium PDF",
        candidates: &[],
        version_args: &["--version"],
        install_hint: chromium_install_hint,
    },
    ToolSpec {
        key: "typst",
        display_name: "typst",
        candidates: &["typst"],
        version_args: &["--version"],
        install_hint: typst_install_hint,
    },
    ToolSpec {
        key: "lualatex",
        display_name: "lualatex",
        candidates: &["lualatex"],
        version_args: &["--version"],
        install_hint: lualatex_install_hint,
    },
    ToolSpec {
        key: "pdf-engine",
        display_name: "PDF engine",
        candidates: &[],
        version_args: &[],
        install_hint: pdf_engine_install_hint,
    },
    ToolSpec {
        key: "kindle-previewer",
        display_name: "Kindle Previewer",
        candidates: &[
            "Kindle Previewer 3",
            "Kindle Previewer",
            "KindlePreviewer",
            "kindlepreviewer",
        ],
        version_args: &["--version"],
        install_hint: kindle_previewer_install_hint,
    },
];

pub fn inspect_default_toolchain() -> ToolchainReport {
    inspect_toolchain_with_env_and_direct_candidates(
        env::var_os("PATH"),
        env::var_os("PATHEXT"),
        true,
    )
}

pub fn run_pandoc_epub(
    executable: &Path,
    inputs: &[PathBuf],
    options: &PandocEpubOptions<'_>,
) -> std::io::Result<ToolRunOutput> {
    let mut command = Command::new(executable);
    command
        .current_dir(options.working_dir)
        .arg("--to")
        .arg("epub3")
        .arg("--standalone")
        .arg("--metadata")
        .arg(format!("title={}", options.title))
        .arg("--metadata")
        .arg(format!("lang={}", options.language));
    for stylesheet in options.stylesheets {
        command.arg("--css").arg(stylesheet);
    }
    if let Some(cover_image) = options.cover_image {
        command.arg("--epub-cover-image").arg(cover_image);
    }
    let command_output = command
        .arg("--output")
        .arg(options.output)
        .args(inputs)
        .output()?;

    Ok(ToolRunOutput {
        status: command_output.status,
        stdout: String::from_utf8_lossy(&command_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&command_output.stderr).into_owned(),
    })
}

pub fn run_pandoc_html(
    executable: &Path,
    inputs: &[PathBuf],
    options: &PandocHtmlOptions<'_>,
) -> std::io::Result<ToolRunOutput> {
    let output =
        run_pandoc_html_with_resource_flag(executable, inputs, options, "--embed-resources")?;
    if output.status.success() || !pandoc_embed_resources_unsupported(&output) {
        return Ok(output);
    }

    run_pandoc_html_with_resource_flag(executable, inputs, options, "--self-contained")
}

fn run_pandoc_html_with_resource_flag(
    executable: &Path,
    inputs: &[PathBuf],
    options: &PandocHtmlOptions<'_>,
    resource_flag: &str,
) -> std::io::Result<ToolRunOutput> {
    let mut command = Command::new(executable);
    command
        .current_dir(options.working_dir)
        .arg("--to")
        .arg("html5")
        .arg("--standalone")
        .arg(resource_flag)
        .arg("--metadata")
        .arg(format!("title={}", options.title))
        .arg("--metadata")
        .arg(format!("lang={}", options.language));
    for stylesheet in options.stylesheets {
        command.arg("--css").arg(stylesheet);
    }
    if options.table_of_contents {
        command.arg("--toc");
    }
    let command_output = command
        .arg("--output")
        .arg(options.output)
        .args(inputs)
        .output()?;

    Ok(ToolRunOutput {
        status: command_output.status,
        stdout: String::from_utf8_lossy(&command_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&command_output.stderr).into_owned(),
    })
}

fn pandoc_embed_resources_unsupported(output: &ToolRunOutput) -> bool {
    if output.status.success() {
        return false;
    }

    let combined = format!("{}\n{}", output.stdout, output.stderr).to_ascii_lowercase();
    combined.contains("embed-resources")
        && (combined.contains("unknown option")
            || combined.contains("unrecognized option")
            || combined.contains("did you mean")
            || combined.contains("invalid option"))
}

pub fn run_pandoc_pdf(
    executable: &Path,
    working_dir: &Path,
    inputs: &[PathBuf],
    output: &Path,
    title: &str,
    language: &str,
    options: &PandocPdfOptions,
) -> std::io::Result<ToolRunOutput> {
    let mut command = Command::new(executable);
    command
        .current_dir(working_dir)
        .arg("--to")
        .arg("pdf")
        .arg("--pdf-engine")
        .arg(options.pdf_engine.as_str())
        .arg("--standalone")
        .arg("--metadata")
        .arg(format!("title={title}"))
        .arg("--metadata")
        .arg(format!("lang={language}"));
    for stylesheet in &options.stylesheets {
        command.arg("--css").arg(stylesheet);
    }
    for (key, value) in &options.variables {
        command.arg("--variable").arg(format!("{key}={value}"));
    }
    for (key, value) in &options.variable_json {
        command.arg("--variable-json").arg(format!("{key}={value}"));
    }
    if options.table_of_contents {
        command.arg("--toc");
    }
    let command_output = command.arg("--output").arg(output).args(inputs).output()?;

    Ok(ToolRunOutput {
        status: command_output.status,
        stdout: String::from_utf8_lossy(&command_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&command_output.stderr).into_owned(),
    })
}

pub fn run_chromium_pdf(
    executable: &Path,
    input_html: &Path,
    output: &Path,
) -> std::io::Result<ToolRunOutput> {
    let command_output = Command::new(executable)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--allow-file-access-from-files")
        .arg("--no-pdf-header-footer")
        .arg(format!("--print-to-pdf={}", output.display()))
        .arg(file_url(input_html))
        .output()?;

    Ok(ToolRunOutput {
        status: command_output.status,
        stdout: String::from_utf8_lossy(&command_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&command_output.stderr).into_owned(),
    })
}

pub fn run_epubcheck(executable: &Path, input_epub: &Path) -> std::io::Result<ToolRunOutput> {
    let command_output = Command::new(executable).arg(input_epub).output()?;

    Ok(ToolRunOutput {
        status: command_output.status,
        stdout: String::from_utf8_lossy(&command_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&command_output.stderr).into_owned(),
    })
}

pub fn run_kindle_previewer_check(
    executable: &Path,
    input_epub: &Path,
    output_dir: &Path,
) -> std::io::Result<ToolRunOutput> {
    fs::create_dir_all(output_dir)?;
    let command_output = Command::new(executable)
        .arg(input_epub)
        .arg("-convert")
        .arg("-output")
        .arg(output_dir)
        .output()?;

    Ok(ToolRunOutput {
        status: command_output.status,
        stdout: String::from_utf8_lossy(&command_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&command_output.stderr).into_owned(),
    })
}

pub fn run_qpdf_check(executable: &Path, input_pdf: &Path) -> std::io::Result<ToolRunOutput> {
    let command_output = Command::new(executable)
        .arg("--check")
        .arg(input_pdf)
        .output()?;

    Ok(ToolRunOutput {
        status: command_output.status,
        stdout: String::from_utf8_lossy(&command_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&command_output.stderr).into_owned(),
    })
}

#[cfg(test)]
fn inspect_toolchain_with_env(
    path_var: Option<OsString>,
    pathext: Option<OsString>,
) -> ToolchainReport {
    inspect_toolchain_with_env_and_direct_candidates(path_var, pathext, false)
}

fn inspect_toolchain_with_env_and_direct_candidates(
    path_var: Option<OsString>,
    pathext: Option<OsString>,
    allow_direct_candidates: bool,
) -> ToolchainReport {
    let host_os = HostOs::detect();
    let mut tools = Vec::new();
    for spec in TOOL_SPECS {
        if spec.key == "pdf-engine" {
            continue;
        }
        tools.push(inspect_tool(
            spec,
            path_var.as_ref(),
            pathext.as_ref(),
            host_os,
            allow_direct_candidates,
        ));
    }
    tools.push(pdf_engine_record(&tools, host_os));

    ToolchainReport { tools }
}

fn inspect_tool(
    spec: &ToolSpec,
    path_var: Option<&OsString>,
    pathext: Option<&OsString>,
    host_os: HostOs,
    allow_direct_candidates: bool,
) -> ToolRecord {
    let candidates = tool_candidates(spec, host_os);
    let resolved = candidates.iter().find_map(|candidate| {
        find_candidate(candidate, path_var, pathext, allow_direct_candidates)
            .map(|path| (candidate.clone(), path))
    });
    let (detected_as, resolved_path) = match resolved {
        Some((candidate, path)) => (Some(candidate), Some(path)),
        None => (None, None),
    };
    let version = resolved_path
        .as_ref()
        .and_then(|path| read_version(path, spec.version_args));

    ToolRecord {
        key: spec.key,
        display_name: spec.display_name,
        status: if resolved_path.is_some() {
            ToolStatus::Available
        } else {
            ToolStatus::Missing
        },
        detected_as,
        resolved_path,
        version,
        install_hint: (spec.install_hint)(host_os),
    }
}

fn tool_candidates(spec: &ToolSpec, host_os: HostOs) -> Vec<String> {
    match spec.key {
        "chromium" => chromium_candidates(host_os),
        "kindle-previewer" => kindle_previewer_candidates(host_os),
        _ => spec
            .candidates
            .iter()
            .map(|candidate| (*candidate).to_string())
            .collect(),
    }
}

fn chromium_candidates(host_os: HostOs) -> Vec<String> {
    chromium_candidates_with_home(host_os, tool_home_dir(host_os).as_deref())
}

fn chromium_candidates_with_home(host_os: HostOs, home_dir: Option<&Path>) -> Vec<String> {
    let mut candidates = vec![
        "chrome-headless-shell".to_string(),
        "chromium-headless-shell".to_string(),
    ];
    candidates.extend(playwright_headless_shell_candidates(host_os, home_dir));
    candidates.extend(
        [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "microsoft-edge-stable",
            "msedge",
            "chrome",
        ]
        .into_iter()
        .map(str::to_string),
    );
    candidates
}

fn kindle_previewer_candidates(host_os: HostOs) -> Vec<String> {
    kindle_previewer_candidates_with_home(host_os, tool_home_dir(host_os).as_deref())
}

fn kindle_previewer_candidates_with_home(host_os: HostOs, home_dir: Option<&Path>) -> Vec<String> {
    let mut candidates = vec![
        "Kindle Previewer 3".to_string(),
        "Kindle Previewer".to_string(),
        "KindlePreviewer".to_string(),
        "kindlepreviewer".to_string(),
    ];

    match host_os {
        HostOs::Macos => candidates.extend(
            [
                "/Applications/Kindle Previewer 3.app/Contents/MacOS/Kindle Previewer 3",
                "/Applications/Kindle Previewer.app/Contents/MacOS/Kindle Previewer",
            ]
            .into_iter()
            .map(str::to_string),
        ),
        HostOs::Windows => {
            if let Some(home_dir) = home_dir {
                candidates.extend(
                    [
                        "AppData/Local/Amazon/Kindle Previewer 3/Kindle Previewer 3.exe",
                        "AppData/Local/Amazon/Kindle Previewer 3/KindlePreviewer.exe",
                    ]
                    .into_iter()
                    .map(|suffix| home_dir.join(suffix).to_string_lossy().into_owned()),
                );
            }
        }
        HostOs::Linux | HostOs::Other => {}
    }

    candidates
}

fn playwright_headless_shell_candidates(host_os: HostOs, home_dir: Option<&Path>) -> Vec<String> {
    let Some(cache_root) = playwright_cache_root(host_os, home_dir) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(cache_root) else {
        return Vec::new();
    };

    let mut installs = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("chromium_headless_shell-"))
        })
        .collect::<Vec<_>>();
    installs.sort();
    installs.reverse();

    let suffixes: &[&str] = match host_os {
        HostOs::Macos => &[
            "chrome-headless-shell-mac-arm64/chrome-headless-shell",
            "chrome-headless-shell-mac-x64/chrome-headless-shell",
        ],
        HostOs::Windows => &["chrome-headless-shell-win64/chrome-headless-shell.exe"],
        HostOs::Linux => &["chrome-headless-shell-linux64/chrome-headless-shell"],
        HostOs::Other => &[],
    };

    installs
        .into_iter()
        .flat_map(|install| {
            suffixes
                .iter()
                .map(move |suffix| install.join(suffix).to_string_lossy().into_owned())
        })
        .collect()
}

fn playwright_cache_root(host_os: HostOs, home_dir: Option<&Path>) -> Option<PathBuf> {
    let home_dir = home_dir?;
    let path = match host_os {
        HostOs::Macos => home_dir.join("Library/Caches/ms-playwright"),
        HostOs::Windows => home_dir.join("AppData/Local/ms-playwright"),
        HostOs::Linux | HostOs::Other => home_dir.join(".cache/ms-playwright"),
    };
    Some(path)
}

fn tool_home_dir(host_os: HostOs) -> Option<PathBuf> {
    match host_os {
        HostOs::Windows => env::var_os("USERPROFILE").map(PathBuf::from),
        HostOs::Macos | HostOs::Linux | HostOs::Other => env::var_os("HOME").map(PathBuf::from),
    }
}

fn pdf_engine_record(tools: &[ToolRecord], host_os: HostOs) -> ToolRecord {
    let detected = tools
        .iter()
        .filter(|tool| matches!(tool.key, "weasyprint" | "chromium" | "typst" | "lualatex"))
        .find(|tool| tool.status == ToolStatus::Available);
    ToolRecord {
        key: "pdf-engine",
        display_name: "PDF engine",
        status: if detected.is_some() {
            ToolStatus::Available
        } else {
            ToolStatus::Missing
        },
        detected_as: detected.and_then(|tool| tool.detected_as.clone()),
        resolved_path: detected.and_then(|tool| tool.resolved_path.clone()),
        version: detected.and_then(|tool| tool.version.clone()),
        install_hint: pdf_engine_install_hint(host_os),
    }
}

fn pandoc_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => "Install pandoc via Homebrew or the official pkg, then ensure `pandoc` is on PATH.".to_string(),
        HostOs::Windows => "Install pandoc with winget/chocolatey or the official installer, then reopen the shell.".to_string(),
        HostOs::Linux => "Install pandoc with your distribution package manager and ensure `pandoc` is on PATH.".to_string(),
        HostOs::Other => "Install pandoc and ensure it is available on PATH.".to_string(),
    }
}

fn epubcheck_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => "Install epubcheck with Homebrew or the official archive and expose the launcher on PATH.".to_string(),
        HostOs::Windows => "Install epubcheck from the official archive or a package manager and expose the launcher on PATH.".to_string(),
        HostOs::Linux => "Install epubcheck from the official archive or your package manager and expose the launcher on PATH.".to_string(),
        HostOs::Other => "Install epubcheck and ensure the launcher is available on PATH.".to_string(),
    }
}

fn qpdf_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => {
            "Install qpdf with Homebrew or the official packages and expose it on PATH.".to_string()
        }
        HostOs::Windows => {
            "Install qpdf from the official packages or a package manager and expose it on PATH."
                .to_string()
        }
        HostOs::Linux => {
            "Install qpdf from your package manager or the official packages and expose it on PATH."
                .to_string()
        }
        HostOs::Other => "Install qpdf and ensure it is available on PATH.".to_string(),
    }
}

fn git_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => {
            "Install Git from Xcode Command Line Tools or Homebrew and ensure `git` is on PATH."
                .to_string()
        }
        HostOs::Windows => {
            "Install Git for Windows or winget/chocolatey and ensure `git` is on PATH.".to_string()
        }
        HostOs::Linux => {
            "Install Git with your distribution package manager and ensure `git` is on PATH."
                .to_string()
        }
        HostOs::Other => "Install Git and ensure `git` is on PATH.".to_string(),
    }
}

fn git_lfs_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => "Install Git LFS via Homebrew or the official package, then run `git lfs install` once.".to_string(),
        HostOs::Windows => "Install Git LFS via winget/chocolatey or the official installer, then run `git lfs install` once.".to_string(),
        HostOs::Linux => "Install Git LFS with your distribution package manager or the official repository, then run `git lfs install` once.".to_string(),
        HostOs::Other => "Install Git LFS and run `git lfs install` once for the current user.".to_string(),
    }
}

fn weasyprint_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => "Install weasyprint with pipx/pip or Homebrew and make sure shared libraries are available.".to_string(),
        HostOs::Windows => "Install weasyprint with pipx/pip and verify the launcher is on PATH.".to_string(),
        HostOs::Linux => "Install weasyprint with pipx/pip or your package manager and ensure required shared libraries are present.".to_string(),
        HostOs::Other => "Install weasyprint and ensure the launcher is on PATH.".to_string(),
    }
}

fn chromium_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => {
            "Install chrome-headless-shell, Google Chrome, Chromium, or Microsoft Edge and ensure a compatible executable is available.".to_string()
        }
        HostOs::Windows => {
            "Install chrome-headless-shell or a Chromium-based browser such as Google Chrome or Microsoft Edge and ensure it is available.".to_string()
        }
        HostOs::Linux => {
            "Install chrome-headless-shell or a Chromium-based browser such as chromium or Google Chrome and ensure it is on PATH.".to_string()
        }
        HostOs::Other => {
            "Install chrome-headless-shell or another headless-capable Chromium executable and ensure it is available.".to_string()
        }
    }
}

fn typst_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => "Install typst via Homebrew or the official release and ensure `typst` is on PATH.".to_string(),
        HostOs::Windows => "Install typst via winget or the official release and ensure `typst` is on PATH.".to_string(),
        HostOs::Linux => "Install typst via your package manager or the official release and ensure `typst` is on PATH.".to_string(),
        HostOs::Other => "Install typst and ensure `typst` is on PATH.".to_string(),
    }
}

fn lualatex_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => {
            "Install a TeX distribution that provides `lualatex` and ensure it is on PATH."
                .to_string()
        }
        HostOs::Windows => {
            "Install TeX Live or MiKTeX with `lualatex` support and ensure it is on PATH."
                .to_string()
        }
        HostOs::Linux => {
            "Install TeX Live with `lualatex` support and ensure it is on PATH.".to_string()
        }
        HostOs::Other => {
            "Install a TeX distribution that provides `lualatex` and ensure it is on PATH."
                .to_string()
        }
    }
}

fn pdf_engine_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => {
            "Install one supported PDF engine: weasyprint, Chromium, typst, or lualatex."
                .to_string()
        }
        HostOs::Windows => {
            "Install one supported PDF engine: weasyprint, Chromium, typst, or lualatex."
                .to_string()
        }
        HostOs::Linux => {
            "Install one supported PDF engine: weasyprint, Chromium, typst, or lualatex."
                .to_string()
        }
        HostOs::Other => {
            "Install one supported PDF engine such as weasyprint, Chromium, typst, or lualatex."
                .to_string()
        }
    }
}

fn kindle_previewer_install_hint(host_os: HostOs) -> String {
    match host_os {
        HostOs::Macos => "Install Kindle Previewer from Amazon if you need device-oriented Kindle checks.".to_string(),
        HostOs::Windows => "Install Kindle Previewer from Amazon if you need device-oriented Kindle checks.".to_string(),
        HostOs::Linux => "Kindle Previewer is usually unavailable on Linux; use another host OS for device-oriented Kindle checks.".to_string(),
        HostOs::Other => "Install Kindle Previewer if you want device-oriented Kindle checks.".to_string(),
    }
}

fn find_candidate(
    candidate: &str,
    path_var: Option<&OsString>,
    pathext: Option<&OsString>,
    allow_direct_candidates: bool,
) -> Option<PathBuf> {
    let direct = Path::new(candidate);
    if allow_direct_candidates
        && (direct.is_absolute() || candidate.contains('/') || candidate.contains('\\'))
        && direct.is_file()
    {
        return Some(direct.to_path_buf());
    }
    find_in_path(candidate, path_var, pathext)
}

fn find_in_path(
    candidate: &str,
    path_var: Option<&OsString>,
    pathext: Option<&OsString>,
) -> Option<PathBuf> {
    if Path::new(candidate).is_absolute() || candidate.contains('/') || candidate.contains('\\') {
        return None;
    }
    let has_extension = Path::new(candidate).extension().is_some();
    let path_var = path_var?;

    for dir in env::split_paths(path_var) {
        if has_extension || !cfg!(windows) {
            let full_path = dir.join(candidate);
            if full_path.is_file() {
                return Some(full_path);
            }
            continue;
        }

        for ext in windows_extensions(pathext) {
            let full_path = dir.join(format!("{candidate}{ext}"));
            if full_path.is_file() {
                return Some(full_path);
            }
        }
    }

    None
}

fn file_url(path: &Path) -> String {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut value = absolute.to_string_lossy().replace('\\', "/");
    if cfg!(windows) && !value.starts_with('/') {
        value.insert(0, '/');
    }
    format!("file://{}", percent_encode_path(&value))
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

fn windows_extensions(pathext: Option<&OsString>) -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }

    pathext
        .and_then(|value| value.to_str())
        .map(|value| {
            value
                .split(';')
                .filter(|entry| !entry.is_empty())
                .map(|entry| entry.to_string())
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                ".COM".to_string(),
                ".EXE".to_string(),
                ".BAT".to_string(),
                ".CMD".to_string(),
            ]
        })
}

fn read_version(path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(path).args(args).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = stdout.lines().find(|line| !line.trim().is_empty()) {
        return Some(line.trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsStr,
        fs,
        path::{Path, PathBuf},
    };

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shosei-toolchain-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reports_missing_tools_on_empty_path() {
        let report =
            inspect_toolchain_with_env(Some(OsString::from("")), Some(OsString::from(".EXE")));

        assert_eq!(report.tool("pandoc").unwrap().status, ToolStatus::Missing);
        assert_eq!(
            report.tool("weasyprint").unwrap().status,
            ToolStatus::Missing
        );
        assert_eq!(report.tool("chromium").unwrap().status, ToolStatus::Missing);
        assert_eq!(
            report.tool("pdf-engine").unwrap().status,
            ToolStatus::Missing
        );
    }

    #[test]
    fn finds_tool_in_custom_path() {
        let dir = temp_dir("find-tool");
        let tool_path = if cfg!(windows) {
            dir.join("pandoc.exe")
        } else {
            dir.join("pandoc")
        };
        fs::write(&tool_path, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&tool_path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&tool_path, permissions).unwrap();
        }

        let report = inspect_toolchain_with_env(
            Some(OsString::from(dir.as_os_str())),
            Some(OsString::from(".EXE;.BAT;.CMD")),
        );

        let pandoc = report.tool("pandoc").unwrap();
        assert_eq!(pandoc.status, ToolStatus::Available);
        let resolved = pandoc.resolved_path.as_ref().unwrap();
        assert_eq!(resolved.parent(), tool_path.parent());
        assert_eq!(resolved.file_stem(), tool_path.file_stem());
        assert_eq!(
            resolved
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.to_ascii_lowercase()),
            tool_path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.to_ascii_lowercase())
        );
    }

    #[test]
    fn finds_tool_via_direct_candidate_path() {
        let dir = temp_dir("find-direct-tool");
        let tool_path = dir.join("chromium");
        fs::write(&tool_path, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&tool_path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&tool_path, permissions).unwrap();
        }

        let resolved = find_candidate(
            tool_path.to_str().unwrap(),
            Some(&OsString::from("")),
            Some(&OsString::from(".EXE;.BAT;.CMD")),
            true,
        );

        assert_eq!(resolved.as_deref(), Some(tool_path.as_path()));
    }

    #[test]
    fn pdf_engine_prefers_first_available_specific_tool() {
        let dir = temp_dir("find-pdf-engine");
        let tool_path = if cfg!(windows) {
            dir.join("typst.exe")
        } else {
            dir.join("typst")
        };
        fs::write(&tool_path, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&tool_path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&tool_path, permissions).unwrap();
        }

        let report = inspect_toolchain_with_env(
            Some(OsString::from(dir.as_os_str())),
            Some(OsString::from(".EXE;.BAT;.CMD")),
        );

        let pdf_engine = report.tool("pdf-engine").unwrap();
        assert_eq!(pdf_engine.status, ToolStatus::Available);
        assert_eq!(pdf_engine.detected_as.as_deref(), Some("typst"));
    }

    #[test]
    fn chromium_candidates_prefer_playwright_headless_shells_before_browser_apps() {
        let home_dir = temp_dir("playwright-headless-shell-home");
        let shell_path = playwright_cache_root(HostOs::Macos, Some(&home_dir))
            .unwrap()
            .join("chromium_headless_shell-1208")
            .join("chrome-headless-shell-mac-arm64")
            .join("chrome-headless-shell");
        fs::create_dir_all(shell_path.parent().unwrap()).unwrap();
        fs::write(&shell_path, "").unwrap();

        let candidates = chromium_candidates_with_home(HostOs::Macos, Some(&home_dir));
        let shell_index = candidates
            .iter()
            .position(|candidate| {
                let path = Path::new(candidate);
                path.file_name() == Some(OsStr::new("chrome-headless-shell"))
                    && path.components().any(|component| {
                        component.as_os_str() == OsStr::new("chromium_headless_shell-1208")
                    })
                    && path.components().any(|component| {
                        component.as_os_str() == OsStr::new("chrome-headless-shell-mac-arm64")
                    })
            })
            .unwrap();
        let chrome_app_index = candidates
            .iter()
            .position(|candidate| {
                candidate == "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
            })
            .unwrap();

        assert!(shell_index < chrome_app_index);
    }

    #[test]
    fn kindle_previewer_candidates_include_standard_app_locations() {
        let home_dir = temp_dir("kindle-previewer-home");
        let mac_candidates = kindle_previewer_candidates_with_home(HostOs::Macos, Some(&home_dir));
        assert!(mac_candidates.iter().any(|candidate| {
            candidate == "/Applications/Kindle Previewer 3.app/Contents/MacOS/Kindle Previewer 3"
        }));

        let windows_candidates =
            kindle_previewer_candidates_with_home(HostOs::Windows, Some(&home_dir));
        assert!(windows_candidates.iter().any(|candidate| {
            Path::new(candidate)
                .ends_with("AppData/Local/Amazon/Kindle Previewer 3/Kindle Previewer 3.exe")
        }));
        assert!(windows_candidates.iter().any(|candidate| {
            Path::new(candidate)
                .ends_with("AppData/Local/Amazon/Kindle Previewer 3/KindlePreviewer.exe")
        }));
    }

    #[test]
    fn run_pandoc_html_falls_back_to_self_contained_when_embed_resources_is_unsupported() {
        if !cfg!(unix) {
            return;
        }

        let dir = temp_dir("pandoc-html-fallback");
        let pandoc = dir.join("pandoc");
        let args_path = dir.join("pandoc-args.txt");
        let output = dir.join("out.html");
        let input = dir.join("chapter.md");
        fs::write(&input, "# Chapter\n").unwrap();
        fs::write(
            &pandoc,
            format!(
                r#"#!/bin/sh
printf '%s\n' "$@" >> "{}"
if printf '%s\n' "$@" | grep -q -- '--embed-resources'; then
  echo 'Unknown option --embed-resources' >&2
  exit 64
fi
out=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--output" ]; then
    out="$arg"
  fi
  prev="$arg"
done
printf '<!doctype html><html></html>' > "$out"
"#,
                args_path.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&pandoc).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&pandoc, permissions).unwrap();
        }

        let result = run_pandoc_html(
            &pandoc,
            &[input],
            &PandocHtmlOptions {
                working_dir: &dir,
                output: &output,
                title: "Sample",
                language: "ja",
                stylesheets: &[],
                table_of_contents: true,
            },
        )
        .unwrap();

        assert!(result.status.success());
        let args = fs::read_to_string(args_path).unwrap();
        assert!(args.contains("--embed-resources"));
        assert!(args.contains("--self-contained"));
        assert!(output.is_file());
    }
}
