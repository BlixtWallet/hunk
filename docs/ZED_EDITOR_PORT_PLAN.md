# Zed Editor Port Plan

## Scope

This plan covers the **minimum Zed editor architecture** we should copy into Hunk to fix live keyboard handling and move the native file editor toward a real GPUI editor surface.

This is based on a fresh local clone of Zed at:

- repo: `.codex/vendor/zed-upstream`
- commit: `d265b32548f0335a5b944ed1c371e9f0afe6a1d0`

Primary Zed files reviewed:

- `.codex/vendor/zed-upstream/crates/editor/src/element.rs`
- `.codex/vendor/zed-upstream/crates/editor/src/editor.rs`
- `.codex/vendor/zed-upstream/crates/editor/src/actions.rs`
- `.codex/vendor/zed-upstream/crates/gpui/src/window.rs`

Primary Hunk files reviewed:

- `crates/hunk-desktop/src/app/render/file_editor_surface.rs`
- `crates/hunk-desktop/src/app/native_files_editor.rs`
- `crates/hunk-desktop/src/app/native_files_editor_input.rs`
- `crates/hunk-desktop/src/app/native_files_editor_element.rs`
- `crates/hunk-editor/src/lib.rs`

## Findings

Hunk already has the right core shape:

- headless text/editor core in `hunk-text` and `hunk-editor`
- custom syntax layer in `hunk-language`
- custom GPUI paint surface in `native_files_editor_element.rs`

But the live input architecture is still too thin.

Current Hunk flow:

- outer container in `file_editor_surface.rs` owns focus
- outer container listens to raw `.on_key_down(...)`
- raw keystrokes are forwarded into `FilesEditor::handle_keystroke(...)`

This is enough for some keys, but it is not how Zed handles editor input.

Zed flow:

- the editor element installs its own `focus_handle`
- the editor element installs a `key_context`
- the editor element installs an `ElementInputHandler`
- the editor element registers editor actions on the focused node
- GPUI dispatches keybindings to actions like `move_up`, `move_down`, `move_to_end_of_line`

Important Zed references:

- action registration: `.codex/vendor/zed-upstream/crates/editor/src/element.rs`
- action types: `.codex/vendor/zed-upstream/crates/editor/src/actions.rs`
- movement handlers: `.codex/vendor/zed-upstream/crates/editor/src/editor.rs`
- GPUI dispatch path: `.codex/vendor/zed-upstream/crates/gpui/src/window.rs`

## What To Copy

We should copy the following parts from Zed.

### 1. Action-driven keyboard architecture

Copy the pattern, and likely some direct code, for:

- action structs for editor movement and selection
- focused-node action registration
- editor-scoped key context instead of raw key matching only

Concrete target for Hunk:

- create a Hunk editor action module similar to Zed `actions.rs`
- register actions from the file editor element instead of the outer shell only
- route arrow keys, selection movement, line-boundary movement, paging, delete, enter, tab through actions

This is the most important part to copy first.

### 2. Editor element input ownership

Copy the pattern from Zed where the editor element itself installs:

- `window.set_key_context(...)`
- `window.handle_input(...)`
- editor action registration during paint/prepaint lifecycle

Concrete target for Hunk:

- stop treating the file editor as a generic `div()` with an `on_key_down` closure
- let `FilesEditorElement` own key context and input handling
- keep the wrapper surface for toolbar/search chrome only

### 3. Movement/selection action split

Copy the separation between:

- `MoveUp`
- `MoveDown`
- `SelectUp`
- `SelectDown`
- `MoveToBeginningOfLine`
- `MoveToEndOfLine`
- `SelectToBeginningOfLine`
- `SelectToEndOfLine`
- line/page scrolling actions

Concrete target for Hunk:

- keep the actual cursor math in `hunk-editor`
- expose explicit Hunk commands/actions instead of one monolithic `handle_keystroke`

### 4. Click-count selection semantics

We already implemented basic double/triple click selection, but Zed’s structure is cleaner:

- click count selects mode
- drag updates use the current selection mode
- selection phase is explicit: begin, extend, update, end

Concrete target for Hunk:

- port Zed’s selection-phase structure after keyboard input is actionized
- keep our simpler read-only/single-cursor scope for now

