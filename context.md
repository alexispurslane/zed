# Investigation: Pane Splitting Functionality Bug

## User-Reported Issues

1. **Keybindings like Ctrl-X 3 don't split panes** — instead they just type a "3" character into the editor
2. **The "Split..." menu in the file picker also doesn't work** — clicking "Split Right" etc. has no effect
3. **The which-key system shows these as valid keybindings** — but they don't work when actually pressed

---

## Code Paths Involved

### 1. Pane Split Actions & Handlers

**File:** `crates/workspace/src/pane.rs`

The split actions are defined via macro (lines 224–250):
```rust
macro_rules! split_structs {
    ($($name:ident => $doc:literal),* $(,)?) => {
        $(
            #[derive(Clone, PartialEq, Debug, Deserialize, JsonSchema, Default, Action)]
            #[action(namespace = pane)]
            pub struct $name { pub mode: SplitMode, }
        )*
    };
}
split_structs!(
    SplitLeft => "Splits the pane to the left.",
    SplitRight => "Splits the pane to the right.",
    SplitUp => "Splits the pane upward.",
    SplitDown => "Splits the pane downward.",
    SplitHorizontal => "Splits the pane horizontally.",
    SplitVertical => "Splits the pane vertically."
);
```

Default `SplitMode` is `ClonePane`.

**Pane action handlers** (lines 4323–4338):
```rust
.on_action(cx.listener(|pane, split: &SplitRight, window, cx| {
    pane.split(SplitDirection::Right, split.mode, window, cx)
}))
```

**The `split()` method** (line 2558) emits `Event::Split`:
```rust
pub fn split(&mut self, direction: SplitDirection, mode: SplitMode, ...) {
    // ...
    cx.emit(Event::Split { direction, mode });
}
```

**Workspace handles the event** (`workspace.rs` line 5363):
```rust
pane::Event::Split { direction, mode } => {
    match mode {
        SplitMode::ClonePane => self.split_and_clone(pane.clone(), *direction, window, cx).detach();
        SplitMode::EmptyPane => self.split_pane(pane.clone(), *direction, window, cx);
        SplitMode::MovePane => self.split_and_move(pane.clone(), *direction, window, cx);
    };
}
```

### 2. Emacs Keymap Bindings for Split

**File:** `assets/keymaps/macos/emacs.json` (lines 157–163)

```json
{
  "context": "Workspace",
  "bindings": {
    "ctrl-x 2": "pane::SplitDown",
    "ctrl-x 3": "pane::SplitRight",
    ...
  }
}
```

**File:** `assets/keymaps/linux/emacs.json` (lines 159–163) — same bindings.

**Critical platform difference:** On Linux, the default keymap has `"ctrl-x": "editor::Cut"` in the Editor context. The Linux emacs keymap adds `"ctrl-x": null` (NoAction/unbind) in the Editor context to unbind it. On macOS, there is no `ctrl-x` binding in the default keymap (Cut is on `cmd-x`), so no unbind is needed.

### 3. Default Keymap Split Bindings

**File:** `assets/keymaps/default-macos.json`

```json
// In Workspace context:
"cmd-\\"      => "pane::SplitRight"
"cmd-k right" => "pane::SplitRight"
"cmd-k left"  => "pane::SplitLeft"
"cmd-k up"    => "pane::SplitUp"
"cmd-k down"  => "pane::SplitDown"

// In Terminal context:
"cmd-d" => "pane::SplitRight"

// In FileFinder context:
"cmd-j" => "pane::SplitDown"
"cmd-k" => "pane::SplitUp"
"cmd-h" => "pane::SplitLeft"
"cmd-l" => "pane::SplitRight"
```

**File:** `assets/keymaps/default-linux.json` — similar but with `ctrl` instead of `cmd`.

**Vim keymap** uses `ctrl-w` prefix for window management (not `ctrl-x`).

### 4. Key Dispatch System

**File:** `crates/gpui/src/window.rs` — `dispatch_key_event()` (line 4437)

