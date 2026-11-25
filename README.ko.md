# foo — BOJ CLI Helper

**Less setup. More solving.**

[English](README.md) · 한국어

백준 온라인 저지(BOJ) 문제집과 풀이 위치를 저장하고, 문제별 소스와 VS Code 실행 설정을 만드는 Rust CLI입니다. Windows, Linux, macOS용 단일 실행 파일로 빌드합니다.

## 설치와 시작

Rust stable과 Cargo, VS Code, 사용할 언어의 컴파일러 또는 인터프리터를 설치합니다. 이 저장소의 루트에서 다음 명령을 실행합니다.

```sh
cargo install --path . --locked
foo init
```

`foo init`에서 사용할 언어와 작업 폴더를 선택합니다. 기본값은 `cpp`와 현재 폴더 아래의 `boj/`입니다. 컴파일러 및 `code`가 `PATH`에 있으면 탐색하고, 필요하면 실행 파일 경로를 직접 입력할 수 있습니다. 기존 VS Code 프로세스가 설치 전 `PATH`를 사용하고 있으면 VS Code를 다시 시작합니다. macOS에서는 명령 팔레트의 `Shell Command: Install 'code' command in PATH`로 `code`를 등록할 수 있습니다. [VS Code 명령줄 문서](https://code.visualstudio.com/docs/configure/command-line)

Apple silicon(arm64) macOS는 CI에서 빌드나 테스트를 하지 않아 동작을 확인하지 못했습니다. Intel macOS, Linux, Windows는 CI에서 테스트합니다.

직접 빌드하려면 `cargo build --release --locked`를 실행하고, `target/release/foo` 또는 Windows의 `target\release\foo.exe`를 `PATH`에 포함된 폴더에 복사합니다. F5 실행에도 같은 `foo`를 사용합니다.

## 문제집 등록과 풀이

일반 문제집(`/workbook/view/<ID>`)과 단계별 문제(`/step/<ID>`)를 등록할 수 있습니다. 두 유형 모두 `acmicpc.net`과 `www.acmicpc.net`의 HTTP 및 HTTPS URL을 지원합니다. 페이지에 표시된 순서대로 문제 번호를 가져오며, 같은 번호가 여러 번 나오면 처음 나온 위치를 사용합니다.

BOJ는 사이트 스크래핑을 금지합니다. 새 문제집은 브라우저에서 직접 저장한 UTF-8 HTML 파일 또는 제목과 문제 번호 목록으로 가져옵니다. [BOJ 이용 규칙](https://help.acmicpc.net/rule)

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

번호를 직접 등록하는 예입니다. 문제 번호의 순서는 문제집 순서가 됩니다.

```sh
foo remember https://www.acmicpc.net/workbook/view/42 --title "입출력 연습" --problems 1000,1001,2557
```

가져온 문제집과 풀이 위치는 로컬에 저장됩니다. `foo remember https://www.acmicpc.net/workbook/view/42`처럼 URL만 주면 저장된 문제집을 다시 선택합니다. 새 HTML이나 목록으로 다시 가져올 때 현재 문제 번호가 목록에 있으면 그 문제에서 이어갑니다. 빠졌다면 기존 순번을 사용하며, 기존 순번이 새 목록의 범위를 넘으면 마지막 문제로 이동합니다.

| 명령 | 동작 |
| --- | --- |
| `foo init` | 언어, 실행 도구, VS Code, 작업 폴더를 대화형으로 설정합니다. |
| `foo remember <URL> [--html <FILE>]` | 저장된 항목을 선택하거나 HTML에서 일반 문제집 또는 단계별 문제를 가져옵니다. `--title`과 `--problems`로 직접 등록할 수도 있습니다. |
| `foo start [index]` | 마지막으로 저장된 위치 또는 1부터 시작하는 문제 순번으로 이동합니다. |
| `foo next` / `foo before` | 문제집에서 한 문제 앞으로 또는 뒤로 이동합니다. |
| `foo status` | 선택한 문제집, 문제 번호, 순번과 언어를 확인합니다. |
| `foo open <problem_id>` | 문제집의 진행 위치와 무관하게 문제 번호로 작업 폴더를 엽니다. |
| `foo config` | 현재 설정을 확인합니다. |
| `foo --lang ko <명령>` | 출력과 도움말을 한국어로 표시합니다. `FOO_LANG=ko` 환경 변수로도 지정할 수 있으며, 기본값은 영어입니다. |
| `foo config --language <lang> --workspace <dir>` | 사용할 언어와 작업 폴더를 바꿉니다. `--compiler`와 `--code`로 실행 파일을 지정할 수 있습니다. |

`foo --no-launch open 2557`처럼 `--no-launch`를 주면 파일을 준비하고 VS Code와 브라우저 실행을 생략합니다. VS Code가 없는 환경에서는 `foo --no-launch init`으로 설정할 수 있습니다. `foo config --reset-compiler`와 `foo config --reset-code`로 지정한 경로를 지우고 자동 탐색으로 돌아갈 수 있습니다.

## VS Code에서 실행

`foo start` 또는 `foo open`으로 열린 문제 폴더에서 `F5`를 누릅니다. 실행 구성 목록이 나오면 `foo: Build and run`을 선택합니다. VS Code의 기본 JavaScript Debugger가 통합 터미널에서 `foo`를 실행하고, `foo`가 언어에 맞춰 소스를 빌드하거나 실행합니다. 통합 터미널에 예제 입력을 붙여 넣고 같은 곳에서 출력을 확인합니다. 이전 실행 세션이 열려 있으면 중지하고 `F5`를 다시 누릅니다. `Tasks: Run Build Task`로도 같은 실행을 할 수 있습니다. 언어별 중단점 디버깅은 별도 구성이 필요합니다.

| 언어 설정값 | 실행 환경 | 소스 파일 |
| --- | --- | --- |
| `c` | `clang` | `main.c` |
| `cpp` | `clang++` | `main.cpp` |
| `python` | Python 3 | `main.py` |
| `pypy` | `pypy3` | `main.py` |
| `rust` | `rustc` | `main.rs` |
| `java` | JDK 21의 `java` | `Main.java` |
| `go` | `go` | `main.go` |
| `javascript` | `node` | `main.js` |
| `kotlin` | Kotlin/JVM의 `kotlinc`, `java` | `Main.kt` |

컴파일러를 찾을 수 없으면 설치 후 터미널을 다시 열거나 `foo config --compiler <경로>`로 지정합니다. C/C++는 Clang을 기본으로 사용합니다. MSVC로 지정하면 `HeresyError: Thou shalt use Clang.`과 함께 Clang 설정 방법을 표시합니다. Java는 JDK 21 소스 파일 모드로 실행합니다. 네이티브 실행 파일과 Kotlin 빌드 결과는 임시 폴더에 생성합니다.

코드 탐색과 완성 기능이 필요하면 VS Code의 확장 탭에서 설치하거나 `code --install-extension <ID>`를 실행합니다. 자주 쓰는 확장 ID로는 C/C++의 `llvm-vs-code-extensions.vscode-clangd`, Python/PyPy의 `ms-python.python`, Rust의 `rust-lang.rust-analyzer`, Java의 `vscjava.vscode-java-pack`, Go의 `golang.go`가 있습니다.

## 파일과 설정

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

작업 폴더와 개별 문제 폴더에 `tasks.json`과 `launch.json`을 둡니다. 같은 이름의 소스 파일과 VS Code 설정이 이미 있으면 그대로 사용합니다. 직접 만든 `launch.json`에 `foo` 실행 구성을 추가하려면 다른 문제 폴더에서 생성된 `foo: Build and run` 구성을 복사합니다. 문제의 언어는 해당 폴더의 `.foo-language`에 기록합니다. `foo config --language`로 선택한 기본 언어는 새 문제에 적용됩니다.

사용자 설정 `config.toml`과 등록 문제집 및 풀이 위치를 담은 `state.toml`은 운영체제별 사용자 설정 폴더에 저장합니다.

| 운영체제 | 설정 폴더 |
| --- | --- |
| Windows | `%APPDATA%\foo` |
| Linux | `$XDG_CONFIG_HOME/foo`, 기본 `~/.config/foo` |
| macOS | `~/Library/Application Support/foo` |

`FOO_CONFIG_DIR` 환경 변수에 절대 경로를 지정하면 별도의 설정 폴더를 사용할 수 있습니다. TOML이 손상되면 `foo`가 오류와 파일 경로를 표시합니다. 기존 내용을 보존하려면 해당 파일을 백업한 뒤 수정합니다.

## 개발과 검증

- `src/cli.rs`와 `src/app.rs`는 명령 인수와 실행 흐름을 처리합니다.
- `src/store.rs`는 사용자 설정과 문제집의 진행 위치를 읽고 저장합니다.
- `src/workbook.rs`는 일반 문제집과 단계별 문제의 URL을 검증하고 로컬 HTML에서 제목과 문제 목록을 읽습니다.
- `src/language.rs`는 언어별 템플릿과 도구 탐색, 빌드와 실행을 처리합니다.
- `src/workspace.rs`는 문제 폴더와 VS Code 설정을 만들고 편집기와 브라우저를 엽니다.
- `tests/cli.rs`는 임시 폴더와 HTML fixture를 사용해 등록, 탐색, 소스 보존, 오류 처리와 실행을 검증합니다.

HTTP 관련 URL 검증은 `reqwest`의 URL 타입을 사용합니다. 문제집 HTML은 `scraper`로 파싱하며, 사이트에 자동으로 요청을 보내지 않습니다. 테스트도 사이트 연결 없이 실행할 수 있습니다.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

HTML 가져오기는 저장 파일에 포함된 문제 행을 읽습니다. 브라우저에서 저장한 파일에 문제 목록이 빠져 있거나 백준이 화면 구조를 변경하면 `--title`과 `--problems`로 등록할 수 있습니다. 로그인, 자동 제출, 정답 코드 수집, AI 풀이와 테스트케이스 생성은 구현 범위에서 제외했습니다.

## 라이선스

[GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-only)로 배포합니다.
