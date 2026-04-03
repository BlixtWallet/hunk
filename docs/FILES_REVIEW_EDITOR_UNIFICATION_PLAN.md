# Files And Review Editor Unification Plan

## Status

Complete for this slice.

This document tracks the current follow-up work to make Hunk's Files and Review behavior feel closer to Zed's regular editor and diff views without regressing scroll performance.

Reference architecture in Zed:

- `/tmp/zed-full/crates/git_ui/src/file_diff_view.rs`
- `/tmp/zed-full/crates/git_ui/src/multi_diff_view.rs`
- `/tmp/zed-full/crates/editor/src/display_map.rs`
- `/tmp/zed-full/crates/editor/src/element.rs`

## Target For This Slice

- Keep Files and Review on shared editor/controller plumbing where practical.
- Keep Review compare state and hydrated editor state alive across workspace tab switches.
- Stop hydrating editor-backed inactive Review sections while scrolling.
- Render inactive Review sections from cached diff-preview rows instead of live editor construction.
- Only hydrate the selected Review file into a real editor session.

## Phase 1: Persist Review State Across Tab Switches

Status: Complete

### TODO

- [x] Stop clearing Review editor sessions merely because the user switched away from the Diff tab.
- [x] Track which compare inputs and workspace snapshot fingerprint produced the currently loaded Review compare state.
- [x] Re-entering the Diff tab reuses the loaded Review compare when the compare inputs and workspace snapshot fingerprint still match.
- [x] Preserve Review scroll position instead of revealing the selected file again on every tab re-entry.

## Phase 2: Remove Scroll-Time Inactive Editor Hydration

Status: Complete

### TODO

- [x] Stop visible-range prefetch from hydrating arbitrary inactive Review files while scrolling.
- [x] Restrict Review editor hydration to the selected file only.
- [x] Keep inactive sections on cached preview rows so revisiting the same scroll range does not trigger file loads again.
- [x] Keep bounded session retention only for explicitly opened Review editor sessions.

## Phase 3: Cached Preview Bodies For Inactive Review Sections

Status: Complete

### TODO

- [x] Restore a lightweight cached diff body for inactive Review sections.
- [x] Keep preview syntax on the cached row path only when idle, with plain-text fallback during active scroll.
- [x] Ensure preview rows stay layout-stable and do not show loading placeholders for already computed diff data.

## Phase 4: Validation And Follow-Up

Status: Complete

### TODO

- [x] Run workspace format/build/clippy/tests once after implementation is complete.
- [x] Update this document to the true final state.
- [x] Record the remaining gap to Zed: a single persistent multibuffer/editor diff surface is still the long-term target, but is not part of this slice.

## Remaining Gap To Zed

Zed keeps regular file editing and multi-file diff viewing inside one persistent editor or multibuffer surface. Hunk still keeps Review as a hybrid:

- the selected Review file is a real editor-backed compare surface
- inactive Review files render from cached diff preview rows

That hybrid model is now fast enough to avoid scroll-time editor hydration and tab re-entry reloads, but it is still not the same architecture as Zed's `MultiDiffView`. A true Hunk multibuffer Review surface remains a future architectural step.

## Validation

- `cargo fmt --all`
- `./scripts/run_with_macos_sdk_env.sh cargo build --workspace`
- `./scripts/run_with_macos_sdk_env.sh cargo clippy --workspace --all-targets -- -D warnings`
- `./scripts/run_with_macos_sdk_env.sh cargo test --workspace`
