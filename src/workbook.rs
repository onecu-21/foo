use std::{
    collections::HashSet,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use reqwest::Url;
use scraper::{ElementRef, Html, Selector};

use crate::text::t;

#[derive(Debug)]
pub struct Workbook {
    pub title: String,
    pub problems: Vec<u32>,
}

impl Workbook {
    pub fn new(title: String, mut problems: Vec<u32>) -> Self {
        let mut seen = HashSet::new();
        problems.retain(|&id| seen.insert(id));
        Self { title, problems }
    }
}

#[derive(Clone, Copy)]
pub enum PageKind {
    Workbook,
    Step,
}

pub struct WorkbookUrl {
    pub canonical: String,
    pub kind: PageKind,
}

#[derive(Debug)]
pub enum WorkbookError {
    InvalidUrl,
    SitePolicy,
    AccessRestricted,
    Read { path: PathBuf, source: io::Error },
    Parse(String),
}

impl fmt::Display for WorkbookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl => f.write_str(&t!(
                "Enter a BOJ workbook (`/workbook/view/<ID>`) or step (`/step/<ID>`) URL.",
                "백준의 일반 문제집(`/workbook/view/<ID>`) 또는 단계별 문제(`/step/<ID>`) URL을 입력하세요."
            )),
            Self::SitePolicy => f.write_str(&t!(
                "BOJ's rules (https://help.acmicpc.net/rule) do not allow the CLI to collect pages. Import a saved page with `foo remember <URL> --html <FILE>`, or register one manually with `--title` and `--problems`.",
                "백준의 이용 규칙(https://help.acmicpc.net/rule)에 따라 CLI에서 페이지를 수집할 수 없습니다. `foo remember <URL> --html <FILE>`로 저장한 HTML을 가져오거나 `--title`과 `--problems`로 직접 등록하세요."
            )),
            Self::AccessRestricted => f.write_str(&t!(
                "The saved HTML is an access-restricted page. Open the page normally in your browser, then save it again.",
                "저장한 HTML이 접근 제한 화면입니다. 브라우저에서 해당 페이지를 정상적으로 연 뒤 다시 저장하세요."
            )),
            Self::Read { path, source } => f.write_str(&t!(
                "Cannot read the HTML file ({}): {source}",
                "HTML 파일 읽기 실패 ({}): {source}",
                path.display()
            )),
            Self::Parse(reason) => f.write_str(&t!(
                "Cannot parse the workbook HTML: {reason}",
                "문제집 HTML 파싱 실패: {reason}"
            )),
        }
    }
}

impl Error for WorkbookError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn parse_url(input: &str) -> Result<WorkbookUrl, WorkbookError> {
    let url = Url::parse(input.trim()).map_err(|_| WorkbookError::InvalidUrl)?;
    if !is_boj_origin(&url) {
        return Err(WorkbookError::InvalidUrl);
    }
    let path = url.path();
    for (prefix, kind) in [
        ("/workbook/view/", PageKind::Workbook),
        ("/step/", PageKind::Step),
    ] {
        if let Some(id) = path.strip_prefix(prefix) {
            let id = positive_id::<u64>(id.strip_suffix('/').unwrap_or(id))
                .ok_or(WorkbookError::InvalidUrl)?;
            return Ok(WorkbookUrl {
                canonical: format!("https://acmicpc.net{prefix}{id}"),
                kind,
            });
        }
    }
    Err(WorkbookError::InvalidUrl)
}

