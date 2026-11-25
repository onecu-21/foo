# foo — BOJ CLI Helper

**Less setup. More solving.**

English · [한국어](README.ko.md)

`foo` is a Rust CLI for [Baekjoon Online Judge (BOJ)](https://www.acmicpc.net). It remembers your workbooks and where you left off, creates a source file and VS Code run configuration for each problem, and builds and runs your solution. It ships as a single executable for Windows, Linux, and macOS.

## Install and get started

Install stable Rust with Cargo, VS Code, and a compiler or interpreter for the language you want to use. Then run this from the repository root:

```sh
cargo install --path . --locked
foo init
```

`foo init` asks for your language and workspace folder. The defaults are `cpp` and a `boj/` folder in the current directory. If your compiler and `code` are on `PATH`, `foo` finds them; otherwise you can enter the executable paths yourself. If VS Code was already running before you installed anything, restart it so it picks up the new `PATH`. On macOS, register `code` with the `Shell Command: Install 'code' command in PATH` command in the Command Palette. See the [VS Code command-line docs](https://code.visualstudio.com/docs/configure/command-line).

macOS on Apple silicon (arm64) is not built or tested in CI. I think it'll work.... maybe? Intel macOS, Linux, and Windows are tested.

Nightly builds of `main` are on the [Nightly release](https://github.com/onecu-21/foo/releases/tag/nightly) page.

To build by hand, run `cargo build --release --locked` and copy `target/release/foo` (`target\release\foo.exe` on Windows) into a folder on your `PATH`. The F5 run configuration uses the same `foo`.

## Workbooks and solving

You can register regular workbooks (`/workbook/view/<ID>`) and steps (`/step/<ID>`). Both accept HTTP and HTTPS URLs on `acmicpc.net` and `www.acmicpc.net`. Problem numbers are read in the order shown on the page; if a number appears more than once, its first position wins.

BOJ prohibits scraping the site, so `foo` never fetches pages. Import a workbook from an HTML file you saved yourself in your browser (UTF-8), or give it a title and a list of problem numbers:

```sh
foo remember https://www.acmicpc.net/workbook/view/42 --html workbook.html
foo remember https://acmicpc.net/step/1 --html step-1.html
foo status
foo start
foo next
foo before
foo start 3
foo open 2557
```

To register a workbook by hand, list the problem numbers in workbook order:

```sh
foo remember https://www.acmicpc.net/workbook/view/42 --title "Input and output" --problems 1000,1001,2557
```

Imported workbooks and your position are stored locally. Running `foo remember https://www.acmicpc.net/workbook/view/42` with just the URL re-selects a stored workbook. When you re-import a workbook with new HTML or a new list, `foo` continues from the current problem if it is still in the list. Otherwise it keeps the old position, or moves to the last problem if the old position is now out of range.

| Command | What it does |
| --- | --- |
| `foo init` | Interactively sets the language, tools, VS Code, and workspace folder. |
| `foo remember <URL> [--html <FILE>]` | Selects a stored workbook or step, or imports one from saved HTML. You can also register one by hand with `--title` and `--problems`. |
| `foo start [index]` | Goes to your last saved position, or to a problem by its 1-based position. |
| `foo next` / `foo before` | Moves one problem forward or back in the workbook. |
| `foo status` | Shows the active workbook, the problem number, your position, and the language. |
| `foo open <problem_id>` | Opens a problem's folder by number, regardless of your workbook position. |
| `foo config` | Shows the current settings. |
| `foo config --language <lang> --workspace <dir>` | Changes the language and workspace folder. `--compiler` and `--code` set executable paths. |

Pass `--no-launch`, as in `foo --no-launch open 2557`, to prepare the files without opening VS Code or the browser. Without VS Code installed, you can still set up with `foo --no-launch init`. `foo config --reset-compiler` and `foo config --reset-code` clear a saved path and go back to auto-detection.

### Output language

Output and `--help` are in English by default. Use `--lang ko`, or set `FOO_LANG=ko`, to get Korean instead. The flag wins over the environment variable.

```sh
foo --lang ko --help
FOO_LANG=ko foo status
```

## Running in VS Code

In a problem folder opened by `foo start` or `foo open`, press `F5`. If a list of run configurations appears, choose `foo: Build and run`. VS Code's built-in JavaScript Debugger runs `foo` in the integrated terminal, and `foo` builds or runs the source for the problem's language. Paste the sample input into the terminal and read the output there. If an earlier run session is still open, stop it and press `F5` again. `Tasks: Run Build Task` does the same thing. Breakpoint debugging needs extra per-language setup that `foo` does not provide.

| `language` value | Toolchain | Source file |
| --- | --- | --- |
| `c` | `clang` | `main.c` |
| `cpp` | `clang++` | `main.cpp` |
| `python` | Python 3 | `main.py` |
| `pypy` | `pypy3` | `main.py` |
| `rust` | `rustc` | `main.rs` |
| `java` | `java` from JDK 21 | `Main.java` |
| `go` | `go` | `main.go` |
| `javascript` | `node` | `main.js` |
| `kotlin` | `kotlinc` and `java` (Kotlin/JVM) | `Main.kt` |
| `csharp` (alias `dotnet`) | `dotnet` (.NET SDK 10 or later) | `main.cs` |
| `cobol` (alias `cob`) | `cobc` (GnuCOBOL) | `main.cob` |

If `foo` cannot find the compiler, install it and reopen your terminal, or point to it with `foo config --compiler <path>`. C and C++ use Clang by default. If you configure MSVC, `foo` stops with `HeresyError: Thou shalt use Clang.` and shows how to set up Clang. Java runs in JDK 21 source-file mode. C# runs in .NET 10 file-based app mode (`dotnet run main.cs`). COBOL is built with `cobc -x -free` (free-format source). Native executables and Kotlin build output are written to a temporary folder.

For code navigation and completion, install extensions from VS Code's Extensions tab or with `code --install-extension <ID>`. Common extension IDs are `llvm-vs-code-extensions.vscode-clangd` for C/C++, `ms-python.python` for Python and PyPy, `rust-lang.rust-analyzer` for Rust, `vscjava.vscode-java-pack` for Java, and `golang.go` for Go, and `ms-dotnettools.csharp` for C#, and `bitlang.cobol` for COBOL

## Files and settings

```text
boj/
├── .vscode/
├── 1000/
│   ├── .vscode/
│   └── main.cpp
└── 2557/
    ├── .vscode/
    └── main.cpp
```

The workspace folder and each problem folder get their own `tasks.json` and `launch.json`. If a source file or VS Code config of the same name already exists, `foo` leaves it untouched. To add the `foo` run configuration to a `launch.json` you wrote yourself, copy the `foo: Build and run` entry from one of the generated problem folders. Each problem's language is recorded in a `.foo-language` file in its folder. The default language you pick with `foo config --language` applies to new problems.

Your settings live in `config.toml`, and your registered workbooks and positions live in `state.toml`. Both are stored in your user config folder:

| OS | Config folder |
| --- | --- |
| Windows | `%APPDATA%\foo` |
| Linux | `$XDG_CONFIG_HOME/foo`, or `~/.config/foo` by default |
| macOS | `~/Library/Application Support/foo` |

Set the `FOO_CONFIG_DIR` environment variable to an absolute path to use a different config folder. If a TOML file is corrupted, `foo` reports the error and the file's path. Back the file up before editing it to keep its contents.

## Development

- `src/cli.rs` and `src/app.rs` handle command-line arguments and the flow of each command.
- `src/store.rs` reads and writes your settings and workbook progress.
- `src/workbook.rs` validates workbook and step URLs and reads the title and problem list from local HTML.
- `src/language.rs` handles per-language templates, tool discovery, and build and run.
- `src/workspace.rs` creates problem folders and VS Code configs, and opens the editor and browser.
- `src/text.rs` picks the output language (English by default, Korean on request).
- `tests/cli.rs` uses temporary folders and HTML fixtures to test registration, navigation, source preservation, error handling, and running.

URL validation uses `reqwest`'s URL type, and workbook HTML is parsed with `scraper`. `foo` never sends requests to the site, and the tests run without a network connection.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

HTML import reads the problem rows in the saved file. If your saved page is missing the problem list, or BOJ changes its page layout, register the workbook by hand with `--title` and `--problems`. Login, automatic submission, collecting accepted solutions, AI solving, and test-case generation are out of scope.

## License

Licensed under the [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-only).
