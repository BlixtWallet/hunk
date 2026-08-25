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
- [ ] Perform a thorough review for correctness, regressions, dead code, accidental compatibility layers, files over 2000 lines, and unnecessary abstractions.
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
baseline. The current flake lock resolves Qt 6.10.2, while the first Nixpkgs
revision containing 6.11.2 has no usable binary cache for this environment.
Hunk therefore consumes the official prebuilt 6.11.2 SDK from a persistent
external cache while Cargo and the rest of the toolchain continue to run through
the Nix shell. Local builds and CI both reject any other Qt version.

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
  <- migration/06-qt-forge
  <- migration/07-qt-diff
  <- migration/08-qt-diff-review-tools
  <- migration/09-qt-diff-selection
  <- migration/10-qt-diff-comment-core
  <- migration/11-qt-diff-comments
  <- migration/12-qt-diff-comments-ui
  <- migration/13-qt-ai
  <- migration/14-qt-cutover-ci
  <- migration/15-release-hardening
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
- [x] Confirm the current Nix lock provides Qt 6.10.2 and document the exact prebuilt-SDK path required to avoid an uncached Qt source build.
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
- [x] Complete the mandatory working loop and stacked PR: <https://github.com/smolcars/hunk/pull/179>.

Phase 3 ownership inventory:

| Workflow | Headless Rust ownership | Temporary GPUI ownership | Qt adapter implication |
| --- | --- | --- | --- |
| Git | `hunk-app::git` retains the legacy frontend's refresh policy and fingerprint coordination. `hunk-git::workspace` owns the toolkit-neutral repository snapshot and production stage, commit, restore, branch, and network commands consumed by Qt. | The root `Entity` stores the last snapshot. `Context` schedules work, merges refresh requests, rejects stale epochs, and notifies paint. | Invoke `hunk-git::workspace` off the Qt thread, reject stale epochs, and apply one batched snapshot on the UI thread. |
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

- [x] Expose the official prebuilt Qt 6.11.2 SDK, Qt Declarative/Quick, private headers required by QtBridge, and build tools inside the macOS/Linux Nix shell without realizing an uncached Qt source build.
- [x] Define the Windows CI installation of the exact same Qt version and MSVC 2022 ABI.
- [x] Pin an audited official QtBridge revision compatible with Qt 6.11.2.
- [x] Add the Qt desktop binary/adapter crate without duplicating domain logic.
- [x] Add a QML module/resource layout and fast development loading path.
- [x] Centralize all colors and metrics in the Qt theme module.
- [x] Recreate the restrained Hunk shell with Diff, Git, and AI navigation only.
- [x] Add lifecycle, logging, panic/error presentation, and asynchronous Rust-to-Qt invocation.
- [x] Build and run through Nix on macOS; visually inspect the rendered Qt shell and compare the retained visual language where useful.
- [x] Establish basic QML smoke tests.
- [x] Complete the mandatory working loop and stacked PR: <https://github.com/smolcars/hunk/pull/180>.

Phase 4 toolchain decisions:

- Qt is installed once with `aqtinstall` 3.3.0 under the configurable
  `HUNK_QT_CACHE_ROOT`; this machine uses `/Volumes/hulk/dev/cache/qt` and the
  repository continues to use its existing external-volume Cargo `target/`.
- `hunk-qt` rejects builds unless `qmake -query QT_VERSION` returns exactly
  6.11.2. QtBridge is pinned to official commit
  `cad0d6cd81d1af294ec87c67f21d39133196dbc1`.
- Linux CI installs and caches `linux_gcc_64` with
  `jurplel/install-qt-action@v4` on an ephemeral Ubuntu 24.04 runner with pinned
  Nix and Cargo caches. Windows uses the same Qt action with aqt pinned to
  upstream merge commit `8c3695d4a4e1ceabf6a74dc6c79681656dc6b74b`,
  which adds the Qt 6.11 Windows repository layout missing from aqt 3.3.0. Both
  jobs cache the exact official prebuilt Qt packages without Qt Account
  credentials.
