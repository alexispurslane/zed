# Report: Agent Sessions as Tabs — Alternative Design

## Revisiting the Premise

Instead of preserving the `AgentPanel` as a dock panel with a sidebar that lists all threads, this report explores a more radical simplification:

> **What if we eliminate the agent panel sidebar entirely, and make agent sessions behave like any other item in the workspace — discoverable via the file finder, openable as tabs, with controls inline per-tab?**

This mirrors how the editor works: you don't have a "files sidebar" that lists every file on disk — you use **cmd-p** to find and open files. The project panel is optional.

---

## The Vision

### Core UX Principle

Agent sessions are **project artifacts**, not panel chrome. They should be treated like:
- **Editor tabs** — draggable, splittable, closable
- **File finder targets** — fuzzy-searchable alongside files in the unified `cmd-p` picker
- **Per-item configurable** — each tab has its own model, profile, settings

### What Changes

| Today | Proposed |
|-------|----------|
| Agent panel sidebar lists all threads | `cmd-p` includes threads alongside files |
| Click sidebar thread → swaps panel view | Select finder result → opens thread in new tab |
| Single "selected agent" per panel | Per-tab agent/model/profile selection |
| Panel toolbar with global controls | Inline controls per agent tab (already in message editor chrome) |
| `cmd-shift-a` to focus agent panel | `cmd-p` for everything; `#` prefix to filter threads |

---

## 1. Unified File Finder: Files + Threads

### 1.1 One Picker, Two Result Types

Extend the existing `cmd-p` file finder to include agent thread results alongside file results. No separate modal, no separate shortcut.

**Empty query (before typing):**

```
┌─────────────────────────────────────────────────────────────┐
│  Search files and threads...                                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Recent Files                                               │
│  ─────────────                                              │
│  📄 src/main.rs                    project/src              │
│  📄 Cargo.toml                     project/                 │
│  📄 lib.rs                         project/crates/agent_ui  │
│                                                             │
│  Recent Agent Sessions                                       │
│  ─────────────────                                          │
│  🤖 Fix login auth bug             2 hours ago              │
│  🤖 Refactor thread metadata       yesterday                │
│  🤖 New Agent Session              ➕ create new             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

When the query is empty, results appear in **labeled sections** (Recent Files, Recent Agent Sessions), but **arrow-key navigation is linear across all sections** — no need to tab between groups. Down-arrow from the last file goes to the first thread; up-arrow from the first thread goes to the last file.

**After typing (e.g. `auth`):**

```
┌─────────────────────────────────────────────────────────────┐
│  auth                                                       │
├─────────────────────────────────────────────────────────────┤
│  📄 src/auth/login.rs              project/src              │  ← high file score
│  📄 src/auth/middleware.rs         project/src              │  ← high file score
│  🤖 Fix login auth bug             2 hours ago              │  ← high thread score
│  📄 tests/auth_test.rs             project/tests            │  ← lower file score
│  🤖 Auth service refactor          3 days ago               │  ← lower thread score
│  📄 auth.rs                        project/crates/auth       │  ← lowest file score
└─────────────────────────────────────────────────────────────┘
```

Results are **interleaved by score** with no section headers once the user types. The ranking order is:

1. Currently opened file (if matching — existing behavior)
2. High-scoring file matches
3. High-scoring thread matches
4. Lower-scoring file matches
5. Lower-scoring thread matches

This ensures that exact/near-exact file matches still rise to the top (preserving muscle memory for `cmd-p main.rs`), but threads with strong title matches appear right below them rather than buried.

### 1.1b Create-from-query Actions

When the user types a query that doesn't match any existing file or thread, the finder shows **two** creation actions at the bottom — one for files (existing behavior) and one for agent sessions (new):

```
┌─────────────────────────────────────────────────────────────┐
│  fix the auth middleware                                    │
├─────────────────────────────────────────────────────────────┤
│  No matches found                                          │
│                                                             │
│  📄 Create file: fix the auth middleware                    │
│  🤖 Start agent session: fix the auth middleware            │
└─────────────────────────────────────────────────────────────┘
```

Selecting "Start agent session" creates a new `AgentSessionItem` tab with the query text pre-filled as the first message in the message editor. The user can immediately hit Enter (or modify the message first). This mirrors how `Match::CreateNew` works for files today.

Both creation entries always appear when there are no matches. When there *are* matches, they appear at the very bottom of the list (after all scored results), same as `CreateNew` does today. This means:

- `cmd-p` → type something unique → ↓ ↓ → Enter = create a file (existing)
- `cmd-p` → type something unique → ↓ ↓ ↓ → Enter = start an agent session with that message (new)
- `# something unique` → ↓ → Enter = start an agent session (only creation option in thread-only mode)

In `#` (thread-only) mode, the "Create file" option is hidden and only "Start agent session" appears. In `$` (file-only) mode, only "Create file" appears (existing behavior).

### 1.2 Prefix Filters

| Prefix | Meaning | Effect |
|--------|---------|--------|
| `#` | Thread-only mode | Only search agent threads; hide file results entirely |
| `$` | File-only mode | Only search files; hide thread results entirely (redundant but explicit) |
| (no prefix) | Unified search | Interleave files + threads by score |

Examples:
- `# fix auth` → shows only threads matching "fix auth"
- `$ main.rs` → shows only files matching "main.rs" (same as today)
- `auth` → shows both files and threads matching "auth", interleaved by score

When a prefix is detected, it's stripped from the query before fuzzy matching, and the placeholder text updates to reflect the filtered mode:

```
┌─────────────────────────────────────────────────────────────┐
│  # fix auth bug                                             │
├─────────────────────────────────────────────────────────────┤
│  (searching agent sessions only)                            │
│  🤖 Fix login auth bug             2 hours ago              │
│  🤖 Auth service refactor          3 days ago               │
│  🤖 New Agent Session              ➕ create new             │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 Visual Differentiation

Thread results are visually distinct from file results:

| Aspect | File Row | Thread Row |
|--------|----------|------------|
| **Icon** | File type icon (from `FileIcons`) | `🤖` XenomorphicAssistant icon (accent color) |
| **Primary label** | File name | Thread title |
| **Secondary label** | Directory path | Worktree chips + relative timestamp |
| **End slot** | (none for search results; history icon for recent) | Relative timestamp ("2h ago") |

In `render_match()`:

```rust
Match::Thread(thread_match) => {
    ListItem::new(ix)
        .start_slot(Icon::new(IconName::XenomorphicAssistant).color(Color::Accent))
        .child(
            h_flex().gap_2()
                .child(HighlightedLabel::new(thread_match.title, thread_match.positions))
                .child(worktree_chips(thread_match.worktree_paths))
        )
        .end_slot(Label::new(thread_match.relative_time).color(Color::Muted).size(LabelSize::Small))
}

Match::CreateSession(query) => {
    ListItem::new(ix)
        .start_slot(Icon::new(IconName::XenomorphicAssistant).color(Color::Muted))
        .child(Label::new(format!("Start agent session: {}", query)))
}

Match::NewSession => {
    ListItem::new(ix)
        .start_slot(Icon::new(IconName::Plus).color(Color::Accent))
        .child(Label::new("New Agent Session").color(Color::Accent))
}

Match::SectionHeader(label) => {
    // Non-selectable visual group label
    ListSubHeader::new(label).inset(true)
}
```

### 1.4 Implementation: Extending `FileFinderDelegate`

**File:** `crates/file_finder/src/file_finder.rs`

Add a `Match::Thread` variant to the existing `Match` enum:

```rust
enum Match {
    History {
        path: FoundPath,
        panel_match: Option<ProjectPanelOrdMatch>,
    },
    Search(ProjectPanelOrdMatch),
    CreateNew(ProjectPath),
    Thread(ThreadMatch),       // ← NEW: existing thread result
    CreateSession(String),    // ← NEW: "Start agent session: <query>"
    SectionHeader(&'static str), // ← NEW: non-selectable group label
    NewSession,               // ← NEW: "New Agent Session" (empty-query)
}
```

**`ThreadMatch` struct** (defined in a shared crate or in `file_finder` itself):

```rust
struct ThreadMatch {
    thread_id: ThreadId,
    session_id: Option<schema::SessionId>,
    title: SharedString,
    worktree_paths: WorktreePaths,
    relative_time: SharedString,
    score: f64,
    positions: Vec<usize>,
}
```

**Query parsing** in `update_matches()`:

```rust
fn update_matches(&mut self, raw_query: String, window, cx) -> Task<()> {
    let (mode, query) = match raw_query.strip_prefix('#') {
        Some(rest) => (SearchMode::ThreadsOnly, rest.trim().to_string()),
        None => match raw_query.strip_prefix('$') {
            Some(rest) => (SearchMode::FilesOnly, rest.trim().to_string()),
            None => (SearchMode::Unified, raw_query.clone()),
        }
    };
    
    // Existing file search (if mode != ThreadsOnly)
    // ...
    
    // NEW: Thread search (if mode != FilesOnly)
    if mode != SearchMode::FilesOnly {
        let thread_matches = search_threads(query.clone(), cx);
        // Insert into self.matches alongside file matches
    }
    
    // NEW: Creation entries when there are no matches (or at bottom of list)
    let has_file_matches = self.matches.iter().any(|m| matches!(m, Match::History{..} | Match::Search(_)));
    let has_thread_matches = self.matches.iter().any(|m| matches!(m, Match::Thread(_)));
    
    if mode != SearchMode::ThreadsOnly && !has_file_matches {
        self.matches.push(Match::CreateNew(project_path));  // existing
    }
    if mode != SearchMode::FilesOnly && !has_thread_matches {
        self.matches.push(Match::CreateSession(query.clone()));  // NEW
    }
}
```

The `Match::CreateSession` confirm handler:

```rust
Match::CreateSession(query) => {
    let item = create_agent_session_item_with_initial_message(
        workspace,
        query.clone(),  // pre-filled as first message
        window,
        cx,
    );
    workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
}
```

**Dependency issue:** `file_finder` doesn't depend on `agent_ui` or `ThreadMetadataStore`. Solutions:

| Option | How | Trade-off |
|--------|-----|-----------|
| **A. Trait-based provider** | `FileFinderDelegate` holds `Vec<Box<dyn FinderProvider>>`; `agent_ui` registers a `ThreadFinderProvider` at init | Clean separation; extensible to other types later; more architecture work |
| **B. Move `ThreadMetadata` to shared crate** | Move to e.g. `crates/agent_thread/src/metadata.rs` so both `file_finder` and `agent_ui` can access it | Simpler; but `file_finder` now depends on `agent_thread` |
| **C. Query callback** | `FileFinderDelegate` stores an `Option<Arc<dyn Fn(&str) -> Vec<ThreadMatch>>>`; set at init time by `agent_ui` | Minimal coupling; callback-based; slightly awkward |

**Recommendation: Option A.** A trait-based provider system is the right long-term architecture. It also opens the door for other result types (symbols, commands, etc.) without further modifying the finder.

```rust
pub trait FinderProvider: Send + Sync + 'static {
    /// Label shown in the empty-query section header.
    fn section_label(&self) -> &'static str;
    
    /// Whether this provider supports the given search mode.
    fn supports_mode(&self, mode: SearchMode) -> bool;
    
    /// Return matches for the given query. Called on a background thread.
    fn search(&self, query: &str, cx: &App) -> Vec<FinderMatch>;
    
    /// Return recent items for the empty-query state.
    fn recent_items(&self, cx: &App) -> Vec<FinderMatch>;
    
