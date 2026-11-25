use std::{error, fmt, io, path::PathBuf};

use crate::{language, text::t, workbook, workspace};

#[derive(Debug)]
pub(crate) enum Error {
    Io(PathBuf, io::Error),
    InvalidData(PathBuf, String),
    Encode(toml::ser::Error),
    ConfigDirectory,
    NotInitialized,
    AlreadyInitialized(PathBuf),
    NoWorkbook,
    Position(usize),
    InvalidTitle,
    InvalidLanguage(String),
    InputClosed,
    Language(language::Error),
    Workbook(workbook::WorkbookError),
    Workspace(workspace::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(path, err) => write!(f, "{}: {err}", path.display()),
            Self::InvalidData(path, err) => f.write_str(&t!(
                "Cannot read the saved file ({}). {err}",
                "저장 파일을 읽을 수 없습니다 ({}). {err}",
                path.display()
            )),
            Self::Encode(err) => f.write_str(&t!(
                "Cannot save the settings as TOML. {err}",
                "설정을 TOML로 저장할 수 없습니다. {err}"
            )),
            Self::ConfigDirectory => f.write_str(&t!(
                "Cannot find the user configuration directory. Set FOO_CONFIG_DIR to a path.",
                "사용자 설정 디렉터리를 찾을 수 없습니다. FOO_CONFIG_DIR에 경로를 지정하세요."
            )),
            Self::NotInitialized => f.write_str(&t!(
                "Not set up yet. Run `foo init` first.",
                "설정이 없습니다. 먼저 `foo init`을 실행하세요."
            )),
            Self::AlreadyInitialized(path) => f.write_str(&t!(
                "Already set up ({}). To change settings, see `foo config --help`.",
                "이미 설정이 있습니다 ({}). 변경하려면 `foo config --help`를 참고하세요.",
                path.display()
            )),
            Self::NoWorkbook => f.write_str(&t!(
                "No workbook is registered. Register one with `foo remember <URL> --html <FILE>`, or see `foo remember --help`.",
                "등록된 문제집이 없습니다. `foo remember <URL> --html <파일>`로 등록하거나 `foo remember --help`를 참고하세요."
            )),
            Self::Position(len) => f.write_str(&t!(
                "Position out of range. Valid positions are 1-{len}.",
                "문제 순서가 범위를 벗어났습니다. 사용 가능한 순서는 1~{len}입니다."
            )),
            Self::InvalidTitle => f.write_str(&t!(
                "Enter a workbook title.",
                "문제집 제목을 입력하세요."
            )),
            Self::InvalidLanguage(name) => f.write_str(&t!(
                "Unsupported language: {name}",
                "지원하지 않는 언어입니다. {name}"
            )),
            Self::InputClosed => f.write_str(&t!(
                "Input ended before setup was complete.",
                "초기 설정에 필요한 입력이 종료되었습니다."
            )),
            Self::Language(err) => err.fmt(f),
            Self::Workbook(err) => err.fmt(f),
            Self::Workspace(err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(_, err) => Some(err),
            Self::Encode(err) => Some(err),
            Self::Language(err) => Some(err),
            Self::Workbook(err) => Some(err),
            Self::Workspace(err) => Some(err),
            _ => None,
        }
    }
}

impl From<language::Error> for Error {
    fn from(value: language::Error) -> Self {
        Self::Language(value)
    }
}

impl From<workbook::WorkbookError> for Error {
    fn from(value: workbook::WorkbookError) -> Self {
        Self::Workbook(value)
    }
}

impl From<workspace::Error> for Error {
    fn from(value: workspace::Error) -> Self {
        Self::Workspace(value)
    }
}
