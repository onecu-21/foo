# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`foo` is a Rust CLI (edition 2024, MSRV 1.89) that manages a local workspace for solving Baekjoon Online Judge (BOJ) problems: it stores workbooks/steps and progress, creates per-problem source files and VS Code `tasks.json`/`launch.json`, and builds/runs solutions. User-facing text is English by default and Korean on request (`--lang ko` or `FOO_LANG=ko`, chosen in `text.rs` before clap runs so `--help` is localized too). Wrap every new user-facing message in `text::t!("English", "한국어")`, and for new clap args add the Korean help to `cli::korean`. `README.md` is English; `README.ko.md` is the Korean copy, so keep the two in sync. Korean string literals in `workbook.rs` (e.g. `문제 제목`, the access-page markers) match BOJ page content, not output, so do not translate them.

## Commands

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test cli <name_substring>      # one integration test
cargo test --bin foo <name_substring>       # one unit test (in src/*.rs `mod tests`)
cargo build --release --locked
```

On Windows run cargo from PowerShell, not Git Bash: Git's `/usr/bin/link.exe` shadows the MSVC linker and breaks the test that compiles a Rust solution.

Integration tests (`tests/cli.rs`) run the real binary via `CARGO_BIN_EXE_foo`, with `FOO_CONFIG_DIR` pointed at a temp dir and `--no-launch` always set. They initialize with the `rust` language, so `rustc` must be on `PATH`; one test actually compiles and runs a Rust solution through `foo __run`. Fixtures in `tests/fixtures/` are saved BOJ pages (`workbook.html`, `step.html`). Tests need no network.

## Architecture

Flow: `main.rs` → `cli.rs` (clap derive) → `app::execute`, which dispatches each subcommand. `app.rs` holds the orchestration; the other modules are mostly independent helpers, each with its own error enum wrapped by `error::Error`.

- **`store.rs`**: two TOML files in the config dir (`FOO_CONFIG_DIR`, else `%APPDATA%\foo`, `~/Library/Application Support/foo`, or `$XDG_CONFIG_HOME`/`~/.config/foo`). `config.toml` holds the language, the absolute workspace path, and optional compiler/VS Code overrides. `state.toml` holds the collections (canonical URL, title, problems, 0-based `position`) and `active_url`. Both use `deny_unknown_fields`, and `state()` checks invariants on load (canonical URLs, no duplicates, `position < len`, the active URL exists). Writes go to a temp file and are then renamed atomically. Commands that change state take the `foo.lock` file lock first via `store.lock()`.
- **`workbook.rs`**: canonicalizes `/workbook/view/<id>` and `/step/<id>` URLs to `https://acmicpc.net...` and parses **locally saved** HTML with `scraper`. Workbook and step pages use different row selectors. Problem IDs are deduplicated, keeping the first occurrence. **Never add network fetching:** BOJ forbids scraping, and `remember <URL>` with no `--html` and no `--title/--problems` deliberately fails with `SitePolicy` unless the URL is already stored.
- **`language.rs`**: the `Language` enum (also a clap `ValueEnum` and serde value), with each language's source filename, template, default compiler, compiler discovery, and the build+run commands. Native and Kotlin build output goes to a temp dir. If an MSVC compiler is configured for C/C++, it is rejected on purpose with `Error::Heresy` ("Thou shalt use Clang"); this is intended behavior.
- **`text.rs`**: `Lang`, the `OnceLock` holding the active language, and the `t!` macro. Tests assert on the English output by default and pass `--lang ko` for the Korean cases.
- **`workspace.rs`**: creates `<workspace>/.vscode` and `<workspace>/<id>/.vscode` plus the source template. `write_new` never overwrites existing files. The generated task/launch configs call the hidden subcommand `foo __run` and prepend the current exe's directory to `PATH` (and pass `FOO_CONFIG_DIR` through) using `runner_env`. It also launches VS Code and the browser; `--no-launch` skips both.
- **Per-problem language**: `<problem dir>/.foo-language` records which language a problem was created with. It takes precedence over the default language in the config. The compiler override in the config applies only when the problem's language equals the default language.
- **Re-importing** a stored workbook keeps the current problem if it is still in the list; otherwise it keeps the old index, clamped to the new length (see `app::remember`).

CI (`.github/workflows/ci.yml`): fmt once, then clippy + full tests + release build on native runners (Linux/Windows x64, x86, arm64; macOS x64; macOS arm64 is deliberately not in CI and unverified), and release build + unit tests only under `cross`/QEMU for Linux powerpc64, powerpc64le, riscv64 (the integration tests need a native `rustc`). riscv32 is intentionally absent: no hosted Rust target.

Out of scope by design (per README): login, auto-submit, collecting solutions, AI solving/testcase generation.

# Very Very Important
- If user is trying to use MSVC, return Heresy Error