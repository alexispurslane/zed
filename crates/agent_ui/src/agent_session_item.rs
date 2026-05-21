use std::any::{Any, TypeId};
use std::sync::Arc;

use agent_thread::{AgentThread, AgentThreadEntry, ThreadStatus};
use anyhow::Result;
use gpui::{
    Action, App, Context, Entity, EntityId, EventEmitter, FocusHandle, Focusable, Render,
    SharedString, Task, WeakEntity, Window,
};
use project::Project;
use ui::{Color, Icon, IconName, IntoElement, prelude::*};
use workspace::{
    Item, ItemId, SerializableItem, Workspace, WorkspaceId,
    delete_unloaded_items,
    item::ItemEvent,
};

use crate::{ConversationView, DEFAULT_THREAD_TITLE};
use crate::thread_metadata_store::ThreadId;

// ── Persistence layer ──────────────────────────────────────────────────

mod persistence {
    use anyhow::Result;
    use db::{
        query,
        sqlez::{domain::Domain, statement::Statement, thread_safe_connection::ThreadSafeConnection},
        sqlez_macros::sql,
    };
    use workspace::{ItemId, WorkspaceDb, WorkspaceId};

    pub(super) struct AgentSessionDb(ThreadSafeConnection);

    impl Domain for AgentSessionDb {
        const NAME: &str = stringify!(AgentSessionDb);

        const MIGRATIONS: &[&str] = &[
            sql!(
                CREATE TABLE IF NOT EXISTS agent_sessions(
                    workspace_id INTEGER,
                    item_id INTEGER,
                    thread_id TEXT NOT NULL,
                    session_id TEXT,
                    PRIMARY KEY(workspace_id, item_id),
                    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
                    ON DELETE CASCADE
                ) STRICT;
            ),
        ];
    }

    db::static_connection!(AgentSessionDb, [WorkspaceDb]);

    impl AgentSessionDb {
        query! {
            pub async fn update_workspace_id(
                new_id: WorkspaceId,
                old_id: WorkspaceId,
                item_id: ItemId
            ) -> Result<()> {
                UPDATE agent_sessions
                SET workspace_id = ?
                WHERE workspace_id = ? AND item_id = ?
            }
        }

        pub async fn save_session(
            &self,
            item_id: ItemId,
            workspace_id: WorkspaceId,
            thread_id: String,
            session_id: Option<String>,
        ) -> Result<()> {
            self.write(move |conn| {
                let query = "INSERT INTO agent_sessions(item_id, workspace_id, thread_id, session_id)
                    VALUES (?1, ?2, ?3, ?4)
                    ON CONFLICT (workspace_id, item_id) DO UPDATE SET
                        item_id = ?1,
                        workspace_id = ?2,
                        thread_id = ?3,
                        session_id = ?4";
                let mut statement = Statement::prepare(conn, query)?;
                let mut next_index = statement.bind(&item_id, 1)?;
                next_index = statement.bind(&workspace_id, next_index)?;
                next_index = statement.bind(&thread_id.as_str(), next_index)?;
                next_index = statement.bind(&session_id.as_deref(), next_index)?;
                statement.bind(&(), next_index)?;
                statement.exec()
            })
            .await
        }

        query! {
            pub fn get_session(item_id: ItemId, workspace_id: WorkspaceId) -> Result<(String, Option<String>)> {
                SELECT thread_id, session_id
                FROM agent_sessions
                WHERE item_id = ? AND workspace_id = ?
            }
        }
    }
}

use persistence::AgentSessionDb;

// ── AgentSessionItem ───────────────────────────────────────────────────

/// A workspace Item wrapper around ConversationView.
///
/// Enables agent sessions to be opened as tabs alongside editors.
/// This is the core of the "Agent Sessions as Tabs" design: instead
/// of a dock panel with a sidebar, each agent conversation lives in
/// its own tab, draggable, splittable, and closable like any editor.
pub struct AgentSessionItem {
    conversation_view: Entity<ConversationView>,
    workspace: WeakEntity<Workspace>,
}

/// Events emitted by `AgentSessionItem` that the workspace item
/// system translates into `ItemEvent`s.
pub enum AgentSessionItemEvent {
    UpdateTab,
    Close,
}

impl AgentSessionItem {
    /// Creates a new `AgentSessionItem` wrapping the given conversation view.
    pub fn new(
        conversation_view: Entity<ConversationView>,
        workspace: WeakEntity<Workspace>,
    ) -> Self {
        Self {
            conversation_view,
            workspace,
        }
    }

    /// Returns a reference to the underlying `ConversationView`.
    pub fn conversation_view(&self) -> &Entity<ConversationView> {
        &self.conversation_view
    }

    /// Returns the `ThreadId` of this session.
    pub fn thread_id(&self, cx: &App) -> ThreadId {
        self.conversation_view.read(cx).thread_id
    }

