# Lessons Learned

This is the append-only engineering log for the GPUI-to-Qt migration. Add a
dated entry whenever a migration slice reveals a constraint, a bug, a failed
approach, or a technique worth preserving. Do not rewrite earlier conclusions;
append a correction when later evidence changes one.

## 2026-08-24 — Migration baseline

- Qt 6.11.2 is the latest stable Qt release. The repository's current locked
  Nixpkgs exposes Qt 6.10.2, so merely adding `qt6` packages to `flake.nix`
  would violate the latest-version requirement; the input lock must move too.
- Official QtBridge is well aligned with Hunk's desired QML-facing adapter:
  QObjects, properties, signals, slots, models, JSON values, and cross-thread
  invocation. Its `Rc<RefCell<_>>` ownership means domain state belongs behind
  command/snapshot adapters rather than directly inside QObjects.
- QtBridge currently marks macOS arm64 experimental and uses Qt private headers.
  Exact Qt and QtBridge revisions must be pinned and validated together on all
  three release platforms.
- Removing the editable Files workspace does not prove that `hunk-editor`,
  `hunk-text`, or `hunk-language` are unused. The current Diff/review path also
  consumes projected rows, snapshots, search, overlays, and syntax highlighting
  from those crates. Remove APIs only after consumer tracing.
- Recent CI runs show Windows, not macOS, as the usual critical path. The
  workflow also compiles the desktop normally and with CEF. Eliminating duplicate
  feature builds and deciding whether CEF remains are separate from removing GPUI.
- `AGENTS.md` had an existing user-owned modification when the migration began.
  It must remain outside migration commits unless the user explicitly asks to
  incorporate that exact worktree change.

## 2026-08-24 — Codex 0.149.1 upgrade

- The `hunk/embedded-apply-patch-fix` fork branch had no fork-only commit delta;
  it was identical to its previous upstream baseline. Upgrading it to the new
  upstream tag therefore required no patch reapplication, despite the branch
  name implying otherwise.
- Cargo packages with the same native `links` value must resolve to one version
  across the workspace. Codex 0.149.1 required `tree-sitter-powershell` 0.26.4
  and `libsqlite3-sys` 0.37, so Hunk moved to `tree-sitter-powershell` 0.26.4
  and `rusqlite` 0.39 before the lockfile could resolve.
- Codex dynamic tools are now canonical namespace objects containing function
  entries. Keeping Hunk's browser and Android tools grouped under their existing
  namespaces preserves the public tool names while satisfying the new protocol.
- Several app-server payloads became boxed or wrapped and gained optional
  fields. Adapt the Hunk protocol seam with explicit defaults rather than
  enabling new capabilities or history modes implicitly.
- Preserve Codex's boxing for large server notifications and requests across
  Hunk's event boundary. Unboxing them enlarged every event to hundreds of
  bytes and failed the workspace's `large_enum_variant` lint.
- Versioned Codex runtime archives and the Codex source audit checkout live in
  `/Volumes/hulk/dev/cache`; Hunk's ignored runtime assets and Cargo `target/`
  remain on the external workspace volume. The cached official archives were
  verified against GitHub's published SHA-256 digests before staging.
- Correction to the migration baseline: the user explicitly requested that the
  existing `AGENTS.md` worktree change be included in the next commit.

## 2026-08-25 — Editable Files product removal

- The standalone Files editor and the Diff surface shared a large module, but
  only the read-only workspace projection, visible-range syntax spans, and a
  few generic line-paint helpers were required by Diff. Consumer tracing made
  it possible to delete the GPUI editor element, input/mutation paths, saving,
  folding, full-file search/replace, and editor-only tests without weakening
  Diff syntax highlighting or search.
- The old editable Files controller also owned generic window lookup and root
  focus restoration helpers used by Git, AI, and terminal workflows. Move
  genuinely shared helpers before deleting a product controller; module names
  are not reliable ownership boundaries.
- The retained Diff sidebar should derive its changed-file rows directly from
  the active comparison. A full checkout tree, ignored-file scan, filesystem
  mutation context menu, and directory expansion cache are File Explorer
  behavior and do not belong in the narrowed product.

## 2026-08-25 — Headless Git application boundary

- GPUI's background executor did not need to move into the headless service.
  Keeping scheduling, refresh epochs, and repaint notifications in the UI
  adapter while moving load policy and snapshot assembly into `hunk-app`
  produces a synchronous service that Qt can also invoke off its UI thread.
- A frontend should not independently coordinate workflow fingerprints, remote
  branches, and per-file line statistics. Returning those values as one owned
  application snapshot prevents the GPUI and Qt adapters from developing
  different repository refresh semantics while all Git access remains in
  `hunk-git`.

## 2026-08-25 — Headless Diff command and projection

- `gpui::SharedString` had leaked into the cached syntax-segment model even
  though the data itself was not UI-specific. Using owned `String` values in
  the headless snapshot keeps renderer-specific allocation and conversion at
  the adapter edge and makes the same projection consumable by Qt.
- Comparison loading and row projection must be one application operation.
  Returning the `CompareSnapshot` together with stable row metadata, binary and
  collapsed states, and optional segment caches prevents each frontend from
  parsing patches or inventing its own row identifiers.

