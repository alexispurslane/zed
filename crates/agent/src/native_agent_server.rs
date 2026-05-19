use std::{any::Any, rc::Rc, sync::Arc};

use agent_servers::{AgentServer, AgentServerDelegate};
use anyhow::Result;
use fs::Fs;
use gpui::{App, Entity, Task};
use project::{AgentId, Project};

use crate::{NativeAgent, NativeAgentConnection, ThreadStore, templates::Templates};

#[derive(Clone)]
pub struct NativeAgentServer {
    fs: Arc<dyn Fs>,
    thread_store: Entity<ThreadStore>,
}

impl NativeAgentServer {
    pub fn new(fs: Arc<dyn Fs>, thread_store: Entity<ThreadStore>) -> Self {
        Self { fs, thread_store }
    }
}

impl AgentServer for NativeAgentServer {
    fn agent_id(&self) -> AgentId {
        crate::XENOMORPHIC_AGENT_ID.clone()
    }

    fn logo(&self) -> ui::IconName {
        ui::IconName::XenomorphicAgent
    }

    fn connect(
        &self,
        _delegate: AgentServerDelegate,
        _project: Entity<Project>,
        cx: &mut App,
    ) -> Task<Result<Rc<dyn agent_thread::AgentConnection>>> {
        log::debug!("NativeAgentServer::connect");
        let fs = self.fs.clone();
        let thread_store = self.thread_store.clone();
        cx.spawn(async move |cx| {
            log::debug!("Creating templates for native agent");
            let templates = Templates::new();

            log::debug!("Creating native agent entity");
            let agent =
                cx.update(|cx| NativeAgent::new(thread_store, templates, fs, cx));

            // Create the connection wrapper
            let connection = NativeAgentConnection(agent);
            log::debug!("NativeAgentServer connection established successfully");

            Ok(Rc::new(connection) as Rc<dyn agent_thread::AgentConnection>)
        })
    }

    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

}