    /// Returns the root `AgentThread` entity, if the conversation has connected.
    fn root_thread(&self, cx: &App) -> Option<Entity<AgentThread>> {
        self.conversation_view.read(cx).root_thread(cx)
    }

    /// Returns the text for the tab, using the "subtitle" approach from the design doc:
    /// - If the thread has a title, use it (O(1) — `title()` checks a stored field).
    /// - If no title but there is a first user message, show "New Agent Thread — {first 30 chars}".
    /// - Otherwise fall back to `DEFAULT_THREAD_TITLE`.
    ///
    /// # Performance
    ///
    /// The `title()` check is O(1) — it reads a stored `Option<SharedString>` field.
    /// Only untitled new threads (with 0–2 entries) fall through to the entry scan,
    /// which is negligible. Once the LLM generates a title, the fast path always
    /// applies.
    fn compute_tab_text(&self, cx: &App) -> SharedString {
        if let Some(thread) = self.root_thread(cx) {
            let thread_ref = thread.read(cx);

            // Fast path: O(1) check. Almost all non-new threads have a title.
            if let Some(title) = thread_ref.title() {
                return title;
            }

            // Slow path: scan entries for a first-user-message subtitle.
            // This only runs for brand-new threads that haven't received a
            // generated title yet, so the entry list is very small (0–2).
            for entry in thread_ref.entries() {
                if let AgentThreadEntry::UserMessage(user_msg) = entry {
                    let text = user_msg.content.to_markdown(cx);
                    // Strip leading "## User" header that to_markdown adds, then trim.
                    let text = text
                        .strip_prefix("## User")
                        .unwrap_or(&text)
                        .trim()
                        .lines()
                        .next()
                        .unwrap_or("");
                    let truncated: String = text.chars().take(30).collect();
                    if !truncated.is_empty() {
                        return SharedString::from(format!(
                            "{} — {}",
                            DEFAULT_THREAD_TITLE, truncated
                        ));
                    }
                    break;
                }
            }
        }

        SharedString::from(DEFAULT_THREAD_TITLE)
    }
}

impl Item for AgentSessionItem {
    type Event = AgentSessionItemEvent;

    fn tab_content_text(&self, _detail: usize, cx: &App) -> SharedString {
        self.compute_tab_text(cx)
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::XenomorphicAssistant).color(Color::Accent))
    }

    fn tab_tooltip_text(&self, cx: &App) -> Option<SharedString> {
        // Prefer the thread title for the tooltip; fall back to session ID.
        if let Some(thread) = self.root_thread(cx) {
            let thread_ref = thread.read(cx);
            if let Some(title) = thread_ref.title() {
                return Some(title);
            }
        }

        let conversation = self.conversation_view.read(cx);
        conversation
            .root_session_id
            .as_ref()
            .map(|sid| SharedString::from(sid.to_string()))
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        match event {
            AgentSessionItemEvent::UpdateTab => f(ItemEvent::UpdateTab),
            AgentSessionItemEvent::Close => f(ItemEvent::CloseItem),
        }
    }

    fn include_in_nav_history() -> bool {
        false
    }

    fn show_toolbar(&self) -> bool {
        // The conversation view has its own inline controls (model/profile
        // selectors in the message editor chrome). No pane toolbar needed.
        false
    }

    fn can_split(&self) -> bool {
        true
    }

    fn clone_on_split(
        &self,
        _workspace_id: Option<WorkspaceId>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Option<Entity<Self>>> {
        // Share the same ConversationView entity — same model as splitting
        // an editor where both panes see the same buffer live.
        let conversation_view = self.conversation_view.clone();
        let workspace = self.workspace.clone();
        Task::ready(Some(cx.new(|_| AgentSessionItem::new(conversation_view, workspace))))
    }

    fn is_dirty(&self, cx: &App) -> bool {
        // Consider the session "dirty" while the agent is actively generating,
        // which prevents the tab from being auto-closed inadvertently.
        self.root_thread(cx)
            .map(|thread| matches!(thread.read(cx).status(), ThreadStatus::Generating))
            .unwrap_or(false)
    }

    fn tab_extra_context_menu_actions(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Vec<(SharedString, Box<dyn Action>)> {
        // Per the design doc (section 3.4), the tab context menu should include:
        // - Regenerate Thread Title
        // - Copy Thread to Clipboard
        // - Open Thread as Markdown
        // - Archive Thread
        //
        // These will be wired up once the corresponding actions are extracted
        // from the panel. For now we return an empty vec as a placeholder.
        Vec::new()
    }

    fn deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // No-op: the ConversationView manages its own deactivation state.
    }

    fn discarded(&self, _project: Entity<Project>, _window: &mut Window, _cx: &mut Context<Self>) {
        // Cleanup when the last tab viewing this thread is closed.
        // The ConversationView's on_release handler already takes care
        // of closing server sessions and removing notification windows.
    }

    fn navigate(
        &mut self,
        _data: Arc<dyn Any + Send>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> bool {
        // Agent sessions don't support navigation (no cursor position).
        false
    }

    fn for_each_project_item(
        &self,
        _cx: &App,
        _f: &mut dyn FnMut(EntityId, &dyn project::ProjectItem),
    ) {
        // Agent sessions don't have project items.
    }

    fn act_as_type<'a>(
        &'a self,
        type_id: TypeId,
        self_handle: &'a Entity<Self>,
        _cx: &'a App,
    ) -> Option<gpui::AnyEntity> {
        if type_id == TypeId::of::<Self>() {
            Some(self_handle.clone().into())
        } else if type_id == TypeId::of::<ConversationView>() {
            Some(self.conversation_view.clone().into())
        } else {
            None
        }
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Agent Session Tab Opened")
    }
}

