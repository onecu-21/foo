use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

const WORKBOOK_URL: &str = "https://www.acmicpc.net/workbook/view/42";
const STEP_URL: &str = "https://acmicpc.net/step/1";

struct TestEnv {
    temp: TempDir,
    workspace: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("create temporary test directory");
        let workspace = temp.path().join("백준 작업 폴더");
        Self { temp, workspace }
    }

    fn config_dir(&self) -> PathBuf {
        self.temp.path().join("settings")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_foo"));
        command
            .env("FOO_CONFIG_DIR", self.config_dir())
            .current_dir(self.temp.path())
            .arg("--no-launch");
        command
    }

    fn init(&self) {
        self.init_with("rust");
    }

    fn init_with(&self, language: &str) {
        let input = format!("{language}\n\n\n{}\n", self.workspace.display());
        let output = run_with_input(self.command().arg("init"), input.as_bytes());
        assert_success(&output);
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().expect("invoke foo")
    }

    fn status(&self) -> String {
        let output = self.run(&["status"]);
        assert_success(&output);
        String::from_utf8(output.stdout).expect("status is UTF-8")
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &Output) {
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty(), "expected an error message");
}

fn run_with_input(command: &mut Command, input: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("invoke foo with standard input");
    child
        .stdin
        .take()
        .expect("child input is piped")
        .write_all(input)
        .expect("write to child's standard input");
    child.wait_with_output().expect("wait for foo")
}

#[test]
fn init_creates_configuration_and_preserves_it_on_repeat() {
    let env = TestEnv::new();
    env.init();
    assert!(env.workspace.join(".vscode/launch.json").is_file());
    let config = env.config_dir().join("config.toml");
    let previous = fs::read(&config).expect("read configuration after init");
    assert_failure(&env.run(&["init"]));
    assert_eq!(fs::read(&config).expect("read configuration"), previous);
    assert_success(&env.run(&["config"]));

    let new_workspace = env.temp.path().join("new workspace");
    let output = env
        .command()
        .args(["config", "--workspace"])
        .arg(&new_workspace)
        .output()
        .expect("change workspace directory");
    assert_success(&output);
    assert!(new_workspace.join(".vscode/launch.json").is_file());
    assert_ne!(
        fs::read(config).expect("read updated configuration"),
        previous
    );
}

#[test]
fn remembers_html_and_resumes_workbook_without_overwriting_source() {
    let env = TestEnv::new();
    env.init();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workbook.html");
    let output = env
        .command()
        .args(["remember", WORKBOOK_URL, "--html"])
        .arg(fixture)
        .output()
        .expect("import workbook fixture");
    assert_success(&output);

    let status = env.status();
    assert!(status.contains("연습 & 구현"));
    assert!(status.contains("2557"));
    assert!(status.to_lowercase().contains("rust"));

    let restricted = env.temp.path().join("restricted.html");
    fs::write(
        &restricted,
        "<title>403 Forbidden</title><h1>Access denied</h1>",
    )
    .expect("write saved access restriction page");
    let output = env
        .command()
        .args(["remember", WORKBOOK_URL, "--html"])
        .arg(restricted)
        .output()
        .expect("try importing the restriction page");
    assert_failure(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("access-restricted"));
    assert_eq!(env.status(), status);

    assert_success(&env.run(&["start", "2"]));
    let problem = env.workspace.join("1000");
    let source = problem.join("main.rs");
    assert!(source.is_file());
    for filename in ["tasks.json", "launch.json"] {
        let path = problem.join(".vscode").join(filename);
        assert!(path.is_file(), "{} was not created", path.display());
    }

    let user_source = b"fn main() { println!(\"my solution\"); }\n";
    fs::write(&source, user_source).expect("save user solution");
    let tasks = problem.join(".vscode/tasks.json");
    let user_tasks = b"{\"version\": \"2.0.0\", \"tasks\": []}\n";
    fs::write(&tasks, user_tasks).expect("save user tasks");
    assert_success(&env.run(&["start"]));
    assert_eq!(fs::read(&source).expect("read user source"), user_source);
    assert_eq!(fs::read(tasks).expect("read user tasks"), user_tasks);

    assert_success(&env.run(&["next"]));
    assert!(env.workspace.join("1001/main.rs").is_file());
    assert_failure(&env.run(&["next"]));
    assert!(env.status().contains("1001"));
    assert_success(&env.run(&["before"]));
    assert!(env.status().contains("1000"));
    assert_failure(&env.run(&["start", "0"]));
    assert_failure(&env.run(&["start", "4"]));
    assert!(env.status().contains("1000"));

    assert_success(&env.run(&["open", "1008"]));
    assert!(env.workspace.join("1008/main.rs").is_file());
    assert!(env.status().contains("1000"));

    assert_success(&env.run(&[
        "remember",
        "https://acmicpc.net/workbook/view/43",
        "--title",
        "두 번째 문제집",
        "--problems",
        "1008,1009",
    ]));
    assert!(env.status().contains("두 번째 문제집"));

    assert_success(&env.run(&["remember", "https://acmicpc.net/workbook/view/42"]));
    assert!(env.status().contains("1000"));
    assert_success(&env.run(&["before"]));
    assert!(env.status().contains("2557"));
    assert_failure(&env.run(&["before"]));
}

