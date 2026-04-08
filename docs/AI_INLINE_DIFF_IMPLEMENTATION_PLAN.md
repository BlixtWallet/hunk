# AI Right-Side Diff Viewer Implementation Plan

## Status

- Implemented
- Owner: Hunk
- Last Updated: 2026-04-08
- Follow-on to: `docs/AI_TIMELINE_WORKSPACE_SURFACE_PLAN.md`

## Summary

Hunk should let users inspect diffs inside the AI workspace without leaving the AI tab, and the AI tab must keep those diffs useful even after the work has been committed and pushed.

The target UX is:

- clicking a diff or file-change block in the AI timeline opens a right-side diff viewer inside the AI tab
- the AI timeline and composer stay on the left
- the right-side pane renders a unified stacked diff with Review-quality syntax highlighting
- the pane supports two AI-only source modes:
  - `AI Diff`
  - `Working Tree`
- `AI Diff` is the default mode when the pane is opened from the AI timeline and renders historical turn patch data for the selected block
- `Working Tree` is an explicit alternate mode that shows the live current worktree diff for the selected thread/workspace
- `Open in Review` remains available for the full side-by-side Review experience
- the AI workspace toolbar gets a diff-viewer button beside the existing “open in editor” control, and `Cmd/Ctrl+D` opens the viewer in `Working Tree` mode

The core product rule is:

- Review stays live and current
- AI stays historical by default, but can switch to live working-tree diff on demand

That split solves the current blank-diff problem without changing Review semantics.

## Product Definition

### V1 behavior

- Clicking an AI diff-summary row opens the right-side in-tab diff pane instead of switching tabs.
- The pane is resizable and closable.
- The pane defaults to `AI Diff` mode when opened from AI timeline diff rows.
- `AI Diff` renders a historical turn diff for the selected row even if the worktree is now clean.
- `Working Tree` renders the live current diff for the selected thread/workspace.
- Users can switch between `AI Diff` and `Working Tree` from the right-side pane header.
- The AI toolbar includes a diff-viewer icon button that opens the right-side pane in `Working Tree` mode for the current thread/workspace.
- `Cmd/Ctrl+D` opens the AI diff viewer in `Working Tree` mode while the AI workspace is focused.
- `Open in Review` still switches to the Review tab, which continues to show the live side-by-side compare.

### V1 non-goals

- No historical diff mode inside the Review tab.
- No inline comments inside the AI pane.
- No AI-pane side-by-side diff layout.
- No search UI inside the AI pane.
- No true cumulative thread diff in V1.
- No attempt to keep Review and AI using one shared viewport or one shared session instance.

## Why The Old Model Is Not Enough

The current AI pane reuses the Review compare session. That produces a live compare:

- branch/base on the left
- current workspace target on the right

That is correct for Review, but it breaks the AI workspace once a turn has already been committed or pushed:

- the timeline block still advertises code changes
- clicking it opens a blank live diff because the working tree no longer contains the changes

This is acceptable in Review, where “current repo state” is the point. It is not acceptable in AI, where the user is often trying to inspect what happened in a specific turn or batch after the fact.

## Current Code Findings

The codebase already contains the critical primitives we need:

- `crates/hunk-codex/src/state.rs` stores raw historical turn patch text in `turn_diffs`
- `crates/hunk-desktop/src/app/ai_workspace_timeline_projection.rs` already summarizes:
  - turn-level unified patch text
  - file-change item summaries
  - grouped file-change summaries
- `crates/hunk-desktop/src/app/review_workspace_session.rs` already builds the high-quality diff session from `CompareSnapshot`
- `crates/hunk-desktop/src/app/ai_inline_review.rs` and `render/ai_inline_review_surface.rs` already render a unified stacked pane from `ReviewWorkspaceSession`

The missing piece is source ownership:

- the AI pane currently reads from the shared `review_workspace_session`
- the AI pane needs its own session source so it can switch between:
  - historical AI snapshot
  - live working-tree snapshot

## Chosen Architecture

### 1. Keep Review semantics unchanged

The Review tab remains the canonical live compare surface:

- compare source resolution stays in `controller/review_compare.rs`
- Git diff loading stays in `crates/hunk-git`
- Review continues to show the current workspace-target compare

### 2. Make the AI pane source-aware

The AI pane needs an explicit source mode and session lifetime separate from Review.

Recommended mode enum:

- `AiInlineDiffSourceMode::Historical`
- `AiInlineDiffSourceMode::WorkingTree`

Recommended selected-source enum:

- `Turn { turn_key: String }`
- `Item { item_key: String }`
- `Group { group_id: String }`

Recommended AI session ownership:

- add `ai_inline_review_session: Option<ReviewWorkspaceSession>`
- add AI-only loading/error/session metadata
- keep `review_workspace_session` dedicated to Review

This avoids cross-contamination between:

- Review live compare state
- AI historical diff state
- AI working-tree mode

### 3. Reuse the Review session pipeline, not the Review tab state

We should continue reusing:

- `CompareSnapshot`
- `ReviewWorkspaceSession::from_compare_snapshot(...)`
- syntax highlighting caches
- diff row segment caches
- unified AI pane renderer

We should not reuse:

- `review_workspace_session` as the only loaded session
- Review scroll state
- Review selected compare pair as the only source of truth for the AI pane

### 4. Historical diff should be the default in AI

Historical AI diffs should be built from persisted turn patch artifacts:

- `TurnDiff` rows use `turn_diffs[turn_key]`
- `fileChange` rows resolve to their parent turn and use that turn’s persisted unified patch text
- `file_change_batch` group rows resolve to their parent turn and use that turn’s persisted unified patch text

This is the default AI experience because it stays stable after commits and pushes, and it avoids depending on transient live worktree state.

### 5. Working-tree diff is an explicit alternate view

The AI pane also needs a live worktree mode for users who want to compare the current repo state without switching tabs.

That mode should:

- reuse the existing Review compare-selection logic for the selected thread/workspace
- build a live `CompareSnapshot` through the existing Git path
- stay in the AI pane

## Historical Diff Model

### V1 scope: row-anchored historical diffs

V1 should support historical diffs for:

- `TurnDiff` rows
- single `fileChange` item rows
- grouped `file_change_batch` rows

This matches the user-visible AI timeline structure and the currently clickable rows. In V1, the clicked row selects the historical anchor, but the rendered historical snapshot is the persisted turn diff for that row’s turn.

### V1 does not implement cumulative thread-level diffs

True thread-level cumulative diffs are not a good V1 target because we do not currently persist a canonical:

- thread start snapshot
- turn-by-turn workspace base snapshot
- cumulative patch application history with strong ordering guarantees for a full thread replay

We can add a thread-level feature later once we decide what a “thread diff” should mean exactly.

## Snapshot Construction Strategy

### Historical turn diff snapshot

Input:

- unified patch text from `AiState.turn_diffs[turn_key]`

Output:

- synthetic `CompareSnapshot`

Construction approach:

- split the unified patch into per-file patches
- derive changed file paths and statuses from patch headers
- compute line stats per file from the patch text
- populate:
  - `files`
  - `file_line_stats`
  - `overall_line_stats`
  - `patches_by_path`

Then:

- build `ReviewWorkspaceSession::from_compare_snapshot(...)`
- render in the AI pane using the existing unified AI surface

### Historical file-change and grouped-batch snapshots

Input:

- clicked `fileChange` row or grouped `file_change_batch` row

Output:

- synthetic `CompareSnapshot`

Construction approach:

- resolve the clicked row back to its parent turn
- load the persisted unified patch text from `AiState.turn_diffs[turn_key]`
- parse that unified patch exactly once into per-file snapshot entries

This keeps AI historical rendering stable after commits/pushes and avoids reconstructing patches from per-item payloads that may not remain canonical over time.

### Working-tree snapshot

Use the existing Review compare-selection pipeline:

- selected thread workspace root
- worktree/current-branch default pair
- existing Git compare loading in `crates/hunk-git`

This mode should not introduce a second Git diff implementation.

## UI And Interaction Plan

### Right-side pane header

The AI diff pane header should include:

- title, for example `Diff`
- source chip or subtitle:
  - `AI Diff`
  - `Working Tree`