    /// Open the selected match.
    fn confirm(&self, match: &FinderMatch, secondary: bool, workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>);
}
```

### 1.5 Empty-Query State: Sectioned Display

When the query is empty, the finder shows sectioned results. The `Matches` struct already supports `separate_history: bool` which separates history from search results. We extend this concept:

```rust
fn push_new_matches(&mut self, ...) {
    if query.is_empty() {
        self.matches.clear();
        
        // Section: Recent Files
        self.matches.push(Match::SectionHeader("Recent Files"));
        self.matches.extend(history_items.map(Match::History));
        
        // Section: Recent Agent Sessions
        self.matches.push(Match::SectionHeader("Recent Agent Sessions"));
        self.matches.extend(thread_provider.recent_items(cx).map(Match::Thread));
        
        // Section: New Agent Session (always present)
        self.matches.push(Match::NewSession);
    }
}
```

**Section headers are non-selectable** — they're visual groupings only. Arrow keys skip over them. The user navigates linearly: file → file → file → thread → thread → "New Session".

### 1.6 "New Agent Session" Entry

Always present as the **last entry in the threads section** (both in empty-query and `#`-prefixed modes). Selecting it and pressing Enter creates a new agent session tab.

```rust
Match::NewSession => {
    ListItem::new(ix)
        .start_slot(Icon::new(IconName::Plus).color(Color::Accent))
        .child(Label::new("New Agent Session").color(Color::Accent))
}
```

---

## 2. Creating New Agent Sessions

### 2.1 Entry Points

| Method | How | UX |
|--------|-----|-----|
| **`cmd-p` → Enter** | "New Agent Session" is always the last entry when query is empty (or in `#` mode) | Opens new tab with message editor focused |
| **`cmd-p` → type → ↓ to bottom → Enter** | "Start agent session: <typed query>" appears when no thread matches | Opens new tab with typed text pre-filled as first message |
| **`cmd-p` → `#` → type → Enter** | Same but filtered to threads only | Same |
| **Command palette** | `cmd-shift-p` → "New Agent Thread" | Same as today but opens a tab |
| **Contextual** | Right-click project panel → "Ask Agent About This" | Opens thread tab with context pre-filled |

**No separate shortcut.** `cmd-p` is the universal entry point. The "New Agent Session" entry is always one `Enter` away when you haven't typed anything.

### 2.2 Implementation

```rust
// In FileFinderDelegate::confirm:
Match::NewSession => {
    let item = create_new_agent_session_item(workspace, window, cx);
    workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
}
Match::Thread(thread_match) => {
    let item = load_agent_session_item(&thread_match, workspace, window, cx);
    workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
}
```

---

## 3. Per-Tab Controls: Replacing the Panel Toolbar

### 3.1 The Problem

Current agent panel controls include:
- Model selector (which LLM)
- Profile selector (which agent profile)
- Options menu (MCP servers, settings, profiles)
- Thread title editor
- Back button (for configuration overlay)
- New thread menu
- Zoom controls

These are currently rendered by `AgentPanel::render_toolbar()` and apply globally to whatever thread is active in the panel.

With per-tab sessions, these need to move **into each tab's content or toolbar**.

### 3.2 Where Each Control Goes

| Control | Current Location | New Location | Mechanism |
|---------|-----------------|--------------|-----------|
| **Model selector** | Panel toolbar | Message editor chrome (per-tab) | Inline in `ConversationView` — already there in `ThreadView::render_message_editor()` |
| **Profile selector** | Panel toolbar | Message editor chrome (per-tab) | Same — already there |
| **Thread title** | Panel toolbar | Tab title | `Item::tab_content_text()` — double-click to edit inline |
| **Regenerate title** | Options menu | Tab context menu | `Item::tab_extra_context_menu_actions()` |
| **Open as Markdown** | Options menu | Tab context menu | Same |
| **Copy thread** | Options menu | Tab context menu | Same |
| **Archive thread** | Options menu | Tab context menu | Same |
| **New session** | Panel toolbar / sidebar buttons | `cmd-p` → "New Agent Session" entry | One Enter away |
| **Zoom** | Panel zoom | Pane zoom | Built into pane |
| **Agent configuration** (MCP, profiles, provider settings) | Panel overlay / modal | **`cmd-,` settings page — consolidated into existing AI page** | Moved to global settings; no per-tab or modal needed |
| **Terminal (in agent)** | Panel tab-like list | Separate terminal tabs | Already `TerminalView` items |

**Key decision:** MCP servers, LLM provider configuration, profiles, and agent settings are **global/per-user**, not per-thread. They belong in the global settings (`cmd-,`) alongside existing agent settings like "Disable AI", "Tool Permissions", etc. There is no need for a separate modal or per-tab access to these.

### 3.3 No Per-Tab Toolbar Needed for Model/Profile Selectors

The model selector and profile selector are **already rendered inline** in the message editor chrome by `ThreadView::render_message_editor()` (at the bottom of the conversation, next to the send button). They don't need to move to the pane toolbar. This is actually better UX — the controls are right where you interact with them (above the input box), not at the top of the pane.

This means **no `AgentTabToolbar` is needed**. The pane toolbar stays clean, showing only breadcrumbs (thread title) and the standard search/navigation items.

```
┌────────────────────────────────────────────────────────────┐
│  my-project > src > main.rs              [breadcrumb bar]   │  ← standard pane toolbar
├────────────────────────────────────────────────────────────┤
│                                                            │
│  [messages scroll here]                                    │
│                                                            │
│  ---                                                       │
│  [add context] [follow] [fast] [thinking]    [token usage] │  ← message editor chrome
│  [profile: Default ▼] [model: GPT-4o ▼]          [Send]   │  ← already here!
│  [Type a message...                                   ]   │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

The only things the tab itself needs to show are:
1. **Tab title** — thread title; double-click to edit
2. **Tab icon** — model-specific icon (for distinguishing tabs at a glance)
3. **Context menu** — thread-level actions (archive, copy, open as markdown, regenerate title)

### 3.4 Tab Context Menu

When right-clicking an agent tab:
```
┌──────────────────────────────┐
│  Close Tab                   │
│  Close Other Tabs            │
│  Split Right                 │
│  --------------------------  │
│  Regenerate Thread Title     │
│  Copy Thread to Clipboard    │
│  Open Thread as Markdown     │
│  --------------------------  │
│  Archive Thread              │
└──────────────────────────────┘
```

Implemented via `Item::tab_extra_context_menu_actions()`.

---

## 4. Consolidating Agent Settings into Global Settings (`cmd-`,`)