#[test]
fn remembers_step_and_resumes_it_alongside_an_existing_workbook() {
    let env = TestEnv::new();
    env.init();
    let config = env.config_dir().join("config.toml");
    let previous_config = fs::read(&config).expect("read existing configuration");
    fs::write(
        env.config_dir().join("state.toml"),
        r#"active_url = "https://acmicpc.net/workbook/view/1"

[[collections]]
url = "https://acmicpc.net/workbook/view/1"
title = "기존 문제집"
problems = [1008, 1008, 1009]
position = 2
"#,
    )
    .expect("write state from an existing installation");
    let workbook_status = env.status();
    assert!(workbook_status.contains("기존 문제집"));
    assert!(workbook_status.contains("Problem: 1009"));
    assert!(workbook_status.contains("Progress: 3/3"));

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/step.html");
    let output = env
        .command()
        .args(["remember", STEP_URL, "--html"])
        .arg(fixture)
        .output()
        .expect("import step fixture");
    assert_success(&output);
    let status = env.status();
    assert!(status.contains("문법 1. 입출력과 사칙연산"));
    assert!(status.contains("Problem: 2557"));
    assert!(status.contains("Progress: 1/13"));

    assert_success(&env.run(&["start"]));
    assert!(env.workspace.join("2557/main.rs").is_file());
    assert_success(&env.run(&["next"]));
    assert!(env.workspace.join("1000/main.rs").is_file());
    assert!(env.status().contains("Progress: 2/13"));
    assert_success(&env.run(&["before"]));
    assert_eq!(env.status(), status);
    assert_success(&env.run(&["start", "13"]));
    let step_status = env.status();
    assert!(step_status.contains("Problem: 10172"));
    assert!(step_status.contains("Progress: 13/13"));

    assert_success(&env.run(&["remember", "http://www.acmicpc.net/workbook/view/1"]));
    assert_eq!(env.status(), workbook_status);
    assert_success(&env.run(&["remember", "http://www.acmicpc.net/step/1/"]));
    assert_eq!(env.status(), step_status);

    let restricted = env.temp.path().join("restricted-step.html");
    fs::write(&restricted, "<title>403 Forbidden</title>").expect("write restriction page");
    let output = env
        .command()
        .args(["remember", STEP_URL, "--html"])
        .arg(restricted)
        .output()
        .expect("try importing the restricted step page");
    assert_failure(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("access-restricted"));
    assert_eq!(env.status(), step_status);
    assert_eq!(
        fs::read(config).expect("read configuration"),
        previous_config
    );
}

#[test]
fn reports_invalid_input_and_leaves_corrupt_configuration_in_place() {
    let env = TestEnv::new();
    env.init();
    assert_failure(&env.run(&["start"]));
    assert_failure(&env.run(&["open", "0"]));
    assert_failure(&env.run(&["remember", "https://example.com/workbook/view/42"]));
    assert_failure(&env.run(&["remember", WORKBOOK_URL]));

    let config = env.config_dir().join("config.toml");
    let previous = fs::read(&config).expect("read current configuration");
    let output = env.run(&["config", "--language", "cpp", "--compiler", "cl.exe"]);
    assert_failure(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("HeresyError: Thou shalt use Clang."));
    assert_eq!(fs::read(&config).expect("read configuration"), previous);

    let bad_toml = b"[unterminated\n";
    fs::write(&config, bad_toml).expect("write broken test configuration");
    assert_failure(&env.run(&["config"]));
    assert_eq!(fs::read(config).expect("read configuration"), bad_toml);
}