- mode switch control
- `Open in Review`
- `Close`

Recommended V1 control:

- a compact segmented control or paired icon/text buttons:
  - `AI Diff`
  - `Working Tree`

### AI toolbar button

In AI mode, add a diff-viewer button in the top-right toolbar cluster:

- place it immediately to the left of the existing “open workspace in preferred editor” action
- use an icon-only presentation
- tooltip should explain:
  - `Open working tree diff (Cmd/Ctrl+D)`

Behavior:

- if a current AI thread/workspace can resolve to a valid diff source, open the pane in `Working Tree` mode
- if the pane is already open in `AI Diff`, switch it in place to `Working Tree`
- if the pane is already open in `Working Tree`, keep it open
- if there is no eligible AI diff source for the current thread, keep the button disabled

### Keyboard shortcut

Add a new action, for example:

- `OpenAiWorkingTreeDiffViewer`

Bind it in the AI workspace to:

- `cmd-d` on macOS
- `ctrl-d` elsewhere

Expected behavior:

- only active in `AiWorkspace`
- opens the right-side pane in `Working Tree` mode
- reuses the last selected/open row for the current thread when possible

### AI timeline block behavior

Clicking a diff-summary block in the AI timeline should:

- open the AI pane if it is closed
- select the clicked historical diff source
- switch the pane into `AI Diff` mode

The pane header should offer `Working Tree` as a switch from that point.

### Switching modes

When the user switches between `AI Diff` and `Working Tree`:

- keep the pane open
- keep the selected AI row/thread anchor
- rebuild only the AI pane session source
- do not switch tabs

## Detailed Engineering To-Do List

### Task 1. Update the documented product contract

Goal:

- make the AI pane’s source semantics explicit before code changes continue

To-do:

- document the AI-vs-Working-Tree mode split
- document toolbar button placement and shortcut behavior
- document that Review stays live/current

Expected file touches:

- `docs/AI_INLINE_DIFF_IMPLEMENTATION_PLAN.md`

### Task 2. Add AI diff-viewer source state

Goal:

- give the AI pane enough state to know what to render and how

To-do:

- add AI-only source mode enum
- add AI-only selected source descriptor enum
- add AI-only loaded session/error/loading state
- preserve existing thread-scoped selected row state

Expected file touches:

- `crates/hunk-desktop/src/app.rs`
- `crates/hunk-desktop/src/app/controller/core_bootstrap.rs`
- `crates/hunk-desktop/src/app/controller/ai/core_workspace.rs`

### Task 3. Build historical AI snapshots from stored diff artifacts

Goal:

- convert AI row data into a synthetic `CompareSnapshot`

To-do:

- add a dedicated builder module, for example:
  - `crates/hunk-desktop/src/app/ai_inline_review_snapshot.rs`
- implement:
  - turn diff patch parsing
  - row-to-turn resolution for historical AI rows
- reuse existing diff/session parsing and line-stat computation where possible

Expected file touches:

- New: `crates/hunk-desktop/src/app/ai_inline_review_snapshot.rs`
- `crates/hunk-desktop/src/app/ai_workspace_timeline_projection.rs` if helper extraction is needed

### Task 4. Load AI historical sessions without touching Review state

Goal:

- make the AI pane render from its own session source

To-do:

- add AI pane session refresh logic for:
  - `Historical`
  - `WorkingTree`
- in historical mode:
  - build synthetic snapshot from AI row source
  - create `ReviewWorkspaceSession` from that snapshot
- in working-tree mode:
  - reuse current Review compare-selection logic
  - build a live snapshot through the Git path
- store the resulting session in AI-only state

Expected file touches:

- `crates/hunk-desktop/src/app/controller/ai/workspace_surface.rs`
- `crates/hunk-desktop/src/app/controller/ai/core_timeline.rs`
- possibly a new AI-specific controller file if this logic grows

### Task 5. Decouple AI rendering from `review_workspace_session`

Goal:

- stop the AI pane from reading the Review tab’s live session directly

To-do:

- update `render/ai_inline_review_surface.rs` to read from `ai_inline_review_session`
- keep geometry, scroll, and syntax cache behavior the same
- preserve virtualization and sticky headers

