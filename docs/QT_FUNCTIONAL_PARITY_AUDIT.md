# Qt Functional Parity Audit

Last updated: 2026-08-26

This audit compares the installed GPUI application with the Qt migration build. It prioritizes retained workflow correctness over frame-rate tuning. The cutover product has two workspaces: Diff and AI. The editor, file explorer, Git workspace, and that workspace's GitHub/GitLab review UI are intentionally removed. Terminal, browser, updates, and AI-owned integrations remain in scope.

## Discovery environment

- GPUI baseline: `/Applications/Hunk.app`
- Qt build: `target/functional/HunkQt.app` on the external-volume workspace, with an independent functional-test bundle identifier and the retained CEF feature
- Native test repositories: `~/Documents/glab`, `~/Documents/lightning-service-rust`, and `~/Documents/zbd-bdk-wallet`
- QtBridge revision: `cad0d6cd81d1af294ec87c67f21d39133196dbc1`
- Qt: 6.11.2
- Rust: 1.98.0
- CEF: `cef-v151.8.0+151.3.24`, backed by Chromium 151.0.7922.174
- CEF runtime: reused from `assets/browser-runtime/cef/macos/runtime`

## Product scope after this batch

| Surface | Cutover decision | Implementation state |
| --- | --- | --- |
| Diff | Retain | Includes branch/worktree comparison selection, file navigation, split/unified presentation, search, comments, and refresh |
| AI | Retain | Includes repository selection, threads, timeline, composer, attachments, requests, session controls, terminal, and embedded browser |
| Git workspace | Remove | Navigation, QML, QtBridge command surface, and desktop `hunk-forge` dependency removed |
| GitHub/GitLab review UI | Remove with Git workspace | Forge panels, dialogs, authentication, and review actions removed from the desktop frontend |
| AI integrations | Retain | Codex integrations and browser/terminal tools remain unchanged in ownership |
| Editor/file explorer | Remove | Already outside the Qt cutover product |

Production Git reads needed by Diff remain in `hunk-git`; removing the Git product does not duplicate or shell out for repository behavior.

## Initial functional discovery

The first native comparison found a common crash family rather than four unrelated product bugs. Opening AI, opening the terminal, refreshing Git, or opening Diff comment history could abort at the QtBridge boundary with:

```text
Failed to borrow for read_property: BorrowError(BorrowError)
panic in ffi function qtbridge_interfaces::qobject::proxy_rust_bridge::ffi::QObjectProxyRust::read_property
```

An attached child `QListModel` was reset while the parent `Backend` still held a mutable `RefCell` borrow. The model reset notified QML synchronously, QML read a parent property, and the nested read aborted. The repair queues and coalesces attached-model updates onto each model's own Qt invocation boundary. The change covers AI threads, timeline, sessions, attachments, Diff files/rows/comments, browser tabs, and terminal tabs/rows.

The same discovery pass also found that the Qt Diff product had lost the GPUI branch/worktree comparison controls and that the reusable macOS terminal build failed when the dependency's simdutf namespace patch was applied twice.

The final native pass found two additional AI-boundary defects. `BrowserBridge::set_active_thread` emitted a child-object signal synchronously while QtBridge still held its mutable Rust borrow, so the AI page could abort before CEF initialized. The bridge now queues its property-notification signals by their registered snake-case Qt meta-method names. Qt also treated Codex's `requires_openai_auth` provider capability as a logged-out state even when `account/read` returned a populated ChatGPT account. The desktop projection now requires sign-in only when the provider needs OpenAI authentication and the account is absent.

## Repair batch

1. Queue attached model mutations so QML cannot re-enter a mutably borrowed parent backend.
2. Restore persisted branch/worktree Diff comparisons through `hunk-git::compare`, including per-file patches and source selectors.
3. Make the libghostty vendored namespace transform idempotent and pin Hunk to the corrected fork revision.
4. Reduce the application shell to Diff and AI, move repository selection into AI, and remove Git/Forge desktop UI and bridge state.
5. Keep PR Linux checks on `ubuntu-self-hosted` with Qt provisioned by Nix.
6. Keep Cargo's shared workspace `target/` directory and reuse the external CEF and dependency caches.
7. Queue `BrowserBridge` notifications outside QtBridge's mutable borrow and distinguish provider authentication requirements from a missing login.
8. Build a reusable, independently addressable macOS debug app under `target/functional`, reusing the staged CEF runtime and normal Cargo cache.

## Automated verification

| Check | Result |
| --- | --- |
| Qt Quick lint and complete QML test suite | Pass |
| Focus-sensitive Diff, AI, terminal, and repository-picker QML cases | Pass |
| `cargo clippy -p hunk-desktop --all-targets --locked -- -D warnings` | Pass |
| Desktop crate integration tests | Pass |
| Non-desktop workspace clippy and tests | Pass |
| `cargo build --workspace --locked` | Pass |
| CEF-enabled `hunk-desktop` debug build | Pass |
| CEF subprocess helper build | Pass |
| Exact cef-rs/Chromium lock and staged runtime validation | Pass |
| Self-hosted/Nix PR workflow contract tests | Pass |

All macOS Rust and Qt commands ran through `nix develop`. No alternate Cargo target directory or additional source checkout was created.

## Final native interaction matrix

Both applications were exercised directly on macOS. Installed GPUI Hunk supplied the behavioral baseline; the independently addressable Qt debug app allowed automation to target the migration build without confusing it with `/Applications/Hunk.app`.

| Workflow | GPUI baseline | Qt result |
| --- | --- | --- |
| Launch and two-tab shell | Launches and restores a repository; includes the intentionally removed legacy products | Pass: launches with exactly Diff and AI and restores a `~/Documents` repository |
| Diff comparison and file navigation | Pass: source selection, files, split diff, and search render | Pass: `main` to Primary Checkout, three files, file switching, 4-result search, and split/unified rendering |
| Diff comments and refresh | Pass | Pass: row selection enables Comment; composer opens and cancels without persisting test data |
| AI navigation and thread catalog | Pass: catalog, transcript, composer, and controls render | Pass: four real threads and a 94-turn transcript load; logged-in controls, context, draft enablement, and attachment action work without the startup abort |
| Terminal open, input, close, and focus restoration | Pass: safe command input and output | Pass: `printf` and `pwd` execute in `~/Documents/zbd-bdk-wallet`; drawer closes and the app remains responsive |
| CEF browser open, navigation, frame, and close | Installed baseline exited when its browser control was opened | Pass: cef-rs `cef-v151.8.0+151.3.24` / Chromium 151.0.7922.174 renders `https://example.com`; GPU, network, storage, and renderer helpers remain live |
| Shutdown | Pass outside the baseline browser failure | Pass: repeated quit/relaunch cycles are clean and no new native crash report is produced |

The native macOS parity matrix is complete. Windows and Linux packaging remain release-hardening work rather than blockers for the Qt product cutover.