pub fn parse_file(path: &Path, kind: PageKind) -> Result<Workbook, WorkbookError> {
    let html = fs::read_to_string(path).map_err(|source| WorkbookError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    parse(&html, kind)
}

fn parse(html: &str, kind: PageKind) -> Result<Workbook, WorkbookError> {
    let document = Html::parse_document(html);
    let Some(table) = problem_table(&document, kind)? else {
        if is_access_page(&document)? {
            return Err(WorkbookError::AccessRestricted);
        }
        return Err(WorkbookError::Parse(t!(
            "Cannot find the problem table.",
            "문제 목록 표를 찾을 수 없습니다."
        )));
    };
    let title = workbook_title(&document)?;

    let rows = selector(":scope > tbody > tr")?;
    let links = selector(":scope > td a[href]")?;
    let number = selector(":scope > td.list_problem_id")?;
    let description = selector(":scope > td[colspan]:only-child")?;
    let mut problems = Vec::new();
    for row in table.select(&rows) {
        let id = match kind {
            PageKind::Workbook => row
                .select(&links)
                .filter_map(|link| link.attr("href"))
                .find_map(problem_id),
            PageKind::Step if row.select(&description).next().is_some() => continue,
            PageKind::Step => row
                .select(&number)
                .next()
                .and_then(|cell| positive_id::<u32>(&text_content(cell))),
        }
        .ok_or_else(|| {
            WorkbookError::Parse(t!(
                "Cannot read a problem number from a row of the problem list.",
                "문제 목록의 행에서 문제 번호를 읽을 수 없습니다."
            ))
        })?;
        problems.push(id);
    }
    if problems.is_empty() {
        return Err(WorkbookError::Parse(t!(
            "The saved problem list is empty.",
            "저장된 문제 목록이 비어 있습니다."
        )));
    }

    Ok(Workbook::new(title, problems))
}

fn problem_table(document: &Html, kind: PageKind) -> Result<Option<ElementRef<'_>>, WorkbookError> {
    let marked_table = selector("table#problemset")?;
    let marked = document.select(&marked_table).next();
    if marked.is_some() || matches!(kind, PageKind::Step) {
        return Ok(marked);
    }

    let tables = selector("table")?;
    let headers = selector("th")?;
    Ok(document.select(&tables).find(|table| {
        table
            .select(&headers)
            .map(text_content)
            .any(|header| header == "문제 제목" || header.eq_ignore_ascii_case("problem title"))
    }))
}

fn is_boj_origin(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && matches!(url.host_str(), Some("acmicpc.net" | "www.acmicpc.net"))
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
}

fn positive_id<T: std::str::FromStr + Default + PartialEq>(id: &str) -> Option<T> {
    if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    id.parse().ok().filter(|id| *id != T::default())
}

fn problem_id(href: &str) -> Option<u32> {
    let path_id = |path: &str| positive_id(path.strip_prefix("/problem/")?);
    if href.starts_with('/') {
        return path_id(href.split(['?', '#']).next()?);
    }
    let url = Url::parse(href).ok()?;
    if !is_boj_origin(&url) {
        return None;
    }
    path_id(url.path())
}

fn selector(pattern: &str) -> Result<Selector, WorkbookError> {
    Selector::parse(pattern).map_err(|_| {
        WorkbookError::Parse(t!(
            "Invalid internal HTML selector.",
            "내부 HTML 선택자가 올바르지 않습니다."
        ))
    })
}

fn workbook_title(document: &Html) -> Result<String, WorkbookError> {
    for pattern in ["#workbook-title", ".headline h1", ".page-header h1", "h1"] {
        let heading = selector(pattern)?;
        if let Some(title) = document
            .select(&heading)
            .map(text_content)
            .find(|text| !text.is_empty())
        {
            return Ok(title);
        }
    }
    Err(WorkbookError::Parse(t!(
        "Cannot find the workbook title.",
        "문제집 제목을 찾을 수 없습니다."
    )))
}