The key dispatch flow when a keystroke is received:

1. **Keystroke extraction** from `KeyDownEvent` or `ModifiersChangedEvent`
2. **Keystroke interceptors** run first (can consume the event)
3. **Pending input check** — if there are pending keystrokes from a previous multi-key binding sequence, they're prepended to the new keystroke
4. **`dispatch_key()`** is called on the dispatch tree with the accumulated input
5. **Three outcomes:**
   - **Pending** (more keys needed): Store pending input, set a 1-second timeout
   - **Matched bindings**: Dispatch the action(s)
   - **No match**: Fall through to `finish_dispatch_key_event()` which sends the key to key listeners and then the IME/input handler

**Key code path for multi-key binding** (window.rs lines 4500–4560):
```rust
let mut currently_pending = self.pending_input.take().unwrap_or_default();
let match_result = self.rendered_frame.dispatch_tree.dispatch_key(
    currently_pending.keystrokes, keystroke, &dispatch_path,
);

if !match_result.pending.is_empty() {
    // Store pending and return (wait for more keys)
    currently_pending.keystrokes = match_result.pending;
    currently_pending.needs_timeout |= match_result.pending_has_binding || text_input_requires_timeout;
    // 1 second timeout
    self.pending_input = Some(currently_pending);
    return;
}

// ...dispatch matched bindings...
```

**Replay mechanism** (window.rs line 4700): When pending input times out (1 second), `flush_dispatch()` converts the pending input to replay events. For `ctrl-x` on Linux, this would replay as `editor::Cut` since there's a valid binding for just `ctrl-x`.

### 5. Keymap Matching — `bindings_for_input()`

**File:** `crates/gpui/src/keymap.rs` (line 155)

This is the critical method for determining which bindings match a given input sequence.

**Algorithm:**
1. Iterate all bindings in reverse order (newest first)
2. Check `binding_enabled()` — does the context predicate match the current context stack? Returns the depth at which it matches.
3. Check `match_keystrokes()` — does the input keystrokes match the binding's keystrokes? Returns `Some(pending)` where `pending=true` means the input is a prefix of the binding.
4. Sort matched bindings by depth descending, then index descending
5. Process matched bindings:
   - **NoAction with meta.0 == 0 (User source)**: Break — user unbind takes priority
   - **NoAction with no meta**: Break — assume user unbind for safety
   - **NoAction with meta.0 != 0 (Base/Default source)**: Continue — skip, look for user overrides
   - **Unbind**: Add to `unbound_bindings` list
   - **Regular action**: Add to `bindings` result
6. Process pending bindings: filter out those shadowed by matched bindings, remove NoAction/Unbind ones

### 6. `binding_enabled()` — CRITICAL CHANGE ON OTHER BRANCH

**File:** `crates/gpui/src/keymap.rs` (line 246)

**On HEAD (current):**
```rust
fn binding_enabled(&self, binding: &KeyBinding, contexts: &[KeyContext]) -> Option<usize> {
    if let Some(predicate) = &binding.context_predicate {
        predicate.depth_of(contexts)
    } else {
        Some(contexts.len())  // Global bindings get HIGHEST depth
    }
}
```

**On `upstream/settings-toggle-fix` branch (commits `50f677a249` + `b0ded23cba`):**
```rust
pub fn binding_enabled(binding: &KeyBinding, contexts: &[KeyContext]) -> Option<usize> {
    if let Some(predicate) = &binding.context_predicate {
        predicate.depth_of(contexts)
    } else {
        Some(0)  // Global bindings get LOWEST depth
    }
}
```

**Impact:** This changes the precedence of global (no-context) bindings from HIGHEST to LOWEST. This affects the sorting in `bindings_for_input()` where depth determines which bindings take priority. The commit message "Fix the bug" suggests it was intended to fix a precedence issue where global bindings incorrectly overrode context-specific ones. However, this change could introduce other issues if there are global bindings that previously relied on having the highest depth.

