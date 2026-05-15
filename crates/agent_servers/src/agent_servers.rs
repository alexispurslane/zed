use collections::HashMap;
use http_client::read_no_proxy_from_env;
use project::{AgentId, Project, agent_server_store::AgentServerStore};

use agent_thread::AgentConnection;
use anyhow::Result;
use gpui::{App, Entity, Task};
use settings::Settings;
use std::{any::Any, rc::Rc};

#[allow(dead_code)]
pub struct AgentServerDelegate {
    store: Entity<AgentServerStore>,
    new_version_available: Option<watch::Sender<Option<String>>>,
}

impl AgentServerDelegate {
    pub fn new(
        store: Entity<AgentServerStore>,
        new_version_tx: Option<watch::Sender<Option<String>>>,
    ) -> Self {
        Self {
            store,
            new_version_available: new_version_tx,
        }
    }
}

pub trait AgentServer: Send {
    fn logo(&self) -> ui::IconName;
    fn agent_id(&self) -> AgentId;
    fn connect(
        &self,
        delegate: AgentServerDelegate,
        project: Entity<Project>,
        cx: &mut App,
    ) -> Task<Result<Rc<dyn AgentConnection>>>;

    fn into_any(self: Rc<Self>) -> Rc<dyn Any>;
}

impl dyn AgentServer {
    pub fn downcast<T: 'static + AgentServer + Sized>(self: Rc<Self>) -> Option<Rc<T>> {
        self.into_any().downcast().ok()
    }
}

/// Load the default proxy environment variables to pass through to the agent
pub fn load_proxy_env(cx: &App) -> HashMap<String, String> {
    let proxy_url = client::ProxySettings::get_global(cx).proxy_url();
    let mut env = HashMap::default();

    if let Some(proxy_url) = &proxy_url {
        let env_var = if proxy_url.scheme() == "https" {
            "HTTPS_PROXY"
        } else {
            "HTTP_PROXY"
        };
        env.insert(env_var.to_owned(), proxy_url.to_string());
    }

    if let Some(no_proxy) = read_no_proxy_from_env() {
        env.insert("NO_PROXY".to_owned(), no_proxy);
    } else if proxy_url.is_some() {
        // We sometimes need local MCP servers that we don't want to proxy
        env.insert("NO_PROXY".to_owned(), "localhost,127.0.0.1".to_owned());
    }

    env
}

/// An agent server that cannot connect, used for agent types that are no longer supported.
pub struct UnsupportedAgentServer {
    agent_id: AgentId,
}

impl UnsupportedAgentServer {
    pub fn new(agent_id: AgentId) -> Self {
        Self { agent_id }
    }
}

impl AgentServer for UnsupportedAgentServer {
    fn logo(&self) -> ui::IconName {
        ui::IconName::Sparkle
    }

    fn agent_id(&self) -> AgentId {
        self.agent_id.clone()
    }

    fn connect(
        &self,
        _delegate: AgentServerDelegate,
        _project: Entity<Project>,
        _cx: &mut App,
    ) -> Task<Result<Rc<dyn AgentConnection>>> {
        let agent_id = self.agent_id.clone();
        Task::ready(Err(anyhow::anyhow!(
            "External agent '{}' is no longer supported. Agent Client Protocol (ACP) has been removed.",
            agent_id
        )))
    }

    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}