fn text_content(element: ElementRef<'_>) -> String {
    element
        .text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_access_page(document: &Html) -> Result<bool, WorkbookError> {
    let challenge = selector("#challenge-form, #cf-challenge-running")?;
    let messages = selector("title, h1, h2, .alert-danger, .alert-warning")?;
    Ok(document.select(&challenge).next().is_some()
        || document.select(&messages).map(text_content).any(|text| {
            let text = text.to_lowercase();
            [
                "접근이 제한",
                "접근이 거부",
                "접근 권한이 없",
                "로그인이 필요",
                "로그인 후 이용",
                "비공개 문제집",
                "access denied",
                "forbidden",
                "401 unauthorized",
                "403 forbidden",
                "429 too many requests",
                "attention required!",
                "just a moment...",
                "too many requests",
            ]
            .iter()
            .any(|&marker| text.starts_with(marker))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKBOOK_HTML: &str = include_str!("../tests/fixtures/workbook.html");
    const STEP_HTML: &str = include_str!("../tests/fixtures/step.html");

    #[test]
    fn normalizes_boj_workbook_urls() -> Result<(), WorkbookError> {
        for origin in [
            "http://acmicpc.net",
            "https://acmicpc.net",
            "http://www.acmicpc.net",
            "https://www.acmicpc.net",
        ] {
            let workbook = parse_url(&format!(" {origin}/workbook/view/0042/?foo=bar#example "))?;
            assert_eq!(workbook.canonical, "https://acmicpc.net/workbook/view/42");
            assert!(matches!(workbook.kind, PageKind::Workbook));

            let step = parse_url(&format!(" {origin}/step/0001/?foo=bar#example "))?;
            assert_eq!(step.canonical, "https://acmicpc.net/step/1");
            assert!(matches!(step.kind, PageKind::Step));
        }
        Ok(())
    }

    #[test]
    fn rejects_unsupported_collection_urls() {
        for url in [
            "acmicpc.net/workbook/view/42",
            "https://acmicpc.net.example.org/workbook/view/42",
            "https://user:pass@acmicpc.net/workbook/view/42",
            "https://acmicpc.net:1234/workbook/view/42",
            "ftp://acmicpc.net/workbook/view/42",
            "https://acmicpc.net/workbook/view/0",
            "https://acmicpc.net/workbook/view/42//",
            "https://acmicpc.net/problem/42",
            "https://acmicpc.net/workbook/view/18446744073709551616",
            "https://acmicpc.net/step",
            "https://acmicpc.net/step/0",
            "https://acmicpc.net/step/abc",
            "https://acmicpc.net/step/1/extra",
            "https://acmicpc.net/step/18446744073709551616",
            "https://example.org/step/1",
        ] {
            assert!(
                matches!(parse_url(url), Err(WorkbookError::InvalidUrl)),
                "{url}"
            );
        }
    }

    #[test]
    fn reads_korean_title_and_deduplicates_problems_in_row_order() -> Result<(), WorkbookError> {
        let workbook = parse(WORKBOOK_HTML, PageKind::Workbook)?;
        assert_eq!(workbook.title, "연습 & 구현");
        assert_eq!(workbook.problems, [2557, 1000, 1001]);
        Ok(())
    }

    #[test]
    fn finds_problem_table_by_heading_when_id_is_absent() -> Result<(), WorkbookError> {
        let workbook = parse(
            &WORKBOOK_HTML.replace(
                " id=\"problemset\"",
                " class=\"table table-striped table-bordered\"",
            ),
            PageKind::Workbook,
        )?;
        assert_eq!(workbook.problems, [2557, 1000, 1001]);
        Ok(())
    }

    #[test]
    fn reads_step_problem_numbers_in_display_order_and_skips_description_rows()
    -> Result<(), WorkbookError> {
        let step = parse(STEP_HTML, PageKind::Step)?;
        assert_eq!(step.title, "문법 1. 입출력과 사칙연산");
        assert_eq!(
            step.problems,
            [
                2557, 1000, 1001, 10998, 1008, 10869, 10926, 18108, 10430, 2588, 11382, 10171,
                10172
            ]
        );
        Ok(())
    }

    #[test]
    fn reads_workbook_title_that_mentions_access_restriction() -> Result<(), WorkbookError> {
        let html =
            WORKBOOK_HTML.replace("연습 &amp; <span>구현</span>", "비공개 문제집에서 공개로");
        assert_eq!(
            parse(&html, PageKind::Workbook)?.title,
            "비공개 문제집에서 공개로"
        );
        Ok(())
    }

    #[test]
    fn rejects_unrelated_and_incomplete_pages() {
        assert!(matches!(
            parse(
                "<h1>연습</h1><a href=\"/problem/42\">문제</a>",
                PageKind::Workbook
            ),
            Err(WorkbookError::Parse(_))
        ));
        assert!(matches!(
            parse(
                "<h1>연습</h1><table id=\"problemset\"><tbody></tbody></table>",
                PageKind::Workbook,
            ),
            Err(WorkbookError::Parse(_))
        ));
        assert!(matches!(
            parse(WORKBOOK_HTML, PageKind::Step),
            Err(WorkbookError::Parse(_))
        ));
    }

    #[test]
    fn recognizes_access_restriction_pages() {
        for kind in [PageKind::Workbook, PageKind::Step] {
            for html in [
                "<title>Just a moment...</title><form id=\"challenge-form\"></form>",
                "<div class=\"alert-danger\">로그인이 필요합니다.</div>",
                "<title>429 Too Many Requests</title>",
            ] {
                assert!(matches!(
                    parse(html, kind),
                    Err(WorkbookError::AccessRestricted)
                ));
            }
        }
    }
}
