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