### 4.1 Why Move Settings?

Currently agent settings are split across two surfaces:

| Setting Category | Current Location | Global Settings Already Has |
|------------------|-----------------|---------------------------|
| LLM Providers (API keys, sign-in) | `AgentConfiguration` modal/page | Nothing (separate) |
| MCP Servers (add/remove/configure) | `AgentConfiguration` modal/page | Nothing (separate) |
| Agent Profiles (create/edit/delete) | `ManageProfilesModal` | Nothing (separate) |
| Tool Permissions (allow/deny patterns) | Sub-page in `ai_page` | ✅ `agent.tool_permissions` |
| Disable AI | `AgentConfiguration` | ✅ `disable_ai` |
| Threads Sidebar Side | `AgentSettings` | ✅ `agent.sidebar_side` |
| Single File Review | `AgentSettings` | ✅ `agent.single_file_review` |
| Enable Feedback | `AgentSettings` | ✅ `agent.enable_feedback` |
| Notify When Agent Waiting | `AgentSettings` | ✅ `agent.notify_when_agent_waiting` |
| Play Sound When Agent Done | `AgentSettings` | ✅ `agent.play_sound_when_agent_done` |
| Expand Edit Card | `AgentSettings` | ✅ `agent.expand_edit_card` |
| Thinking Display | `AgentSettings` | ✅ `agent.thinking_display` |
| Cancel Generation On Terminal Stop | `AgentSettings` | ✅ `agent.cancel_generation_on_terminal_stop` |
| Use Modifier To Send | `AgentSettings` | ✅ `agent.use_modifier_to_send` |
| Message Editor Min Lines | `AgentSettings` | ✅ `agent.message_editor_min_lines` |

The bottom 12+ settings are **already in the global settings page** (`Settings → AI`). Only the top 3 categories (providers, MCP, profiles) live in separate modals/pages. Moving them into the same settings page consolidates everything into one surface.

### 4.2 What Moves Where

**`Settings → AI` page additions:**

```
┌────────────────────────────────────────────────────────────┐
│  Settings                                     [x]          │
├────────────────────────────────────────────────────────────┤
│  [General] [Appearance] [Editor] [...] [AI] [Network]      │
│                                                            │
│  AI                                                        │
│  ──────────────────────────────────────────────────────    │
│  General                                                   │
│    [ ] Disable AI                                          │
│    Threads Sidebar Side: [Left ▼]                          │
│                                                            │
│  NEW: LLM Providers  ──────────────────────────────────    │
│    OpenAI        [Sign In] [Configure ▼]                   │
│    Anthropic     [Signed In ✓] [Configure ▼]               │
│    Ollama        [Configure ▼]                             │
│    [+ Add Provider]                                        │
│                                                            │
│  NEW: MCP Servers  ──────────────────────────────────────  │
│    filesystem    [Running ▼] [Configure] [Uninstall]       │
│    github        [Stopped ▼] [Configure] [Uninstall]       │
│    [+ Add Server] [Install from Extensions]                │
│                                                            │
│  NEW: Agent Profiles  ───────────────────────────────────  │
│    Default     [Active] [Edit ▼]                           │
│    Code Review [        ] [Edit ▼] [Duplicate] [Delete]    │
│    [+ New Profile]                                         │
│                                                            │
│  Agent Configuration                                       │
│    Tool Permissions >                                      │
│    Single File Review [✓]                                  │
│    Enable Feedback [✓]                                     │
│    ...                                                     │
└────────────────────────────────────────────────────────────┘
```

### 4.3 Implementation: Settings Page Integration

**File:** `crates/settings_ui/src/page_data.rs` — extend `ai_page()`

```rust
fn ai_page(cx: &App) -> SettingsPage {
    // Existing sections: general_section(), agent_configuration_section()
    // NEW sections:

    let llm_providers_section = llm_providers_section(cx);
    let mcp_servers_section = mcp_servers_section(cx);
    let agent_profiles_section = agent_profiles_section(cx);

    let mut items = vec![
        SettingsPageItem::SectionHeader("General"),
        // ... existing general items ...
    ];

    items.push(SettingsPageItem::SectionHeader("LLM Providers"));
    items.extend(llm_providers_section);

    items.push(SettingsPageItem::SectionHeader("MCP Servers"));
    items.extend(mcp_servers_section);

    items.push(SettingsPageItem::SectionHeader("Agent Profiles"));
    items.extend(agent_profiles_section);

    items.push(SettingsPageItem::SectionHeader("Agent Configuration"));
    items.extend(agent_configuration_section(cx));

    SettingsPage { title: "AI", items: items.into_boxed_slice() }
}
```

**LLM Providers section:** Reuses `AgentConfiguration::render_provider_configuration_section()` logic — iterates visible providers from `LanguageModelRegistry`, shows their configuration blocks (API key inputs, sign-in buttons, etc.).

**MCP Servers section:** Reuses `AgentConfiguration::render_context_servers_section()` — lists servers from `ContextServerStore`, shows status, configure/uninstall buttons.

**Agent Profiles section:** Extract the chooser from `ManageProfilesModal` into a simpler settings component — list profiles with name, active indicator, edit/duplicate/delete actions. Clicking "Edit" opens a sub-page (same pattern as "Tool Permissions" sub-page today).

