# Qt Functional Parity Audit

Last updated: 2026-08-26

This audit compares the installed GPUI application with the Qt migration build. It prioritizes retained workflow correctness over frame-rate tuning. The cutover product has two workspaces: Diff and AI. The editor, file explorer, Git workspace, and that workspace's GitHub/GitLab review UI are intentionally removed. Terminal, browser, updates, and AI-owned integrations remain in scope.

## Discovery environment

- GPUI baseline: `/Applications/Hunk.app`
- Qt build: the external-volume workspace's shared `target/` build, launched with the retained CEF feature
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

## Repair batch

1. Queue attached model mutations so QML cannot re-enter a mutably borrowed parent backend.
2. Restore persisted branch/worktree Diff comparisons through `hunk-git::compare`, including per-file patches and source selectors.
3. Make the libghostty vendored namespace transform idempotent and pin Hunk to the corrected fork revision.
4. Reduce the application shell to Diff and AI, move repository selection into AI, and remove Git/Forge desktop UI and bridge state.
5. Keep PR Linux checks on `ubuntu-self-hosted` with Qt provisioned by Nix.
6. Keep Cargo's shared workspace `target/` directory and reuse the external CEF and dependency caches.

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

The CEF-enabled Qt binary is built and running, but the final interaction pass is pending because the macOS session locked during validation. These rows stay unclaimed until both applications can be observed directly.

| Workflow | GPUI baseline | Qt result |
| --- | --- | --- |
| Launch and two-tab shell | Pending final observation | Pending final observation |
| Diff comparison and file navigation | Pending final observation | Pending final observation |
| Diff comments and refresh | Pending final observation | Pending final observation |
| AI navigation and thread catalog | Pending final observation | Pending final observation |
| Terminal open, input, close, and focus restoration | Pending final observation | Pending final observation |
| CEF browser open, navigation, frame, and close | Pending final observation | Pending final observation |
| Shutdown | Pending final observation | Pending final observation |

The PR should not claim native parity until this matrix is completed.
