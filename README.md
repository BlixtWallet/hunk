# hunk

A cross-platform Git diff viewer and Codex orchestrator with a Rust core and a Qt Quick/QML desktop UI.

## License

Hunk is licensed under the GNU General Public License v3.0 or later. See [`LICENSE`](./LICENSE).

## Why?

Nobody writes code anymore, people just review code. So we need the best diff viewer possible so that vibe engineers can review code and tell AI what to fix.
Hunk is also has full codex integration so you can use Codex inside of Hunk instead of codex-cli or any other desktop app.

<img width="3320" height="2032" alt="Hunk (Window) 2026-03-07 04:15 PM" src="https://github.com/user-attachments/assets/7d7284e1-3f2f-4bae-ae6b-bb51eea9e06b" />
<img width="3320" height="2032" alt="Hunk (Window) 2026-03-07 04:16 PM" src="https://github.com/user-attachments/assets/ff696ad9-a0ef-4023-ae1d-3d2c6b5036de" />
<img width="3320" height="2032" alt="Hunk (Window) 2026-03-07 04:36 PM" src="https://github.com/user-attachments/assets/2ae2fd5f-13b1-4270-b11f-d6d6c0dffe35" />
<img width="3320" height="2032" alt="Hunk (Window) 2026-03-07 04:53 PM" src="https://github.com/user-attachments/assets/7270c825-61cb-4c64-b63a-17d038808269" />

## What it includes

- Uses a native Git backend built on `gix` with narrow `git2` fallbacks for unsupported write flows
- Managed Git worktrees with per-worktree branch publishing
- Changed-file list for diff navigation
- Side-by-side diff viewer with per-line styling and line numbers
- Review compare mode for `base branch <-> workspace target` and custom branch/worktree pairs
- AI drafts and threads scoped to the selected project checkout or worktree
- Embedded terminal and Chromium browser surfaces
- Light/Dark mode toggle
- Refresh action
- `anyhow`-based error handling
- `tracing` + `tracing-subscriber` logging

## Workspace Layout

- `crates/hunk-terminal`: terminal integration, shell/session support, and terminal-facing workspace surfaces
- `crates/hunk-text`: rope-backed text model, positions/ranges, transactions, and undo/redo primitives
- `crates/hunk-language`: Tree-sitter language registry, syntax highlighting, preview highlighting, folds, and language-intelligence seams
- `crates/hunk-domain`: shared config/state types, SQLite-backed comment storage, markdown, and other app domain logic
- `crates/hunk-git`: Git backend for repo discovery, diffing, branches, commits, push, publish, and sync
- `crates/hunk-forge`: forge integration logic for GitHub and related remote/review workflows
- `crates/hunk-updater`: app update/download/install logic
- `crates/hunk-browser`: embedded browser runtime state, CEF backend integration, offscreen frames, input routing, snapshots, console logs, and safety checks
- `crates/hunk-browser-helper`: CEF subprocess helper binary used by the embedded browser runtime
- `crates/hunk-sleep-inhibitor`: cross-platform idle sleep prevention for long-running AI turns
- `crates/hunk-desktop`: Qt desktop app, QtBridge models, QML UI, and native Qt rendering adapters
- `crates/hunk-codex`: embedded Codex app-server integration, thread service, AI reducers, and protocol boundary

## Requirements

- Rust 1.98
- Qt 6.11.2
- macOS, Windows, or Linux development tools for the target platform

### Run Dev Locally

Preferred local entry points:

```bash
just start-mac
```

```bash
just start-linux
```

```powershell
just start-windows
```

Those helpers apply the macOS SDK wrapper when needed and stage the bundled Windows Codex runtime automatically where required.

If you want the raw macOS Cargo command, enter the pinned development shell:

```bash
nix develop -c cargo run -p hunk-desktop
```

Launch from anywhere, then use `File > Open Project...` (or `Cmd/Ctrl+Shift+O`) to choose a Git repository.

The app still launches from Terminal in local dev, so macOS may present it like a terminal-launched app.

## Worktrees