### 4.4 Profile Editing Sub-Page

Profiles need a rich editing UI (model picker, tool picker, MCP picker). The existing `ManageProfilesModal` is already a multi-screen editor. Convert it to a **settings sub-page**:

```rust
// In settings_ui/src/pages/agent_profile_setup.rs
pub fn render_agent_profile_setup_page(
    profile_id: AgentProfileId,
    settings_content: &mut SettingsContent,
    window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> Vec<SettingsPageItem> {
    vec![
        SettingsPageItem::SectionHeader("Profile Settings"),
        SettingsPageItem::Custom(render_profile_name_editor(profile_id)),
        SettingsPageItem::Custom(render_default_model_selector(profile_id)),
        SettingsPageItem::Custom(render_tool_picker(profile_id)),
        SettingsPageItem::Custom(render_mcp_picker(profile_id)),
    ]
}
```

This mirrors how `render_tool_permissions_setup_page` works today — a dedicated sub-page with custom rendered UI.

### 4.5 Actions that Currently Open Configuration

| Action | Current Behavior | New Behavior |
|--------|-----------------|--------------|
| `OpenSettings` (from agent panel) | Focuses agent panel, opens configuration overlay | Opens `cmd-,` settings, navigates to AI page |
| `ManageProfiles` | Opens `ManageProfilesModal` | Opens `cmd-,` settings, navigates to AI → Profiles section |
| `AddContextServer` | Opens `ConfigureContextServerModal` | Opens `cmd-,` settings, navigates to AI → MCP Servers, focuses "Add Server" |

### 4.6 What Gets Deleted

| Code | Why Delete |
|------|-----------|
| `AgentConfiguration` struct and overlay | Replaced by settings page sections |
| `ManageProfilesModal` | Replaced by settings sub-page |
| `ConfigureContextServerModal` | Replaced by inline add/configure in settings |
| `AgentPanel::configuration` field | No overlay state needed |
| `AgentPanel::overlay_view` enum | No overlays at all |
| `AgentPanel::go_back()` | No navigation stack |
| `AgentPanel::render_toolbar_back_button()` | No back button needed |
| `AgentPanel::open_configuration()` | Replaced by `window.dispatch_action(OpenSettings, cx)` |

---

## 5. Thread Metadata and Discovery

### 5.1 What's Searchable

`ThreadMetadata` already contains everything needed for finder integration:

```rust
pub struct ThreadMetadata {
    pub thread_id: ThreadId,
    pub session_id: Option<schema::SessionId>,
    pub agent_id: AgentId,          // "native_agent" or custom
    pub title: Option<SharedString>,
    pub updated_at: DateTime<Utc>,   // For sorting
    pub worktree_paths: WorktreePaths, // For filtering by project
    pub archived: bool,
}
```

### 5.2 Filtering by Current Project

When the user is in workspace A, should they see threads from workspace B?

| Option | Behavior |
|--------|----------|
| **Project-scoped** (default) | Only show threads whose `worktree_paths` intersect with current workspace's worktrees |
| **Global** | Show all threads, with worktree indicators for context |
| **Smart** | Prioritize current project, show others below a separator |

**Recommendation: Smart** — same pattern as command palette's "This Window" vs "Recent Projects" sections in the project picker.

### 5.3 Pre-loading Thread Titles

Currently `ThreadsArchiveView` shows rich metadata. For the finder, we need fast list access. `ThreadMetadataStore` already persists to SQLite and supports listing all entries. The finder would:

1. Load `ThreadMetadataStore::global(cx).read(cx).entries()` (cached in memory)
2. Filter by current workspace's worktree paths
3. Run fuzzy match on titles
4. Display with `ThreadItem` rows

---

## 6. What Happens to the Sidebar?

### 6.1 Complete Removal

The `crates/sidebar/src/sidebar.rs` `Sidebar` (the thread list) becomes unnecessary. Threads are accessed via the file finder or open tabs.

**Benefits:**
- Less screen real estate consumed by default
- No need to maintain a separate sidebar state (width, scroll position, selection)
- One less panel to serialize/deserialize
- Consistent with "files as primary artifacts" mental model

### 6.2 Optional: Mini Thread Overview

