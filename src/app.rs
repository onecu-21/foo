use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::ValueEnum;

use crate::{
    cli::{Action, Cli, ConfigEdit, Remember},
    error::Error,
    language::Language,
    store::{Collection, Config, Store, absolute},
    text::t,
    workbook::{self, Workbook, WorkbookError},
    workspace,
};

const PROBLEM_LANGUAGE: &str = ".foo-language";

pub(crate) fn execute(cli: Cli) -> Result<(), Error> {
    let store = Store::locate()?;
    let no_launch = cli.no_launch;
    match cli.command {
        Action::Init => init(&store, no_launch),
        Action::Remember(args) => remember(&store, args),
        Action::Start { index } => navigate(&store, Navigation::Start(index), no_launch),
        Action::Next => navigate(&store, Navigation::Next, no_launch),
        Action::Before => navigate(&store, Navigation::Before, no_launch),
        Action::Status => status(&store),
        Action::Open { problem_id } => open(&store, problem_id, no_launch),
        Action::Config(edit) => configure(&store, edit),
        Action::Run => run(&store),
    }
}

fn init(store: &Store, no_launch: bool) -> Result<(), Error> {
    if store.config()?.is_some() {
        return Err(Error::AlreadyInitialized(store.config_path()));
    }

    let languages = Language::value_variants()
        .iter()
        .filter_map(ValueEnum::to_possible_value)
        .map(|value| value.get_name().to_owned())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "{}",
        t!("Supported languages: {languages}", "지원 언어: {languages}")
    );
    let language = match prompt(&t!("Default language [cpp]", "기본 언어 [cpp]"))?.as_str() {
        "" => Language::from_str("cpp", true),
        name => Language::from_str(name, true),
    }
    .map_err(Error::InvalidLanguage)?;
    let compiler = prompt_path(&t!(
        "Compiler or interpreter path [auto-detect: {}]",
        "실행 도구 경로 [자동 탐색: {}]",
        language.default_compiler()
    ))?;
    let selected_compiler = language.find_compiler(compiler.as_deref())?;
    let code = prompt_path(&t!(
        "VS Code path [auto-detect]",
        "VS Code 경로 [자동 탐색]"
    ))?;
    let selected_code = if no_launch && code.is_none() {
        match workspace::find_vscode(None) {
            Ok(path) => Some(path),
            Err(err) => {
                eprintln!("{err}");
                None
            }
        }
    } else {
        Some(workspace::find_vscode(code.as_deref())?)
    };
    let workspace = absolute(
        prompt_path(&t!("Workspace directory [./boj]", "작업 디렉터리 [./boj]"))?
            .unwrap_or_else(|| "boj".into()),
    )?;
    let config = Config {
        language,
        workspace,
        compiler: compiler.map(|_| selected_compiler.clone()),
        code: code.and(selected_code.clone()),
    };

    let _lock = store.lock()?;
    if store.config()?.is_some() {
        return Err(Error::AlreadyInitialized(store.config_path()));
    }
    workspace::setup_root(&config.workspace)?;
    store.save_config(&config, true)?;
    println!(
        "{}",
        t!("Compiler: {}", "실행 도구: {}", selected_compiler.display())
    );
    if let Some(code) = selected_code {
        println!("VS Code: {}", code.display());
    }
    if let Some(extension) = language.vscode_extension() {
        println!(
            "{}",
            t!(
                "Optional: install the VS Code extension with `code --install-extension {extension}`",
                "선택 사항인 VS Code 확장 설치: code --install-extension {extension}"
            )
        );
    }
    print_config(store, &config);
    Ok(())
}

fn prompt(label: &str) -> Result<String, Error> {
    print!("{label}: ");
    io::stdout()
        .flush()
        .map_err(|err| Error::Io("stdout".into(), err))?;
    let mut line = String::new();
    if io::stdin()
        .read_line(&mut line)
        .map_err(|err| Error::Io("stdin".into(), err))?
        == 0
    {
        return Err(Error::InputClosed);
    }
    Ok(line.trim().to_owned())
}

fn prompt_path(label: &str) -> Result<Option<PathBuf>, Error> {
    let response = prompt(label)?;
    Ok((!response.is_empty()).then(|| response.into()))
}

