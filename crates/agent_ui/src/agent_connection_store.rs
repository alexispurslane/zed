use std::rc::Rc;

use agent_thread::{AgentConnection, LoadError};
use agent_servers::{AgentServer, AgentServerDelegate};
use anyhow::Result;
use collections::HashMap;
use fs::Fs;
use futures::{FutureExt, future::Shared};
use gpui::{App, AppContext, Context, Entity, Global, Task};

use project::Project;

use crate::Agent;

pub enum AgentConnectionEntry {
    Connecting {
        connect_task: Shared<Task<Result<AgentConnectedState, LoadError>>>,
    },
    Connected(AgentConnectedState),
    Error {
        error: LoadError,
    },
}

#[derive(Clone)]
pub struct AgentConnectedState {
    pub connection: Rc<dyn AgentConnection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

impl AgentConnectionEntry {
    pub fn wait_for_connection(&self) -> Shared<Task<Result<AgentConnectedState, LoadError>>> {
        match self {
            AgentConnectionEntry::Connecting { connect_task } => connect_task.clone(),
            AgentConnectionEntry::Connected(state) => Task::ready(Ok(state.clone())).shared(),
            AgentConnectionEntry::Error { error } => Task::ready(Err(error.clone())).shared(),
        }
    }

    pub fn status(&self) -> AgentConnectionStatus {
        match self {
            AgentConnectionEntry::Connecting { .. } => AgentConnectionStatus::Connecting,
            AgentConnectionEntry::Connected(_) => AgentConnectionStatus::Connected,
            AgentConnectionEntry::Error { .. } => AgentConnectionStatus::Disconnected,
        }
    }
}

#[derive(Clone)]
pub struct ActiveAgentConnection {
    pub agent_id: project::AgentId,
    pub connection: Rc<dyn AgentConnection>,
}

pub struct AgentConnectionStore {
    project: Entity<Project>,
    entries: HashMap<Agent, Entity<AgentConnectionEntry>>,
}

/// Global wrapper so that all tabs in a process share one connection store.
/// Initialized in `agent_ui::init`.
struct GlobalConnectionStore(Entity<AgentConnectionStore>);

impl Global for GlobalConnectionStore {}

impl AgentConnectionStore {
    /// Initializes the global `AgentConnectionStore`. Called from
    /// `create_conversation_view` on first use, or from `agent_ui::init`
    /// when the project is available.
    pub fn init_global(project: Entity<Project>, cx: &mut App) {
        if cx.try_global::<GlobalConnectionStore>().is_some() {
            return;
        }

        let fs = <dyn Fs>::global(cx);
        let thread_store = agent::ThreadStore::global(cx);

        let store = cx.new(|cx| {
            let mut store = AgentConnectionStore::new(project.clone(), cx);
            store.request_connection(
                Agent::NativeAgent,
                Agent::NativeAgent.server(fs.clone(), thread_store.clone()),
                cx,
            );
            store
        });

        cx.set_global(GlobalConnectionStore(store));
    }

    /// Returns the global `AgentConnectionStore`, if initialized.
    pub fn try_global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalConnectionStore>().map(|g| g.0.clone())
    }

    /// Returns the global `AgentConnectionStore`, panicking if not initialized.
    pub fn global(cx: &App) -> Entity<Self> {
        Self::try_global(cx).expect("AgentConnectionStore::init_global has not been called")
    }
}

impl AgentConnectionStore {
    pub fn new(project: Entity<Project>, _cx: &mut Context<Self>) -> Self {
        Self {
            project,
            entries: HashMap::default(),
        }
    }

    pub fn project(&self) -> &Entity<Project> {
        &self.project
    }

    pub fn entry(&self, key: &Agent) -> Option<&Entity<AgentConnectionEntry>> {
        self.entries.get(key)
    }

    pub fn connection_status(&self, key: &Agent, cx: &App) -> AgentConnectionStatus {
        self.entries
            .get(key)
            .map(|entry| entry.read(cx).status())
            .unwrap_or(AgentConnectionStatus::Disconnected)
    }