If users want a visible thread list, it could be:
- A toggle in the **status bar** showing "3 agent sessions" that opens `cmd-p` with `#` pre-filled
- Part of a "workspace overview" panel (like VS Code's outline view)

### 6.3 ThreadSwitcher

There's already `ThreadSwitcher` in `crates/sidebar/src/thread_switcher.rs`. This could be repurposed as a **tab switcher** (like `cmd-tab` but for agent session tabs within the pane):

```rust
// cmd-` opens thread switcher (cycle open agent tabs in this pane)
```

---

## 7. Implementation Roadmap

### Phase 1: Foundation (Prerequisite)

Before the finder integration, agent sessions must first-class `Item`s. See the previous report (`agent_sessions_as_tabs_report.md`) for details. Briefly:

1. Create `AgentSessionItem` wrapper in new file
   - Implements `Item` with minimal behavior
   - Delegates to existing `ConversationView`
   - `tab_content_text` uses thread title
   - `can_split` returns false initially

2. Add open-as-tab support without removing panel
   - New command/action: `OpenThreadInTab`
   - Opens clicked thread as `AgentSessionItem` in active pane
   - Original panel behavior stays untouched

3. Test serialization for `AgentSessionItem`
   - Save/restore a single tab
   - Verify thread state is preserved

### Phase 2: Move Settings to Global Settings Page

1. **Extract provider config UI from `AgentConfiguration`**
   - Make `render_provider_configuration_section()` a standalone component
   - Add to `settings_ui/src/page_data.rs::ai_page()`

2. **Extract MCP server UI from `AgentConfiguration`**
   - Make `render_context_servers_section()` a standalone component
   - Add to AI settings page

3. **Convert `ManageProfilesModal` to settings sub-page**
   - Create `settings_ui/src/pages/agent_profile_setup.rs`
   - Register as sub-page renderer
   - Add profile list + "New Profile" button to AI settings page

4. **Delete `AgentConfiguration` overlay**
   - Remove `AgentPanel::configuration`, `overlay_view`, back button
   - `OpenSettings` action now opens global settings at AI page

### Phase 3: Unified File Finder

1. **Add `FinderProvider` trait** in `crates/file_finder/`
   - Define `FinderProvider`, `FinderMatch`, `SearchMode` types
   - Refactor `FileFinderDelegate` to hold `Vec<Box<dyn FinderProvider>>`
   - Existing file search becomes `FileFinderProvider` implementing the trait

2. **Implement `ThreadFinderProvider`** in `crates/agent_ui/`
   - Queries `ThreadMetadataStore`, fuzzy matches on title
   - Returns `FinderMatch::Thread` results with icon, chips, timestamp
   - Implements `confirm()` to open thread as tab or create new session

3. **Add `Match::Thread` and `Match::SectionHeader` variants**
   - Extend `Match` enum in `file_finder.rs`
   - Implement `render_match()` for thread rows
   - Implement `confirm()` for thread selection

4. **Parse `#` and `$` prefixes**
   - Strip prefix from query
   - Set `SearchMode` to filter providers
   - Update placeholder text to reflect filtered mode

5. **Sectioned empty-query display**
   - Show "Recent Files" and "Recent Agent Sessions" sections
   - Section headers are non-selectable, arrow keys skip them
   - Always append "New Agent Session" at bottom of thread section

6. **Create-from-query actions**
   - Add `Match::CreateSession(String)` variant (mirrors `Match::CreateNew`)
   - When no thread matches exist, show "Start agent session: <query>" at bottom of results
   - When no file matches exist, show "Create file: <query>" (existing behavior)
   - Both appear together when neither files nor threads match
   - In `#` mode, only CreateSession appears; in `$` mode, only CreateNew appears
   - Confirming CreateSession opens a new tab with the query pre-filled as the first message

6. **Interleaved scoring**
   - Sort by: high-score files > high-score threads > low-score files > low-score threads
   - Thread matches use fuzzy score on title only (no path matching)

### Phase 4: Full Integration

7. **Enable split/drag for `AgentSessionItem`**
   - Implement `clone_on_split` (shared thread approach)
   - Test drag-and-drop between panes

8. **Thin the `AgentPanel`**
   - Remove toolbar, base_view, overlays, configuration
   - Remove `cmd-shift-a` action registration entirely

9. **Migrate terminal handling**
   - Agent panel terminals become regular `TerminalView` items

### Phase 5: Remove Sidebar

10. **Remove `ToggleWorkspaceSidebar`** thread list UI
    - Delete sidebar thread list rendering
    - Remove `AgentPanel` dock registration
    - Repurpose `ThreadArchiveView` as the `ThreadFinderProvider` backend

11. **Update serialization**
    - Remove sidebar width/state from workspace serialization
    - Ensure open agent tabs serialize via `SerializableItem`

12. **Update welcome page / onboarding**
    - Remove references to "agent panel button"
    - Add "Press `cmd-p` then Enter to start chatting with the agent"

---

## 8. Open Questions

### 8.1 Where Do New Users Start?

First-time users already know `cmd-p`. The "New Agent Session" entry is right there at the bottom of the empty-query results. Options for additional discoverability:
- **Welcome page** includes a prominent "Start New Agent Chat" button (which dispatches `ToggleFileFinder`)
- **Empty workspace** shows a hint: "Press `cmd-p` and Enter to chat with the agent"
- **Command palette** has "New Agent Thread" (for users who think in terms of commands)

### 8.2 Thread Listing Without Memorizing Names

If you have 50 threads, remembering titles to fuzzy-find is hard. Solutions:
- **Empty-query shows recents**: The sectioned empty-query display shows recent threads sorted by `updated_at`, just like the sidebar does today
- **Time-bucketed `#` mode**: When filtering with `#`, group results by Today / Yesterday / This Week (reuse `ThreadsArchiveView` bucketing)
- **Workspace-scoped by default**: Only show threads relevant to current project

### 8.3 Agent Panel Terminals

Currently agent panel has embedded terminals (`HashMap<TerminalId, AgentTerminal>`). Without a panel:
- Agent terminals become regular `TerminalView` tabs
- They can be associated with a thread (e.g., thread tab shows a "terminal" button to spawn one)
- Or terminals are entirely separate, and the user uses the regular terminal panel

### 8.4 Inline Assistant Integration

The inline assistant (`InlineAssistant`) currently registers with the workspace and can spawn agent threads. With the panel gone:
- Inline assistant still works (it's triggered from editor context menu / keybinding)
- It opens a new agent session tab instead of panel
- The inline assistant's thread is just another tab

### 8.5 Status Bar Indicator

Today the status bar can show an agent panel button. Without a panel:
- Remove the button entirely (finder-only access)
- Or replace with a count of open agent sessions (click opens `cmd-p` with `#` pre-filled)

---

## 9. Comparison: Sidebar vs. Unified Finder

| Aspect | Sidebar (Current) | Unified Finder + Tabs (Proposed) |
|--------|-------------------|----------------------------------|
| **Discoverability** | Visual, always visible | Same shortcut as files (`cmd-p`) |
| **Screen space** | Sidebar width (~250px) | Zero (modal) |
| **Multi-project threads** | Shown with worktree labels | Shown with worktree labels in finder |
| **Creating threads** | Click "+" button | `cmd-p` → Enter (one step) |
| **Switching threads** | Click in sidebar | `cmd-p` → type → Enter, or `cmd-tab` |
| **Closing threads** | Click ✕ in sidebar list | Close tab (familiar) |
| **Filtering to threads** | Sidebar shows only threads | `#` prefix |
| **Filtering to files** | N/A (sidebar only shows threads) | `$` prefix or no prefix |
| **Consistency with files** | Poor (different model) | Excellent (same picker, same shortcut) |
| **Learning curve** | Low | Low (reuses `cmd-p` muscle memory) |
| **Power user efficiency** | Medium | High (keyboard-centric, unified) |

---

## 10. Files to Modify (Updated)

### New Files
| File | Purpose |
|------|---------|
| `crates/agent_ui/src/agent_session_item.rs` | `Item` impl for agent sessions |
| `crates/agent_ui/src/thread_finder_provider.rs` | `FinderProvider` impl for agent threads |
| `crates/file_finder/src/provider.rs` | `FinderProvider` trait, `FinderMatch`, `SearchMode` types |
| `crates/settings_ui/src/pages/agent_profile_setup.rs` | Profile editing as settings sub-page |

### Major Modifications
| File | Changes |
|------|---------|
| `crates/file_finder/src/file_finder.rs` | Add `Match::Thread`, `Match::SectionHeader`, `Match::NewSession`; parse `#`/`$` prefixes; hold `Vec<Box<dyn FinderProvider>>`; interleaved scoring; sectioned empty-query display |
| `crates/agent_ui/src/agent_panel.rs` | Remove toolbar, base_view, overlays, configuration; remove `cmd-shift-a` registration |
| `crates/agent_ui/src/conversation_view.rs` | Add event emission for `ItemEvent` integration |
| `crates/agent_ui/src/agent_ui.rs` | Export `ThreadFinderProvider`, register with file finder |
| `crates/settings_ui/src/page_data.rs` | Add LLM Providers, MCP Servers, Profiles to AI page |
| `crates/settings_ui/src/pages.rs` | Export new profile setup page |
| `crates/sidebar/src/sidebar.rs` | Remove thread list, keep only if repurposing |
| `crates/workspace/src/status_bar.rs` | Remove agent panel button or replace with `cmd-p #` trigger |

### Deletions
| File / Code | Reason |
|-------------|--------|
| `AgentConfiguration` struct | Replaced by settings page sections |
| `ManageProfilesModal` | Replaced by settings sub-page |
| `ConfigureContextServerModal` | Replaced by inline settings UI |
| `AgentPanel::configuration` field | No overlay state |
| `AgentPanel::overlay_view` enum | No overlays |
| `AgentPanel::base_view` | Pane manages active item |
| `AgentPanel::render_toolbar()` | Controls are inline or moved to settings |
| `AgentPanel::terminals` | Use regular `TerminalView` items |
| `ToggleFocus`, `Toggle` actions | Replaced by `cmd-p` |
| Threads sidebar UI | Replaced by unified finder |
| `cmd-shift-a` shortcut | Removed — `cmd-p` handles everything |

---

## 11. Summary

This design eliminates the `AgentPanel` entirely as a dock panel with a sidebar. Instead:

1. **Agent sessions are tabs** — draggable, splittable, closable, just like editors
2. **Discovery is unified** — `cmd-p` searches files and threads together; `#` prefix filters to threads only, `$` to files only
3. **Empty-query shows sections** — Recent Files and Recent Agent Sessions as labeled groups, with linear arrow-key navigation and "New Agent Session" always one Enter away
4. **Typed queries interleave by score** — high-scoring files above high-scoring threads above low-scoring files
5. **Controls are inline** — model/profile selectors already live in the message editor chrome; no separate toolbar needed
6. **Settings are global** — MCP servers, LLM providers, and agent profiles move to the `cmd-,` settings page alongside existing agent settings

The result is a **simpler, more consistent mental model**: `cmd-p` is the single entry point for everything in your workspace — files and agent sessions alike.

---

## Appendix: Edge Cases and Implementation Details

### A.1 Thread Naming and Tab Identities

**Problem:** Before the first title is generated, every new thread is titled `DEFAULT_THREAD_TITLE` ("New Agent Thread"). Multiple untitled tabs would be indistinguishable.

**Solutions:**

| Approach | How | Trade-off |
|----------|-----|-----------|
| **Auto-generate title on first message** | After user sends first message, fire a lightweight LLM call for a title | Burns tokens; adds latency to first response |
| **User-provided name on creation** | Finder's "New Session" puts cursor in editor; user names thread on `cmd-enter` | Extra keystroke; many users won't bother |
| **Subtitle from first message** | Tab shows: `"New Agent Thread — how do I use entities?"` | Clever; works without LLM call; truncates elegantly |
| **Disambiguation counter** | `"New Agent Thread (2)"`, `"(3)"` | Ugly; doesn't help recall |

**Recommendation: Subtitle approach.** `tab_content_text()` returns `"New Agent Thread"` as primary and the first ~30 chars of the first user message as secondary/clipped detail. Once the LLM generates a proper title, the subtitle disappears.

```rust
fn tab_content_text(&self, detail: usize, cx: &App) -> SharedString {
    if let Some(title) = self.thread.read(cx).title() {
        return title;
    }
    if let Some(first_msg) = self.thread.read(cx).first_user_message() {
        return format!("New Agent Thread — {}", first_msg.chars().take(30).collect::<String>()).into();
    }
    DEFAULT_THREAD_TITLE.into()
}
```

### A.2 Splitting and Shared Thread State

When an agent tab is split, `clone_on_split()` creates a second `ConversationView`.

**Key question:** Do both views share the same `AgentThread` entity?

**Decision: Yes, share the entity.** This is the same model as splitting an editor: both panes see the same buffer live. The `AgentThread` is the "buffer"; each `ConversationView` is a "view" with its own message editor (like each editor has its own cursor).

Both views subscribe to `AgentThreadEvent`s from the shared entity. When a message is sent from either pane:
1. The sending pane's `message_editor` clears its draft
2. `AgentThread` emits `NewEntry`
3. Both panes receive the event and re-render the message list

**Caveat: Streaming state.** If the model is streaming a response, both views show the stream. If one pane is closed, the stream continues (the thread entity is kept alive by the other view). Only when the last view of a thread is closed does cleanup happen.

### A.3 Model/Profile Indicators on Inactive Tabs

**Problem:** If you have three agent tabs using different models, you can't tell which is which without activating each one.

**Solutions:**

| Approach | Example |
|----------|---------|
| **Icon+color coding** | GPT-4o = purple sparkle icon, Claude = yellow diamond icon |
| **Suffix in tab title** | `"Bug fix (GPT-4o)"` — but tab width is limited |
| **Tooltip** | Hover tab → shows full model name and profile |
| **Tab decoration** | Badges or colored dots on tab (not currently supported by `Item`) |

**Recommendation: Icon + tooltip.** Add `tab_icon()` that returns a model-specific icon. GPUI already supports `IconDecoration` on tabs (used for diagnostics, git status). We could add a small model indicator decoration.

### A.4 The `ThreadFinderProvider` vs. `ThreadsArchiveView`

`ThreadsArchiveView` already implements a rich thread browser with:
- Time-bucket grouping (Today, Yesterday, This Week...)
- Fuzzy text filtering
- Worktree chip display
- Keyboard navigation (arrow keys, enter)

The `ThreadFinderProvider` reuses the same data source (`ThreadMetadataStore`) and the same fuzzy matching logic. The visual presentation differs (picker rows vs. full-width list), but the filtering and sorting logic can be shared.

In particular, the empty-query "Recent Agent Sessions" section in the finder should show the same ordering as the sidebar: sorted by `updated_at` desc, with the most recent threads first.

### A.5 Connection Store Ownership

Today `AgentPanel` owns `AgentConnectionStore`. Without a panel:

```rust
// Move to workspace-level global or singleton
impl Workspace {
    fn agent_connection_store(&self, cx: &App) -> Entity<AgentConnectionStore> {
        // Already global? Check agent_ui::init
        AgentConnectionStore::global(cx)
    }
}
```

The store should become a GPUI `Global`, initialized once at startup. Each `ConversationView` requests a connection from the global store by `Agent` type.

### A.6 Status Bar Changes

Current status bar code:
```rust
// crates/workspace/src/status_bar.rs
Tooltip::for_action("Open Threads Sidebar", &ToggleWorkspaceSidebar, cx)
```

Without a sidebar, options for the status bar agent indicator:

| Approach | Visual | Action on Click |
|----------|--------|-----------------|
| **Remove entirely** | Nothing | N/A |
| **Thread count badge** | "🤖 3" | Opens `cmd-p` with `#` pre-filled |
| **Conversation status** | Pulsing dot when streaming | Nothing (just indicator) |

**Recommendation: Thread count badge.** Small, unobtrusive, communicates state. Click opens the file finder with `#` pre-filled (thread-only mode). This gives power users a one-click path to threads while keeping the model unified.

### A.7 External Agent Lifecycle

Custom agents (MCP servers) require a connection. Currently the panel manages these connections and shows loading/disconnected states.

With per-tab connections:
- Each `ConversationView` requests its connection from `AgentConnectionStore`
- Connection states (connecting, connected, error) render **inside the tab content**
- `ConversationView` already handles `ServerState::Loading` and `ServerState::LoadError`
- No panel-level state needed

### A.8 Configuration Overlay

`AgentPanel` currently shows configuration as an overlay (`OverlayView::Configuration`). Without the panel, configuration lives in the global settings page. The `AgentConfiguration` view and its sub-components are reused as settings page sections — no modal needed.

### A.9 Project-Diff and Review Integration

Actions like `ReviewBranchDiff` and `ResolveConflictsWithAgent` currently:
1. Focus the agent panel
2. Create a thread with pre-filled content

In the new model:
```rust
fn review_branch_diff(workspace, action, window, cx) {
    let item = create_agent_session_with_initial_content(
        "Review this diff...",
        window, cx
    );
    workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
}
```

Same behavior, different container.

### A.10 Tab Serialization and Workspace Restore

When restoring a workspace:

1. Deserialize open agent tabs from workspace state
2. For each serialized tab:
   - If `session_id` present → `load_agent_thread(session_id)` → open tab
   - If no `session_id` → was a draft; create new draft tab
3. If no agent tabs were open, don't open any (unlike today where panel always opens)

This gives users a workspace with **only editor tabs** if they never had agent tabs open — a cleaner default.

### A.11 Empty-State Onboarding

First-time user with no threads:

**Current:** Welcome page says "Click the agent panel button" (or shows it in the sidebar by default).

**New:** Welcome page or empty workspace shows:
```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│                   Welcome to Xenomorphic                     │
│                                                              │
│     Open a file          Start chatting        New folder    │
│     (cmd-p)              (cmd-p → Enter)       (cmd-o)       │
│                                                              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

The `Start chatting` action dispatches `ToggleFileFinder`, which opens the picker with the empty-query state showing "New Agent Session" at the bottom. One Enter creates the session.

### A.12 Keyboard Shortcuts Reference

| Shortcut | Action | Context |
|----------|--------|---------|
| `cmd-p` | Open unified file finder | Global |
| `cmd-p` → Enter | New Agent Session | Finder open, empty query |
| `cmd-p` → type → Enter | Open file or thread by score | Finder open |
| `#` prefix | Filter to threads only | Inside finder |
| `$` prefix | Filter to files only | Inside finder |
| `cmd-w` | Close agent tab | Agent tab active |
| `cmd-\` | Split agent tab | Agent tab active |
| `cmd-shift-r` | Regenerate thread title | Agent tab active |
| `cmd-option-m` | Open as Markdown | Agent tab active |

Today's `ToggleFocus`, `Toggle`, `NewThread`, and `cmd-shift-a` actions are **all removed**. `cmd-p` handles everything.
