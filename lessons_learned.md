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
