use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    C,
    #[value(alias = "c++")]
    Cpp,
    #[value(alias = "python3")]
    Python,
    #[value(name = "pypy", alias = "pypy3")]
    PyPy,
    Rust,
    Java,
    Go,
    #[value(name = "javascript", alias = "js")]
    JavaScript,
    Kotlin,
    #[value(name = "csharp", alias = "c#", alias = "cs", alias = "dotnet")]
    CSharp,
    #[value(alias = "cob")]
    Cobol,
}

impl Language {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Python => "python",
            Self::PyPy => "pypy",
            Self::Rust => "rust",
            Self::Java => "java",
            Self::Go => "go",
            Self::JavaScript => "javascript",
            Self::Kotlin => "kotlin",
            Self::CSharp => "csharp",
            Self::Cobol => "cobol",
        }
    }

    pub const fn source_name(self) -> &'static str {
        match self {
            Self::C => "main.c",
            Self::Cpp => "main.cpp",
            Self::Python | Self::PyPy => "main.py",
            Self::Rust => "main.rs",
            Self::Java => "Main.java",
            Self::Go => "main.go",
            Self::JavaScript => "main.js",
            Self::Kotlin => "Main.kt",
            Self::CSharp => "main.cs",
            Self::Cobol => "main.cob",
        }
    }

    pub const fn default_compiler(self) -> &'static str {
        match self {
            Self::C => "clang",
            Self::Cpp => "clang++",
            Self::Python => "python3",
            Self::PyPy => "pypy3",
            Self::Rust => "rustc",
            Self::Java => "java",
            Self::Go => "go",
            Self::JavaScript => "node",
            Self::Kotlin => "kotlinc",
            Self::CSharp => "dotnet",
            Self::Cobol => "cobc",
        }
    }

    pub(crate) const fn template(self) -> &'static str {
        match self {
            Self::C => "#include <stdio.h>\n\nint main(void) {\n    return 0;\n}\n",
            Self::Cpp => concat!(
                "#include <iostream>\n\nint main() {\n",
                "    std::ios::sync_with_stdio(false);\n    std::cin.tie(nullptr);\n",
                "    return 0;\n}\n",
            ),
            Self::Python | Self::PyPy => {
                "def main():\n    pass\n\n\nif __name__ == \"__main__\":\n    main()\n"
            }
            Self::Rust => "fn main() {\n}\n",
            Self::Java => {
                "public class Main {\n    public static void main(String[] args) {\n    }\n}\n"
            }
            Self::Go => "package main\n\nfunc main() {\n}\n",
            Self::JavaScript => "'use strict';\n\nfunction main() {\n}\n\nmain();\n",
            Self::Kotlin => "fun main() {\n}\n",
            Self::CSharp => {
                "using System;\n\nclass Program\n{\n    static void Main()\n    {\n    }\n}\n"
            }
            Self::Cobol => concat!(
                "IDENTIFICATION DIVISION.\n",
                "PROGRAM-ID. MAIN.\n\n",
                "PROCEDURE DIVISION.\n",
                "    STOP RUN.\n",
            ),
        }
    }

    pub const fn vscode_extension(self) -> Option<&'static str> {
        match self {
            Self::C | Self::Cpp => Some("llvm-vs-code-extensions.vscode-clangd"),
            Self::Python | Self::PyPy => Some("ms-python.python"),
            Self::Rust => Some("rust-lang.rust-analyzer"),
            Self::Java => Some("vscjava.vscode-java-pack"),
            Self::Go => Some("golang.go"),
            Self::CSharp => Some("ms-dotnettools.csharp"),
            Self::Cobol => Some("bitlang.cobol"),
            Self::JavaScript | Self::Kotlin => None,
        }
    }

    pub fn find_compiler(self, configured: Option<&Path>) -> Result<PathBuf, Error> {
        if matches!(self, Self::C | Self::Cpp) && configured.is_some_and(is_msvc) {
            return Err(Error::Heresy(self.default_compiler()));
        }

        let default = Path::new(self.default_compiler());
        let compiler = match (self, configured) {
            (_, Some(executable)) => find_executable(executable),
            (Self::Java, None) => find_java(),
            (Self::Python, None) if cfg!(windows) => find_executable(default)
                .or_else(|| find_executable(Path::new("python")))
                .or_else(|| find_executable(Path::new("py"))),
            _ => find_executable(default),
        }
        .ok_or_else(|| Error::MissingCompiler {
            language: self,
            executable: configured.unwrap_or(default).to_path_buf(),
        })?;

        if self == Self::Kotlin && find_java().is_none() {
            return Err(Error::MissingJavaForKotlin);
        }
        Ok(compiler)
    }

    pub fn run(self, compiler: &Path, directory: &Path) -> Result<(), Error> {
        let source = self.source_name();
        let source_path = directory.join(source);
        if !source_path.is_file() {
            return Err(Error::MissingSource(source_path));
        }

        let mut command = Command::new(compiler);
        command.current_dir(directory);
        let command = match self {
            Self::Python | Self::PyPy | Self::JavaScript => {
                if self == Self::Python && cfg!(windows) && executable_is(compiler, "py") {
                    command.arg("-3");
                }
                return execute(command.arg(source), "Run");
            }
            Self::CSharp => return run_csharp(compiler, directory, &source_path),
            Self::Java => {
                return execute(
                    command.args(["--source", "21", source]),
                    "Build and run (JDK 21+)",
                );
            }
            Self::C | Self::Cpp => {
                let standard = if self == Self::C {
                    "-std=c17"
                } else {
                    "-std=c++17"
                };
                command.args([standard, "-O2", "-Wall", "-Wextra", source, "-o"])
            }
            Self::Rust => command.args(["--edition=2021", "-O", source, "-o"]),
            Self::Go => command.args(["build", "-o"]),
            Self::Kotlin => command.args([source, "-include-runtime", "-d"]),
            Self::Cobol => command.args(["-x", "-free", "-O2", source, "-o"]),
        };

        let build = tempfile::tempdir().map_err(Error::TemporaryDirectory)?;
        let binary_name = match self {
            Self::Kotlin => "solution.jar",
            _ if cfg!(windows) => "solution.exe",
            _ => "solution",
        };
        let binary = build.path().join(binary_name);
        command.arg(&binary);
        if self == Self::Go {
            command.arg(source);
        }
        execute(command, "Build")?;

        if self == Self::Kotlin {
            let java = find_java().ok_or(Error::MissingJavaForKotlin)?;
            execute(
                Command::new(java)
                    .current_dir(directory)
                    .arg("-jar")
                    .arg(binary),
                "Run",
            )
        } else {
            execute(Command::new(binary).current_dir(directory), "Run")
        }
    }

    fn install_help(self) -> &'static str {
        match self {
            Self::C | Self::Cpp => {
                "Install LLVM Clang and the platform's C/C++ linker and libraries."
            }
            Self::Python => "Install Python 3.",
            Self::PyPy => "Install PyPy3.",
            Self::Rust => "Install the stable Rust toolchain with rustup.",
            Self::Java => {
                "Install JDK 21 or later and set JAVA_HOME to its installation directory."
            }
            Self::Go => "Install the Go toolchain.",
            Self::JavaScript => "Install Node.js.",
            Self::Kotlin => "Install the Kotlin/JVM command-line compiler and a JDK.",
            Self::CSharp => "Install the .NET SDK 6 or later.",
            Self::Cobol => "Install GnuCOBOL (cobc).",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Python => "Python",
            Self::PyPy => "PyPy",
            Self::Rust => "Rust",
            Self::Java => "Java",
            Self::Go => "Go",
            Self::JavaScript => "JavaScript",
            Self::Kotlin => "Kotlin",
            Self::CSharp => "C#",
            Self::Cobol => "COBOL",
        })
    }
}

