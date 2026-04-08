# AI Right-Side In-Tab Diff Implementation Plan

## Status

- Proposed
- Owner: Hunk
- Last Updated: 2026-04-07
- Follow-on to: `docs/AI_TIMELINE_WORKSPACE_SURFACE_PLAN.md`

## Summary

Hunk should let users inspect AI-generated file changes without leaving the AI workspace.

The target UX is:

- clicking a diff or file-change summary in the AI timeline opens a diff pane on the right inside the AI tab
- the AI timeline and composer stay on the left
- the right-side pane renders a unified up-and-down diff, not the Review tab's side-by-side layout
- the right-side pane header includes an `Open in Review` action for the full Review experience
- syntax highlighting and change emphasis should match Review quality by reusing the existing compare/session/highlighting pipeline

The important architectural constraint is that we should reuse the existing review compare loading and syntax/highlight machinery, but we should not reuse the side-by-side Review surface state as-is. The AI right-side pane needs its own renderer and surface state because it is width-constrained and intentionally unified rather than split.

## Product Definition

### V1 behavior

- Clicking an AI diff-summary row opens the right-side in-tab diff pane instead of switching tabs.
- The right pane is resizable and closable.
- The pane renders the selected thread's compare snapshot in a unified stacked layout.
- The pane is read-only.
- `Open in Review` switches to the Review tab for side-by-side diffing, search, comments, and the rest of the existing review workflow.

### V1 non-goals

- No inline comments inside the AI right-side pane.
- No Review-tab search UI inside the AI pane.
- No attempt to make the AI pane side-by-side.
- No new diff-loading pipeline sourced only from timeline summary text.
- No turn-scoped historical diff reconstruction unless the existing thread compare model proves insufficient.

## Current Code Findings

The codebase already contains most of the scaffolding for this feature:

- `crates/hunk-desktop/src/app/render/ai_workspace_sections.rs` already has a dormant right-side split and pane chrome for in-tab review.
- `crates/hunk-desktop/src/app/controller/ai/workspace_surface.rs` already has thread-scoped right-pane review selection state and an unused `ai_open_inline_review_for_row(...)`.
- `crates/hunk-desktop/src/app/ai_workspace_surface.rs` still routes diff-summary clicks to `ai_open_review_tab(...)`, so the user is pushed into the Review tab instead of opening the inline pane.
- `crates/hunk-desktop/src/app/review_workspace_session.rs` already builds the compare-session model, projected display rows, syntax spans, changed ranges, sticky headers, and viewport snapshots we want to reuse.
- `crates/hunk-desktop/src/app/controller/review_compare.rs` already owns compare-source resolution and `ReviewWorkspaceSession::from_compare_snapshot(...)`.
- `crates/hunk-desktop/src/app/render/review_workspace_code_row.rs` already has the text-run construction logic that maps syntax spans and changed ranges into painted code rows.

The main missing piece is not data loading. The main missing piece is a dedicated AI-side surface that consumes the same review session data but renders it as a unified stacked diff in the right-side pane.

## Constraints And Assumptions

- Keep production diff loading and Git behavior on the existing review/hunk-git path. Do not add a second Git diff implementation just for AI.
- Preserve the 8ms frame budget target by staying virtualized and paint-driven.
- Reuse existing syntax/highlight output so the AI pane is not a lower-fidelity preview.
- Keep files under 1000 lines by splitting new inline-diff code into dedicated modules instead of growing `review_workspace_session.rs` or `ai_workspace_sections.rs` indefinitely.
- V1 assumes the AI right-side diff pane shows the same compare snapshot the Review tab would show for the selected thread/workspace. That preserves current semantics and avoids inventing turn-specific patch reconstruction in the first pass.

## Chosen Architecture

### 1. Reuse compare loading and review session data

Keep these as the canonical diff source:

- compare-source resolution in `controller/review_compare.rs`
- `CompareSnapshot`
- `ReviewWorkspaceSession`
- existing display-row generation and syntax span generation
- existing `DiffRowSegmentCache` and syntax/change highlighting pipeline

This is the highest-leverage reuse point and keeps syntax/highlight quality aligned with Review.

### 2. Split surface state from diff data

The current `ReviewWorkspaceSurfaceState` is tuned for side-by-side review and mixes together:

- scroll handle
- split ratio
- row selection
- cached review viewport snapshot
- display owner/editor handles

That coupling is fine for the Review tab, but it is the wrong shape for the AI right-side pane. The AI pane needs a separate surface state because:

- it has different viewport geometry
- it should not share side-by-side split settings
- it should not share Review-tab scroll/selection state
- it will render unified rows rather than split columns

The plan is:

- keep one shared diff/session owner for producing display rows and syntax spans
- add a separate AI right-pane diff surface state for scroll, cached pane snapshot, and pane selection

### 3. Add a unified right-pane diff viewport model

The AI right-side pane should not render the Review tab's left/right viewport directly. Instead it should build an AI-specific viewport snapshot from the already-projected review display rows.

That model should contain unified row variants such as:

- file header
- hunk header
- context row
- removed row
- added row
- meta row

This lets the AI pane stay readable in a narrower column while still using the exact same syntax and changed-range data the Review path already computes.

## Detailed Implementation Plan

### Phase 1. Wire the existing right-side pane shell to the right interaction

Goal: make AI diff-summary clicks open the existing right-side split-pane shell instead of switching tabs.

Changes:

- Replace the current diff-summary click action in `ai_workspace_surface.rs`.
- Route diff-summary clicks to `ai_open_inline_review_for_row(...)`.
- Reserve `ai_open_review_tab(...)` for the explicit header button inside the right-side pane.
- Keep `ai_inline_review_selected_row_id_by_thread` as the thread-scoped open/close state for V1.

Recommended cleanup:

- Replace the generic `open_review_tab: bool` on `AiWorkspaceBlock` with a more explicit primary action enum, for example:
  - `None`
  - `OpenInlineReviewPane`
- This avoids the current ambiguity where the only action available on a diff-summary row is a tab switch.

Expected file touches:

- `crates/hunk-desktop/src/app/ai_workspace_session.rs`
- `crates/hunk-desktop/src/app/ai_workspace_render.rs`
- `crates/hunk-desktop/src/app/ai_workspace_surface.rs`
- `crates/hunk-desktop/src/app/controller/ai/workspace_surface.rs`
- `crates/hunk-desktop/src/app/render/ai_workspace_sections.rs`

### Phase 2. Factor shared diff-display ownership away from Review-tab-only surface state

Goal: let both the Review tab and AI right-side pane consume the same compare session and display rows without sharing scroll/layout state.

Changes:

- Lift the display-row producer currently hidden behind `ReviewWorkspaceSurfaceOwner` into a shared owner/coordinator object.
- Keep one shared source of:
  - left/right workspace editors
  - `build_display_rows_for_viewport(...)`
  - syntax spans by display row
- Leave `ReviewWorkspaceSurfaceState` responsible only for Review-tab viewport state.
- Introduce an `AiInlineReviewSurfaceState` for:
  - scroll handle
  - cached inline viewport snapshot
  - inline line-number width state
  - inline selected row/path if needed

Why this matters:

- It preserves reuse where it is valuable.
- It prevents AI and Review from fighting over one scroll position and one viewport cache.
- It avoids forcing unified rendering into a side-by-side state object.

Expected file touches:

- `crates/hunk-desktop/src/app.rs`
- `crates/hunk-desktop/src/app/controller/review_compare.rs`
- `crates/hunk-desktop/src/app/controller/core_runtime.rs`
- `crates/hunk-desktop/src/app/controller/ai/core_workspace.rs`

### Phase 3. Build an AI-specific unified viewport snapshot for the right-side pane

Goal: transform the existing review display-row output into stacked right-pane rows without recomputing syntax.

Recommended structure:

- Add a new module such as `crates/hunk-desktop/src/app/ai_inline_review_session.rs`.
- Keep `review_workspace_session.rs` focused on shared compare/session logic.
- Build the AI right-pane viewport from:
  - `ReviewWorkspaceSession`
  - shared display rows
  - shared syntax spans
  - shared changed-range caches

Projection rules:

- Preserve file headers and hunk headers.
- For unchanged/context lines, emit one unified row that shows both old and new line numbers.
- For removed-only rows, emit one removed row.
- For added-only rows, emit one added row.
- For modified pairs, emit two visual rows in order:
  - removed
  - added
- Preserve wrapped display-row order by projecting from `ReviewWorkspaceDisplayRowEntry` rather than reconstructing from raw patch text.

Important detail:

- The AI right-side pane should consume the existing display-row/syntax-span output, not raw diff-summary text from the AI timeline row.
- That is what keeps syntax highlighting, wrapping, and changed-range emphasis aligned with the Review tab.

Expected file touches:

- New: `crates/hunk-desktop/src/app/ai_inline_review_session.rs`
- `crates/hunk-desktop/src/app/review_workspace_session.rs`
- `crates/hunk-desktop/src/app/controller/core_runtime.rs`

### Phase 4. Extract shared text-run building for syntax and changed-range painting

Goal: avoid duplicating the logic that turns syntax spans and changed ranges into painted runs.

Changes:

- Extract the reusable portion of `build_review_workspace_text_runs(...)` from `render/review_workspace_code_row.rs` into a small shared helper.
- Reuse the same theme-token mapping and changed-range background treatment in both renderers.
- Keep Review-side rendering side-by-side and AI-side rendering unified, but both should draw from the same text styling helper.

This gives the AI right-side pane:

- the same syntax token palette
- the same intraline changed highlighting quality
- the same future color fixes whenever Review changes

Expected file touches:

- `crates/hunk-desktop/src/app/render/review_workspace_code_row.rs`
- New helper module under `crates/hunk-desktop/src/app/render/`

### Phase 5. Render the AI right-side diff pane

Goal: replace the placeholder Review-surface embedding with a dedicated unified AI right-side diff surface.

Recommended structure:

- Add a renderer such as `crates/hunk-desktop/src/app/render/ai_inline_review_surface.rs`.
- Keep the existing `h_resizable(...)` split in `ai_workspace_sections.rs`.
- Use a dedicated scroll handle for the right-side diff body.

Pane header should include:

- title, for example `Diff`
- short compare/source subtitle
- `Open in Review`
- `Close`

Pane body should include:

- virtualized painted viewport
- sticky file headers
- compact unified gutters
- syntax-highlighted code rows
- added/removed/context row backgrounds aligned with Review colors
- loading, empty, and error states derived from the shared compare/session state

V1 intentionally omits:

- search bar
- comment affordances
- file-tree navigation inside the pane

Those remain reasons to jump to Review.

Expected file touches:

- New: `crates/hunk-desktop/src/app/render/ai_inline_review_surface.rs`
- `crates/hunk-desktop/src/app/render/ai_workspace_sections.rs`
- `crates/hunk-desktop/src/app/theme.rs` if any new diff colors are required
- `crates/hunk-desktop/src/app/workspace_surface.rs` if shared hit-testing helpers are worth extracting

### Phase 6. Hook up navigation and lifecycle rules

Goal: make the feature feel predictable and not fragile.

Rules:

- Opening a diff-summary row syncs compare selection to the selected thread/workspace, but keeps `WorkspaceViewMode::Ai`.
- `Open in Review` reuses the existing `ai_open_review_tab(...)` flow.
- Switching threads closes the pane if the selected row is no longer valid.
- If compare loading starts again, the inline pane should show loading rather than stale content.
- If the selected diff row disappears from the thread, clear the right-pane diff selection.

Nice-to-have but not required for V1:

- persist the right-pane width
- persist right-pane selected file/row per thread

### Phase 7. Testing and verification

Add targeted tests before running the full workspace verification pass.

Unit and render tests:

- `ai_workspace_render_tests.rs`
  - diff-summary click opens the right-side pane instead of switching tabs
  - non-diff blocks do not trigger inline review
- controller tests under `controller/ai/tests`
  - thread-scoped right-pane review selection persists correctly
  - invalid selected rows are pruned when timeline rows rebuild
- new inline review snapshot tests
  - modified line pair renders as removed then added
  - unchanged lines render once with both line numbers
  - file and hunk headers remain stable
  - syntax spans survive projection into unified rows

Manual QA checklist:

- open diff from AI without losing composer draft or thread position
- resize pane and confirm timeline/composer remain usable
- open full Review from the pane header
- verify Rust, TS/JS, Markdown, and shell diffs show syntax colors
- verify large diffs stay smooth while scrolling

End-of-task verification:

- run workspace build once
- run workspace clippy once
- run the relevant tests once

## Risks

### Shared owner refactor risk

Pulling display-row generation out of `ReviewWorkspaceSurfaceState` touches a central path. Keep the refactor small and mechanical before adding the unified right-pane renderer.

### Unified-row projection complexity

Projecting wrapped left/right display rows into a readable unified stream is the trickiest logic in the feature. That should be isolated in its own module with dedicated tests.

### Accidental second diff pipeline

Do not render the AI pane from timeline summary strings or ad hoc patch parsing. That would immediately diverge from Review quality and create two syntax/highlight paths to maintain.

## Open Questions

- Is the current thread-level compare snapshot sufficient for V1, or do we eventually need true turn-scoped historical diffs?
- Should the AI right-side pane expose file-header `Open file` actions in V1, or should that remain Review-only?
- Do we want to persist right-pane width and selected file per thread in the first pass, or keep V1 stateless beyond open/closed row selection?

## Recommended Order Of Work

1. Wire diff-summary clicks to the existing right-pane selection state.
2. Split shared diff-display ownership from per-surface state.
3. Add the unified inline snapshot/projection module.
4. Extract shared text-run painting helpers.
5. Render the new AI right-side diff pane and header actions.
6. Add tests, then run build/clippy/test verification once at the end.