- The current self-hosted macOS runner cannot mount `/Volumes/hulk`. The Qt
  migration PR workflow therefore omits its macOS job instead of downloading
  an SDK and rebuilding onto constrained internal storage. macOS is validated
  locally through Nix against the exact cached external SDK; restore the CI job
  only after its runner can use the external volume.
- Qt migration branches no longer compile GPUI in CI. PR #179 is the validated
  legacy-frontend baseline; subsequent layers test the headless workspace with
  `hunk-desktop` excluded, build `hunk-qt` on Linux and Windows in CI, and run
  the same Qt build and smoke gates locally on macOS.
- The debug executable loads QML directly for fast iteration, the release build
  compiles the same module into `qrc:/qml`, and the offscreen smoke suite captures
  the rendered desktop-size shell under Cargo's ignored `target/` for visual
  inspection.

Phase 4 macOS validation through Nix:

- `qmake -query QT_VERSION` reported 6.11.2 from the external cached SDK.
- `cargo build --workspace --all-targets` passed; the first post-lockfile build
  took 5 minutes 14 seconds and populated the existing external Cargo target.
- `cargo test --workspace` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- The release-only `hunk-qt` resource build passed, the live Qt executable loaded
  without QML errors, and all five offscreen QML smoke assertions passed.

Visual thesis: a dense, calm code-review workspace with a near-black neutral
surface, one restrained blue accent, sharp typography, minimal chrome, and no
dashboard-card treatment.

Interaction thesis: tab/workspace transitions preserve spatial context;
streaming AI content settles without reflowing unrelated rows; diff hover,
selection, and expansion feedback is immediate and never ornamental.

### 5. Git Tab