## 2026-08-25 — Headless AI worker boundary

- Move the existing worker rather than wrapping or cloning it. `hunk-app` now
  owns the single command/event loop, reconnect policy, rollout fallback,
  workspace paths, and dynamic-tool execution; the GPUI layer only applies
  events and performs renderer-owned browser confirmation and frame work.
- A thin compatibility re-export lets the current frontend keep compiling while
  the application boundary becomes the future Qt contract. This keeps the
  cutover incremental without allowing GPUI types into the headless crate.
- White-box worker tests still matter after extracting a public service. Keeping
  their source under `crates/hunk-app/tests/support` and including it only for
  the library test build preserves private policy coverage without exposing
  reconnect and protocol helpers as production API.
- Moving an implementation also moves its direct dependencies. Removing the
  now-orphaned `base64`, `hunk-mobile`, and `webbrowser` dependencies from the
  GPUI crate prevents the temporary adapter from hiding ownership mistakes.
- Compatibility re-exports used only by the legacy frontend's internal tests
  must be gated with `cfg(test)`; test-target checks can otherwise conceal an
  unused import that the normal binary and workspace clippy correctly reject.

## 2026-08-25 — Qt foundation and external SDK cache

- Correction to the migration baseline: moving the flake to the first Nixpkgs
  revision containing Qt 6.11.2 would require locally building hundreds of
  uncached derivations, including Qt itself. That is incompatible with the
  machine's constrained internal storage. Keep Cargo inside the Nix shell, but
  consume Qt's official prebuilt SDK from the persistent external-volume cache.
- Pinning both sides matters: `hunk-qt` verifies `qmake` reports exactly 6.11.2,
  and the official QtBridge dependency is locked to one commit instead of a
  moving branch or wildcard version.
- QtBridge's experimental macOS arm64 path discovers the online SDK's framework
  headers but does not add the parent framework search directory. Supplying the
  SDK's `lib` directory through `-F` fixes generated `<QtCore/...>` includes
  without patching or forking QtBridge.
- Cross-thread QtBridge invocation serializes arguments through `QVariant`.
  Rust `String` is valid for QObject slots but is not itself a `QVariantValue`;
  queue a `QString` and let the slot boundary convert it back to Rust.
- The first QtBridge/C++ build is material, but subsequent focused checks reuse
  the existing workspace `target/`. CI should cache the exact Qt SDK and avoid
  compiling the legacy desktop a second time solely for its CEF feature while
  Qt is still being introduced.
