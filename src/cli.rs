use std::{ffi::OsString, path::PathBuf};

use clap::{
    Arg, Args, CommandFactory, FromArgMatches, Parser, Subcommand, builder::EnumValueParser,
};

use crate::{language::Language, text::Lang};

#[derive(Parser)]
#[command(
    version,
    about = "Manage a local workspace for solving Baekjoon Online Judge problems. Less setup. More solving."
)]
pub(crate) struct Cli {
    /// Do not launch VS Code or the browser
    #[arg(long, global = true)]
    pub(crate) no_launch: bool,

    #[command(subcommand)]
    pub(crate) command: Action,
}

#[derive(Subcommand)]
pub(crate) enum Action {
    /// Interactively set the language and workspace
    Init,
    /// Register or select a workbook or step
    Remember(Remember),
    /// Open the current problem, or the one at the given position
    Start {
        /// Position in the workbook (starting at 1)
        index: Option<usize>,
    },
    /// Open the next problem in the workbook
    Next,
    /// Open the previous problem in the workbook
    Before,
    /// Show the active workbook and your progress
    Status,
    /// Open a workspace by problem number
    Open {
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        problem_id: u32,
    },
    /// Show or change the current settings
    Config(ConfigEdit),
    #[command(name = "__run", hide = true)]
    Run,
}

#[derive(Args)]
pub(crate) struct Remember {
    /// BOJ workbook or step URL
    pub(crate) url: String,

    /// UTF-8 HTML file saved from your browser
    #[arg(long, conflicts_with_all = ["title", "problems"])]
    pub(crate) html: Option<PathBuf>,

    /// Title for a manually registered workbook
    #[arg(long, requires = "problems")]
    pub(crate) title: Option<String>,

    /// Problem numbers for a manually registered workbook (comma-separated)
    #[arg(long, requires = "title", value_delimiter = ',', value_parser = clap::value_parser!(u32).range(1..))]
    pub(crate) problems: Vec<u32>,
}

#[derive(Args, Default)]
pub(crate) struct ConfigEdit {
    /// Default language for new problems
    #[arg(long, value_enum)]
    pub(crate) language: Option<Language>,

    /// Compiler or interpreter executable
    #[arg(long)]
    pub(crate) compiler: Option<PathBuf>,

    /// Go back to automatic compiler or interpreter discovery
    #[arg(long, conflicts_with = "compiler")]
    pub(crate) reset_compiler: bool,

    /// Directory that holds the per-problem folders
    #[arg(long)]
    pub(crate) workspace: Option<PathBuf>,

    /// VS Code executable
    #[arg(long)]
    pub(crate) code: Option<PathBuf>,

    /// Go back to automatic VS Code discovery
    #[arg(long, conflicts_with = "code")]
    pub(crate) reset_code: bool,
}

impl ConfigEdit {
    pub(crate) fn has_changes(&self) -> bool {
        self.language.is_some()
            || self.compiler.is_some()
            || self.reset_compiler
            || self.workspace.is_some()
            || self.code.is_some()
            || self.reset_code
    }
}

/// Parses `args`, with help text in the requested language.
pub(crate) fn parse(args: Vec<OsString>, lang: Lang) -> Cli {
    let mut command = Cli::command().arg(
        Arg::new("lang")
            .long("lang")
            .global(true)
            .value_name("LANG")
            .value_parser(EnumValueParser::<Lang>::new())
            .help("Output language, or set FOO_LANG [default: en]"),
    );
    if lang == Lang::Ko {
        command = korean(command);
    }
    let mut matches = command.get_matches_from(args);
    Cli::from_arg_matches_mut(&mut matches).unwrap_or_else(|error| error.exit())
}

fn korean(command: clap::Command) -> clap::Command {
    command
        .about("백준 문제 풀이 작업공간 관리. Less setup. More solving.")
        .mut_arg("no_launch", |arg| {
            arg.help("VS Code와 브라우저를 실행하지 않습니다")
        })
        .mut_arg("lang", |arg| {
            arg.help("출력 언어. FOO_LANG 환경 변수로도 지정할 수 있습니다 [기본값: en]")
        })
        .mut_subcommand("init", |cmd| {
            cmd.about("대화형으로 언어와 작업공간을 설정합니다")
        })
        .mut_subcommand("remember", |cmd| {
            cmd.about("일반 문제집이나 단계별 문제를 등록하거나 선택합니다")
                .mut_arg("url", |arg| {
                    arg.help("백준의 일반 문제집 또는 단계별 문제 URL")
                })
                .mut_arg("html", |arg| {
                    arg.help("브라우저에서 저장한 UTF-8 HTML 파일")
                })
                .mut_arg("title", |arg| arg.help("직접 등록할 문제집의 제목"))
                .mut_arg("problems", |arg| {
                    arg.help("직접 등록할 문제 번호 (쉼표로 구분)")
                })
        })
        .mut_subcommand("start", |cmd| {
            cmd.about("현재 문제 또는 지정한 순서의 문제를 엽니다")
                .mut_arg("index", |arg| arg.help("문제집에서의 순서 (1부터 시작)"))
        })
        .mut_subcommand("next", |cmd| cmd.about("문제집의 다음 문제를 엽니다"))
        .mut_subcommand("before", |cmd| cmd.about("문제집의 이전 문제를 엽니다"))
        .mut_subcommand("status", |cmd| {
            cmd.about("활성 문제집과 진행 위치를 표시합니다")
        })
        .mut_subcommand("open", |cmd| cmd.about("문제 번호로 작업공간을 엽니다"))
        .mut_subcommand("config", |cmd| {
            cmd.about("현재 설정을 보거나 변경합니다")
                .mut_arg("language", |arg| arg.help("새 문제에 사용할 기본 언어"))
                .mut_arg("compiler", |arg| {
                    arg.help("컴파일러 또는 인터프리터 실행 파일")
                })
                .mut_arg("reset_compiler", |arg| {
                    arg.help("컴파일러 또는 인터프리터 자동 탐색으로 복귀")
                })
                .mut_arg("workspace", |arg| arg.help("문제별 디렉터리를 저장할 경로"))
                .mut_arg("code", |arg| arg.help("VS Code 실행 파일"))
                .mut_arg("reset_code", |arg| arg.help("VS Code 자동 탐색으로 복귀"))
        })
}