impl SerializableItem for AgentSessionItem {
    fn serialized_item_kind() -> &'static str {
        "AgentSessionItem"
    }

    fn cleanup(
        workspace_id: WorkspaceId,
        alive_items: Vec<ItemId>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        let db = AgentSessionDb::global(cx);
        workspace::delete_unloaded_items(alive_items, workspace_id, "agent_sessions", &db, cx)
    }

    fn deserialize(
        _project: Entity<Project>,
        workspace: WeakEntity<Workspace>,
        workspace_id: WorkspaceId,
        item_id: ItemId,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Entity<Self>>> {
        // Read serialized data from the database.
        let db = AgentSessionDb::global(cx);
        let serialized = db.get_session(item_id, workspace_id);

        window.spawn(cx, async move |cx| {
            // The DB query returns Option<(String, Option<String>)> but
            // get_session is a sync query returning Result directly.
            let (thread_id_str, session_id_str) = serialized
                .map_err(|e| anyhow::anyhow!("Failed to query agent session state: {e}"))?;

            // Reconstruct the ThreadId from the JSON string stored in the DB.
            // The stored value is a bare UUID string like "550e8400-..."; wrap
            // it in quotes to form valid JSON for deserialization.
            let thread_id_json = format!("\"{}\"", thread_id_str);
            let thread_id: ThreadId = serde_json::from_str(&thread_id_json)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize thread_id: {e}"))?;

            // Reconstruct the SessionId if present.
            let session_id = session_id_str.map(agent_thread::schema::SessionId::new);

            // Build a ConversationView for this session via the
            // thread_finder_provider helper. Use the WeakEntity's
            // update_in which works with AsyncWindowContext.
            let conversation_view = workspace.update_in(cx, |workspace, window, cx| {
                crate::thread_finder_provider::create_conversation_view(
                    session_id,
                    None,
                    None,
                    None,
                    workspace,
                    window,
                    cx,
                )
            })?;

            Ok(cx.new(|_| {
                AgentSessionItem::new(conversation_view, workspace)
            }))
        })
    }

    fn serialize(
        &mut self,
        _workspace: &mut Workspace,
        item_id: ItemId,
        _closing: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Task<Result<()>>> {
        let conversation = self.conversation_view.read(cx);
        let thread_id = conversation.thread_id;
        let session_id = conversation.root_session_id.clone();

        // Serialize ThreadId to a JSON string.
        // ThreadId(Uuid) serializes as a JSON string like "550e8400-..."
        // (including surrounding quotes). Strip the quotes for storage.
        let thread_id_json = serde_json::to_string(&thread_id).ok()?;
        let thread_id_clean = thread_id_json.trim_matches('"').to_string();
        let session_id_str = session_id.as_ref().map(|sid| sid.to_string());

        // Use the workspace parameter directly — it's already mutably
        // borrowed by the caller (Workspace::serialize_items), so calling
        // self.workspace.read(cx) would trigger a double-lease panic.
        let Some(workspace_id) = _workspace.database_id() else {
            return None;
        };

        let db = AgentSessionDb::global(cx);
        Some(cx.background_spawn(async move {
            db.save_session(item_id, workspace_id, thread_id_clean, session_id_str)
                .await
        }))
    }

    fn should_serialize(&self, event: &Self::Event) -> bool {
        matches!(event, AgentSessionItemEvent::UpdateTab)
    }
}

impl Render for AgentSessionItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Delegate rendering entirely to the ConversationView.
        // The size_full() is critical: ConversationView uses flex layout
        // internally (the message list is flex_1, the editor is at the bottom).
        // Without size_full() on the parent, the flex_1 message list
        // collapses to zero height while the editor (which has intrinsic
        // height from its text content) still shows.
        div().size_full().child(self.conversation_view.clone())
    }
}

impl Focusable for AgentSessionItem {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.conversation_view.focus_handle(cx)
    }
}

impl EventEmitter<AgentSessionItemEvent> for AgentSessionItem {}