#[test]
fn vscode_runner_builds_and_executes_rust_with_standard_input() {
    let env = TestEnv::new();
    env.init();
    assert_success(&env.run(&["open", "1000"]));
    let problem_dir = env.workspace.join("1000");
    fs::write(
        problem_dir.join("main.rs"),
        r#"fn main() {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let total: i32 = input.split_whitespace().map(|part| part.parse::<i32>().unwrap()).sum();
    println!("{total}");
}"#,
    )
    .expect("write Rust solution");

    let mut command = env.command();
    command.current_dir(&problem_dir).arg("__run");
    let output = run_with_input(&mut command, b"20 22\n");
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[test]
fn help_is_english_by_default_and_korean_on_request() {
    let env = TestEnv::new();
    let english = env.run(&["--help"]);
    assert_success(&english);
    let english = String::from_utf8_lossy(&english.stdout);
    assert!(english.contains("Register or select a workbook or step"));
    assert!(!english.contains("문제집"));

    let korean = env.run(&["--lang", "ko", "--help"]);
    assert_success(&korean);
    assert!(String::from_utf8_lossy(&korean.stdout).contains("등록하거나 선택합니다"));

    let from_env = env.command().env("FOO_LANG", "ko").arg("--help").output();
    assert!(String::from_utf8_lossy(&from_env.expect("invoke foo").stdout).contains("문제집"));

    let flag_wins = env
        .command()
        .env("FOO_LANG", "ko")
        .args(["--lang", "en", "remember", "--help"])
        .output()
        .expect("invoke foo");
    assert!(String::from_utf8_lossy(&flag_wins.stdout).contains("UTF-8 HTML file"));

    assert_failure(&env.run(&["--lang", "fr", "--help"]));
}

#[test]
fn messages_can_be_switched_to_korean() {
    let env = TestEnv::new();
    env.init();
    assert_success(&env.run(&[
        "remember",
        WORKBOOK_URL,
        "--title",
        "연습",
        "--problems",
        "1000,1001",
    ]));
    let output = env.run(&["--lang", "ko", "status"]);
    assert_success(&output);
    let status = String::from_utf8_lossy(&output.stdout);
    assert!(status.contains("문제: 1000"));
    assert!(status.contains("진행: 1/2"));

    let output = env.run(&["--lang", "ko", "start", "9"]);
    assert_failure(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("오류: "));
    assert!(stderr.contains("범위를 벗어났습니다"));

    let output = env.run(&["start", "9"]);
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("error: "));
}

fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path)
            .any(|dir| dir.join(tool).is_file() || dir.join(format!("{tool}.exe")).is_file())
    })
}

#[test]
fn vscode_runner_builds_and_executes_cobol_with_standard_input() {
    if !on_path("cobc") {
        assert!(
            std::env::var_os("FOO_REQUIRE_COBOL").is_none(),
            "FOO_REQUIRE_COBOL is set but cobc is not on PATH"
        );
        eprintln!("skipped: GnuCOBOL (cobc) is not installed");
        return;
    }
    let env = TestEnv::new();
    env.init_with("cobol");
    assert_success(&env.run(&["open", "1000"]));
    let problem_dir = env.workspace.join("1000");
    assert!(problem_dir.join("main.cob").is_file());
    fs::write(
        problem_dir.join("main.cob"),
        "IDENTIFICATION DIVISION.
PROGRAM-ID. MAIN.

DATA DIVISION.
WORKING-STORAGE SECTION.
01 TEXT-IN PIC X(80).
01 TOTAL PIC S9(18) VALUE 0.
01 SHOWN PIC -(17)9.

PROCEDURE DIVISION.
    ACCEPT TEXT-IN
    COMPUTE TOTAL = TOTAL + FUNCTION NUMVAL(TEXT-IN)
    ACCEPT TEXT-IN
    COMPUTE TOTAL = TOTAL + FUNCTION NUMVAL(TEXT-IN)
    MOVE TOTAL TO SHOWN
    DISPLAY FUNCTION TRIM(SHOWN)
    STOP RUN.
",
    )
    .expect("write COBOL solution");

    let mut command = env.command();
    command.current_dir(&problem_dir).arg("__run");
    let output = run_with_input(&mut command, b"20\n22\n");
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}
