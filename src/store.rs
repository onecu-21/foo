use std::{
    collections::HashSet,
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tempfile::NamedTempFile;

use crate::{error::Error, language::Language, text::t, workbook};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) language: Language,
    pub(crate) workspace: PathBuf,
    pub(crate) compiler: Option<PathBuf>,
    pub(crate) code: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    pub(crate) active_url: Option<String>,
    #[serde(default)]
    pub(crate) collections: Vec<Collection>,
}

impl State {
    pub(crate) fn active(&self) -> Result<&Collection, Error> {
        Ok(&self.collections[self.active_index()?])
    }

    pub(crate) fn active_index(&self) -> Result<usize, Error> {
        self.collections
            .iter()
            .position(|entry| Some(&entry.url) == self.active_url.as_ref())
            .ok_or(Error::NoWorkbook)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Collection {
    pub(crate) url: String,
    pub(crate) title: String,
    pub(crate) problems: Vec<u32>,
    pub(crate) position: usize,
}

pub(crate) struct Store(PathBuf);

impl Store {
    pub(crate) fn locate() -> Result<Self, Error> {
        if let Some(path) = env_path("FOO_CONFIG_DIR") {
            return absolute(path).map(Self);
        }

        #[cfg(windows)]
        let base = env_path("APPDATA");
        #[cfg(target_os = "macos")]
        let base = env_path("HOME").map(|home| home.join("Library/Application Support"));
        #[cfg(all(not(windows), not(target_os = "macos")))]
        let base = env_path("XDG_CONFIG_HOME")
            .filter(|path| path.is_absolute())
            .or_else(|| env_path("HOME").map(|home| home.join(".config")));

        let base = base.ok_or(Error::ConfigDirectory)?;
        Ok(Self(base.join("foo")))
    }

    pub(crate) fn config_path(&self) -> PathBuf {
        self.0.join("config.toml")
    }

    pub(crate) fn config(&self) -> Result<Option<Config>, Error> {
        let path = self.config_path();
        let config: Option<Config> = read_toml(&path)?;
        if let Some(config) = &config
            && !config.workspace.is_absolute()
        {
            return Err(Error::InvalidData(
                path,
                t!(
                    "workspace must be an absolute path.",
                    "workspace는 절대 경로여야 합니다."
                ),
            ));
        }
        Ok(config)
    }

    pub(crate) fn save_config(&self, config: &Config, new: bool) -> Result<(), Error> {
        self.save(&self.config_path(), config, new)
    }

    pub(crate) fn state(&self) -> Result<State, Error> {
        let path = self.0.join("state.toml");
        let state: State = read_toml(&path)?.unwrap_or_default();
        let mut seen = HashSet::new();
        for entry in &state.collections {
            if !workbook::parse_url(&entry.url).is_ok_and(|url| url.canonical == entry.url)
                || !seen.insert(&entry.url)
                || entry.title.trim().is_empty()
                || entry.problems.contains(&0)
                || entry.position >= entry.problems.len()
            {
                return Err(Error::InvalidData(
                    path,
                    t!(
                        "Check the workbook URL, title, problem numbers, and position.",
                        "문제집 URL, 제목, 문제 번호 또는 진행 위치를 확인하세요."
                    ),
                ));
            }
        }
        if state
            .active_url
            .as_ref()
            .is_some_and(|url| !seen.contains(url))
        {
            return Err(Error::InvalidData(
                path,
                t!(
                    "The active workbook is not in the saved list.",
                    "활성 문제집이 저장 목록에 없습니다."
                ),
            ));
        }
        Ok(state)
    }

    pub(crate) fn save_state(&self, state: &State) -> Result<(), Error> {
        self.save(&self.0.join("state.toml"), state, false)
    }

    pub(crate) fn lock(&self) -> Result<File, Error> {
        fs::create_dir_all(&self.0).map_err(|err| Error::Io(self.0.clone(), err))?;
        let path = self.0.join("foo.lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|err| Error::Io(path.clone(), err))?;
        file.lock().map_err(|err| Error::Io(path, err))?;
        Ok(file)
    }

    fn save(&self, path: &Path, value: &impl Serialize, new: bool) -> Result<(), Error> {
        let content = toml::to_string_pretty(value).map_err(Error::Encode)?;
        let mut temp =
            NamedTempFile::new_in(&self.0).map_err(|err| Error::Io(self.0.clone(), err))?;
        let io = |err| Error::Io(path.to_owned(), err);
        temp.write_all(content.as_bytes()).map_err(io)?;
        temp.as_file().sync_all().map_err(io)?;
        let result = if new {
            temp.persist_noclobber(path)
        } else {
            temp.persist(path)
        };
        result.map_err(|err| io(err.error))?;
        Ok(())
    }
}

pub(crate) fn absolute(path: PathBuf) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(env::current_dir()
            .map_err(|err| Error::Io(PathBuf::from("."), err))?
            .join(path))
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, Error> {
    match fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content)
            .map(Some)
            .map_err(|err| Error::InvalidData(path.to_owned(), err.to_string())),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(Error::Io(path.to_owned(), err)),
    }
}
