use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

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
    pub resolved_path: Option<PathBuf>,
    pub version: Option<String>,
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

struct ToolSpec {
    key: &'static str,
    display_name: &'static str,
    candidates: &'static [&'static str],
    version_args: &'static [&'static str],
    implemented: bool,
}

const TOOL_SPECS: &[ToolSpec] = &[
    ToolSpec {
        key: "pandoc",
        display_name: "pandoc",
        candidates: &["pandoc"],
        version_args: &["--version"],
        implemented: true,
    },
    ToolSpec {
        key: "epubcheck",
        display_name: "epubcheck",
        candidates: &["epubcheck", "epubcheck.cmd", "epubcheck.bat"],
        version_args: &["--version"],
        implemented: true,
    },
    ToolSpec {
        key: "git",
        display_name: "git",
        candidates: &["git"],
        version_args: &["--version"],
        implemented: true,
    },
    ToolSpec {
        key: "git-lfs",
        display_name: "git-lfs",
        candidates: &["git-lfs"],
        version_args: &["version"],
        implemented: true,
    },
    ToolSpec {
        key: "pdf-engine",
        display_name: "PDF engine",
        candidates: &[],
        version_args: &[],
        implemented: false,
    },
    ToolSpec {
        key: "kindle-previewer",
        display_name: "Kindle Previewer",
        candidates: &[],
        version_args: &[],
        implemented: false,
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
) -> std::io::Result<ToolRunOutput> {
    let command_output = Command::new(executable)
        .arg("--to")
        .arg("epub3")
        .arg("--standalone")
        .arg("--metadata")
        .arg(format!("title={title}"))
        .arg("--metadata")
        .arg(format!("lang={language}"))
        .arg("--output")
        .arg(output)
        .args(inputs)
        .output()?;

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
) -> std::io::Result<ToolRunOutput> {
    let command_output = Command::new(executable)
        .arg("--to")
        .arg("pdf")
        .arg("--standalone")
        .arg("--metadata")
        .arg(format!("title={title}"))
        .arg("--metadata")
        .arg(format!("lang={language}"))
        .arg("--output")
        .arg(output)
        .args(inputs)
        .output()?;

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
    ToolchainReport {
        tools: TOOL_SPECS
            .iter()
            .map(|spec| inspect_tool(spec, path_var.as_ref(), pathext.as_ref()))
            .collect(),
    }
}

fn inspect_tool(
    spec: &ToolSpec,
    path_var: Option<&OsString>,
    pathext: Option<&OsString>,
) -> ToolRecord {
    if !spec.implemented {
        return ToolRecord {
            key: spec.key,
            display_name: spec.display_name,
            status: ToolStatus::NotYetImplemented,
            resolved_path: None,
            version: None,
        };
    }

    let resolved_path = spec
        .candidates
        .iter()
        .find_map(|candidate| find_in_path(candidate, path_var, pathext));
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
        resolved_path,
        version,
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
            report.tool("pdf-engine").unwrap().status,
            ToolStatus::NotYetImplemented
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
        assert_eq!(pandoc.resolved_path.as_ref(), Some(&tool_path));
    }
}
