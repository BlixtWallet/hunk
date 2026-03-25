# Lessons Learned

- Focus restoration bug: Drawer, modal, or panel close paths can restore focus too early while the UI is still unmounting, which leaves the workspace without a stable focused target and causes keyboard shortcuts to stop working until the user clicks again.
  Fix: Capture the pre-open focus target and restore focus with a deferred action after the overlay or panel has fully closed.
- Context-sensitive focus bug: Temporary surfaces such as terminals or popovers can steal focus from an editor or workspace root and fail to return it to the correct place on close.
  Fix: Record where focus came from, such as an editor vs. a workspace container, and route every close path back to that same target.
- Multi-project AI architecture: The AI view became materially heavier after moving from a single-project model to a workspace-wide model with project-grouped threads, per-workspace runtimes, and cross-project thread selection.
  Fix: Treat AI as its own workspace surface with its own cached visible-frame state instead of piggybacking on the non-AI active-project model.
- AI performance regression: The initial multi-project implementation kept rebuilding expensive AI state during scroll, which made the AI view feel much slower even though the chat row renderer itself had not changed much.
  Fix: Add instrumentation first, then use it to separate controller-state costs from render-path costs before refactoring further.
- Root cause isolation: The biggest regressions were not where they first appeared. Sidebar grouping, composer feedback, and markdown parsing all contributed at different times, but the sustained bottleneck came from doing expensive work in the root AI render path.
  Fix: Measure render frequency and per-phase timing for the root view, the timeline rows, markdown parsing, and the composer before deciding which path to optimize.
- Cached visible AI state: Recomputing visible thread, workspace, approvals, publish state, and timeline metadata during render is too expensive once AI becomes workspace-aware.
  Fix: Build a cached visible-frame state in controller code and let render consume precomputed state instead of rescanning workspace-wide AI data every frame.
- Timeline rendering: Virtualizing the thread sidebar helped, but it did not solve the chat slowdown by itself because the root AI view was still rebuilding too much around the timeline.
  Fix: Isolate the timeline render path from unrelated AI UI work and keep scroll-driven updates as local as possible.
- Composer feedback strip: A small surface can dominate render time if it performs snapshot scans every frame. In our case, the feedback strip was repeatedly scanning in-progress AI items to derive the current activity label.
  Fix: Cache composer feedback in the visible-frame state and pass it into render as plain data.
- Markdown in chat rows: Repeatedly parsing the same markdown message while scrolling is unnecessary and becomes visible once larger bottlenecks are removed.
  Fix: Cache parsed markdown blocks and selection surfaces by row id plus message content.
- Child-entity refactors: Splitting a hot UI path into a child GPUI entity is the right direction, but only after the render helpers no longer depend on `Context<DiffViewer>` for unrelated state.
  Fix: First decouple helper functions from root-view context requirements, then extract child entities in smaller cuts to avoid blank or partially wired surfaces.
- Refactor discipline: Large architectural changes plus performance work can create avoidable churn if they are done in one pass without enough observability.
  Fix: Stage the work in this order: stabilize product semantics, instrument, cache the visible state, then extract child entities only where the measurements justify it.