- [x] Expose repository status, change groups, branches, and commits through QtBridge models.
- [x] Implement refresh, stale-result rejection, and repository switching without blocking the Qt thread.
- [x] Implement stage/unstage/discard/commit and required confirmations.
- [x] Implement branch selection/activation and recent commits.
- [x] Implement forge authentication/review actions while skipping unattended keychain-blocked validation paths.
- [x] Add Rust service tests and QML interaction tests.
- [x] Visually inspect empty, loading, error, and populated states.
- [x] Complete the mandatory working loop and stacked PRs
  ([#181](https://github.com/smolcars/hunk/pull/181) and
  [#182](https://github.com/smolcars/hunk/pull/182)).

Phase 5 implementation decisions:

- The production Git snapshot and commands live in `hunk-git::workspace`; the
  Qt adapter owns only background scheduling, stale epochs, model replacement,
  presentation state, and confirmation UI.
- Background refreshes place owned snapshots in an epoch-keyed Rust mailbox;
  the queued Qt callback transfers only the epoch. This avoids serializing and
  parsing a large repository snapshot on the frame-sensitive Qt thread.
- Repository, branch, commit, and file rows use three reset-in-batch QtBridge
  list models. QML uses recycling `ListView` delegates with bounded cache
  buffers; it does not create one QObject per repository item.
- Hunk restores the active project from the existing application state and
  exposes a native Qt folder chooser for changing repositories. A selection is
  persisted only after the selected folder loads successfully as a Git repo.
- Discard is the sole destructive working-copy action in this slice and always
  requires an explicit confirmation before Rust receives the command.
- Review-remote resolution, preferred base selection, find-or-create policy,
  and the cross-platform keyring adapter live in `hunk-forge`. Both the GPUI
  frontend and Qt migration consume the same credential-store implementation.
- QML receives account labels, auth mode, review metadata, and operation state;
  it never receives a stored token. PAT validation, GitHub device-flow polling,
  keychain access, and PR/MR API calls run on background threads. Repository or
  branch changes advance the same forge epoch and cancel stale callbacks.
- GitHub.com uses the existing OAuth device flow. GitHub Enterprise and GitLab
  use PAT entry. PAT text is cleared as soon as QML hands it to Rust, and tests
  use a fake backend so unattended validation never opens the real keychain.
- `hunk-domain` keeps its database and Markdown/language features enabled by
  default, but permits narrow consumers such as `hunk-git` to opt out. This
  prevents a Git-only Qt build from compiling SQLite, Comrak, and every
  Tree-sitter grammar.

Phase 5 macOS build observations through Nix, all using the same default
external-volume `target/` and Cargo cache:

- The initial real `hunk-qt` build through monolithic `hunk-app` took 4 minutes
  26 seconds and compiled 1,408 units, including the Codex/browser/AWS graph.
- Moving the adapter to `hunk-git::workspace` reduced the next affected build
  to 2 minutes 54 seconds.
- Disabling optional Hunk Domain database/Markdown features for the Git-only
  graph reduced the next affected build to 1 minute 24 seconds. An immediate
  warm rebuild took 0.41 seconds, and a warm focused check took 0.76 seconds.
- These are local dependency-boundary observations, not clean-machine CI
  promises. Vendored libgit2/OpenSSL and QtBridge C++ compilation remain
  material on cold macOS builds.
- The final green PR #181 run measured 2 minutes 47 seconds on Windows and
  11 minutes 42 seconds on Linux. The warm Windows Qt build itself took 8
  seconds. Linux spent 1 minute 44 seconds in workspace Clippy and 6 minutes
  51 seconds compiling/running the full non-GPUI workspace test suite; the QML
  suite took 3 seconds. Moving off GPUI materially removes the second desktop
  build, but the retained Rust/Codex dependency graph remains the dominant
  Linux CI cost and must be narrowed separately rather than attributed to Qt.
- The first PR #182 run after moving keyring/network dependencies into the Qt
  graph measured 9 minutes 52 seconds on Windows and 29 minutes 23 seconds on
  Linux. Windows spent 3 minutes in terminal tests and 3 minutes 39 seconds
  building `hunk-qt`; Linux spent 11 minutes 7 seconds in Clippy and 14 minutes
  11 seconds in tests. This is the cache-invalidated case, not a steady-state
  Qt cost, and demonstrates that lockfile/dependency-graph churn can still
  dominate even after GPUI is gone from the gate.
- A subsequent cache-reuse run exposed a Linux `SIGILL` before the
  `hunk-terminal` unit tests started. `libghostty-vt-sys` lets Zig auto-detect
  native CPU features, while the dependency target cache can move its static
  library between heterogeneous GitHub runners. The Linux cache key therefore
  uses a fresh namespace plus the runner CPU model and feature flags;
  compatible runners still share artifacts without falling back to native code
  built for a different CPU.
- The QML suite exercises 1,500 file rows and verifies distant rows are not
  instantiated. The final 8 ms/120 Hz hardware audit remains a cutover gate,
  especially for the substantially heavier Diff and streaming AI surfaces.

Phase 5 local validation through Nix:

- The locked full workspace built successfully in 47.04 seconds on the shared
  warm external-volume cache, and the locked full workspace test suite passed.
- Workspace Clippy passed for all targets with warnings denied.
- All 12 QML interaction/visual-state tests passed together with `qmllint`,
  including PAT clearing, GitHub device-flow command routing, and PR/MR form
  submission through the fake backend.
- The real `hunk_qt` binary launched offscreen without QML or backend errors.

### 6. Diff Tab

- [x] Define immutable, batched selected-file row snapshots with stable identifiers.
- [x] Implement virtualized side-by-side diff presentation.
- [x] Implement virtualized unified diff presentation.
- [x] Restore syntax highlighting and current-file search/navigation.
- [x] Restore row selection, copied diff text, and keyboard/hunk review navigation.
- [ ] Restore comments, folding, and remaining review affordances required by the narrowed product.
- [x] Avoid one QObject per token or other high-churn bridge designs.
- [ ] Instrument frame time, model update time, object count, and allocation hot paths.
- [ ] Verify ordinary QML delegates against representative large repositories.
- [ ] Add the narrow custom `QQuickItem`/scene-graph renderer only if measurements require it.
- [ ] Meet the 8 ms frame budget at 120 Hz for scroll, resize, selection, and streamed updates.
- [ ] Complete the remaining mandatory working loop and stacked PRs. Delivered
  Diff layers: [#185](https://github.com/smolcars/hunk/pull/185),
  [#186](https://github.com/smolcars/hunk/pull/186),
  [#187](https://github.com/smolcars/hunk/pull/187), and
  [#188](https://github.com/smolcars/hunk/pull/188).

Phase 6 initial Qt slice decisions:

- Qt consumes `hunk-app::diff` as a narrowly feature-gated dependency instead
  of duplicating patch projection or pulling the AI/browser/mobile dependency
  graph into the Qt build. The default `hunk-app` feature set remains unchanged
  for the legacy frontend during migration.
- `hunk-git` loads one selected changed-file patch on a background thread;
  `hunk-app::diff` projects stable rows and binary/error states there before a
  single QtBridge model reset. Selection and repository changes invalidate the
  previous epoch so stale work cannot replace the active file.
- The Diff sidebar is a changed-file navigator, not a restored File Explorer.
  It contains only paths in the active working-tree comparison and exposes no
  arbitrary directory traversal or file mutation behavior.
- QML renders fixed-height code rows through recycling `ListView` delegates and
  a horizontally scrollable review canvas. Syntax, search, selection, comments,
  folding, unified mode, instrumentation, and the final 120 Hz hardware gate
  remain subsequent Diff slices rather than hidden scope in this foundation.

Phase 6 review-tools decisions:

- Syntax and intra-line segments are produced with the selected-file payload on
  the existing background worker. Rust emits one escaped semantic-markup string
  per side and QML resolves its small token palette through `Theme.qml`; tokens
  are not bridged as QObjects.
- Detailed intra-line segments are retained through 4,000 projected rows. Larger
  files use syntax-only segments to bound background projection cost until the
  representative-repository performance gate provides a measured threshold.
- Split and unified views derive from the same stable side-by-side row model.
  A paired removal/addition consumes two fixed-height unified lines without a
  second patch parse, model, or repository request.
- Current-file search uses a lowercase row index built beside the payload off
  the Qt thread. Keystrokes scan those pre-normalized strings, publish only row
  indices, and move the existing virtualized `ListView` to the active match.
- Semantic placeholder delimiters are HTML-escaped inside source text before
  QML performs theme substitution, preventing code such as `@keyword@` from
  being rewritten as renderer metadata.

Phase 6 selection-layer decisions:

- Selection anchor/head indices, focus, and selected-row presentation remain
  QML state. File changes and model resets clear or clamp that state; no
  selection QObject or mutation is added to the Rust domain model.
- Exact copied diff lines are precomputed with the immutable payload and moved
  beside the row/search data in one model reset. Copying an inclusive range
  preserves the legacy unified `-`, `+`, and context prefixes without asking
  QML to reconstruct patch semantics.
- Up/Down, Shift+Up/Down, platform Select All/Copy, and F7/Shift+F7 route through
  the focused Diff workspace. Wrapped hunk targeting scans the Rust row model
  without allocating an intermediate hunk list.
- Row taps use `TapHandler` rather than a child `MouseArea`, allowing selection
  taps to coexist with the nested horizontal Flickable and vertical ListView
  without blocking drag-to-scroll gesture arbitration.
- A single transparent QML `TextEdit` performs the native clipboard operation
  for the Rust-projected selected text, then restores focus to the Diff surface.

Phase 6 comment-core decisions:

- Comment line-side vocabulary and deterministic anchor hashing are pure domain
  behavior, so they remain available without activating the SQLite feature.
  The existing `hunk_domain::db` re-exports remain intact for legacy callers;
  persistence is enabled only by the subsequent comment-store layer.
- `hunk-app::diff` projects comment anchors with the legacy two-row context
  radius, same-file context bounds, current hunk header, stable row identifier,
  and left/right/meta location semantics. Hunk and synthetic state rows remain
  non-commentable.
- The selected-file worker builds anchors beside syntax, search, and copy data.
  Filtering the internal file-header row removes its aligned anchor in the same
  pass, and the resulting vectors move into the Qt list model in one reset.
- Qt enables the narrow `hunk-app` comments feature explicitly. The legacy
  default feature set stays unchanged, and this prerequisite adds no QML object
  or database mutation before the asynchronous comment UI/store layer.

Phase 6 comment-store decisions:

- `hunk-app` owns scope-checked comment-store commands while `hunk-domain`
  continues to own SQLite records and migrations. Qt enables persistence through
  an explicit `comment-store` feature; no Qt or QtBridge type enters either
  headless crate.
- Database access, exact/hash/fuzzy matching, and list projection run on one
  serialized background worker. Results carry repository/branch scope plus both
  comment and Diff epochs, so switching repositories, branches, or selected
  files cannot apply stale row matches on the Qt thread.
- Reconciliation preserves the inherited two-miss threshold. It touches matched
  comments immediately, defers changed files whose selected-file diff has not
  loaded, marks missing anchors stale, and resolves unchanged files. When any
  rename is present, unmatched old-path comments remain open until a renamed
  diff can prove or disprove the fuzzy match.
- Shared `Arc` ownership keeps the row model and background matcher aligned
  without cloning every anchor. Visible comment items are capped at the legacy
  64-item preview limit while counts and reconciliation still cover every
  record, bounding the Qt-thread model reset.
- This layer deliberately exposes the QtBridge model, properties, commands,
  copy payloads, row counts, and jump targets without presentation. The
  contextual composer, row badges, and virtualized inspector are the next stack
  layer so persistence/concurrency can be reviewed independently from QML.

Phase 6 initial-slice macOS validation through Nix, using the existing
external-volume Cargo target and caches:

- The full workspace build passed in 47.66 seconds.
- The full workspace test suite passed, including the new Qt projection tests.
- Workspace Clippy passed for all targets with warnings denied in 15.52 seconds.
- `qmllint` and all 16 QML interaction/virtualization/visual tests passed. The
  Diff smoke case verified that a 5,000-row model does not instantiate a
  distant row, and the desktop-size rendered snapshot was visually inspected.

Phase 6 review-tools macOS validation through Nix, reusing the same target and
external caches:

- The full workspace build passed in 9.61 seconds after the changed Qt crate
  compiled, and the complete workspace test suite passed.
- Workspace Clippy passed for all targets with warnings denied in 11.48 seconds.
- `qmllint` and all 18 QML interaction/virtualization/visual tests passed in
  467 milliseconds. The rendered desktop snapshot was inspected with semantic
  syntax colors, preserved indentation, split controls, and cleared search.

Phase 6 selection-layer macOS validation through Nix, reusing the same target
and external caches:

- The full workspace build passed in 10.59 seconds, and the complete workspace
  test suite passed, including four Qt Diff model tests.
- Workspace Clippy passed for all targets with warnings denied in 3.38 seconds.
- `qmllint` and all 20 QML interaction/virtualization/visual tests passed in
  472 milliseconds. The rendered desktop snapshot was inspected with syntax,
  split diff coloring, and the selected-row treatment visible together.

Phase 6 comment-core macOS validation through Nix, reusing the same target and
external caches:

- The full workspace build passed in 47.91 seconds, and the complete workspace
  test suite passed in 1 minute 26 seconds, including anchor-side/context and Qt
  row-alignment coverage.
- Workspace Clippy passed for all targets with warnings denied in 7.10 seconds.
- `qmllint` and all 20 existing QML interaction/virtualization/visual tests
  passed in 482 milliseconds. This prerequisite intentionally changes no QML
  presentation; comment editing and persistence are the next Diff layer.

Phase 6 comment-store macOS validation through Nix, reusing the same target and
external caches:

- The full workspace build passed in 32.21 seconds after the changed Qt and
  headless comment crates compiled.
- The complete workspace test suite passed, including scope-guarded CRUD,
  exact/hash/fuzzy matching, two-miss reconciliation, rename preservation, the
  64-item projection bound, and Qt comment-list coverage.
- Workspace Clippy passed for all targets with warnings denied in 10.43 seconds.
- `qmllint` and all 20 existing QML interaction/virtualization/visual tests
  passed in 486 milliseconds. The store seam intentionally adds no QML surface;
  the composer, badges, and inspector remain the next layer.

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