fn configure(store: &Store, edit: ConfigEdit) -> Result<(), Error> {
    if !edit.has_changes() {
        print_config(store, &require_config(store)?);
        return Ok(());
    }

    let _lock = store.lock()?;
    let mut config = require_config(store)?;
    if let Some(language) = edit.language
        && language != config.language
    {
        config.language = language;
        config.compiler = None;
    }
    if edit.reset_compiler {
        config.compiler = None;
    }
    if let Some(path) = edit.compiler {
        config.compiler = Some(config.language.find_compiler(Some(&path))?);
    } else if edit.language.is_some() {
        config.language.find_compiler(config.compiler.as_deref())?;
    }
    if edit.reset_code {
        config.code = None;
    }
    if let Some(path) = edit.code {
        config.code = Some(workspace::find_vscode(Some(&path))?);
    }
    if let Some(path) = edit.workspace {
        config.workspace = absolute(path)?;
    }

    workspace::setup_root(&config.workspace)?;
    store.save_config(&config, false)?;
    print_config(store, &config);
    Ok(())
}

fn print_config(store: &Store, config: &Config) {
    println!(
        "{}",
        t!(
            "Config file: {}",
            "설정 파일: {}",
            store.config_path().display()
        )
    );
    println!("{}", t!("Language: {}", "언어: {}", config.language));
    println!(
        "{}",
        t!(
            "Workspace: {}",
            "작업 디렉터리: {}",
            config.workspace.display()
        )
    );
    println!(
        "{}",
        t!(
            "Compiler: {}",
            "실행 도구: {}",
            configured_path(config.compiler.as_deref())
        )
    );
    println!("VS Code: {}", configured_path(config.code.as_deref()));
}

fn configured_path(path: Option<&Path>) -> String {
    path.map_or_else(
        || t!("auto-detect", "자동 탐색"),
        |path| path.display().to_string(),
    )
}

fn remember(store: &Store, args: Remember) -> Result<(), Error> {
    let url = workbook::parse_url(&args.url)?;
    let workbook = if let Some(path) = args.html {
        Some(workbook::parse_file(&path, url.kind)?)
    } else if let Some(title) = args.title {
        if title.trim().is_empty() {
            return Err(Error::InvalidTitle);
        }
        Some(Workbook::new(title.trim().to_owned(), args.problems))
    } else {
        None
    };
    let url = url.canonical;

    let _lock = store.lock()?;
    let mut state = store.state()?;
    let existing = state.collections.iter_mut().find(|item| item.url == url);
    match (workbook, existing) {
        (None, None) => return Err(WorkbookError::SitePolicy.into()),
        (None, Some(_)) => {}
        (Some(workbook), Some(entry)) => {
            let current = entry.problems[entry.position];
            entry.position = workbook
                .problems
                .iter()
                .position(|&id| id == current)
                .unwrap_or_else(|| entry.position.min(workbook.problems.len() - 1));
            entry.title = workbook.title;
            entry.problems = workbook.problems;
        }
        (Some(workbook), None) => state.collections.push(Collection {
            url: url.clone(),
            title: workbook.title,
            problems: workbook.problems,
            position: 0,
        }),
    }
    state.active_url = Some(url);
    store.save_state(&state)?;
    let entry = state.active()?;
    println!(
        "{}",
        t!(
            "Workbook: {} ({} problems)",
            "문제집: {} ({}문제)",
            entry.title,
            entry.problems.len()
        )
    );
    Ok(())
}

fn status(store: &Store) -> Result<(), Error> {
    let state = store.state()?;
    let entry = state.active()?;
    let id = entry.problems[entry.position];
    println!("{}", t!("Workbook: {}", "문제집: {}", entry.title));
    println!("{}", t!("Problem: {id}", "문제: {id}"));
    println!(
        "{}",
        t!(
            "Progress: {}/{}",
            "진행: {}/{}",
            entry.position + 1,
            entry.problems.len()
        )
    );
    if let Some(config) = store.config()? {
        let directory = config.workspace.join(id.to_string());
        println!(
            "{}",
            t!(
                "Language: {}",
                "언어: {}",
                problem_language(&directory, config.language)?
            )
        );
    } else {
        println!(
            "{}",
            t!(
                "Language: not set up (run `foo init`)",
                "언어: 설정 전 (foo init)"
            )
        );
    }
    Ok(())
}

