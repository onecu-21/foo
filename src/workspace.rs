use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use crate::language::{Language, find_executable};

#[derive(Debug)]
pub enum Error {
    MissingVscode(Option<PathBuf>),
    Filesystem {
        path: PathBuf,
        source: io::Error,
    },
    Launch {
        application: &'static str,
        source: io::Error,
    },
    BrowserExit(std::process::ExitStatus),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingVscode(configured) => {
                f.write_str("Visual Studio Code was not found")?;
                if let Some(path) = configured {
                    write!(f, " at {}", path.display())?;
                }
                f.write_str(
                    ". Install VS Code, add `code` to PATH, or run `foo config --code <path>`.",
                )
            }
            Self::Filesystem { path, source } => {
                write!(f, "Could not create {}: {source}", path.display())
            }
            Self::Launch {
                application,
                source,
            } => write!(f, "Could not open {application}: {source}"),
            Self::BrowserExit(status) => {
                write!(f, "Could not open the default browser ({status}).")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Filesystem { source, .. } | Self::Launch { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn find_vscode(configured: Option<&Path>) -> Result<PathBuf, Error> {
    match configured {
        Some(path) => find_executable(path),
        None => find_executable(Path::new("code")).or_else(find_vscode_installation),
    }
    .map(prefer_vscode_binary)
    .ok_or_else(|| Error::MissingVscode(configured.map(Path::to_path_buf)))
}

#[cfg(windows)]
fn find_vscode_installation() -> Option<PathBuf> {
    [
        ("LOCALAPPDATA", "Programs/Microsoft VS Code/Code.exe"),
        ("ProgramFiles", "Microsoft VS Code/Code.exe"),
        ("ProgramFiles(x86)", "Microsoft VS Code/Code.exe"),
    ]
    .into_iter()
    .filter_map(|(variable, relative)| {
        env::var_os(variable).map(|base| PathBuf::from(base).join(relative))
    })
    .find_map(|path| find_executable(&path))
}

#[cfg(target_os = "macos")]
fn find_vscode_installation() -> Option<PathBuf> {
    let relative = "Applications/Visual Studio Code.app/Contents/Resources/app/bin/code";
    find_executable(&Path::new("/").join(relative)).or_else(|| {
        let home = env::var_os("HOME")?;
        find_executable(&PathBuf::from(home).join(relative))
    })
}

#[cfg(not(any(windows, target_os = "macos")))]
fn find_vscode_installation() -> Option<PathBuf> {
    None
}

fn prefer_vscode_binary(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("code.cmd"))
        && let Some(installation) = path.parent().and_then(Path::parent)
        && let Some(binary) = find_executable(&installation.join("Code.exe"))
    {
        return binary;
    }
    path
}

pub fn setup_root(workspace: &Path) -> Result<(), Error> {
    setup_vscode(workspace, "${fileDirname}")
}

pub fn prepare_problem(
    workspace: &Path,
    problem_id: u32,
    language: Language,
) -> Result<PathBuf, Error> {
    let directory = workspace.join(problem_id.to_string());
    setup_vscode(&directory, "${workspaceFolder}")?;
    write_new(
        &directory,
        language.source_name(),
        language.template().as_bytes(),
    )?;
    Ok(directory)
}

fn setup_vscode(directory: &Path, cwd: &str) -> Result<(), Error> {
    let vscode = directory.join(".vscode");
    fs::create_dir_all(&vscode).map_err(|source| Error::Filesystem {
        path: vscode.clone(),
        source,
    })?;
    let env = runner_env();
    let tasks = json!({
        "version": "2.0.0",
        "tasks": [{
            "label": "foo: Build and run",
            "type": "process",
            "command": "foo",
            "args": ["__run"],
            "options": { "cwd": cwd, "env": env },
            "group": { "kind": "build", "isDefault": true },
            "presentation": { "focus": true },
            "problemMatcher": []
        }]
    });
    let launch = json!({
        "version": "0.2.0",
        "configurations": [{
            "name": "foo: Build and run",
            "type": "node-terminal",
            "request": "launch",
            "command": "foo __run",
            "cwd": cwd,
            "env": env
        }]
    });
    write_new(&vscode, "tasks.json", format!("{tasks:#}\n").as_bytes())?;
    write_new(&vscode, "launch.json", format!("{launch:#}\n").as_bytes())
}

fn runner_env() -> Value {
    let mut variables = serde_json::Map::new();
    let executable = env::current_exe().ok();
    let parent = executable
        .as_deref()
        .filter(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.eq_ignore_ascii_case("foo"))
        })
        .and_then(Path::parent)
        .and_then(Path::to_str);
    if let Some(parent) = parent {
        let (variable, separator) = if cfg!(windows) {
            ("Path", ';')
        } else {
            ("PATH", ':')
        };
        // An executable path must occupy one PATH entry and must survive VS Code's variable substitution.
        if !parent.contains(separator) && !parent.contains("${") {
            variables.insert(
                variable.into(),
                json!(format!("{parent}{separator}${{env:{variable}}}")),
            );
        }
    }
    let config_dir = env::var_os("FOO_CONFIG_DIR")
        .filter(|path| !path.is_empty())
        .and_then(|path| std::path::absolute(path).ok());
    if let Some(path) = config_dir.as_deref().and_then(Path::to_str)
        && !path.contains("${")
    {
        variables.insert("FOO_CONFIG_DIR".into(), json!(path));
    }
    Value::Object(variables)
}