### 7. Context Stack Construction

When an editor is focused inside a workspace, the context stack is typically:
```
[Workspace, Pane, Editor(mode=full)]
```

The `depth_of()` method returns the deepest slice of this stack that satisfies a binding's context predicate:
- `"Workspace"` context → depth=1 (matches at `[Workspace]`)
- `"Editor"` context → depth=3 (matches at `[Workspace, Pane, Editor]`)
- `"Pane"` context → depth=2 (matches at `[Workspace, Pane]`)

Bindings at greater depth have higher precedence, so Editor bindings beat Pane bindings which beat Workspace bindings.

### 8. Which-Key System

**File:** `crates/which_key/src/which_key.rs`

The which-key system shows pending keybinding options. It observes `pending_input_keystrokes()` changes:

```rust
cx.observe_pending_input(window, move |workspace, window, cx| {
    if window.pending_input_keystrokes().is_none() {
        // Dismiss modal if no pending input
    }
    // Show modal after delay_ms
})
```

**File:** `crates/which_key/src/which_key_modal.rs`

The modal reads possible bindings using `window.possible_bindings_for_input(pending_keys)`:
```rust
fn update_pending_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(pending_keys) = window.pending_input_keystrokes() else {
        cx.emit(DismissEvent);
        return;
    };
    let bindings = window.possible_bindings_for_input(pending_keys);
    // ...display bindings...
}
```

**`possible_bindings_for_input()`** calls `keymap.possible_next_bindings_for_input()` which returns all bindings that START with the given input and have more keystrokes to follow. This is a KEYMAP-LEVEL query, not a dispatch-level query. It shows all bindings that COULD match, regardless of whether they would actually be reached in the dispatch tree.

**This explains symptom #3:** The which-key shows the bindings because the KEYMAP has them registered. But the DISPATCH might not reach them if the action handlers aren't in the dispatch path or if the binding resolution gives different results at dispatch time.

### 9. File Finder Split Menu

**File:** `crates/file_finder/src/file_finder.rs`

The file finder has a "Split…" popover menu (line 2330) with "Split Left/Right/Up/Down" items:
```rust
PopoverMenu::new("split-menu-popover")
    .trigger(ButtonLike::new("split-trigger").child(Label::new("Split…")))
    .menu(move |window, cx| {
        Some(ContextMenu::build(window, cx, {
            move |menu, _, _| {
                menu.context(focus_handle)
                    .action("Split Left", pane::SplitLeft::default().boxed_clone())
                    .action("Split Right", pane::SplitRight::default().boxed_clone())
                    .action("Split Up", pane::SplitUp::default().boxed_clone())
                    .action("Split Down", pane::SplitDown::default().boxed_clone())
            }
        }))
    })
```

**File finder action handlers** (lines 285–397):
```rust
.on_action(cx.listener(Self::go_to_file_split_left))
.on_action(cx.listener(Self::go_to_file_split_right))
.on_action(cx.listener(Self::go_to_file_split_up))
.on_action(cx.listener(Self::go_to_file_split_down))
```

These handlers call `go_to_file_split_inner(split_direction, ...)` which:
1. Gets the selected match
2. Calls `workspace.split_path_preview(path, false, Some(split_direction), window, cx)`

**Key bindings in FileFinder context** (default-macos.json):
```json
"cmd-j": "pane::SplitDown",
"cmd-k": "pane::SplitUp",
"cmd-h": "pane::SplitLeft",
"cmd-l": "pane::SplitRight"
```

Note: The Emacs keymap has **NO FileFinder-specific bindings** — it relies on the default keymap's FileFinder bindings.

### 10. Context Menu Action Dispatch

**File:** `crates/ui/src/components/context_menu.rs`

When a context menu item (like "Split Right") is clicked:

```rust
handler: Rc::new(move |context, window, cx| {
    if let Some(context) = &context {
        window.focus(context, cx);     // 1. Focus the context (file finder's focus handle)
    }
    window.dispatch_action(action.boxed_clone(), cx);  // 2. Dispatch action (DEFERRED!)
}),
```

