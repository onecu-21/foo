//! Output language selection. English is the default; Korean is opt-in via
//! `--lang ko` or the `FOO_LANG=ko` environment variable.

use std::{env, ffi::OsString, sync::OnceLock};

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum Lang {
    #[default]
    #[value(name = "en")]
    En,
    #[value(name = "ko")]
    Ko,
}

static LANG: OnceLock<Lang> = OnceLock::new();

/// Picks the language before clap runs, so that `--help` can be localized too.
/// Precedence: `--lang` on the command line, then `FOO_LANG`, then English.
pub(crate) fn detect(args: &[OsString]) -> Lang {
    let mut flag = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        let Some(arg) = arg.to_str() else { continue };
        if arg == "--" {
            break;
        }
        if arg == "--lang" {
            flag = iter.next().and_then(|value| value.to_str());
        } else if let Some(value) = arg.strip_prefix("--lang=") {
            flag = Some(value);
        }
    }
    let from_env = env::var("FOO_LANG").ok();
    flag.or(from_env.as_deref())
        .and_then(|value| Lang::from_str(value, true).ok())
        .unwrap_or_default()
}

pub(crate) fn set(lang: Lang) {
    let _ = LANG.set(lang);
}

pub(crate) fn is_korean() -> bool {
    LANG.get() == Some(&Lang::Ko)
}

/// `t!("English {}", "한국어 {}", arg)` formats the message in the active language.
macro_rules! t {
    ($en:literal, $ko:literal $(, $arg:expr)* $(,)?) => {
        if $crate::text::is_korean() {
            format!($ko $(, $arg)*)
        } else {
            format!($en $(, $arg)*)
        }
    };
}
pub(crate) use t;