enum Navigation {
    Start(Option<usize>),
    Next,
    Before,
}

fn navigate(store: &Store, navigation: Navigation, no_launch: bool) -> Result<(), Error> {
    let lock = store.lock()?;
    let mut state = store.state()?;
    let index = state.active_index()?;
    let entry = &mut state.collections[index];
    let position = match navigation {
        Navigation::Start(Some(index)) => index.checked_sub(1),
        Navigation::Start(None) => Some(entry.position),
        Navigation::Next => entry.position.checked_add(1),
        Navigation::Before => entry.position.checked_sub(1),
    }
    .filter(|&index| index < entry.problems.len())
    .ok_or(Error::Position(entry.problems.len()))?;
    let id = entry.problems[position];
    let config = require_config(store)?;
    let prepared = prepare(&config, id, no_launch)?;
    if position != entry.position {
        entry.position = position;
        store.save_state(&state)?;
    }
    drop(lock);
    prepared.launch(id)
}

fn open(store: &Store, id: u32, no_launch: bool) -> Result<(), Error> {
    let lock = store.lock()?;
    let config = require_config(store)?;
    let prepared = prepare(&config, id, no_launch)?;
    drop(lock);
    prepared.launch(id)
}

struct Prepared {
    directory: PathBuf,
    language: Language,
    code: Option<PathBuf>,
}

impl Prepared {
    fn launch(self, id: u32) -> Result<(), Error> {
        println!(
            "{}",
            t!(
                "Problem {id} · {} · {}",
                "문제 {id} · {} · {}",
                self.language,
                self.directory.display()
            )
        );
        println!("{}", workspace::problem_url(id));
        if let Some(code) = self.code {
            let editor =
                workspace::open_problem(&code, &self.directory, self.language.source_name());
            let browser = workspace::open_browser(id);
            editor.and(browser)?;
        }
        Ok(())
    }
}

fn prepare(config: &Config, id: u32, no_launch: bool) -> Result<Prepared, Error> {
    let code = if no_launch {
        None
    } else {
        Some(workspace::find_vscode(config.code.as_deref())?)
    };
    let directory = config.workspace.join(id.to_string());
    let language = problem_language(&directory, config.language)?;
    find_compiler(config, language)?;
    let directory = workspace::prepare_problem(&config.workspace, id, language)?;
    save_problem_language(&directory, language)?;
    Ok(Prepared {
        directory,
        language,
        code,
    })
}

fn require_config(store: &Store) -> Result<Config, Error> {
    store.config()?.ok_or(Error::NotInitialized)
}

fn find_compiler(config: &Config, language: Language) -> Result<PathBuf, Error> {
    let override_path = config
        .compiler
        .as_deref()
        .filter(|_| config.language == language);
    Ok(language.find_compiler(override_path)?)
}

fn problem_language(directory: &Path, fallback: Language) -> Result<Language, Error> {
    let path = directory.join(PROBLEM_LANGUAGE);
    match fs::read_to_string(&path) {
        Ok(content) => {
            Language::from_str(content.trim(), true).map_err(|err| Error::InvalidData(path, err))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(fallback),
        Err(err) => Err(Error::Io(path, err)),
    }
}

fn save_problem_language(directory: &Path, language: Language) -> Result<(), Error> {
    let path = directory.join(PROBLEM_LANGUAGE);
    if path.is_file() {
        return Ok(());
    }
    let mut temp = tempfile::NamedTempFile::new_in(directory)
        .map_err(|err| Error::Io(directory.to_owned(), err))?;
    writeln!(temp, "{}", language.as_str()).map_err(|err| Error::Io(path.clone(), err))?;
    match temp.persist_noclobber(&path) {
        Ok(_) => Ok(()),
        Err(err) if err.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(Error::Io(path, err.error)),
    }
}

fn run(store: &Store) -> Result<(), Error> {
    let config = require_config(store)?;
    let directory = env::current_dir().map_err(|err| Error::Io(".".into(), err))?;
    let language = problem_language(&directory, config.language)?;
    let compiler = find_compiler(&config, language)?;
    language.run(&compiler, &directory)?;
    Ok(())
}