**CRITICAL: `window.focus()` clears pending keystrokes!** (window.rs line 1782):
```rust
pub fn focus(&mut self, handle: &FocusHandle, cx: &mut App) {
    if !self.focus_enabled || self.focus == Some(handle.id) { return; }
    self.focus = Some(handle.id);
    self.clear_pending_keystrokes();  // ← THIS
    // ...
}
```

**`window.dispatch_action()` is deferred** (window.rs line 1879):
```rust
pub fn dispatch_action(&mut self, action: Box<dyn Action>, cx: &mut App) {
    let focus_id = self.focused(cx).map(|handle| handle.id);
    cx.defer(move |cx| {
        window.update(cx, |_, window, cx| {
            let node_id = window.focus_node_id_in_rendered_frame(focus_id);
            window.dispatch_action_on_node(node_id, action.as_ref(), cx);
        }).log_err();
    })
}
```

### 11. Context Menu's `on_action_dispatch` Interception

**File:** `crates/ui/src/components/context_menu.rs` (line 1338)

When an action (like SplitRight) is dispatched WHILE the context menu is focused (e.g., via a keybinding), the context menu intercepts it:

```rust
pub fn on_action_dispatch(&mut self, dispatched: &dyn Action, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(ix) = self.items.iter().position(|item| action matches) {
        self.select_index(ix, window, cx);
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(50)).await;
            this.update(cx, |this, cx| {
                this.cancel(&menu::Cancel, window, cx);
                window.dispatch_action(action, cx);  // Re-dispatch after 50ms
            })
        }).detach_and_log_err(cx);
    } else {
        cx.propagate()
    }
}
```

This means: if SplitRight is triggered via keybinding while the context menu is open, the menu intercepts it, waits 50ms, then dismisses itself and re-dispatches the action.

---

## Potential Root Causes

### Hypothesis A: NoAction Meta Handling on Linux

On Linux with the Emacs keymap, `"ctrl-x": null` has `KeybindSource::Base` (meta=2).

In `bindings_for_input()`, when processing matched bindings:
```rust
if is_no_action(&*binding.action) {
    if let Some(meta) = binding.meta {
        if meta.0 == 0 { break; }  // User unbind — stop
    } else {
        break;  // No meta — assume user, stop
    }
    continue;  // Non-user NoAction — skip, keep looking
}
```

Since `meta.0 = 2` (Base source), the code `continue`s past the NoAction binding and finds the Default keymap's `"ctrl-x": editor::Cut` (meta=3). This IS added to `bindings`. However, because there are also PENDING bindings (ctrl-x 3, ctrl-x ctrl-x, etc.), the method returns `(bindings=[Cut], pending=true)`.

The `dispatch_key()` method prioritizes the `pending` branch: when `pending=true`, it returns pending mode regardless of `bindings`. So `ctrl-x` correctly enters pending mode.

**However**, the `pending_has_binding` flag is set to true (because `bindings` is non-empty). This causes the `needs_timeout` flag to be set, triggering the 1-second timeout. If the user types `3` within 1 second, the system correctly matches `ctrl-x 3` → SplitRight. If they wait longer, `ctrl-x` is replayed as Cut.

**Verdict:** This path appears correct, but the 1-second timeout could be a source of confusion. If the timeout fires before the user presses `3`, `ctrl-x` would be replayed as `editor::Cut` (on Linux), and then `3` would be typed as text.

### Hypothesis B: Which-Key Modal Focus Interference

The which-key modal is shown after a configurable delay (`delay_ms` in `which_key_settings.rs`). When it appears:

1. It's rendered as a `ModalView` via `workspace.toggle_modal()`
2. The `ModalLayer.show_modal()` calls `window.focus(&new_modal.focus_handle(cx), cx)` (deferred)
3. `WhichKeyModal` returns the current editor's focus handle from `focus_handle()`, so the deferred focus is a no-op (focus already on editor)