    pub fn active_agent_connections(&self, cx: &App) -> Vec<ActiveAgentConnection> {
        self.entries
            .values()
            .filter_map(|entry| match entry.read(cx) {
                AgentConnectionEntry::Connected(state) => Some(ActiveAgentConnection {
                    agent_id: state.connection.agent_id(),
                    connection: state.connection.clone(),
                }),
                AgentConnectionEntry::Connecting { .. } | AgentConnectionEntry::Error { .. } => {
                    None
                }
            })
            .collect()
    }

    pub fn restart_connection(
        &mut self,
        key: Agent,
        server: Rc<dyn AgentServer>,
        cx: &mut Context<Self>,
    ) -> Entity<AgentConnectionEntry> {
        if let Some(entry) = self.entries.get(&key) {
            if matches!(entry.read(cx), AgentConnectionEntry::Connecting { .. }) {
                return entry.clone();
            }
        }

        self.entries.remove(&key);
        self.request_connection(key, server, cx)
    }

    pub fn request_connection(
        &mut self,
        key: Agent,
        server: Rc<dyn AgentServer>,
        cx: &mut Context<Self>,
    ) -> Entity<AgentConnectionEntry> {
        if let Some(entry) = self.entries.get(&key) {
            return entry.clone();
        }

        let connect_task = self.start_connection(server, cx);
        let connect_task = connect_task.shared();

        let entry = cx.new(|_cx| AgentConnectionEntry::Connecting {
            connect_task: connect_task.clone(),
        });

        self.entries.insert(key.clone(), entry.clone());
        cx.notify();

        cx.spawn({
            let entry = entry.downgrade();
            async move |this, cx| match connect_task.await {
                Ok(connected_state) => {
                    this.update(cx, move |this, cx| {
                        if this.entries.get(&key) != entry.upgrade().as_ref() {
                            return;
                        }

                        entry
                            .update(cx, move |entry, cx| {
                                if let AgentConnectionEntry::Connecting { .. } = entry {
                                    *entry = AgentConnectionEntry::Connected(connected_state);
                                    cx.notify();
                                }
                            })
                            .ok();
                        cx.notify();
                    })
                    .ok();
                }
                Err(error) => {
                    this.update(cx, move |this, cx| {
                        if this.entries.get(&key) != entry.upgrade().as_ref() {
                            return;
                        }

                        entry
                            .update(cx, move |entry, cx| {
                                if let AgentConnectionEntry::Connecting { .. } = entry {
                                    *entry = AgentConnectionEntry::Error { error };
                                    cx.notify();
                                }
                            })
                            .ok();
                        this.entries.remove(&key);
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();

        entry
    }

    /// Like `request_connection`, but replaces any existing entry for the same key.
    /// This is needed in tests where different connections are used for the same
    /// agent key (e.g. loadable vs non-loadable stub connections both keyed as `Agent::Stub`).
    #[cfg(any(test, feature = "test-support"))]
    pub fn force_request_connection(
        &mut self,
        key: Agent,
        server: Rc<dyn AgentServer>,
        cx: &mut Context<Self>,
    ) -> Entity<AgentConnectionEntry> {
        // Remove the existing entry if present so a new connection is established.
        self.entries.remove(&key);
        self.request_connection(key, server, cx)
    }

    /// Removes the cached connection entry for the given agent key.
    /// This allows a subsequent `request_connection` call to establish a new connection.
    #[cfg(any(test, feature = "test-support"))]
    pub fn clear_connection(&mut self, key: &Agent) {
        self.entries.remove(key);
    }

    fn start_connection(
        &self,
        server: Rc<dyn AgentServer>,
        cx: &mut Context<Self>,
    ) -> Task<Result<AgentConnectedState, LoadError>> {
        let delegate = AgentServerDelegate;
        let connect_task = server.connect(delegate, self.project.clone(), cx);
        cx.spawn(async move |_this, _cx| match connect_task.await {
            Ok(connection) => Ok(AgentConnectedState { connection }),
            Err(err) => match err.downcast::<LoadError>() {
                Ok(load_error) => Err(load_error),
                Err(err) => Err(LoadError::Other(gpui::SharedString::from(err.to_string()))),
            },
        })
    }
}