- Nix development shells replace the caller's `TMPDIR` with an internal-disk
  shell directory. Export the configurable `HUNK_BUILD_TMPDIR` again from the
  shell hook (and prefer this machine's existing external cache) so generated
  C++ and linker temporary files do not consume scarce internal storage.
- aqtinstall 3.3.0 cannot resolve Qt 6.11 Windows packages after Qt changed the
  repository from a shared `qt6_6112` child to architecture-specific children.
  Qt's supported unattended Online Installer accepts the exact
  `qt.qt6.6112.win64_msvc2022_64` package, so use that pinned, checksummed path
  for Windows CI and cache the resulting SDK.
- On the self-hosted Mac, generic setup actions tried to create a hosted-tool
  cache under an unavailable `/Users/runner` path. Reusing the already verified
  external Qt SDK is faster, avoids another internal-disk copy, and removes that
  hosted-runner assumption.
- The macOS Actions runner also cannot mount `/Volumes/hulk`; a path available
  to the interactive Mac session is not automatically available to its runner
  service. Do not silently fall back to internal storage for a multi-gigabyte
  Qt SDK and Rust target. Keep the exact macOS gates local until the runner is
  explicitly provisioned with external-volume access.
- Correction to the Windows installer decision: the official Online Installer
  requires Qt Account credentials even for this unattended open-source package
  command. Upstream aqt merge commit `8c3695d4` contains the unreleased Qt 6.11
  Windows repository-layout fix, so pin that revision and keep CI account-free
  instead of introducing personal credentials.
- A warm self-hosted Linux machine is not faster when no matching runner accepts
  the job: the Qt foundation check remained unassigned for more than 13 minutes.
  Use an ephemeral Ubuntu runner with pinned Nix, Qt, and Rust caches for PR
  feedback; keep release-runner changes separate until Qt packaging is ready.

## 2026-08-25 — Qt Git adapter and dependency boundaries

- Depending on a broad application crate from a thin UI adapter defeats much
  of the build-time reason for separating the frontend. The first real Qt Git
  build through `hunk-app` compiled the Codex, browser, AWS, image, and language
  graph and took 4 minutes 26 seconds. Moving the production snapshot/command
  API to `hunk-git` reduced the affected build to 2 minutes 54 seconds while
  keeping Qt types out of the core.
- A small type dependency can still hide a large compile graph. `hunk-git`
  needed only Hunk Domain config/path/state types, but unconditional database
  and Markdown dependencies pulled SQLite, Comrak, Hunk Language, and every
  Tree-sitter grammar. Optional default-on Domain features preserve existing
  consumers while letting the Git-only graph build in 1 minute 24 seconds on
  the same cache; its immediate rebuild took 0.41 seconds.
- QtBridge list models should be replaced once per owned snapshot on the Qt
  thread. Recycling QML `ListView` delegates with bounded cache buffers kept a
  1,500-file interaction test virtualized without per-row Rust QObjects.
- Cross-thread invocation does not require converting an owned snapshot to
  JSON. Store it in a small epoch-keyed Rust mailbox and queue only the epoch
  through QtBridge; the Qt thread can then take the already-built payload
  without parse work inside the frame budget.
- Switching repositories is also a refresh-cancellation boundary. Increment
  the adapter epoch before starting the new load so an older background result
  cannot overwrite the newly selected root.
- Installing the Qt SDK on Ubuntu does not satisfy the OpenGL linker entry that
  Qt Quick exposes as `-lGL` inside a Nix shell. Installing a host package is
  insufficient when Nix isolates its linker paths; include `libglvnd` and its
  library directory in the dev shell itself. QML-only smoke tests can otherwise
  conceal the missing native link dependency.
- Do not run headless Nix-built tests through the Linux host-graphics wrapper.
  Adding Ubuntu's full library directory to `LD_LIBRARY_PATH` can replace the
  matching Nix glibc and fail on private symbols. CI tests should unset the
  graphics runner and expose only the external Qt directory plus the
  Nix-provided runtime-library path.
- Fontconfig does not put FreeType itself on a path consumable by an externally
  installed Qt SDK. Qt GUI links `libfreetype` directly, so include `freetype`
  explicitly in the Nix-owned Linux runtime closure instead of relying on a
  transitive package relationship.
- A Qt migration should not leave OS credential behavior owned by the outgoing
  toolkit crate. Moving the existing keyring adapter into `hunk-forge` lets
  both frontends share one serialized platform implementation while QML sees
  only non-secret presentation state. Fake-client and fake-backend tests can
  then cover review/auth routing without opening an unattended keychain prompt.
- Removing GPUI from PR gates cuts a material desktop build, but it does not
  make the retained Rust graph free. In the final green Qt Git run, Windows
  finished in 2 minutes 47 seconds while Linux took 11 minutes 42 seconds; 6
  minutes 51 seconds belonged to the full non-GPUI workspace tests. Treat core
  dependency boundaries and test partitioning as separate CI work from the UI
  toolkit migration.
- Measure both exact-cache and dependency-invalidated CI runs. Moving forge
  dependencies into the Qt graph changed the lockfile ownership and the first
  follow-up run rose to 9 minutes 52 seconds on Windows and 29 minutes 23
  seconds on Linux, despite GPUI already being absent from the gate. Toolkit
  choice cannot compensate for a cold retained Rust dependency graph.
- Do not share native Zig dependency artifacts across heterogeneous runners
  without a CPU-aware cache key. `libghostty-vt-sys` intentionally lets Zig
  detect the native CPU; restoring that static library on a different Linux
  runner caused the test binary to exit with `SIGILL` before running tests. Use
  a fresh cache namespace too, so fallback lookup cannot select an older unsafe
  artifact before the first CPU-partitioned cache is saved.

## 2026-08-25 — Qt Diff projection and selective application features

- Reusing a headless module through a monolithic application crate can still
  defeat frontend build-time goals. Feature-gating `hunk-app` lets Qt consume
  the existing stable-row, binary-state, and patch-projection implementation
  without compiling unrelated Codex, browser, mobile, image, or AWS code.
- A selected-file diff model bounds background work and Qt model replacement
  for large repositories. Repository and file selection must both advance the
  adapter epoch; otherwise choosing a second file while the first patch loads
  can leave the new selection waiting on work that is no longer relevant.
- Keep the changed-file navigator distinct from the removed File Explorer. A
  list derived only from Git comparison state preserves review navigation
  without reintroducing arbitrary filesystem traversal or editing behavior.
- QML `ListView` recycling applies to diff rows as well as file lists. A
  5,000-row smoke case kept distant delegates uninstantiated, but that proves
  object-count behavior only; real 8 ms frame measurements still belong at the
  cutover performance gate.

## 2026-08-25 — Qt Diff review tools

- Theme-controlled syntax can cross QtBridge without a QObject per token. Build
  styled segments on the background projection worker, encode one semantic
  markup string per diff side, and resolve its small color vocabulary in QML.
- Renderer placeholders need the same injection discipline as HTML. Escaping
  the placeholder delimiter inside source text prevents literal code such as
  `@keyword@` from being replaced by a theme color during QML rendering.
- Search preprocessing belongs beside diff projection rather than in a QML
  keystroke handler or Qt model reset. Moving normalized row strings with the
  immutable payload keeps interactive search to allocation-free row scans apart
  from the normalized query and avoids repeatedly lowercasing source lines.
- Unified mode does not require another backend representation. Projecting a
  paired removal/addition as two fixed-height delegates from the existing
  side-by-side row preserves stable IDs, virtualization, and one loading path.
- When collected rows are consumed to build a secondary index before their
  eventual struct assignment, annotate the collection type at the projection
  boundary; the later payload field is no longer sufficient for Rust inference.

## 2026-08-25 — Qt Diff selection and navigation

- Diff row selection is presentation state. Keeping anchor/head indices in the
  QML workspace avoids bridge notifications for every pointer or arrow-key move;
  Rust only needs to provide semantic operations that depend on row content.
- Precompute each row's unified copy text beside syntax/search projection. A
  copy action can then concatenate the selected slice without cloning every
  intermediate string or teaching QML how removals, additions, and context map
  back to patch prefixes.
- Avoid building a temporary hunk-index vector for each F7 press. Forward and
  reverse iterator scans provide wrapped navigation with no per-action
  allocation and keep this small interaction well inside the UI-thread budget.
- Nested Flickables may take a mouse grab before a delegate `MouseArea` observes
  it. A `TapHandler` participates in pointer gesture arbitration, preserving row
  taps while allowing the gesture to become horizontal or vertical scrolling.
- QtTest keyboard synthesis reaches the focused shell reliably, while pointer
  synthesis in the offscreen nested-Flickable fixture does not. Test pointer
  range semantics directly, assert the delegate contains its `TapHandler`, and
  retain rendered/manual interaction checks rather than weakening production
  gesture handling for the harness.

## 2026-08-25 — Qt Diff comment anchors

- Do not make a pure UI projection activate persistence merely because its
  value types historically lived in a database module. Moving comment-side
  vocabulary and deterministic anchor hashing into an always-available domain
  module keeps the Qt Diff graph free of bundled SQLite until CRUD is required,
  while compatibility re-exports avoid churn in the legacy frontend.
- Build comment anchors before removing internal projection rows, then filter
  rows and anchors together. Filtering independently risks shifting every
  visible row-to-anchor association after the first hidden row.
- Comment context must be both radius-bounded and file-bounded. The inherited
  behavior uses two surrounding surface rows, includes same-file header/meta
  text when it is in range, and never allows an adjacent file to contaminate a
  persisted anchor hash.
- When a Qt list-model reset moves several aligned vectors together, name the
  replacement tuple. Besides satisfying type-complexity linting, the alias
  makes it explicit that rows, search text, copy text, and anchors form one
  atomic snapshot.

## 2026-08-25 — Qt Diff comment persistence boundary

- Repository and branch epochs are not sufficient for a selected-file UI. A
  background comment projection can remain in the same scope while its Diff
  anchors become stale, so every result must also carry the selected Diff epoch
  and be reprojected when that epoch changes.
- A selected-file renderer cannot safely resolve every unmatched old-path
  comment while a rename exists elsewhere in the working tree. Preserve the
  comment until the renamed Diff is loaded and fuzzy matching can run; delayed
  cleanup is safer than silently resolving a still-relevant review note.
- Keep SQLite, fuzzy matching, and reconciliation off the Qt thread, but also
  bound the final model reset. Reusing the legacy 64-item preview limit keeps
  UI-thread string cloning predictable while aggregate counts and background
  reconciliation continue to cover the full store.
- Sharing immutable comment anchors with `Arc` lets the Qt row model and worker
  projection consume one aligned snapshot. Per-visible-row queries should read
  only an index/count and must not clone the anchor's path, hunk, and context
  strings inside the frame budget.
- A QtBridge `#[qobject]` list-model module is a real nested Rust module and does
  not inherit outer imports. When an aligned model field changes from `Vec` to
  `Arc<Vec<_>>`, import `Arc` inside that generated module as well; otherwise the
  outer payload compiles far enough to obscure the missing model-local type.

## 2026-08-25 — Qt Diff comment interaction layer

- A root `Keys.BeforeItem` handler can silently steal arrow and clipboard input
  from a nested comment editor. Explicitly yield whenever that editor owns
  focus; keeping workspace navigation correct is not enough if native text
  editing becomes unusable.
- A contextual editor does not need to expand every recycling Diff delegate.
  One `Loader` can map the active instantiated row through the ListView scroll
  position and clamp a single overlay into the viewport, avoiding a text editor
  object in every visible row and avoiding delegate-height churn.
- A row badge can remain fixed at the visible edge of a horizontally scrolling
  Diff by offsetting it with the outer Flickable's `contentX`. Recompute its
  O(1) Rust count only when a coarse comment-version property changes so normal
  scrolling does not cross the bridge.
- Do not close an asynchronous comment composer immediately after invoking a
  void QtBridge command. Keep the draft until the shared state signal reports a
  successful status; on failure, restore editability and focus so a transient
  SQLite error does not erase review text.
- Model role names must also be valid required QML property names. Exposing
  `comment_id` instead of a generic `id` avoids colliding with QML's reserved
  object-identity syntax and keeps strongly required delegate roles available.
- Do not make an animated panel visible only when its animated width is already
  positive. On its first zero-to-open transition that can reserve layout space
  while its contents remain transparent. Include the requested open state in
  the visibility binding and keep the width check only to finish closing.

## 2026-08-25 — Toolkit-neutral Codex runtime discovery

- Packaging logic can look toolkit-neutral while still depending on the crate
  that compiled it. In particular, moving `env!("CARGO_PKG_NAME")` from
  `hunk-desktop` into `hunk-app` would silently search Linux package resources
  under `hunk-app`. Encode the supported legacy and Qt package directory names
  explicitly, while preferring the actual executable name, when ownership
  moves below both frontends.
- Runtime discovery is an application service, not UI state. Keeping override,
  development asset, app-bundle, Windows launcher, and Linux packager rules in
  `hunk-app::ai` prevents the Qt migration from growing a second resolver that
  would drift only after packaging on a different operating system.

## 2026-08-25 — Qt Codex worker and thread-catalog boundary

- A queued Qt callback is not a sufficient cancellation boundary. Pair the
  mailbox with a repository epoch, clear it before stopping the old worker, and
  make old callbacks harmless so a late snapshot cannot overwrite the newly
  selected repository.
- Snapshot coalescing must preserve semantic event ordering. Replacing only a
  consecutive snapshot tail bounds streaming pressure without moving a snapshot
  across status, error, tool-call, or disconnect events.
- Disabling a dynamic tool in a new frontend does not remove its response
  obligation. Browser requests already queued during a reset or arriving from a
  stale worker must receive a terminal unavailable response or the Codex worker
  can remain blocked waiting for a frontend that discarded the event.
- Worker cleanup must not turn Qt object destruction or repository selection
  into a request-timeout pause. Send the cooperative shutdown command on drop,
  then join the worker and event listener on a detached reaper thread instead of
  the Qt event thread.
- A worker normally emits `Fatal` immediately before its channel disconnects.
  Preserve the specific fatal message when the later disconnect callback runs;
  replacing it with a generic disconnect error discards the actionable cause.
- Cargo's workspace feature unification can hide the first-build cost of the
  exact shipping frontend graph. A full-workspace test and a focused Qt test may
  produce different `hunk-app`/Codex artifacts; validate and cache the production
  Qt feature combination directly instead of assuming the broader workspace
  artifact will be reusable.

## 2026-08-25 — Qt Codex catalog and streaming timeline

- A bounded Qt model does not make full reducer projection safe on the event
  thread. Project catalog and timeline payloads on the existing listener thread,
  then let the queued Qt callback apply only bounded strings and counters. Drop
  the unused full `AiSnapshot` there too; replacing a large `BTreeMap` snapshot
  on the Qt thread can consume a frame even when rendering is virtualized.
- Stable row IDs matter twice during streaming: they avoid per-token QObject
  creation and allow QtBridge to emit targeted row changes instead of resetting
  the whole `ListView`. Keep resets for actual membership/order changes only.
- Consecutive snapshot coalescing is still useful after projection, but drop the
  superseded payload after releasing the mailbox mutex. Deallocating hundreds of
  projected strings while holding the lock can delay the queued Qt drain.
- A fixed-size newest-thread window can hide the selected thread. Reserve one
  catalog slot for an older active thread so selection, header metadata, and the
  visible highlight cannot disagree when a repository has more than 200 threads.
- Lifecycle acknowledgements are not authoritative selected-thread snapshots.
  Relabeling the old timeline immediately on `ThreadStarted` briefly attributes
  it to the new thread; wait for the worker snapshot before changing selected
  metadata or timeline ownership.
- Reducer content, metadata summaries, paths, errors, and thread titles are all
  untrusted display strings. `Text.AutoText` can interpret markup, so AI QML
  delegates and shared confirmation copy must explicitly use plain-text mode.
- Tail following must respond to streamed row-height growth as well as row-count
  changes, but it must turn off as soon as the user begins scrolling. Otherwise
  a long response repeatedly pulls the viewport away from the history being read.
- Qt Quick `Text` and `TextEdit` do not expose an identical typography surface:
  `TextEdit` supports read-only selection but has no `lineHeight` property. Let
  QML lint the component after converting a message body instead of assuming a
  styling property transfers between the two types.

## 2026-08-25 — Qt Codex text composer and turn controls

- Successfully enqueueing an AI command is not the same as accepting the user's
  prompt. Keep the draft until a domain acknowledgement arrives; otherwise a
  worker failure after channel delivery silently loses user input.
- Stream sequence growth is not a safe steer acknowledgement because the
  already-running turn can produce output before the steer is processed. Use
  the explicit `SteerAccepted` event for steering, and use new turn identity or
  aggregate turn-count growth for an idle prompt.
- A workspace `Loader` destroys its QML child when the user changes tabs. Draft
  ownership must sit above that loader if drafts are expected to survive a
  Diff/Git round trip, while repository-root changes should replace the store so
  text cannot leak into another workspace.
- Pending state needs defense at both boundaries. Disabling buttons prevents
  accidental interaction, but Rust must still reject duplicate prompt,
  interrupt, select, create, and archive commands because QML methods and future
  callers can bypass pointer enabled state.
- Interrupt intent is scoped to a `(thread_id, turn_id)` pair. Clear its pending
  state only after an authoritative snapshot shows that exact turn is no longer
  active, or after a terminal worker error; a generic running-count change can
  refer to a different thread.
- Keep acceptance revisions monotonic for the backend lifetime. Resetting the
  revision during a reconnect can make a still-retained QML receipt look newly
  accepted even though no prompt acknowledgement occurred.

## 2026-08-25 — Qt Codex approvals and request-user-input

- A bounded question projection must fail closed when it cannot preserve the
  complete response contract. Truncating the ninth question or an option label
  and then submitting the visible subset can answer a different request than
  the user saw; expose the bounded preview but disable response instead.
- Presentation strings may be trimmed or ellipsized, but values that return to
  the protocol are semantic data. Preserve option labels exactly for supported
  requests and revalidate exact request/question IDs and answer membership in
  Rust immediately before enqueueing the worker command.
- Do not mark attention by retaining every pending thread ID. Intersect pending
  requests with the already bounded visible thread catalog on the listener
  thread, while explicitly preserving the selected thread, so the UI contract
  is complete for visible rows and remains bounded under pathological input.
- A failed request response is not an acknowledgement. Keep the in-memory QML
  answers while the authoritative request ID remains, clear only on request
  replacement/removal, and restore composer focus after that blocking request
  actually disappears.
- Secret input masking is separate from secret lifecycle. Password echo protects
  the visible surface; keeping answers out of Rust properties, models, logs,
  drafts, and persistence limits how long the plaintext survives elsewhere.
- A request panel owned by a workspace `Loader` cannot own the only copy of
  partial answers. Keep the memory-only answer map above that loader, key it by
  exact request ID, and prune it against only the bounded current requests that
  Qt can actually display so tab/thread navigation neither erases answers nor
  turns the UI projection into an unbounded pending-request clone.
- An attention marker is also a mutation guard. Disable Archive on attention
  rows and reject it again in Rust; otherwise archiving a thread can leave its
  still-pending approval or input request counted but unreachable.
- `Item.visible` reflects ancestor visibility, so it is not a stable source for
  a component's intrinsic size. Derive `implicitHeight` from the authoritative
  request state, and give a repeated `Column` explicit `childrenRect` geometry;
  otherwise an offscreen/loaded panel can report zero content height and never
  become scrollable.
- Custom keyboard controls need the whole interaction contract: accessible
  roles and names, a visible focus state, bounded/wrapped labels, and
  focus-driven scrolling. Pointer support plus `activeFocusOnTab` alone can move
  users into clipped controls with no visible or assistive indication.
- An exact Rust baseline has more owners than `rust-toolchain.toml`. Cargo MSRV
  metadata, inherited crate manifests, Nix's rust-overlay selection and lock,
  non-Nix CI setup actions, and patched vendor manifests must move together or
  different platforms silently compile with different toolchains.

## 2026-08-25 — Qt Codex thread lifecycle

- A lifecycle command needs an exact receipt, not a generic busy bit. Select,
  create, fork, and archive complete under different authoritative snapshot
  conditions; clearing any of them on an unrelated catalog update re-enables
  conflicting commands before the requested mutation is known to have landed.
- Reconnect success does not imply command success. When a non-idempotent
  command cannot be safely replayed, emit a terminal failure for that command so
  every frontend clears its pending receipt and asks the user to retry. A
  connection-restored status alone can leave controls disabled forever.
- Backend guards and visible enablement must describe the same state. If Rust
  rejects prompts, interrupts, approvals, or catalog changes during a lifecycle
  receipt, QML must disable those controls too; enabled silent no-ops are state
  bugs even when the backend remains safe.
- Blocking and resolving are different presentation states. An approval that
  arrives during a thread mutation should be disabled without claiming it is
  already being submitted, then receive focus when the mutation completes.
- Deferred QML focus work should belong to the component that owns the control.
  A zero-delay `Timer` is destroyed with its panel; a free `Qt.callLater`
  callback can outlive a workspace loader and invoke a method on a dead object.

## 2026-08-25 — Qt Codex queued follow-ups and recovery

- Queue lifecycle belongs beside the Rust worker boundary, not in QML. Keep
  per-thread FIFO ordering, delivery state, authoritative acknowledgement, and
  reconnect recovery in one repository-scoped owner; expose only bounded rows
  and counts to the frontend.
- Channel acceptance is not durable acknowledgement. A queued prompt must stay
  visible until the exact steer receipt or an exact later user-message record
  confirms it, and an unconfirmed runtime failure must return it to queued state.
- Count limits do not bound text memory. Charge every retained queued or
  recovered prompt against per-message and aggregate byte caps before it can
  cross into QML or trigger a large `TextEdit` layout on the UI thread.
- Queued messages are timeline rows, not a second footer view. Keeping them in
  the same recycling `ListView` preserves virtualization, while notified suffix
  insert/remove/update operations avoid invalidating every authoritative
  delegate and scroll position.
- Interrupt recovery is a draft transition. Preserve distinct existing text,
  deduplicate only normalized exact equality, and explicitly return focus after
  the interrupt lock clears; substring matching can discard instructions with
  opposite meaning.
- QtBridge `#[qobject]` property attributes are intentionally constrained. Keep
  derived queue properties as backend getters/signals and move cohesive command
  helpers into sibling modules when the 2,000-line backend boundary is reached.

## 2026-08-25 — Qt Codex thread bookmarks

- Persistent ordering belongs in the bounded Rust projection. Sorting only the
  200 visible rows means an old bookmark can never re-enter the catalog.
- Optimistic UI state must remain authoritative over already-projected worker
  events. Reapply the current bookmark set as a snapshot reaches the Qt thread,
  then request a fresh projection to reconcile catalog membership.
- Multiple load-modify-save features sharing one state file need a serialized
  write boundary. Otherwise a bookmark save and repository-selection save can
  each preserve its own field while silently reverting the other.
- Background persistence needs application-lifecycle ownership. Retain and join
  outstanding saves on shutdown, and replace the state file through a flushed
  temporary sibling so process exit cannot leave a truncated TOML.
- Failure recovery must restore the complete previous set and attempt to put
  that set back on disk. Flipping one ID is insufficient after rapid toggles
  because superseded optimistic states may never have been persisted.
- Icon-only actions still need stable semantic names. Keep the compact star in
  the dense rail, but expose “Bookmark thread” and “Remove bookmark” to
  accessibility clients instead of reading the glyph aloud.
- A single role change should not reset a bounded `QAbstractListModel`. Emit
  `dataChanged` for the row and a notified move when its sort position changes,
  preserving `ListView.reuseItems` and the 8 ms interaction budget.
- A row-wide selection `MouseArea` should end before contextual actions begin.
  Disjoint hit regions are more reliable than depending on sibling z-order to
  keep a star or archive button clickable.
- After a model move, Qt Quick Test can expose a pooled, non-visible delegate
  through `itemAtIndex()` under the offscreen runner. Assert authoritative model
  ordering and action signal wiring separately instead of sending synthetic
  pointer input to a pooled object.

## 2026-08-25 — Qt Codex session and context controls

- QtBridge derives a property's meta-type from getter signatures inside the
  annotated `#[qobject]` implementation. Keep thin property getters in that
  implementation and move only their calculations into sibling modules; a
  getter defined solely in another inherent `impl` is invisible to the macro.
- A shared QObject notify signal turns every dependent QML binding into work.
  Granular session notifications plus a `Loader` that unloads closed popup
  content prevent model scans and token-string allocations during unrelated
  streamed AI events.
- Runtime catalog strings need two bounds: a UTF-8-safe byte cap at the Rust
  projection boundary and a maximum/elided width in QML. Either bound alone can
  still allow service data to dictate header geometry or shaping cost.
- ComboBox activation is optimistic UI state. If Rust rejects a raced selection,
  restore the authoritative index with `Qt.binding` so the immediate value and
  its future backend updates both remain correct.
- Persisting one feature into a shared state file must load the latest state
  under the common writer lock and mutate only owned fields. Capturing and
  rewriting an earlier full state can silently undo bookmarks or repository
  selection even when every individual save is atomic.
- Qt Quick Test keyboard helpers target the currently focused item;
  `keyClick(Qt.Key_Down)` does not take a control argument. Force and verify
  active focus first, then send the key, or the test can spend its timeout
  waiting for a command that never ran.

## 2026-08-25 — Qt Codex prompt attachments

- A native file picker does not make filesystem work UI-safe. Bound and decode
  its URLs on the Qt side, but canonicalize paths and inspect files on retained
  Rust workers; removable and network-backed volumes can make even ordinary
  metadata calls exceed an 8 ms frame budget.
- Async locks must be scoped to the resource they protect. A backend-global
  attachment-pending bit makes one slow image freeze every thread; key pending
  epochs by thread and derive the exposed property for only the active thread.
- Model contents and separately notified count properties can drift. When a
  recovery slot resets a `QAbstractListModel` from inside an existing state
  callback, bind visibility and submission state to the view's notified count
  instead of recursively emitting a broad backend signal.
- An async interaction needs a completion focus path. Picker acceptance can
  schedule validation and immediately make the editor non-editable, so track
  the pending transition and restore focus on `true` to `false`, not only when
  the picker closes.
- Drag/drop should consume attachments as copies and only after the bounded
  bridge request is accepted. Never honor a source-proposed move operation for
  prompt images.
- Attachment lifecycle follows message acknowledgement, not button clicks.
  Preserve paths through direct-send receipts, queued delivery, edit recovery,
  interrupts, and reconnects; clear only on an authoritative acceptance or an
  explicit successful move into the Rust-owned queue.
- Cargo integration-test working directories are crate roots, so a relative
  `target/` temp fixture can silently target a nonexistent per-crate folder.
  Resolve the workspace target from `CARGO_MANIFEST_DIR` when tests must keep
  temporary data on the external-volume build cache.

## 2026-08-25 — Qt terminal surface

- Hiding a live terminal is not a geometry operation. Collapsing an attached
  terminal item to zero height can debounce a resize into the PTY's minimum
  grid and reflow the entire hidden session; unload the QML presentation and
  preserve the last valid grid instead.
- Coalescing after projection is too late for an 8 ms UI budget. Drain bursts
  on the terminal listener, project only the newest VT snapshot off the Qt
  thread, and limit the queued GUI callback to model patches and scalar state.
- Recycled terminal rows need direct required model roles. Depending on an
  implicit `model` object stops working once a delegate type declares required
  properties and can silently produce blank rows that component-only tests miss.
- A tabbed screen model needs explicit ownership of the rows currently lent to
  the Qt list model. Move rows into the active model without cloning hot markup,
  then return them to the previous tab when switching so inactive tabs remain
  immediately restorable.
- Bottom-panel focus restoration should retain the actual prior focus item.
  Focusing a workspace root on close does not restore the AI composer, Git
  commit editor, or any other nested editing control.

## 2026-08-25 — Qt embedded browser surface

- Keep large offscreen browser frames out of QML value bindings. A narrow
  `QQuickItem` can wrap a retained Rust `Arc<[u8]>` in a read-only `QImage`,
  release it through the image cleanup callback after upload, and reuse one
  QRhi texture without a second CPU pixel copy, base64, data-URL, or
  image-provider churn.
- Browser frame cadence and shell cadence are separate budgets. Retaining the
  CEF adapter's 60 fps publication limit does not prevent the surrounding Qt
  Quick application from rendering at 120 Hz; it prevents duplicate 8 ms
  uploads for a source that publishes at half that rate.
- Lazy initialization is also a build/test boundary. Keep CEF behind the Qt
  production feature and start it on first browser use so ordinary model and
  QML tests do not touch browser profiles, helper processes, or keychain-facing
  application state.
- Machine-specific cache policy belongs in the invocation environment, not a
  repository script. Portable preparation defaults can use `TMPDIR` or
  `XDG_CACHE_HOME`; this machine passes `/Volumes/hulk/dev/cache/cef-rs`
  explicitly through `HUNK_CEF_RS_DIR`.
- Qt Quick's `visible` value is affected by ancestor visibility. An offscreen
  shell integration test should assert the browser's authoritative open state
  and the control label, while the component-level rendered test owns direct
  surface visibility assertions.
- `Loader.Ready` only proves that a native browser item exists; it does not
  prove that CEF has delivered pixels. Expose a first-frame property from the
  native item and keep pointer, keyboard, and focus forwarding disabled until
  that property is true.
- An external CEF message pump and an external begin-frame request are separate
  responsibilities. Keep servicing the backend loop while it is active, but
  request pixels only for the visible session and only while its pane is shown;
  otherwise background tabs consume frame-copy and upload budget unnecessarily.
- Cache the exact rows exposed by a Qt list model, not a broader domain summary.
  Browser tab summaries include changing frame metadata; comparing those at
  presentation time needlessly reprojects tab rows for every browser frame even
  when the tab label, loading state, and identity did not change.
- Visibility includes the application window, not only the pane. A minimized
  window should keep servicing CEF's lightweight message loop but must stop
  requesting pixels that cannot be presented.
- Dialog focus restoration needs a visible fallback. If a browser approval
  hides the control captured before the dialog opened, resolution should hand
  focus to the already-ready browser surface instead of leaving the window
  without an active focus item.

## 2026-08-25 — Atomic Qt cutover

- Preserve the stable package and binary contract at cutover. Moving the
  reviewed Qt adapter into `crates/hunk-desktop` and keeping the
  `hunk_desktop` binary avoids simultaneous churn in launchers, runtime lookup,
  updater targets, and release artifact names.
- Regenerating the lockfile after removing the old frontend is the clearest
  dependency audit. Deleting GPUI, GPUI Component, and the orphaned editor
  crate removed 349 locked packages; searches of manifests and the lockfile
  then prove that remaining historical GPUI text is attribution or migration
  documentation rather than a production dependency.
- The removed File Explorer can leave non-visual residue. A shortcut field
  referenced only by config defaults and tests is still dead product surface,
  even when it no longer has a controller, so remove it together with stale
  README examples.
- A legacy performance script is not useful merely because its filename still
  sounds relevant. The old harness targeted a deleted GPUI integration test and
  had to be removed; retain the toolkit-neutral large-diff fixture and rebuild
  measured 120 Hz coverage around the Qt renderer during release hardening.
- File-size cleanup should follow ownership. Extracting macOS CEF sidecar
  staging into a focused module brought the backend below 2,000 lines without
  changing cross-platform browser logic or disguising the limit with formatting.
- A much smaller UI dependency graph does not make the retained application
  graph small. The first all-target cutover build still spent most of its time
  compiling the embedded Codex graph, so packaging and CI optimization must be
  measured independently from the frontend replacement.

## 2026-08-25 — Qt release deployment

- A successful Qt executable link is not a deployable desktop artifact. The
  release must also contain the platform plugin, transitive QML modules, image
  and TLS plugins, and every native dependency reachable from those modules.
- Use Qt's deployment tools where Qt supplies them: `macdeployqt` on macOS and
  `windeployqt` on Windows. Linux still needs an application-private runtime
  tree, `qt.conf`, recursive ELF dependency staging, and relative RPATHs.
- Validate every nested native binary, not only the main executable. CEF helper
  apps retained a Nix `libiconv` path after the primary Qt executable was clean;
  recursively relinking the helpers exposed and removed that host dependency.
- Qt SDKs can include optional database drivers linked to client libraries that
  are absent on user machines. Retain the self-contained driver the deployed
  QML graph needs and exclude ODBC, PostgreSQL, and Mimer plugins from Hunk.
- The aqt archive name for Qt's Linux Wayland module is
  `qtwaylandcompositor`, not `qtwayland`; confirm repository module names before
  encoding them into every release workflow.
- Nix-wrapped Darwin toolchains and prebuilt universal Qt frameworks are a poor
  match for deployment-time stripping. Pass `-no-strip` to `macdeployqt`, then
  perform dependency validation and code signing explicitly.
- A self-hosted runner's OS can be newer than GitHub's Python toolcache.
  `install-qt-action` delegated to `actions/setup-python` and failed before Qt
  installation on Ubuntu 25.10. Run Hunk's exact aqt installer inside Nix on
  Linux instead, with Nix owning Python/virtualenv and the runner cache owning
  the downloaded Qt SDK.