#[derive(Debug)]
pub enum Error {
    Heresy(&'static str),
    MissingCompiler {
        language: Language,
        executable: PathBuf,
    },
    MissingJavaForKotlin,
    MissingSource(PathBuf),
    TemporaryDirectory(io::Error),
    Spawn {
        phase: &'static str,
        executable: PathBuf,
        source: io::Error,
    },
    Process {
        phase: &'static str,
        executable: PathBuf,
        status: ExitStatus,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Heresy(clang) => write!(
                f,
                "HeresyError: Thou shalt use Clang. Install LLVM Clang and run `foo config --compiler {clang}`."
            ),
            Self::MissingCompiler {
                language,
                executable,
            } => write!(
                f,
                "Compiler or runtime for {language} was not found: {}. {} Add it to PATH or set `foo config --compiler <path>`.",
                executable.display(),
                language.install_help()
            ),
            Self::MissingJavaForKotlin => f.write_str(
                "Kotlin/JVM requires Java. Install a JDK and add java to PATH or set JAVA_HOME.",
            ),
            Self::MissingSource(path) => write!(f, "Source file was not found: {}", path.display()),
            Self::TemporaryDirectory(error) => {
                write!(f, "Could not create a temporary build directory: {error}")
            }
            Self::Spawn {
                phase,
                executable,
                source,
            } => write!(
                f,
                "{phase} could not start {}: {source}",
                executable.display()
            ),
            Self::Process {
                phase,
                executable,
                status,
            } => {
                write!(f, "{phase} failed: {} ({status})", executable.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TemporaryDirectory(source) | Self::Spawn { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub(crate) fn find_executable(candidate: &Path) -> Option<PathBuf> {
    if candidate.is_absolute() || candidate.components().count() != 1 {
        return resolve_executable(candidate);
    }
    let search_path = env::var_os("PATH")?;
    env::split_paths(&search_path)
        .find_map(|directory| resolve_executable(&directory.join(candidate)))
}

fn resolve_executable(path: &Path) -> Option<PathBuf> {
    if is_executable(path) {
        return std::path::absolute(path).ok();
    }
    #[cfg(windows)]
    if path.extension().is_none() {
        return ["exe", "cmd", "bat", "com"]
            .into_iter()
            .map(|ext| path.with_extension(ext))
            .find(|path| is_executable(path))
            .and_then(|path| std::path::absolute(path).ok());
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    let has_executable_extension =
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                ["exe", "cmd", "bat", "com"]
                    .iter()
                    .any(|kind| ext.eq_ignore_ascii_case(kind))
            });
    has_executable_extension
        && fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() > 0)
}

/// Matches BOJ's .NET 6 (C# 10). A newer runtime is used only if .NET 6 is not installed.
const CSHARP_PROJECT: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net6.0</TargetFramework>
    <LangVersion>10.0</LangVersion>
    <RollForward>Major</RollForward>
    <UseAppHost>false</UseAppHost>
    <CheckEolTargetFramework>false</CheckEolTargetFramework>
  </PropertyGroup>
</Project>
"#;

fn run_csharp(compiler: &Path, directory: &Path, source: &Path) -> Result<(), Error> {
    let build = tempfile::tempdir().map_err(Error::TemporaryDirectory)?;
    fs::write(build.path().join("solution.csproj"), CSHARP_PROJECT)
        .and_then(|()| fs::copy(source, build.path().join("main.cs")))
        .map_err(Error::TemporaryDirectory)?;
    execute(
        Command::new(compiler)
            .current_dir(build.path())
            .env("DOTNET_NOLOGO", "1")
            .env("MSBUILDTERMINALLOGGER", "off")
            // `dotnet build` would print a summary into the solution's output.
            .args([
                "msbuild",
                "solution.csproj",
                "-restore",
                "-nologo",
                "-v:quiet",
            ])
            .args(["-p:Configuration=Release", "-p:OutDir=out/"]),
        "Build (.NET 6)",
    )?;
    execute(
        Command::new(compiler)
            .current_dir(directory)
            .arg(build.path().join("out").join("solution.dll")),
        "Run",
    )
}

fn find_java() -> Option<PathBuf> {
    env::var_os("JAVA_HOME")
        .and_then(|home| resolve_executable(&PathBuf::from(home).join("bin").join("java")))
        .or_else(|| find_executable(Path::new("java")))
}

fn executable_is(path: &Path, name: &str) -> bool {
    path.file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|stem| stem.eq_ignore_ascii_case(name))
}

fn is_msvc(path: &Path) -> bool {
    executable_is(path, "cl") || executable_is(path, "msvc")
}

fn execute(command: &mut Command, phase: &'static str) -> Result<(), Error> {
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let executable = PathBuf::from(command.get_program());
    let status = command.status().map_err(|source| Error::Spawn {
        phase,
        executable: executable.clone(),
        source,
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Process {
            phase,
            executable,
            status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_names_match_cli_and_configuration() {
        for &language in Language::value_variants() {
            assert_eq!(Language::from_str(language.as_str(), false), Ok(language));
            let encoded = serde_json::to_string(&language).expect("serialize language");
            assert_eq!(encoded, format!("\"{}\"", language.as_str()));
            assert!(language.template().ends_with('\n'));
        }
    }

    #[test]
    fn rejects_msvc_for_both_c_family_languages() {
        for (language, alternative) in [(Language::C, "clang"), (Language::Cpp, "clang++")] {
            let error = language
                .find_compiler(Some(Path::new("CL.EXE")))
                .unwrap_err();
            assert!(matches!(error, Error::Heresy(tool) if tool == alternative));
            assert!(
                error
                    .to_string()
                    .contains("HeresyError: Thou shalt use Clang.")
            );
        }
    }

    #[test]
    fn csharp_has_dotnet_aliases() {
        for name in ["csharp", "c#", "cs", "dotnet"] {
            assert_eq!(
                Language::from_str(name, true),
                Ok(Language::CSharp),
                "{name}"
            );
        }
        assert_eq!(Language::CSharp.source_name(), "main.cs");
        assert_eq!(Language::CSharp.default_compiler(), "dotnet");
    }

    #[test]
    fn cobol_uses_gnucobol_with_cob_sources() {
        assert_eq!(Language::from_str("cob", true), Ok(Language::Cobol));
        assert_eq!(Language::Cobol.source_name(), "main.cob");
        assert_eq!(Language::Cobol.default_compiler(), "cobc");
    }

    #[test]
    fn allows_clang_cl_and_mingw() {
        for tool in ["clang-cl", "x86_64-w64-mingw32-g++", "g++", "gcc"] {
            for language in [Language::C, Language::Cpp] {
                // Whether the tool is installed depends on the machine; it just must not be a Heresy.
                let result = language.find_compiler(Some(Path::new(tool)));
                assert!(
                    !matches!(result, Err(Error::Heresy(_))),
                    "{tool} must not be rejected as MSVC"
                );
            }
        }
    }

    #[test]
    fn running_without_a_source_does_not_start_the_toolchain() {
        let directory = tempfile::tempdir().expect("create problem directory");
        let error = Language::Cpp
            .run(Path::new("not-a-compiler"), directory.path())
            .unwrap_err();
        assert!(matches!(error, Error::MissingSource(source) if source.ends_with("main.cpp")));
    }
}