fn write_new(directory: &Path, name: &str, contents: &[u8]) -> Result<(), Error> {
    let path = directory.join(name);
    if path.is_file() {
        return Ok(());
    }
    let file_error = |source| Error::Filesystem {
        path: path.clone(),
        source,
    };
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(file_error)?;
    temporary.write_all(contents).map_err(file_error)?;
    match temporary.persist_noclobber(&path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists && path.is_file() => {
            Ok(())
        }
        Err(error) => Err(file_error(error.error)),
    }
}

pub fn open_problem(vscode: &Path, directory: &Path, source: &str) -> Result<(), Error> {
    Command::new(vscode)
        .arg("--new-window")
        .arg(directory)
        .arg(directory.join(source))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|source| Error::Launch {
            application: "Visual Studio Code",
            source,
        })
}

pub fn problem_url(problem_id: u32) -> String {
    format!("https://acmicpc.net/problem/{problem_id}")
}

pub fn open_browser(problem_id: u32) -> Result<(), Error> {
    let mut command = browser_command(&problem_url(problem_id));
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(browser_error)?;
    // xdg-open can stay attached to a newly launched browser for its entire lifetime.
    let status = if cfg!(any(windows, target_os = "macos")) {
        Some(child.wait().map_err(browser_error)?)
    } else {
        child.try_wait().map_err(browser_error)?
    };
    match status {
        Some(status) if !status.success() => Err(Error::BrowserExit(status)),
        _ => Ok(()),
    }
}

fn browser_error(source: io::Error) -> Error {
    Error::Launch {
        application: "the default browser",
        source,
    }
}

#[cfg(windows)]
fn browser_command(url: &str) -> Command {
    let system_root = env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let mut command = Command::new(system_root.join("System32/cmd.exe"));
    // The URL is built only from the fixed BOJ origin and an integer problem ID.
    command.args(["/d", "/c", "start", url]);
    command
}

#[cfg(not(windows))]
fn browser_command(url: &str) -> Command {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut command = Command::new(program);
    command.arg(url);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_f5_and_task_configuration_for_root_and_problem() {
        let workspace = tempfile::tempdir().expect("create workspace");
        setup_root(workspace.path()).expect("configure root");
        let directory =
            prepare_problem(workspace.path(), 1000, Language::Cpp).expect("prepare problem");
        for (directory, cwd) in [
            (workspace.path(), "${fileDirname}"),
            (directory.as_path(), "${workspaceFolder}"),
        ] {
            let vscode = directory.join(".vscode");
            let launch: Value =
                serde_json::from_slice(&fs::read(vscode.join("launch.json")).expect("read launch"))
                    .expect("parse launch");
            let task: Value =
                serde_json::from_slice(&fs::read(vscode.join("tasks.json")).expect("read tasks"))
                    .expect("parse tasks");
            assert_eq!(launch["configurations"][0]["type"], "node-terminal");
            assert_eq!(launch["configurations"][0]["command"], "foo __run");
            assert_eq!(launch["configurations"][0]["cwd"], cwd);
            assert_eq!(task["tasks"][0]["options"]["cwd"], cwd);
        }
        assert!(directory.join("main.cpp").is_file());
    }

    #[test]
    fn reopening_keeps_existing_source_and_vscode_files() {
        let workspace = tempfile::tempdir().expect("create workspace");
        let directory =
            prepare_problem(workspace.path(), 2557, Language::Rust).expect("prepare problem");
        let source = directory.join("main.rs");
        let launch = directory.join(".vscode/launch.json");
        fs::write(&source, "// 기존 풀이\n").expect("make existing solution");
        fs::write(&launch, "// existing configuration\n").expect("make existing setup");
        prepare_problem(workspace.path(), 2557, Language::Rust).expect("reopen problem");
        assert_eq!(
            fs::read_to_string(source).expect("read solution"),
            "// 기존 풀이\n"
        );
        assert_eq!(
            fs::read_to_string(launch).expect("read setup"),
            "// existing configuration\n"
        );
    }
}
