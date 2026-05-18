use collections::HashMap;
use http_client::read_no_proxy_from_env;
use project::{AgentId, Project};

use agent_thread::AgentConnection;
use anyhow::Result;
use gpui::{App, Entity, Task};
use settings::Settings;
use std::{any::Any, rc::Rc};

/// Opaque delegate passed to `AgentServer::connect()`.
pub struct AgentServerDelegate;

pub trait AgentServer: Send {
    fn logo(&self) -> ui::IconName;
    fn agent_id(&self) -> AgentId;
    fn connect(
        &self,
        _delegate: AgentServerDelegate,
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
