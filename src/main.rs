mod app;
mod cli;
mod error;
mod language;
mod store;
mod text;
mod workbook;
mod workspace;

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let args = env::args_os().collect::<Vec<_>>();
    let lang = text::detect(&args);
    text::set(lang);
    match app::execute(cli::parse(args, lang)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}: {error}", text::t!("error", "오류"));
            ExitCode::FAILURE
        }
    }
}