**But**: The `ModalLayer` creates its own focus handle (`cx.focus_handle()`) and tracks it with `.track_focus()`. This could potentially interfere with the key dispatch if the modal layer's focus handle somehow captures focus.

**Verdict:** Unlikely to be the direct cause, since which-key explicitly preserves the editor's focus. But worth investigating if timing issues exist.

### Hypothesis C: `window.focus()` Clears Pending Keystrokes

When ANY focus change occurs during a multi-key binding sequence, pending keystrokes are lost. This is by design (to prevent keybinding leakage across focus changes), but it could cause issues if:

1. The which-key modal causes a focus change
2. A context menu populates and changes focus
3. The file finder's popover menu changes focus

For the file finder's "Split..." menu specifically:
1. User opens file finder (ctrl-x ctrl-f)
2. File finder appears as a modal → focus shifts to FileFinder
3. User clicks "Split…" button → PopoverMenu's ContextMenu opens → focus shifts to context menu
4. User clicks "Split Right" → context menu calls `window.focus(file_finder_focus_handle, cx)` which CLEARS PENDING KEYSTROKES, then `window.dispatch_action(SplitRight, cx)` which is DEFERRED
5. Context menu emits DismissEvent → PopoverMenu handles it by restoring previous focus → ANOTHER focus change
6. Deferred dispatch runs → looks up focus_id in the rendered frame → if re-render happened, focus_id might not be found → falls back to root node → root has no SplitRight handler → ACTION IS LOST

**Verdict:** This is a STRONG candidate for the file finder split menu bug. The deferred `dispatch_action` combined with focus changes during context menu dismissal could cause the action to be dispatched on the wrong node.

### Hypothesis D: `binding_enabled` Change on `settings-toggle-fix` Branch

The change from `Some(contexts.len())` to `Some(0)` for global bindings has cascading effects:

1. **Precedence inversion:** Global bindings go from highest to lowest precedence. Any global binding that previously shadowed context-specific bindings now gets shadowed.
2. **NoAction handling:** If a global NoAction binding previously blocked all matched bindings (due to highest depth), it now has lowest depth and gets processed AFTER context-specific bindings. This could cause context-specific actions to fire even when a global unbind exists.
3. **Pending binding interaction:** Global pending bindings now have lower depth than context-specific ones, potentially changing which bindings are considered "pending" vs "matched."

**Example:** A global binding like `"x": "some::Action"` with context `[Workspace, Pane, Editor]` used to get depth=3 and match at Editor level. Now it gets depth=0 and matches at the global level. If there's also an Editor-specific binding for `"x"`, the Editor one takes priority.

**Verdict:** This change is a likely source of subtle binding precedence bugs, especially for keymaps that rely on global fallback bindings (like the Vim or Emacs keymaps that add bindings without explicit contexts).

### Hypothesis E: Dispatch Path Missing Pane for Modal Views

When a modal (like the FileFinder) has focus, the dispatch path from the focused node goes through:
```
Root(Workspace) → ModalLayer → FileFinder
```

The **Pane is NOT in this path**. When SplitRight is dispatched on the FileFinder's node, the dispatch path bubble phase is:
1. FileFinder → has SplitRight handler → dispatches
2. (Propagation stops)

The FileFinder's `go_to_file_split_right` handler calls `workspace.split_path_preview(...)`. This requires a valid workspace reference and a selected match. If either is missing, the split silently fails.

**Verdict:** The split should work if the file finder has a valid workspace reference and a selected match. But the action is dispatched asynchronously (via `dispatch_action` which is deferred), and between the click and the dispatch, the state could change.

### Hypothesis F: Context Stack Divergence Between Which-Key and Dispatch

The which-key modal uses `window.possible_bindings_for_input()` which queries the keymap with the WINDOW's current context stack. But when the which-key modal is visible, the window's focus might be on the modal, not the editor. If the modal changes the context stack, the which-key display might show bindings that are valid for one context but not another.