Hunk treats the primary checkout and each linked Git worktree as separate workspace targets.

- Create and switch worktrees from the Git tab.
- Managed worktrees live under `~/.hunkdiff/worktrees/<repo-key>/worktree-N`.
- The Diff and Git tabs follow the currently active workspace target.
- The Review tab defaults to comparing the active workspace target against the repo base branch, but you can also compare custom branch/worktree pairs.
- The AI tab can start a new thread in the primary checkout with `Cmd/Ctrl+N` or in a worktree-targeted draft with `Cmd/Ctrl+Shift+N`.

### Validate Workspace

On macOS, use the pinned Nix shell:

```bash
nix develop -c cargo build --workspace
nix develop -c cargo test --workspace
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
```

On Linux and Windows, run the same Cargo commands directly or use the `just` recipes so Cargo writes to its default `target/` directory.

Example:

```bash
cargo test --workspace
```

### For Production builds

```bash
cargo install cargo-packager
just bundle
open ./target/packager/Hunk.app
```

Cross-platform binary build helpers:

```bash
./scripts/build_linux.sh
./scripts/build_windows.sh
```

Optional flags:

- `--target <triple>` to override the default target triple (must match the script platform).
- `--debug` to build debug artifacts.
- `--no-stage-runtime` to skip copying the bundled Codex runtime into the target output tree.

Release packaging helpers:

```bash
./scripts/package_macos_release.sh
./scripts/package_linux_release_zed_like.sh
pwsh ./scripts/package_windows_release.ps1
```

These produce:

- macOS ARM64: `Hunk-<version>-macos-arm64.dmg`, signed/notarized when Apple secrets are configured
- Linux x86*64: `Hunk-<version>-linux-x86_64.tar.gz`, `hunk-desktop*<version>-1_amd64.deb`, and `hunk-desktop-<rpm-version>-1.x86_64.rpm`
- Windows x86_64: `Hunk-<version>-windows-x86_64.msi`

Linux release packaging is custom and does not use `cargo packager`. On Ubuntu hosts you can either enter `nix develop` to get the packaging toolchain from the flake, or install host deps with `just install-linux-packaging-deps-ubuntu`, then use:

```bash
just package-linux-release
just package-linux-deb-release
just package-linux-rpm-release
just smoke-test-linux-deb
just smoke-test-linux-rpm
```

The Debian smoke test installs the package in an Ubuntu container. The RPM smoke test installs it in a Fedora container.

## Prepare Bundled Codex Runtime

Hunk embeds a native `codex` runtime and launches app-server mode with:
`codex app-server --listen ws://127.0.0.1:<port>`.

Expected embedded runtime paths:

- `assets/codex-runtime/macos/codex`
- `assets/codex-runtime/linux/codex`
- `assets/codex-runtime/windows/codex.cmd`
- `assets/codex-runtime/windows/codex.exe`

Runtime layout details also live in [`assets/codex-runtime/README.md`](./assets/codex-runtime/README.md).

### Download the pinned Codex release binary

The pinned Codex baseline is tracked in [`docs/AI_CODEX_SPEC.md`](./docs/AI_CODEX_SPEC.md).

```bash
./scripts/download_codex_runtime_unix.sh macos
```

Platform-specific download helpers:

- macOS ARM64: `./scripts/download_codex_runtime_unix.sh macos`
- Linux x86_64: `./scripts/download_codex_runtime_unix.sh linux`
- Windows x86_64: `pwsh ./scripts/download_codex_runtime_windows.ps1`

The Windows helper stages both `assets/codex-runtime/windows/codex.exe` and the bundled launcher `assets/codex-runtime/windows/codex.cmd`.

These pull the pinned release assets directly from the official Codex GitHub release for the version locked in `Cargo.lock`:

- macOS ARM64: `codex-aarch64-apple-darwin.tar.gz`
- Linux x86_64: `codex-x86_64-unknown-linux-musl.tar.gz`
- Windows x86_64: `codex-x86_64-pc-windows-msvc.exe.zip`

