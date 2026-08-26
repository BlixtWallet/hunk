# Hunk

A Qt Quick desktop app for fast git diff viewing and Codex orchestration. Hunk keeps its core behavior in Rust and its cross-platform desktop UI in QML.

- Production Git behavior should live in `crates/hunk-git`, using `gix` first and narrow `git2` fallbacks only when necessary. Do not shell out to the Git CLI from app code.
- When fixing a bug or adding a new feature, always switch to plan mode and plan first before writing code.
- After making an implementation plan, keep it updated as you work. When you finish a step or the status changes, update the plan before continuing.
- Simplicity first, make every change as simple as possible.
- Make sure code is scalable.
- Don't make files over 2000 lines long.
- When working with the frontend, always use colors from `crates/hunk-desktop/src/qml/Hunk/Theme.qml`.
- Tests always in crate-level `tests` directories (for example `crates/hunk-git/tests`)
- Make sure workspace clippy passes
- Make sure workspace builds pass
- Use Cargo's default `target/` directory instead of overriding `CARGO_TARGET_DIR`.
- For CARGO_HOME check this path /Volumes/hulk/dev/cache/cargo or the default CARGO_HOME path for rust, nowhere else on the machine.
- Do not run clippy and tests over and over again, run them after you finished your work and make sure they pass at the end. Just once is enough.
- When building on MacOS use nix shell `nix develop -c ...`.
- When asked to update Codex, follow `docs/AI_CODEX_UPGRADE_WORKFLOW.md`. Hunk now consumes a fork branch, so upgrades require rebasing the fork onto the target upstream `rust-v...` tag, reapplying Hunk patches there, pushing the fork branch, and then refreshing Hunk's lockfile against that fork commit.
- Refresh the bundled runtimes after a Codex upgrade. The runtime download scripts pull official `openai/codex` release assets that match the locked Codex crate version unless explicitly overridden with `HUNK_CODEX_RUNTIME_REPO`.
- Update `docs/AI_CODEX_SPEC.md` with the new upstream tag/SHA and current fork commit, search for stale Codex version strings in docs, and expect small protocol/API fixes in `hunk-desktop` or `hunk-codex` after the bump.
- Qt Quick documentation: https://doc.qt.io/qt-6/qtquick-index.html
- QML language documentation: https://doc.qt.io/qt-6/qtqml-index.html
- Lessons learned note for terminal focus restoration: `lessons_learned.md`
- Use frontend-skill whenever you're doing designs
- Frames must take no more than 8ms (120fps)

Important paths:

- `crates/hunk-terminal`: terminal integration, shell/session support, and terminal-facing workspace surfaces.
- `crates/hunk-text`: headless rope-backed text buffer, positions/ranges, transactions, and undo/redo primitives.
- `crates/hunk-language`: Tree-sitter language registry, queries, syntax highlighting, folding, preview highlighting, and language-intelligence seams.
- `crates/hunk-domain`: shared config/state types, markdown preview, and SQLite comment storage/migrations.
- `crates/hunk-git`: shared Git read/write behavior; keep production Git logic here instead of app crates.
- `crates/hunk-forge`: forge integration logic, remote/review workflows, and hosting-service coordination.
- `crates/hunk-updater`: update/download/install logic for desktop releases.
- `crates/hunk-browser`: embedded browser runtime state, CEF backend integration, offscreen frames, input routing, snapshots, console logs, and safety checks.
- `crates/hunk-browser-helper`: CEF subprocess helper binary used by the embedded browser runtime.
- `crates/hunk-sleep-inhibitor`: cross-platform idle sleep prevention used while long-running AI turns are active.
- `crates/hunk-desktop`: Qt desktop binary, QtBridge objects/models, QML UI, app lifecycle, and native Qt rendering adapters.
- `crates/hunk-codex`: embedded Codex app-server integration, thread service, protocol boundary, and AI reducer/state logic.
- `crates/hunk-desktop/src/qml/Hunk`: Qt Quick UI components; `Theme.qml` owns app colors.

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:

- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:

- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:

- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:

- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:

```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```