**Verdict:** The which-key modal explicitly preserves the editor's focus handle, so the context stack should be the same. But this could be a source of inconsistency if focus handling changes.

---

## Architecture Diagram

```
User types ctrl-x
       │
       ▼
Platform KeyDownEvent
       │
       ▼
window.dispatch_key_event()          (window.rs:4437)
       │
       ├── Extract keystroke
       ├── Check keystroke interceptors
       ├── Get pending input (if any)
       │
       ▼
dispatch_tree.dispatch_key()          (key_dispatch.rs:490)
       │
       ├── Append keystroke to input → [ctrl-x] or [ctrl-x, 3]
       ├── bindings_for_input()       (keymap.rs:155)
       │   ├── Iterate bindings (newest first)
       │   ├── binding_enabled()      (keymap.rs:246) ← CRITICAL: depth calculation
       │   ├── match_keystrokes()
       │   ├── Sort by depth desc, ix desc
       │   ├── Process NoAction/Unbind
       │   └── Return (bindings, pending)
       │
       ├── IF pending: store, set timeout, return
       ├── IF matched: dispatch_action_on_node()
       └── IF neither: finish_dispatch_key_event() → key listeners → IME input handler
                │
                ▼ (for "3" with no binding)
           InputHandler.dispatch_input("3")  ← types "3" into editor
```

```
File Finder "Split..." Menu Click
       │
       ▼
ContextMenu click handler              (context_menu.rs:728)
       │
       ├── window.focus(context, cx)     ← CLEARS PENDING KEYSTROKES
       ├── window.dispatch_action(SplitRight, cx)  ← DEFERRED
       │
       │ (…meanwhile…)
       ├── Context menu emits DismissEvent
       ├── PopoverMenu handles DismissEvent → window.focus(previous, cx)  ← MORE FOCUS CHANGES
       │
       ▼
Deferred dispatch_action runs
       │
       ├── focus_node_id_in_rendered_frame(focus_id)
       │   ├── If focus_id not found → root node → NO SplitRight handler → ACTION LOST
       │   └── If found → dispatch on that node → FileFinder SplitRight handler fires
       │
       ▼
FileFinder.go_to_file_split_right()
       │
       ├── workspace.split_path_preview(path, false, Some(Right), window, cx)
       │   ├── split_pane() → adds new pane
       │   └── open_item() → opens file in new pane
```

---

## Key Files Reference

| File | Role |
|------|------|
| `crates/gpui/src/key_dispatch.rs` | Dispatch tree, `dispatch_key()`, pending input logic |
| `crates/gpui/src/keymap.rs` | `bindings_for_input()`, `binding_enabled()`, NoAction/Unbind handling |
| `crates/gpui/src/window.rs` | `dispatch_key_event()`, `dispatch_action()`, `focus()`, pending input storage |
| `crates/workspace/src/pane.rs` | Split actions (`SplitRight` etc.), `split()` method, action handlers |
| `crates/workspace/src/workspace.rs` | `split_and_clone()`, `split_pane()`, `split_path_preview()`, event handling |
| `crates/workspace/src/modal_layer.rs` | Modal lifecycle, focus management, `track_focus()` |
| `crates/which_key/src/which_key.rs` | Which-key initialization, pending input observation |
| `crates/which_key/src/which_key_modal.rs` | Modal display, `possible_bindings_for_input()` |
| `crates/which_key/src/which_key_settings.rs` | Which-key settings (delay, enabled) |
| `crates/file_finder/src/file_finder.rs` | File finder split menu, `go_to_file_split_right()` etc. |
| `crates/ui/src/components/context_menu.rs` | Context menu action dispatch, `on_action_dispatch()` |
| `crates/ui/src/components/popover_menu.rs` | Popover menu, focus handling on dismiss |
| `assets/keymaps/macos/emacs.json` | Emacs keymap (macOS), ctrl-x bindings |
| `assets/keymaps/linux/emacs.json` | Emacs keymap (Linux), includes ctrl-x unbind |
| `assets/keymaps/default-macos.json` | Default keymap (macOS) |
| `assets/keymaps/default-linux.json` | Default keymap (Linux), includes ctrl-x = Cut |
| `crates/gpui/src/keymap/binding.rs` | `KeyBinding` struct, `KeyBindingMetaIndex` |
| `crates/gpui/src/keymap/context.rs` | `KeyContext`, `depth_of()` |

