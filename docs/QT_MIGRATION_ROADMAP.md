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
  <- migration/13-ai-runtime-paths
  <- migration/14-qt-ai-runtime
  <- migration/15-qt-ai-catalog
  <- migration/16-qt-ai-composer
  <- migration/17-qt-ai-requests
  <- additional independently reviewable Qt AI layers
  <- atomic Qt cutover and CI replacement
  <- release hardening
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
- [x] Restore comment creation, row counts, branch-scoped history, status changes, deletion, copy, and jump behavior.
- [ ] Restore folding and remaining review affordances required by the narrowed product.
- [x] Avoid one QObject per token or other high-churn bridge designs.
- [ ] Instrument frame time, model update time, object count, and allocation hot paths.
- [ ] Verify ordinary QML delegates against representative large repositories.
- [ ] Add the narrow custom `QQuickItem`/scene-graph renderer only if measurements require it.
- [ ] Meet the 8 ms frame budget at 120 Hz for scroll, resize, selection, and streamed updates.
- [ ] Complete the remaining mandatory working loop and stacked PRs. Delivered
  Diff layers: [#185](https://github.com/smolcars/hunk/pull/185),
  [#186](https://github.com/smolcars/hunk/pull/186),
  [#187](https://github.com/smolcars/hunk/pull/187), and
  [#188](https://github.com/smolcars/hunk/pull/188). Comment persistence:
  [#189](https://github.com/smolcars/hunk/pull/189).

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

Phase 6 comment-UI decisions:

- QML owns only transient presentation state: whether the inspector is open,
  which row has the single active composer, its draft text, and local focus.
  Comment records, counts, matching, mutations, and jump targets remain Rust
  model/property/command state.
- The code canvas remains primary. A single 380-pixel composer is loaded only
  for the selected commentable row and follows its visible scroll position; a
  360-pixel right inspector narrows the canvas only while explicitly open.
  Entries use separators rather than nested cards and the only motion is the
  110-millisecond inspector width/opacity transition.
- Row badges call the Rust count seam only for instantiated Diff delegates and
  recompute when `diffCommentsVersion` changes. The inspector uses a recycling
  `ListView` over the already bounded 64-item model, preserving virtualization
  without one QObject or QML model object per source row.
- The composer waits for the asynchronous Rust result. Database errors leave
  the draft and focus intact; `Comment added.` closes the composer and reveals
  the inspector. The workspace-level BeforeItem key handler yields while the
  editor owns focus so arrow, selection, copy, and submit keys remain native.
- Copy uses the existing hidden native `TextEdit` clipboard proxy for one
  comment, all open comments, and selected diff text. Jump notifications carry
  a monotonically changing revision so repeated jumps to the same row still
  reposition the virtualized Diff list.

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

Phase 6 comment-UI macOS validation through Nix, reusing the same target and
external caches:

- The full workspace build passed in 10.21 seconds, and the complete workspace
  test suite passed in about 30 seconds, including five Qt comment-model tests.
- Workspace Clippy passed for all targets with warnings denied in 3.35 seconds.
- `qmllint` and all 24 QML interaction/virtualization/visual tests passed in
  987 milliseconds. The inspected 1280-by-760 render showed the contextual
  composer, narrowed Diff canvas, and populated right inspector together; that
  visual pass also caught and fixed the inspector's initial animated-visibility
  lifecycle before submission.

### 7. AI Tab

- [x] Move Codex executable discovery and validation below both frontends.
- [x] Establish the lazy, repository-scoped Qt worker lifecycle, bounded thread catalog, active-thread state, and basic refresh/start/select/archive commands.
- [x] Expose the thread catalog, active thread, bounded read-only turn timeline, streaming rows, and runtime state through QtBridge.
- [x] Expose the text composer and its send/steer/interrupt state through QtBridge.
- [x] Implement thread load/start/resume/fork/archive and cwd scoping.
- [x] Implement streaming messages and tool output without per-token QObject churn or structural model resets.
- [x] Implement approvals and request-user-input with exact pending-command state.
- [x] Implement text steering, interruption, and plan-state presentation.
- [x] Implement queued message recovery.
- [x] Implement persistent bookmark ordering and interaction.
- [x] Implement context usage, model/settings, collaboration-mode, approval-policy, and service-tier controls still in product scope.
- [x] Implement prompt attachments still in product scope.
- [x] Port required terminal surfaces with correct input, focus, cursor, selection, and resize behavior.
- [ ] Port the required embedded CEF browser surface, controls, input routing, and AI tool bridge to Qt.
- [ ] Validate key flows without triggering unattended keychain prompts.
- [ ] Complete the mandatory working loop and stacked PR.

Product decision recorded on 2026-08-25: both the terminal and embedded browser
remain required parts of Hunk and must work in the Qt application before the
atomic cutover. Reuse the existing `hunk-terminal` and `hunk-browser`/CEF domain
and runtime layers. Replace their GPUI presentation and input adapters with
narrow Qt adapters; do not introduce Qt WebEngine or a second browser engine.

Qt terminal decisions:

- The Qt application uses one repository-scoped bottom drawer shared by Diff,
  Git, and AI. It supports up to 12 live tabs and resets those sessions when the
  active repository changes, avoiding both the removed Files workspace and a
  second per-thread presentation model.
- `hunk-terminal` remains the only PTY, VT, key encoding, mouse protocol,
  scrollback, and shell-resolution authority. The former desktop-only shell
  environment helper moved into that crate so GPUI and Qt consume the same
  configured shell and login-environment behavior.
- Shell startup, VT screen projection, and burst coalescing run off the Qt
  thread. Listener threads retain only the latest projected screen and terminal
  end event per tab generation and schedule at most one queued Qt callback.
  Raw output events do not cross QtBridge, stale tab generations are rejected,
  and dropping a tab drops its session handle without joining on the
  frame-sensitive Qt thread.
- The QML surface uses one recycled delegate per visible row rather than one
  delegate per cell. Rust projects bounded rich-text rows, patches only changed
  rows when the grid size is stable, preserves wide/combining-cell selection,
  and keeps all terminal palette values in `Theme.qml`.
- Closing the drawer unloads its QML tree without resizing or stopping live PTY
  sessions. Hidden sessions keep only their latest projected screen; reopening
  applies it once and restores focus to the exact control that owned focus
  before the terminal opened.
- The retained interaction contract includes shell tabs, keyboard and IME text,
  bracketed paste, focus reporting, cursor shapes and blink, scrollback,
  selection/copy, VT mouse and wheel modes, PTY resize, and the existing
  platform terminal toggle/tab shortcuts. AI command rows can explicitly send
  an untruncated command to the matching cwd; commands never run automatically.

Qt terminal validation on 2026-08-25 used the shared external-volume caches and
did not launch Hunk, Codex, or any keychain-facing runtime:

- `cargo build --workspace --all-targets` passed through the Nix shell.
- `cargo test --workspace` passed through the Nix shell.
- `cargo clippy --workspace --all-targets -- -D warnings` passed through the Nix shell.
- Qt 6.11.2 `qmllint` accepted the changed terminal QML; its expected warnings
  are limited to dynamic QtBridge backend properties that have no static QML type.
- The offscreen Qt Quick suite passed all 92 tests, including terminal delegate
  roles, hidden resize preservation, tab overflow visibility, and exact AI/Git
  focus restoration. Live 120 Hz hardware profiling remains part of the atomic
  cutover performance gate rather than this non-interactive validation layer.

Prompt attachment decisions:

- Keep image validation in the shared headless application boundary so GPUI and
  Qt accept the same formats during the migration.
- Keep per-thread attachment drafts in Rust. QML owns only the native picker,
  drag/drop interaction, bounded chip list, and focus behavior.
- Bound candidate count and serialized input before crossing QtBridge, then run
  canonicalization and filesystem checks on retained Rust workers rather than
  the Qt thread. Pending validation is keyed by thread so one slow volume cannot
  block composing and sending in an unrelated Codex thread.
- Retain attachments through direct-send acknowledgement, queued follow-up
  delivery, interrupt recovery, and edit-last recovery. Clear them only after
  authoritative acceptance or an explicit successful queue transition.
- Derive QML attachment presence from the list model's own notified row count;
  do not export a separately notified count property that can drift from model
  resets during recovery.

Prompt attachment validation on macOS through Nix:

- `cargo build --workspace --all-targets`
- `cargo test --workspace -- --test-threads=1`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `qmllint` for the production Hunk QML module
- Qt Quick Test: 69 passed, 0 failed, 0 skipped on Qt 6.11.2

Phase 7 runtime-path prerequisite decisions:

- Codex executable discovery and validation belong to `hunk-app::ai`, not the
  GPUI controller or Qt backend. During migration both frontends must resolve
  `HUNK_CODEX_EXECUTABLE`, development workspace assets, packaged resources,
  and platform fallbacks through the same toolkit-neutral implementation.
- Moving the resolver across crates must not let `env!("CARGO_PKG_NAME")`
  silently change Linux package lookup from `hunk-desktop` to `hunk-app`.
  Preserve both legacy desktop package spellings and add the Qt package
  spellings explicitly while retaining the current executable name as the
  highest-priority Linux package directory.
- Resolver tests live under `crates/hunk-app/tests` and create inert fake
  launchers only. This prerequisite neither starts Codex nor reads keychain
  credentials; the following Qt worker layer can therefore reuse it without
  introducing an authentication side effect during ordinary validation.

Phase 7 runtime-path prerequisite macOS validation through Nix, reusing the
same external-volume target and Cargo cache:

- The full workspace build passed in 38.26 seconds, and the complete workspace
  test suite passed in about 63 seconds.
- All seven new `hunk-app` runtime-path tests passed alongside the retained
  GPUI compatibility cases, covering workspace assets, adjacent and macOS
  bundled resources, packaged-layout isolation, and Unix validation.
- Workspace Clippy passed for all targets with warnings denied in 9.41 seconds.
  Validation used inert temporary launchers and did not start Codex or access
  the keychain.

Phase 7 Qt worker-foundation decisions:

- Qt starts one retained `hunk-app::ai` worker lazily when the AI workspace is
  first selected. Diff/Git-only launches do not start Codex, and changing the
  repository invalidates and shuts down the previous worker before the new
  repository can publish AI state.
- A mutex-protected epoch mailbox coalesces only consecutive full snapshots and
  schedules at most one queued Qt callback until the UI thread drains it. Status,
  error, tool, and lifecycle events remain ordered and are never coalesced.
- Repository switches and QObject destruction drain abandoned events. Every
  queued or late browser-tool request receives an explicit unavailable response,
  even though browser tools are disabled in this foundation, so the Codex worker
  cannot wait forever on a frontend that no longer owns the request.
- Worker shutdown never joins on the Qt thread. Dropping the repository-scoped
  session sends `Shutdown` and moves worker/listener joins to a small reaper
  thread, keeping tab and repository interaction non-blocking.
- The QtBridge thread model excludes archived threads, preserves the GPUI
  created-time ordering, marks the active and in-progress rows, and caps visible
  replacement at 200 items. The worker foundation initially retained the latest
  `AiSnapshot`; the following timeline layer replaces that temporary retention
  with bounded listener-thread projections so QML still receives no
  reducer/domain state and the Qt thread never drops a full reducer snapshot.
- This layer exposes runtime state and refresh/start/select/archive commands but
  deliberately adds no placeholder AI presentation. Timeline projection,
  composer/streaming behavior, approvals, and the final visual surface remain
  separate reviewable layers.

Phase 7 Qt worker-foundation macOS validation through Nix, reusing the external
Cargo target and cache:

- The warm full-workspace all-target build passed in 14.80 seconds, and the full
  workspace test suite passed in about 57 seconds.
- Workspace Clippy passed for all targets with warnings denied. Its review found
  and removed large-enum padding in the mailbox by boxing worker events; the
  post-fix focused `hunk-qt` suite passed all 22 tests and doc tests.
- The first exact `hunk-qt` test-profile build after enabling `hunk-app/ai` took
  2 minutes 48 seconds. The workspace-wide test artifact had used a different
  feature-unified dependency combination, so Cargo correctly built the Qt-only
  AI combination once. This cost came from the retained Codex/network/image
  graph, not GPUI or Qt rendering, and now populates the shared external cache.
- Validation constructed only inert Qt models and mailbox events. It did not
  launch Hunk or Codex and did not access the keychain.

Phase 7 Qt catalog/timeline decisions:

- The selected conversation is projected to the latest 80 turns and at most
  1,000 visible rows. Aggregate turn/row counts remain available so the UI can
  state when older history is omitted instead of silently implying completeness.
- Snapshot projection runs on the existing AI event-listener thread before the
  Qt callback is scheduled. The projected payload contains only catalog,
  timeline, authentication, and selected-thread metadata; the full cloned
  reducer snapshot is dropped off the Qt thread.
- Stable row IDs turn streamed content changes into targeted QtBridge model
  updates. The model resets only when row order or membership changes, and
  consecutive projected snapshots retain the mailbox's ordering-aware
  coalescing behavior.
- The 200-thread catalog always retains the active thread even when it is older
  than the newest visible window. Archived or otherwise missing active IDs are
  rejected before the timeline is projected.
- QML uses recycling `ListView` delegates with bounded caches. User, assistant,
  plan, system, and compact tool rows render all reducer-provided strings as
  plain text, allow message selection, follow streamed height growth only while
  the user remains at the tail, and yield immediately when the user scrolls.
- The initial Qt AI presentation follows the useful hierarchy of the already
  open legacy app—a dense thread rail and dominant unboxed conversation plane—
  without recreating its outgoing GPUI implementation or adding a fake composer.
  Composer/send/steer behavior remains the next independently reviewable layer.

Phase 7 Qt catalog/timeline macOS validation through Nix, reusing the external
Cargo target, Cargo cache, and Qt 6.11.2 SDK:

- The full workspace all-target build passed in 26.08 seconds; only `hunk-qt`
  required compilation.
- The complete workspace test suite passed in about 50 seconds, including the
  new catalog pinning, projected-mailbox, timeline ordering/bounds, UTF-8
  truncation, metadata fallback, and stable-row update coverage.
- `qmllint` and all 30 QML interaction, state, virtualization, and rendered
  snapshot tests passed in 1.13 seconds. The inspected 1280-by-760 AI snapshot
  showed the dense catalog, literal plain-text fixture, assistant/plan rows, and
  compact streaming tool row without instantiating the distant 1,000-row item.
- Workspace Clippy passed for all targets with warnings denied in 6.32 seconds.
  Validation used model fixtures only and did not launch Hunk, Codex, or any
  credential/keychain path.

Phase 7 Qt text-composer decisions:

- Qt sends text-only prompts through the retained `AiWorkerCommand::SendPrompt`
  path. The worker continues to decide whether that command starts a new turn or
  steers the current in-progress turn; QML does not duplicate Codex lifecycle
  policy.
- Channel delivery is not treated as prompt acceptance. The backend keeps a
  receipt for the selected thread and baseline turn. A new-turn prompt is
  accepted only when an authoritative snapshot exposes a new active turn or a
  larger turn count, while a steer waits for the worker's explicit
  `SteerAccepted` event.
- The submitted draft stays visible and disabled until that receipt is accepted.
  Worker errors and disconnects clear backend pending state without advancing
  the acceptance revision, allowing QML to restore the exact draft for editing.
- Drafts are memory-only, keyed by thread, and owned above the workspace loader
  so they survive Diff/Git tab switches. Changing repository roots replaces the
  entire draft store; prompts, images, tokens, or other composer data are not
  persisted or logged by this layer.
- Interrupt commands capture both the selected thread and exact active turn.
  Duplicate send/steer/interrupt commands and catalog mutations are disabled in
  QML and rejected again by Rust until the authoritative state resolves the
  pending command.
- The fixed bottom composer uses only theme colors, plain-text `TextEdit`, and a
  bounded local `Flickable`. Editing mutates only a small JavaScript draft entry;
  streamed timeline updates retain the existing bounded model and virtualized
  delegates.
- This layer intentionally excludes attachments, skills, approvals, queued
  message recovery, model/service-tier controls, and persistence. Those remain
  separate AI layers instead of entering the first writable composer contract.

Phase 7 Qt text-composer macOS validation through Nix, reusing the external
Cargo target, Cargo cache, and Qt 6.11.2 SDK:

- The full workspace all-target build passed in 27.08 seconds; only `hunk-qt`
  required compilation. The complete workspace test suite then passed,
  including the new prompt-receipt and running-turn projection tests.
- `qmllint` and all 36 QML interaction, state, virtualization, keyboard, draft,
  command-deduplication, and rendered snapshot tests passed in 1.34 seconds.
- The inspected 1280-by-760 render showed a focused bottom composer below the
  still-virtualized timeline, with theme-native Send/Steer and exact-turn Stop
  controls. The conversation hierarchy and dense catalog remain unchanged.
- Workspace Clippy passed for all targets with warnings denied in 7.30 seconds.
  Validation used Rust/QML fixtures only and did not launch Hunk, Codex, or any
  credential/keychain path.

Phase 7 Qt approval/request-input decisions:

- Pending approvals and request-user-input records are projected on the Codex
  listener thread. Qt receives aggregate counts, a bounded set of visible
  attention-thread IDs, and only the selected thread's oldest actionable
  request; it never retains or drops the full reducer snapshot on the UI thread.
- Command/file approvals retain their exact request IDs through Accept or
  Decline. User-input responses are validated again in Rust against the current
  exact request, exact question IDs, one answer per question, offered options,
  and bounded answer sizes before reaching `AiWorkerCommand`.
- The panel shows approvals before user input to preserve retained worker
  ordering. A resolving request disables duplicate responses and catalog
  mutations until an authoritative snapshot removes or replaces that exact ID;
  navigation remains available so users can reach another attention thread.
- Questions and options are capped before crossing QtBridge. An oversized,
  duplicate-ID, or semantically truncated request is displayed as unsupported
  and cannot be partially submitted. Every affected thread still visible in
  the 200-row catalog receives an attention role.
- Answers live only in a memory-only QML store above the workspace loader,
  keyed by exact request ID. They survive failures plus Diff/Git and attention-
  thread round trips, are pruned against the bounded current requests Qt can
  display, and never enter persistence, production logs, or Rust properties.
  Secret fields additionally use password echo.
- Attention threads cannot be archived from QML and are rejected again at the
  Rust command boundary. Request arrival hands keyboard focus from the composer
  to the first response control, completion restores it, and focus-driven
  scrolling keeps all eight bounded questions reachable inside the capped
  panel.
- Long semantic option labels remain exact for submission but wrap inside the
  viewport. Custom options, text fields, and shared action buttons expose
  accessible roles/names and visible focus treatment; unsupported requests use
  a generic fail-closed explanation rather than mislabeling every validation
  failure as a size problem.
- This layer also raises the repository baseline to Rust 1.98. Cargo workspace
  metadata, all member manifests, `rust-toolchain.toml`, Nix, Windows/Linux CI
  actions, and the patched vendor manifest agree on the version. The
  rust-overlay lock is refreshed so `nix develop` resolves the exact toolchain
  instead of a moving latest-stable alias.

Phase 7 Qt approval/request-input validation through Nix, reusing the external
Cargo target, Cargo cache, and Qt 6.11.2 SDK:

- The exact compiler was `rustc 1.98.0 (88d9e12ae 2026-08-18)`. The full
  workspace all-target build passed in 32.39 seconds, and the complete workspace
  test suite passed in about 2 minutes 57 seconds, including six new Rust
  request-projection and validation tests.
- Rust 1.98 introduced stricter lints and error-size checks in existing code.
  The compatibility updates box one large JSON-RPC error variant, use guarded
  notification matches, use the fixed-size RGBA chunk API, and express a rename
  traversal as `while let`; workspace Clippy then passed for all targets with
  warnings denied on the warm cache.
- `qmllint` passed for the complete module, and all 40 QML interaction, failure,
  retry, focus, overflow, virtualization, and rendered-state tests passed in
  1.261 seconds. The inspected 1280-by-760 render remained structurally intact.
- The repository's seven Qt agent skills and their lock metadata are included
  in this layer. Their deterministic QML lint plus six focused review passes
  drove the final accessibility, focus, bounded-state, archive, and geometry
  corrections; system `qmllint` remained clean afterward.
- Validation used Rust/QML fixtures only and did not launch Hunk, Codex, or any
  credential/keychain path.

Phase 7 Qt thread-lifecycle decisions:

- Qt thread selection resumes and reads the exact catalog thread through the
  shared worker. Create, full-history Fork, and Archive also remain worker
  commands; QML does not call Codex or duplicate repository policy.
- Fork is available only for an idle selected thread. The shared service injects
  the repository cwd, rejects returned threads outside that workspace, and asks
  Codex to defer inherited goal continuation so the copied history does not
  begin working again before the user sends a message.
- Qt accepts lifecycle targets only from its repository-scoped catalog. Each
  mutation retains an exact receipt: Select waits for that thread to become
  active, Create and Fork wait for both the returned new ID and its active
  snapshot, and Archive waits for the exact ID to leave the catalog.
- Lifecycle receipts lock prompts, interrupts, approvals, request input, and
  other catalog mutations at both the QML and Rust boundaries. Controls regain
  focus only when the authoritative receipt clears, and an open archive
  confirmation is canceled if another blocking action arrives.
- Non-idempotent Start, Fork, and Archive commands are never replayed after a
  transport reconnect. A restored connection reports the abandoned command as
  an error rather than a passive status, which clears pending receipts in both
  frontends and tells the user to retry instead of leaving the UI locked.

Phase 7 Qt thread-lifecycle validation through Nix, reusing the external Cargo
target, Cargo cache, and Qt 6.11.2 SDK:

- The full workspace all-target build passed in 1 minute 59 seconds, and the
  complete workspace all-target test suite passed in about 2 minutes 48
  seconds, including the new reconnect policy and exact lifecycle receipt tests.
- System `qmllint` passed for the complete module, and all 42 QML interaction,
  lifecycle-lock, focus-recovery, virtualization, and rendered-state tests
  passed in 1.185 seconds.
- The deterministic QML lint and six focused read-only review passes found and
  drove fixes for reconnect receipt cleanup, reciprocal command guards,
  misleading request progress, archive-dialog cancellation, and deferred focus
  ownership. System lint and the QML suite remained clean afterward.
- The inspected 1280-by-760 render preserved the dense thread rail and unboxed
  conversation plane, with one compact Fork action in the header and no new
  per-frame model or object work.
- Workspace Clippy passed for all targets with warnings denied in 51.44 seconds.
  Validation used Rust/QML fixtures only and did not launch Hunk, Codex, or any
  credential/keychain path.

Phase 7 Qt queued-message decisions:

- Queued follow-ups are Rust-owned, repository-scoped, and FIFO per thread.
  The queue holds at most 64 messages, rejects prompts larger than 256 KiB,
  and caps total retained queued/recovered text at 1 MiB. QML receives only
  bounded timeline projections and aggregate counts.
- A message waits for the authoritative thread to become idle. Delivery marks
  it as sending but does not remove it; removal requires either the exact
  `SteerAccepted` prompt or a later exact user-message fingerprint after the
  captured sequence. Runtime failures return unconfirmed sends to queued state.
- Interrupting a turn or losing an available thread moves its queued text back
  into that thread's in-memory draft. Same-repository runtime restarts preserve
  the queue, while repository-root changes clear it before the new worker can
  publish state.
- Queued rows share the existing virtualized timeline model. QtBridge emits
  incremental tail insert/remove/update notifications instead of resetting the
  `ListView`; authoritative plus queued rows remain capped at 1,000, with any
  displaced authoritative rows reflected in the hidden-row count.
- Plain Tab queues only while a turn is running. Ctrl+Shift+Up moves the newest
  still-queued message back to an empty composer, never overwrites an existing
  draft, and acknowledgement or interrupt recovery restores keyboard focus.

Phase 7 Qt queued-message validation through Nix, reusing the repository target,
external Cargo cache, and Qt 6.11.2 SDK:

- The full workspace all-target build passed in 1 minute 27 seconds, and the
  complete workspace all-target test suite passed, including eight queue and
  seven timeline-model tests.
- System `qmllint` and all 47 QML interaction, recovery, focus, virtualization,
  and rendered-state tests passed in 1.277 seconds. The inspected 1280-by-760 AI
  render preserved the dense catalog, dominant timeline, and compact composer.
- The deterministic QML lint plus six focused read-only review passes drove the
  final suffix-model, byte-bound, stable-ID, focus, draft-preservation, and
  keyboard corrections. Workspace Clippy then passed for all targets with
  warnings denied in 7.01 seconds on the warm cache.
- Validation used only Rust/QML fixtures. It did not launch Hunk or Codex and
  did not access any credential or keychain path.

Phase 7 Qt bookmark decisions:

- Bookmarks remain global application state, matching the retained
  `AppState.ai_bookmarked_thread_ids` contract. Qt loads them before the first
  AI projection and persists changes without moving thread ownership into QML.
- The listener sorts bookmarks before applying the 200-thread catalog bound, so
  an older bookmark stays reachable. The Qt thread reapplies its current
  bookmark set when accepting a projected snapshot so an already-queued stale
  snapshot cannot undo an optimistic interaction.
- Bookmark writes run away from the Qt thread and share one serialized
  load-modify-save boundary with repository selection. Superseded writes are
  skipped, and a failed latest write restores the complete prior set in memory
  and on disk rather than guessing from one row.
- The backend retains outstanding bookmark-save tasks and joins them during
  shutdown. `AppStateStore` writes through a flushed same-directory temporary
  file and atomically replaces the previous TOML, so closing during a save
  cannot truncate the shared state file.
- An optimistic toggle emits one role update and, when needed, one notified row
  move. It never resets the 200-row model or invalidates every recycled
  delegate for a single bookmark click.
- The dense thread rail adds one persistent star for bookmarked rows and reveals
  the unbookmarked action with the existing hover/active controls. The action
  has a semantic accessible name while preserving the restrained unboxed
  conversation layout. The row-selection hit area stops at the action strip so
  selection cannot intercept bookmark or archive input.

Phase 7 Qt bookmark validation through Nix, reusing the repository target,
external Cargo cache, and Qt 6.11.2 SDK:

- The full workspace all-target build passed in 2 minutes 10 seconds, and the
  complete workspace all-target test suite passed in about 2 minutes 43
  seconds, including atomic state replacement and incremental bookmark-model
  coverage.
- System `qmllint` and all 48 QML interaction, accessibility, virtualization,
  and rendered-state tests passed; the final QML run completed in 1.256
  seconds. The inspected 1280-by-760 AI render preserved the dense thread rail
  and dominant conversation plane with the compact star beside Archive.
- The deterministic QML lint and six focused read-only review passes found and
  drove fixes for detached persistence tasks, non-atomic state writes, and
  full catalog resets on one bookmark click. The interaction suite additionally
  separated row-selection and action hit regions.
- Workspace Clippy passed for all targets with warnings denied in 1 minute 3
  seconds. Validation used Rust/QML fixtures only and did not launch Hunk or
  Codex or access any credential/keychain path.

Phase 7 Qt session-control decisions:

- Session selection remains Rust-owned and preserves the retained precedence:
  exact thread override, then repository workspace override, then product
  defaults. Qt projects only bounded model/effort/service-tier choices and
  sends the resolved override with new threads, direct prompts, and recovered
  queued prompts.
- Approval policy and hidden-model behavior are loaded before the repository
  worker starts. Approval-policy changes update the live worker and the same
  persisted workspace setting; failed persistence restores both the projected
  selection and worker mode.
- Model descriptions, display labels, catalogs, and per-model effort lists are
  byte/item bounded before crossing QtBridge. The compact header additionally
  caps and elides the selected model summary so service-provided names cannot
  consume the workspace title area.
- Context usage mirrors the retained baseline-adjusted Codex window math and
  keeps raw token arithmetic in Rust. QML receives only percentages and compact
  display values, and the detailed settings subtree is unloaded while its
  popup is closed.
- Session properties use a granular change signal rather than the streamed AI
  event signal. Catalog labels and formatted token strings therefore do not
  cross the Rust/QML boundary for unrelated streaming updates, preserving the
  8 ms frame budget.
- Session writes share the serialized atomic application-state boundary and
  retain their background tasks through shutdown. Each write reloads current
  state before changing only session-owned fields, so bookmarks and repository
  selection cannot be overwritten by a stale full-state snapshot.
- Settings lock while the active turn, prompt receipt, interrupt, thread
  lifecycle action, authentication, or startup state makes a change unsafe.
  Rust validates every selection again, and QML restores the authoritative
  index if a raced command is rejected.
- Attachments remain the next independent AI layer; this change does not add
  file picking, image payloads, or skill selection to the composer.

Phase 7 Qt session-control validation through Nix, reusing the repository
target, external Cargo cache, and Qt 6.11.2 SDK:

- The full workspace all-target build passed in 1 minute 59 seconds, and the
  complete workspace all-target test suite passed, including six focused
  session-projection, selection, context-window, and persistence tests.
- System `qmllint` produced no warnings for the changed controls and shell
  fixtures. All 58 QML interaction, accessibility, virtualization, recovery,
  and rendered-state tests passed in 1.371 seconds.
- Six focused read-only QML review passes drove the final popup-lifecycle,
  bounded-layout, authoritative-selection, delegate-width, and text-rendering
  corrections. Workspace Clippy then passed for all targets with warnings
  denied in 6.21 seconds on the warm cache.
- Validation used only Rust/QML fixtures. It did not launch Hunk or Codex and
  did not access any credential or keychain path.

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
