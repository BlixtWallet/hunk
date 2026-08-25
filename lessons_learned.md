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