If you already have a local native binary you want to use for macOS instead, you can still override the source path:
`./scripts/install_codex_runtime_macos.sh /absolute/path/to/codex`

### Validate + stage + bundle (macOS local workflow)

Download and stage the pinned macOS runtime:

```bash
./scripts/install_codex_runtime_macos.sh
./scripts/validate_codex_runtime_bundle.sh --strict --platform macos
./scripts/stage_codex_runtime_macos.sh
cargo test -p hunk-codex --test real_runtime_smoke -- --ignored
just bundle
```

## GitHub Actions Release Flow

- `.github/workflows/pr-build.yml` stays as the main PR CI workflow.
- `.github/workflows/release.yml` builds DMG/MSI/tarball/DEB/RPM artifacts and publishes them to a GitHub Release when you push a `v*` tag.

## Syntax Highlighting

Diff previews and markdown fenced-code highlighting use the shared Tree-sitter language registry in `crates/hunk-language`.

Built-in languages/config formats:

- Rust
- JavaScript
- TypeScript
- TSX
- Bash / shell
- Python
- PowerShell
- JSON
- YAML
- Go
- HTML
- CSS
- TOML
- Java
- Kotlin
- C
- C++
- C#
- SQL
- Markdown
- Dockerfile
- Nix
- Terraform / HCL
- Swift

Apple signing/notarization secrets used by the workflows:

- `APPLE_CERTIFICATE_P12_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_NOTARY_API_KEY_BASE64`
- `APPLE_NOTARY_KEY_ID`
- `APPLE_NOTARY_ISSUER`

Windows signing is not configured in this repo yet. The current release workflow produces an unsigned MSI until you add a paid Windows signing solution.

## Large Diff Stress Fixture

Generate a synthetic Git repository with a very large working-copy diff:

```bash
./scripts/create_large_diff_repo.sh --lines 25000 --files 1 --force
```

The script prints the generated repo path and total diff size. Open that folder in Hunk to stress diff projection and scrolling/render performance.

Generate code-like diffs instead of plain text (`txt`, `js`, or `ts`):

```bash
./scripts/create_large_diff_repo.sh --lines 25000 --files 20 --lang ts --force
./scripts/create_large_diff_repo.sh --lines 25000 --files 20 --lang js --force
```

To spread the same total load across multiple files:

```bash
./scripts/create_large_diff_repo.sh --lines 6000 --files 4 --force
```

## Config

Hunk reads config from `~/.hunkdiff/config.toml`.
Keyboard shortcuts are configured in the `[keyboard_shortcuts]` table:

```toml
[keyboard_shortcuts]
open_project = ["cmd-shift-o", "ctrl-shift-o"]
open_settings = ["cmd-,", "ctrl-,"]
quit_app = ["cmd-q"]
```

Use an empty list to disable a shortcut action:

```toml
[keyboard_shortcuts]
quit_app = []
```

## Icons

Generate git-diff icon variants and rebuild the bundle:

```bash
./scripts/generate_diff_icons.py
./scripts/build_macos_icon.sh
./scripts/build_windows_icon.sh
just bundle
```

Generated assets:

- `assets/icons/hunk-icon-default.png` (default/full color)
- `assets/icons/hunk-icon-dark.png` (dark appearance variant)
- `assets/icons/hunk-icon-mono.png` (monochrome/tint-friendly variant)

Current packaging uses `Hunk.icns`, `Hunk.ico`, and `hunk_new.png` through `cargo-packager`.

## Hot Reload (Bacon)

Install bacon once:

```bash
cargo install bacon
```

Start hot reload (default job is `run`):

```bash
bacon
```

Useful jobs:

```bash
bacon check
bacon test
bacon clippy
```

Keybindings in bacon UI:

- `r` -> run
- `c` -> check
- `t` -> test
- `l` -> clippy

## Credits

- [Zed](https://zed.dev/) was a major inspiration for Hunk, especially its review, text, and terminal architecture.
- [Qt](https://www.qt.io/) provides Hunk's cross-platform native desktop UI and scene graph.
