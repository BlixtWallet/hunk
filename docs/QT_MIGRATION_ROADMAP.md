# Hunk Qt Migration Roadmap

## Status

- Active
- Started: 2026-08-24
- Target platforms: macOS arm64, Windows x86_64, Linux x86_64
- UI target: Qt Quick/QML on Qt 6.11.2
- Rust bridge target: official `qt/qtbridge-rust`, pinned to an audited revision
- Performance gate: no frame may exceed the existing 8 ms budget in the measured 120 Hz review workflow

## Goal

Replace the GPUI frontend with a Qt Quick frontend while retaining Hunk's Rust
domain logic. The resulting desktop application is intentionally narrower:

1. Diff and review workspace
2. Git operations
3. AI/Codex threads and tools
4. Supporting terminal/browser surfaces required by those workflows
5. Settings, updates, authentication, and desktop integration

The editable Files workspace, File Explorer, file creation, file modification,
quick-open, and standalone file renderer are removed. Headless text, language,
and editor primitives are retained only when a current Diff, AI, or terminal
workflow has a proven dependency on them.

This removal applies to the user-facing editable workspace. Codex file changes,
apply-patch behavior, approvals, and reviewing resulting diffs remain required.

## Non-negotiable Working Loop

Every implementation item below must use this loop before the next item starts:

- [ ] Restate the slice's assumptions, scope, and verification gates in the live plan.
- [ ] Do not write implementation code before the slice is planned.
- [ ] Preserve unrelated and user-owned worktree changes.
- [ ] Implement the smallest complete version of the planned slice.
- [ ] Run build, test, and validation commands through `nix develop -c ...` on macOS.
- [ ] Use Cargo's default `target/` and only the repository-approved Cargo home.
- [ ] Perform a thorough review for correctness, regressions, dead code, accidental compatibility layers, files over 1000 lines, and unnecessary abstractions.
- [ ] Fix review findings before starting the next item.
- [ ] Append a dated entry to the repository-root `lessons_learned.md` when the slice reveals a constraint, bug, failure mode, or reusable technique.
- [ ] Make a small coherent commit after the slice is reviewed.
- [ ] Inspect the branch chain with `gh stack view`.
- [ ] Open or update a stacked PR with `gh stack submit` when the layer is independently reviewable.

Workspace clippy, tests, and builds are run once at the end of a completed
implementation layer, in accordance with `AGENTS.md`. Focused diagnostics may
be run while investigating, but repeated full-workspace validation is avoided.

## Version Decisions

### Qt

Qt 6.11.2 is the latest stable Qt release as of 2026-08-24 and is the required
baseline. The current flake lock resolves Qt 6.10.2, so Qt work cannot be called
complete until Nix supplies 6.11.2 on supported development and CI systems.

References:

- <https://www.qt.io/blog/qt-6.11.2-released>
- <https://doc.qt.io/qt-6/qt-releases.html>
- <https://doc.qt.io/qt-6/supported-platforms.html>

### QtBridge

Use the official bridge for QML-facing Rust objects, properties, signals,
slots, list/table models, JSON values, and cross-thread UI invocation. Keep all
QtBridge objects in a thin adapter layer; do not put Hunk's core state directly
inside `Rc<RefCell<_>>` QObjects.

Pin an exact QtBridge revision. Do not use the README's wildcard dependency.
QtBridge is currently beta, requires Qt 6.10 or newer and private Qt headers,
and marks macOS arm64 experimental. The adapter boundary must make updating or
forking the bridge possible without contaminating domain crates.

Custom Qt scene-graph or C++-only APIs may use the narrowest possible CXX-Qt or
C++ shim only after QtBridge is proven insufficient. Ordinary application
state and controls must use QtBridge.

Reference: <https://github.com/qt/qtbridge-rust>

## Intended Architecture

```text
Qt Quick / QML
  shell, navigation, Git, AI, settings, ordinary diff chrome
                         |
                         v
hunk-qt adapter layer
  QtBridge QObjects and models; UI commands; immutable snapshots
                         |
                         v
Headless Rust services
  hunk-git, hunk-forge, hunk-codex, hunk-domain, hunk-updater,
  hunk-terminal, hunk-sleep-inhibitor, and proven diff primitives
                         |
                         v
Optional narrow custom renderer seam
  QQuickItem/scene graph only where measurement proves QML delegates miss 8 ms
```

