use std::{
    env,
    ffi::OsString,
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
    inspect_toolchain_with_env(env::var_os("PATH"), env::var_os("PATHEXT"))
}

pub fn run_pandoc_epub(
    executable: &Path,
    inputs: &[PathBuf],
    output: &Path,
    title: &str,
    language: &str,
    cover_image: Option<&Path>,
) -> std::io::Result<ToolRunOutput> {
    let mut command = Command::new(executable);
    command
        .arg("--to")
        .arg("epub3")
        .arg("--standalone")
        .arg("--metadata")
        .arg(format!("title={title}"))
        .arg("--metadata")
        .arg(format!("lang={language}"));
    if let Some(cover_image) = cover_image {
        command.arg("--epub-cover-image").arg(cover_image);
    }
    let command_output = command.arg("--output").arg(output).args(inputs).output()?;

    Ok(ToolRunOutput {
        status: command_output.status,
        stdout: String::from_utf8_lossy(&command_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&command_output.stderr).into_owned(),
    })
}

pub fn run_pandoc_pdf(
    executable: &Path,
    inputs: &[PathBuf],
    output: &Path,
    title: &str,
    language: &str,
    options: &PandocPdfOptions,
) -> std::io::Result<ToolRunOutput> {
    let mut command = Command::new(executable);
    command
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

fn inspect_toolchain_with_env(
    path_var: Option<OsString>,
    pathext: Option<OsString>,
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
) -> ToolRecord {
    let resolved = spec.candidates.iter().find_map(|candidate| {
        find_in_path(candidate, path_var, pathext).map(|path| ((*candidate).to_string(), path))
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

fn pdf_engine_record(tools: &[ToolRecord], host_os: HostOs) -> ToolRecord {
    let detected = tools
        .iter()
        .filter(|tool| matches!(tool.key, "weasyprint" | "typst" | "lualatex"))
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
            "Install one supported PDF engine: weasyprint, typst, or lualatex.".to_string()
        }
        HostOs::Windows => {
            "Install one supported PDF engine: weasyprint, typst, or lualatex.".to_string()
        }
        HostOs::Linux => {
            "Install one supported PDF engine: weasyprint, typst, or lualatex.".to_string()
        }
        HostOs::Other => {
            "Install one supported PDF engine such as weasyprint, typst, or lualatex.".to_string()
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

fn find_in_path(
    candidate: &str,
    path_var: Option<&OsString>,
    pathext: Option<&OsString>,
) -> Option<PathBuf> {
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
    use std::{fs, path::PathBuf};

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
}