Expected file touches:

- `crates/hunk-desktop/src/app/render/ai_inline_review_surface.rs`

### Task 6. Add AI pane mode switching UI

Goal:

- let users switch between historical and working-tree diffs without leaving AI

To-do:

- add `AI Diff` and `Working Tree` controls to the AI pane header
- default to `AI Diff` when the pane opens from a timeline block
- allow `Working Tree` mode to rebuild the AI pane session in place

Expected file touches:

- `crates/hunk-desktop/src/app/render/ai_workspace_sections.rs`
- AI controller files for action handlers

### Task 7. Add toolbar button and shortcut

Goal:

- let users open the working-tree diff viewer from the AI toolbar and keyboard

To-do:

- add a new global action:
  - `OpenAiWorkingTreeDiffViewer`
- add key bindings:
  - `cmd-d`
  - `ctrl-d`
- render the toolbar icon button in AI mode beside the editor-open action
- disable it when there is no valid AI diff source

Expected file touches:

- `crates/hunk-desktop/src/app.rs`
- `crates/hunk-desktop/src/app/render/toolbar.rs`
- AI controller action handler files
- config/keybinding files if shortcuts are centrally declared there

### Task 8. Refine open/close lifecycle behavior

Goal:

- make the pane feel predictable across thread switches and stale rows

To-do:

- closing the pane should preserve the last selected source per thread
- toolbar reopen should restore the last selected row anchor and open in `Working Tree`
- switching threads should update availability and preserve per-thread memory
- if a selected row disappears, clear only that thread’s invalid source
- if historical data is missing, show a clear empty/error state instead of a blank pane

Expected file touches:

- `crates/hunk-desktop/src/app/controller/ai/core_workspace.rs`
- `crates/hunk-desktop/src/app/controller/ai/core_timeline.rs`
- `crates/hunk-desktop/src/app/controller/ai/workspace_surface.rs`

### Task 9. Testing

Goal:

- cover the new source semantics and avoid regressions

To-do:

- add snapshot-construction tests for:
  - turn diffs
- add controller tests for:
  - default AI pane mode is `AI Diff`
  - switching to `Working Tree` rebuilds correctly
  - committed historical turns still render
  - toolbar/shortcut open working-tree mode for the current thread/workspace
- keep existing Review behavior tests intact

Expected file touches:

- `crates/hunk-desktop/src/app/controller/ai/tests/runtime_path_and_session.rs`
- `crates/hunk-desktop/src/app/controller/ai/tests/timeline.rs`
- New AI historical snapshot tests if needed

## Risks

### Risk 1. Repeated edits to the same file inside one grouped batch

If a grouped file-change batch contains multiple diffs for the same file, composing them into one patch may be tricky.

Mitigation:

- isolate patch-composition logic
- add tests
- allow a conservative V1 fallback if necessary

### Risk 2. Session duplication cost

Separate AI and Review sessions can increase memory and loading work.

Mitigation:

- only load the AI session when the pane is open
- keep virtualization and segment caching unchanged
- reuse the existing `ReviewWorkspaceSession` implementation rather than duplicating renderer logic

### Risk 3. Confusing source semantics

Users may not understand whether they are seeing historical or live data.

Mitigation:

- explicit mode labels
- toolbar tooltip
- clear pane subtitle

## Open Questions

- For grouped `file_change_batch` rows, should repeated edits to the same file be rendered as:
  - one composed file patch
  - multiple file sections in sequence
- When the toolbar button opens the pane and no diff row has ever been selected in the thread, should it:
  - pick the newest available diff row automatically
  - stay disabled until the user clicks a timeline block

Recommended V1 answer:

- pick the newest available diff row automatically so the toolbar button is useful immediately

## Recommended Delivery Order

1. Land the doc and product contract.
2. Add AI pane source state and AI-only session ownership.
3. Implement historical snapshot builders.
4. Repoint AI pane rendering to the AI-only session.
5. Add source-mode switcher in the pane header.
6. Add toolbar button and `Cmd/Ctrl+D`.
7. Add regression tests.
8. Run plain `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test -p hunk-desktop`.