Qt/QML must not directly own repository, Codex, or terminal domain state.
Commands flow into Rust; immutable snapshots and events flow back to Qt. UI
callbacks must remain non-blocking, and expensive work stays off the Qt thread.

## Stacked Delivery Order

Intended branch chain:

```text
master
  <- migration/00-roadmap
  <- migration/01-codex-upgrade
  <- migration/02-remove-files-workspace
  <- migration/03-headless-app-core
  <- migration/04-qt-foundation
  <- migration/05-qt-git
  <- migration/06-qt-diff
  <- migration/07-qt-ai
  <- migration/08-qt-cutover-ci
  <- migration/09-release-hardening
```

Branch names may be divided into smaller layers when a listed layer would not
be independently reviewable. Their bottom-to-top dependency order must remain
equivalent to this roadmap.

## Migration Checklist

### 0. Roadmap and Baseline

- [x] Read and apply `AGENTS.md`.
- [x] Establish the mandatory working loop above.
- [x] Establish the stack order.
- [x] Create the requested `lessons_learned.md` log.
- [x] Confirm latest stable Qt is 6.11.2.
- [x] Confirm the current Nix lock provides Qt 6.10.2 and therefore needs an update.
- [x] Record current CI structure and recent wall-clock ranges.
- [x] Review this layer and fix its scope/path findings.
- [x] Commit this layer.
- [x] Open the bottom stacked PR: <https://github.com/smolcars/hunk/pull/175>.

Current PR CI baseline from the five most recent successful runs inspected on
2026-08-24:

- macOS: approximately 1-2 minutes
- Linux: approximately 3-10 minutes
- Windows: approximately 10-19 minutes and usually the critical path

The current workflow performs workspace clippy/tests and then builds the
desktop more than once, including a second CEF-enabled build. Qt migration
must not reproduce that redundant matrix.

### 1. Upgrade Embedded Codex

- [x] Resolve the latest supported upstream `rust-v...` tag from authoritative OpenAI sources and the upstream repository.
- [x] Audit the current Hunk fork delta and determine which patches remain necessary upstream.
- [x] Rebase or recreate `hunk/embedded-apply-patch-fix` on the selected upstream tag.
- [x] Reapply and review required Hunk patches.
- [x] Push the fork branch only after its exact target and patch set are verified.
- [x] Refresh all root workspace Codex dependencies in `Cargo.lock`.
- [x] Refresh macOS, Linux, and Windows bundled runtime assets.
- [x] Update `docs/AI_CODEX_SPEC.md` and stale version references.
- [x] Fix protocol/API drift in headless crates before Qt consumes the API.
- [x] Verify required thread, turn, approval, user-input, and apply-patch flows without blocking on keychain prompts.
- [x] Complete the mandatory working loop and stacked PR: <https://github.com/smolcars/hunk/pull/176>.

### 2. Remove the Editable Files Product

- [x] Remove Files navigation and workspace modes.
- [x] Remove File Explorer/tree UI and filesystem mutation commands used only by it.
- [x] Remove quick-open and standalone file editor/file renderer UI.
- [x] Remove user-facing file creation, direct editing, save, undo/redo, editor search, and editor-only keybindings without removing Codex file changes or apply-patch review.
- [x] Remove editor-only tests and assets while preserving tests for retained headless primitives.
- [x] Trace every `hunk-editor`, `hunk-text`, and `hunk-language` consumer.
- [x] Keep only APIs required by Diff, AI, terminal, markdown, or review workflows.
- [x] Confirm Diff, Git, and AI remain reachable in the temporary GPUI shell.
- [x] Complete the mandatory working loop and stacked PR: <https://github.com/smolcars/hunk/pull/178>.

### 3. Establish a Headless Application Boundary

- [x] Inventory GPUI `Entity`, `Context`, `App`, `Window`, focus, subscription, and task ownership in retained workflows.
- [x] Define UI-independent command and snapshot types for Diff, Git, and AI.
- [x] Move retained behavior out of `hunk-desktop` controllers where it is currently coupled to GPUI.
- [x] Keep production Git operations in `hunk-git` with `gix` first and narrow `git2` fallbacks.
- [x] Keep Codex protocol/reducer/lifecycle behavior in `hunk-codex`.
- [x] Keep Qt types out of all domain crates.
- [x] Add crate-level tests for the extracted behavior.
- [ ] Complete the mandatory working loop and stacked PR.