---

## Recent Relevant Changes (Git History)

| Commit | Description | Impact |
|--------|-------------|--------|
| `b0ded23cba` | "Fix the bug" — `binding_enabled()` returns `0` for global bindings | **HIGH** — Changes precedence for all global bindings. On `settings-toggle-fix` branch only. |
| `50f677a249` | "Add test" — makes `binding_enabled()` a static method, adds `test_binding_enabled_order` | **MEDIUM** — Companion to the above. Also on `settings-toggle-fix` branch only. |
| `4a965d18e6` | "Feat unbind" (#52047) — Targeted Unbind support | **HIGH** — Introduces `Unbind` action type and meta-based NoAction handling |
| `83ca2c9e88` | Add Vim-like Which-key Popup menu (#43618) | **MEDIUM** — Adds which-key system that observes pending input |
| `228dff6dbb` | Fix all test failures — removes stale keymap bindings | **LOW** — Removes collab_panel etc. bindings, doesn't affect split |
| `b3ebcef5c6` | gpui: Only time out multi-stroke bindings when current prefix matches (#42659) | **MEDIUM** — Changes timeout behavior for pending input |
| `3f90bc81bd` | gpui: Filter out NoAction bindings from pending input (#30260) | **HIGH** — Affects how `ctrl-x: null` interacts with pending bindings |
| `30177b87d6` | Fix detection of pending bindings when binding in parent context matches (#34856) | **HIGH** — Fixes edge case where parent context binding shadows pending child binding |
| `f084e20c56` | Fix stale pending keybinding indicators on focus change (#44678) | **MEDIUM** — Related to pending input clearing on focus change |

---

## Specific Disconnects Found

### 1. NoAction Meta Handling vs. KeybindSource Precedence

**Location:** `keymap.rs` lines 200–210

The NoAction handling uses `meta.0` (KeybindSource) to decide whether to break or continue:
- `meta.0 == 0` (User): Break — user's unbind wins
- `meta.0 != 0` (Base/Default): Continue — skip, look for user overrides

**Problem:** On Linux with the Emacs keymap (`"ctrl-x": null`, KeybindSource::Base, meta=2), the NoAction is SKIPPED. The Default keymap's `"ctrl-x": editor::Cut` (KeybindSource::Default, meta=3) is then added to `bindings`. Even though pending takes priority, this means:
- `bindings = [Cut]`, `pending = true`, `pending_has_binding = true`
- The `pending_has_binding` flag causes a 1-second timeout to be triggered
- If the 1-second timeout fires before the user completes the multi-key sequence, `ctrl-x` is replayed as `editor::Cut`

**This is likely the root cause of the "3 is typed" issue on Linux:** If the user is slow to press "3" after "ctrl-x" (>1 second), the pending input times out, `ctrl-x` fires as Cut, and then "3" is typed as regular text.

### 2. macOS: No Unbind for `ctrl-x`

**Location:** `assets/keymaps/macos/emacs.json` vs `assets/keymaps/linux/emacs.json`

The macOS Emacs keymap does NOT have `"ctrl-x": null` in the Editor context because there's no default `ctrl-x` binding on macOS. This means `ctrl-x` should correctly enter pending mode without any competing exact-match binding.

**If the user is on macOS**, the `ctrl-x 3` sequence should work correctly in theory. The issue on macOS might be something else entirely — perhaps the pending input timeout is firing too early, or there's a focus change happening between keypresses.

### 3. `window.dispatch_action()` Deferred + Focus Change Race

**Location:** `window.rs` line 1879

When the context menu's "Split Right" item is clicked:
1. `window.focus(file_finder_focus_handle, cx)` — clears pending keystrokes
2. `window.dispatch_action(SplitRight, cx)` — captures focus_id, defers
3. Context menu dismisses → focus may shift again
4. Deferred dispatch runs → looks up focus_id in CURRENT rendered frame

If the rendered frame changed between steps 2 and 4 (due to context menu dismissal), the focus_id might resolve to a different node or the root node (which has no SplitRight handler).

**The fallback to root node is silent** — the action just doesn't get handled, no error, no feedback.

### 4. Which-Key Context vs. Dispatch Context

The which-key system shows bindings based on `window.possible_bindings_for_input()`, which queries the keymap with the window's context stack. The actual dispatch uses `dispatch_tree.dispatch_key()` which also queries the keymap but through the dispatch tree's context stack.

These SHOULD be the same, but if the which-key modal's appearance triggers a re-render that changes the dispatch tree, the context stacks could diverge.

---

## Recommended Investigation Steps

1. **Verify the Linux emacs keymap timeout issue:** On Linux, with the Emacs keymap active, type `ctrl-x` and then WAIT 1 second before typing `3`. Does `ctrl-x` fire as `editor::Cut`? Then type `3` — it appears as text. This would confirm Hypothesis A.

2. **Test on macOS:** If the bug also occurs on macOS (where there's no `ctrl-x: Cut` binding), the issue is NOT the NoAction meta handling. It might be something else, like the pending input being cleared by a focus change triggered by the which-key modal.

3. **Check the which-key delay setting:** If `delay_ms` is very short (e.g., 0), the which-key modal appears immediately and could potentially interfere with focus/pending input. Try disabling which-key and see if `ctrl-x 3` works.

4. **Test the file finder split menu with debug logging:** Add logging to `dispatch_action_on_node` and `focus_node_id_in_rendered_frame` to see which node the action is dispatched to after the context menu click. If it's the root node, the action is being lost.

5. **Check the `binding_enabled` change on the settings-toggle-fix branch:** If the bug only occurs on that branch, the change from `Some(contexts.len())` to `Some(0)` for global bindings is likely the culprit. Revert this change and test.

6. **Investigate the `pending_has_binding` interaction with the input handler:** When `pending_has_binding` is true AND `text_input_requires_timeout` is true, both conditions trigger the timeout. On Linux with Emacs, `pending_has_binding` is true because `"ctrl-x": editor::Cut` is a matched binding. This could cause unexpected timeout behavior.

---

## Summary of Findings

The pane splitting bug likely has MULTIPLE contributing factors:

| Factor | Severity | Explanation |
|--------|----------|-------------|
| **NoAction meta handling (Linux)** | **HIGH** | On Linux, the Emacs keymap's `"ctrl-x": null` is treated as a non-user NoAction (meta=2), which is SKIPPED during binding resolution. This allows `editor::Cut` to remain as a matched binding, setting `pending_has_binding=true` and triggering the 1-second timeout. If the user is slow, `ctrl-x` fires as Cut, and `3` types as text. |
| **Deferred dispatch + focus changes (File Finder)** | **HIGH** | The context menu's click handler calls `window.focus()` (clearing keystrokes) and `window.dispatch_action()` (deferred). Between these and the actual dispatch, focus changes from context menu dismissal can cause the action to be dispatched on the wrong node (root, which has no handler). |
| **`binding_enabled` change (settings-toggle-fix branch)** | **MEDIUM** | Changes global binding depth from `contexts.len()` to `0`, potentially disrupting precedence for global/multi-key bindings. |
| **Which-key modal focus timing** | **LOW** | The which-key modal preserves editor focus but its appearance could trigger focus-related side effects in edge cases. |
| **1-second pending timeout** | **LOW** | The timeout for uncommitted multi-key sequences is quite short. On Linux with the Emacs keymap, the timeout behavior is affected by the `pending_has_binding` flag. |