## What Not To Copy

Do not copy these parts yet.

- Zed multi-buffer excerpts
- minimap
- collaboration selections
- hover popovers
- context menus
- inlay hint system
- signature help
- completions UI
- Vim wrapper behavior
- full `display_map`

Reason:

- these are not needed to fix Hunk’s current live keyboard issue
- they would explode the scope
- Hunk already has a simpler custom display model that works well enough for the current feature set

## Port Strategy

### Phase 1. Keyboard Dispatch

Goal:

- make live editor movement use editor actions instead of raw wrapper keystroke branching

Todo:

- add `crates/hunk-desktop/src/app/native_files_editor_actions.rs`
- define Hunk editor actions for:
  - move left/right/up/down
  - select left/right/up/down
  - move to start/end of line
  - select to start/end of line
  - page up/down
  - insert text
  - backspace/delete
  - newline/tab
  - undo/redo
  - select all
- move key-to-action mapping out of the outer `file_editor_surface.rs` closure
- make `FilesEditorElement` install editor key context
- register actions on the editor-focused node like Zed does
- keep `FilesEditor::handle_keystroke` only as a temporary fallback during the transition
- add regression tests for:
  - live action routing for up/down
  - line-boundary movement
  - selection extension
  - text insertion and deletion through actions
- review:
  - verify no editor movement still depends on raw string matching in outer UI glue
  - verify focused editor actions do not fire while markdown preview or search input owns focus

### Phase 2. Input Handler Ownership

Goal:

- make the file editor element the real GPUI input owner

Todo:

- introduce a `FocusHandle` and input-handler ownership model aligned with Zed’s editor element
- move text input handling closer to the editor element
- ensure the editor element can accept text input directly through GPUI input handling
- remove broad editor keyboard logic from `render/file_editor_surface.rs`
- keep only shell-level shortcuts there, such as open search
- add tests for:
  - editor receives arrow keys when focused
  - shell controls do not steal focused editor movement keys
  - focus transfers correctly between file tree, editor, search input, and markdown preview
- review:
  - verify the focused-node path is the only path for editor movement
  - verify there is no double-handling between wrapper and element

### Phase 3. Selection State Machine

Goal:

- replace ad hoc mouse selection handling with explicit selection phases

Todo:

- add selection phase types similar to Zed:
  - begin
  - extend
  - update
  - end
- teach drag updates to preserve click-count selection mode
- make double-click drag extend by word
- make triple-click drag extend by line
- add tests for:
  - double-click and drag expands word selection by word units
  - triple-click and drag expands line selection by full lines
  - shift-click extension interacts correctly with existing selection
- review:
  - verify drag anchor logic is no longer overloaded for every selection mode
  - refactor any duplicated boundary logic into the editor/text core

### Phase 4. Core Command Cleanup

Goal:

- align Hunk’s editor core API with the action architecture

Todo:

- split `FilesEditor::handle_keystroke` into:
  - action registration
  - action handlers
  - pure editor commands
- consider adding explicit `hunk-editor` commands for:
  - select up/down
  - move/select to line boundary
  - word movement/select
- move reusable selection-boundary logic into `hunk-editor` or `hunk-text`
- add tests in crate-level `tests/` directories for new editor-core commands
- review:
  - remove dead fallback paths
  - ensure the public editing surface reads like a coherent editor API, not a bag of UI helpers

## Recommendation

Copy only the following Zed pieces first:

- action definitions pattern from `crates/editor/src/actions.rs`
- action registration pattern from `crates/editor/src/element.rs`
- focused-node input/key-context setup from `crates/editor/src/element.rs`
- movement/select action handler structure from `crates/editor/src/editor.rs`

Do not start by copying Zed rendering or display-map internals.

That would be the wrong frontier. The live Hunk bug is in keyboard dispatch architecture, not text painting.

## First Concrete PR

The first PR should do only this:

- add editor action types
- register those actions on `FilesEditorElement`
- move arrow keys and line-boundary movement off raw `.on_key_down(...)`
- prove `up/down` works in the live focused editor path

If that lands cleanly, the second PR should move insert/delete/newline/tab onto the same action path.