Phase 3 ownership inventory:

| Workflow | Headless Rust ownership | Temporary GPUI ownership | Qt adapter implication |
| --- | --- | --- | --- |
| Git | `hunk-app::git` owns refresh policy, repository loading, workflow fingerprints, line statistics, and owned snapshots; production operations remain in `hunk-git`. | The root `Entity` stores the last snapshot. `Context` schedules work, merges refresh requests, rejects stale epochs, and notifies paint. | Invoke the same synchronous service off the Qt thread and apply one snapshot on the UI thread. |
| Diff | `hunk-app::diff` owns compare/historical commands, patch parsing, stable row projection, syntax/intra-line segments, and binary/error/collapsed states. | The root `Entity` owns active selection, viewport, search, comments, expansion, focus, and renderer caches. `Window` and `Context` own input and repaint. | Bridge immutable batched projections; keep selection, viewport, and scene-graph state in Qt. |
| AI | `hunk-app::ai` owns the single worker thread, command/event channels, bootstrap/reconnect policy, workspace paths, rollout fallback, and dynamic-tool execution. `hunk-codex` still owns app-server protocol, reducer state, thread lifecycle semantics, and the embedded client. | The root `Entity` polls and applies worker events. `Window`, focus handles, dialogs, notifications, browser frames, and confirmation presentation remain frontend concerns. | Keep one Rust worker. The Qt layer sends commands, batches event application, and presents renderer/platform interactions. |
| Terminal/browser support | Existing headless runtime crates retain session/browser state. | GPUI still owns terminal elements, browser frame presentation, focus routing, subscriptions, and input translation. | These adapters are explicitly deferred to the AI Qt phase; no domain state should move into QObjects. |

Across retained workflows, `Entity`, `Context`, `App`, `Window`, focus handles,
subscriptions, and GPUI tasks now represent presentation, scheduling, or event
application rather than authoritative Git, Diff, or Codex domain behavior.

Phase 3 macOS validation through Nix:

- `cargo build --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

All three gates passed using Cargo's default external-volume `target/` and the
shared Cargo cache under `/Volumes/hulk/dev/cache`.

### 4. Qt Foundation

- [ ] Update the flake lock and flake packages so macOS and Linux expose Qt 6.11.2, Qt Declarative/Quick, private headers required by QtBridge, and required build tools.
- [ ] Define the Windows CI installation of the exact same Qt version and MSVC 2022 ABI.
- [ ] Pin an audited official QtBridge revision compatible with Qt 6.11.2.
- [ ] Add the Qt desktop binary/adapter crate without duplicating domain logic.
- [ ] Add a QML module/resource layout and fast development loading path.
- [ ] Centralize all colors and metrics in the Qt theme module.
- [ ] Recreate the restrained Hunk shell with Diff, Git, and AI navigation only.
- [ ] Add lifecycle, logging, panic/error presentation, and asynchronous Rust-to-Qt invocation.
- [ ] Build and run through Nix on macOS; visually compare the open GPUI app where useful.
- [ ] Establish basic QML smoke tests.
- [ ] Complete the mandatory working loop and stacked PR.

Visual thesis: a dense, calm code-review workspace with a near-black neutral
surface, one restrained blue accent, sharp typography, minimal chrome, and no
dashboard-card treatment.

Interaction thesis: tab/workspace transitions preserve spatial context;
streaming AI content settles without reflowing unrelated rows; diff hover,
selection, and expansion feedback is immediate and never ornamental.

### 5. Git Tab

- [ ] Expose repository status, change groups, branches, commits, and review metadata through QtBridge models.
- [ ] Implement refresh and cancellation without blocking the Qt thread.
- [ ] Implement stage/unstage/discard/commit and required confirmations.
- [ ] Implement branch selection/activation and recent commits.
- [ ] Implement forge authentication/review actions while skipping unattended keychain-blocked validation paths.
- [ ] Add Rust service tests and QML interaction tests.
- [ ] Visually inspect empty, loading, error, and populated states.
- [ ] Complete the mandatory working loop and stacked PR.

### 6. Diff Tab

- [ ] Define immutable, batched visible-row snapshots with stable identifiers.
- [ ] Implement virtualized unified and side-by-side diff presentation.
- [ ] Restore syntax highlighting, selection, search, comments, folding, and review navigation required by the narrowed product.
- [ ] Avoid one QObject per token or other high-churn bridge designs.
- [ ] Instrument frame time, model update time, object count, and allocation hot paths.
- [ ] Verify ordinary QML delegates against representative large repositories.
- [ ] Add the narrow custom `QQuickItem`/scene-graph renderer only if measurements require it.
- [ ] Meet the 8 ms frame budget at 120 Hz for scroll, resize, selection, and streamed updates.
- [ ] Complete the mandatory working loop and stacked PR.

### 7. AI Tab

- [ ] Expose thread catalog, active thread, turn timeline, composer, and runtime state through QtBridge.
- [ ] Implement thread load/start/resume/fork/archive and cwd scoping.
- [ ] Implement streaming messages and tool output without per-token QObject churn.
- [ ] Implement approvals, request-user-input, queued messages, steering, interruption, and plan state.
- [ ] Implement attachments, bookmarks, context usage, model/settings, and service-tier controls still in product scope.
- [ ] Port required terminal surfaces with correct input, focus, cursor, selection, and resize behavior.
- [ ] Decide retained embedded-browser requirements from product use; remove CEF if unneeded rather than automatically replacing it with Qt WebEngine.
- [ ] Validate key flows without triggering unattended keychain prompts.
- [ ] Complete the mandatory working loop and stacked PR.

### 8. Atomic Qt Cutover and CI Replacement

- [ ] Make the Qt binary the workspace default desktop application.
- [ ] Remove GPUI, `gpui_platform`, GPUI Component, GPUI assets, shaders, and GPUI-only build inputs.
- [ ] Remove all remaining GPUI source and compatibility adapters.
- [ ] Remove orphaned editor/file-workspace crates or features proven unused after Diff and AI are complete.
- [ ] Replace GPUI-oriented packaging scripts and resources with Qt deployment tooling.
- [ ] Change PR CI so no job resolves or builds a GPUI package.
- [ ] Cache or preinstall exact-version Qt binaries; never compile Qt from source in ordinary CI.
- [ ] Run core fmt/clippy/tests independently from the final Qt desktop build.
- [ ] Build one production feature configuration per platform instead of plain and CEF duplicates.
- [ ] Add path-aware QML validation so QML-only changes do not trigger needless Rust rebuild work.
- [ ] Build the Qt app on Linux, Windows, and macOS in PR CI.
- [ ] Complete the mandatory working loop and stacked PR.

The GPUI build is removed in the same reviewed layer that makes Qt the default.
There must not be a merged state where the shipping frontend has no CI build.

### 9. Release Hardening and Completion Audit

- [ ] Package Qt libraries, platform plugins, image plugins, QML modules, and accessibility plugins required by the application.
- [ ] Produce and install-test macOS DMG/app, Windows MSI, and Linux tarball/DEB/RPM artifacts.
- [ ] Verify updater manifests and OTA behavior for the renamed/repackaged binary.
- [ ] Verify DPI scaling, fonts, IME, clipboard, drag/drop, shortcuts, notifications, dialogs, accessibility, and sleep inhibition on all platforms.
- [ ] Verify terminal and any retained browser surface on all platforms.
- [ ] Run automated QML tests and representative UI smoke tests.
- [ ] Run the full workspace build, tests, and clippy once at the end through Nix on macOS.
- [ ] Search the repository and lockfile for all remaining GPUI references and prove none are production dependencies.
- [ ] Repeat the 120 Hz performance audit on supported hardware.
- [ ] Perform a final deep review and fix all findings.
- [ ] Audit every checklist item against authoritative evidence.
- [ ] Submit the final stack and confirm every PR base/head relationship.

## Definition of Done

The migration is complete only when:

1. The default Hunk desktop application uses Qt Quick/QML and official QtBridge.
2. No production or CI target resolves or builds GPUI.
3. The editable Files/File Explorer/File Renderer product and its orphaned code are gone.
4. Diff, Git, and AI/Codex workflows meet their functional gates.
5. Qt 6.11.2 builds and packages on macOS, Windows, and Linux.
6. The measured review workflow remains within the 8 ms frame budget.
7. Codex crates, fork baseline, docs, and bundled runtimes agree on the upgraded version.
8. Full build, clippy, tests, packaging, UI smoke tests, and the completion audit pass.
9. All migration layers have completed their code review and lessons/commit/stack-PR loop.
