#[cfg(any(test, feature = "test-support"))]
pub mod test;

mod proxy;
pub mod telemetry;

use anyhow::Result;
use clock::SystemClock;
use credentials_provider::CredentialsProvider;
use gpui::{App, AsyncApp, Context, Entity, Global, SharedString, SharedUri, actions};
use http_client::{HttpClient, HttpClientWithUrl, read_proxy_from_env};
use parking_lot::RwLock;
use postage::watch;
use serde::Deserialize;
use settings::{RegisterSetting, Settings, SettingsContent};
use std::{
    path::PathBuf,
    sync::{Arc, LazyLock},
};
use telemetry::Telemetry;
use url::Url;

pub use rpc::*;
pub use telemetry_events::Event;

static XENOMORPHIC_SERVER_URL: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("XENOMORPHIC_SERVER_URL").ok());

actions!(
    client,
    [
        SignIn,
        SignOut,
    ]
);

#[derive(Deserialize, RegisterSetting)]
pub struct ClientSettings {
    pub server_url: String,
    pub credentials_url: Option<String>,
}

impl Settings for ClientSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        if let Some(server_url) = &*XENOMORPHIC_SERVER_URL {
            return Self {
                server_url: server_url.clone(),
                credentials_url: content.credentials_url.clone(),
            };
        }
        Self {
            server_url: content.server_url.clone().unwrap(),
            credentials_url: content.credentials_url.clone(),
        }
    }
}

#[derive(Deserialize, Default, RegisterSetting)]
pub struct ProxySettings {
    pub proxy: Option<String>,
}

impl ProxySettings {
    pub fn proxy_url(&self) -> Option<Url> {
        self.proxy
            .as_deref()
            .map(str::trim)
            .filter(|input| !input.is_empty())
            .and_then(|input| {
                input
                    .parse::<Url>()
                    .inspect_err(|e| log::error!("Error parsing proxy settings: {}", e))
                    .ok()
            })
            .or_else(read_proxy_from_env)
    }
}

impl Settings for ProxySettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        Self {
            proxy: content
                .proxy
                .as_deref()
                .map(str::trim)
                .filter(|proxy| !proxy.is_empty())
                .map(ToOwned::to_owned),
        }
    }
}

pub fn init(_client: &Arc<Client>, _cx: &mut App) {}

struct GlobalClient(Arc<Client>);

impl Global for GlobalClient {}

pub struct Client {
    http: Arc<HttpClientWithUrl>,
    telemetry: Arc<Telemetry>,
    credentials_provider: ClientCredentialsProvider,
    state: RwLock<ClientState>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Status {
    SignedOut,
    Connected,
}

impl Status {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }
    pub fn is_signed_out(&self) -> bool {
        matches!(self, Self::SignedOut)
    }
    pub fn was_connected(&self) -> bool { false }
    pub fn is_or_was_connected(&self) -> bool { self.is_connected() }
    pub fn is_signing_in(&self) -> bool { false }
}

struct ClientState {
    status: (watch::Sender<Status>, watch::Receiver<Status>),
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            status: watch::channel_with(Status::SignedOut),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credentials {
    pub user_id: u64,
    pub access_token: String,
}

pub struct ClientCredentialsProvider {
    provider: Arc<dyn CredentialsProvider>,
}

impl ClientCredentialsProvider {
    pub fn new(cx: &App) -> Self {
        Self {
            provider: xenomorphic_credentials_provider::global(cx),
        }
    }
}

impl Client {
    pub fn new(
        clock: Arc<dyn SystemClock>,
        http: Arc<HttpClientWithUrl>,
        cx: &mut App,
    ) -> Arc<Self> {
        Arc::new(Self {
            telemetry: Telemetry::new(clock, http.clone(), cx),
            http,
            credentials_provider: ClientCredentialsProvider::new(cx),
            state: Default::default(),
        })
    }

    pub fn production(cx: &mut App) -> Arc<Self> {
        let clock = Arc::new(clock::RealSystemClock);
        let http = Arc::new(HttpClientWithUrl::new_url(
            cx.http_client(),
            &ClientSettings::get_global(cx).server_url,
            cx.http_client().proxy().cloned(),
        ));
        Self::new(clock, http, cx)
    }

    pub fn http_client(&self) -> Arc<HttpClientWithUrl> {
        self.http.clone()
    }

    pub fn credentials_provider(&self) -> Arc<dyn CredentialsProvider> {
        self.credentials_provider.provider.clone()
    }

    pub fn global(cx: &App) -> Arc<Self> {
        cx.global::<GlobalClient>().0.clone()
    }

    pub fn set_global(client: Arc<Client>, cx: &mut App) {
        cx.set_global(GlobalClient(client))
    }

    pub fn status(&self) -> watch::Receiver<Status> {
        self.state.read().status.1.clone()
    }

    pub fn telemetry(&self) -> &Arc<Telemetry> {
        &self.telemetry
    }

    pub async fn has_credentials(&self, cx: &AsyncApp) -> bool {
        let url = cx.update(|cx| ClientSettings::get_global(cx).server_url.clone()).ok();
        let Some(url) = url else { return false };
        self.credentials_provider.provider.read_credentials(&url, cx).await.ok().flatten().is_some()
    }

    pub fn user_id(&self) -> Option<u64> { None }
    pub fn peer_id(&self) -> Option<proto::PeerId> { None }

    #[cfg(any(test, feature = "test-support"))]
    pub fn teardown(&self) {}
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_id(&self, _id: u64) -> &Self { self }
}

// === Compatibility types (previously from cloud_api_types/cloud_api_client) ===

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    #[default]
    XenomorphicFree,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ChannelId(pub u64);
impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ProjectId(pub u64);
impl ProjectId {
    pub fn to_proto(self) -> u64 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticipantIndex(pub u32);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct User {
    pub legacy_id: u64,
    pub github_login: SharedString,
    pub avatar_uri: SharedUri,
    pub name: Option<String>,
}
impl PartialOrd for User {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for User {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.github_login.cmp(&other.github_login) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collaborator {
    pub peer_id: proto::PeerId,
    pub replica_id: text::ReplicaId,
    pub user_id: u64,
    pub is_host: bool,
    pub committer_name: Option<String>,
    pub committer_email: Option<String>,
}

pub struct UserStore;
pub enum UserStoreEvent {}
impl gpui::EventEmitter<UserStoreEvent> for UserStore {}

impl UserStore {
    pub fn new(_client: Arc<Client>, _cx: &gpui::Context<Self>) -> Self { Self }
    pub fn plan(&self) -> Option<Plan> { Some(Plan::XenomorphicFree) }
    pub fn current_user(&self) -> Option<Arc<User>> { None }
    pub fn account_too_young(&self) -> bool { false }
    pub fn has_overdue_invoices(&self) -> bool { false }
    pub fn edit_prediction_usage(&self) -> Option<EditPredictionUsage> { None }
    pub fn current_organization(&self) -> Option<Arc<Organization>> { None }
    pub fn current_organization_configuration(&self) -> Option<&OrganizationConfiguration> { None }
    pub fn get_cached_user(&self, _user_id: u64) -> Option<Arc<User>> { None }
    pub fn watch_current_user(&self) -> watch::Receiver<Option<Arc<User>>> {
        let (tx, rx) = watch::channel();
        drop(tx);
        rx
    }
    pub fn contacts(&self) -> &[Arc<Contact>] { &[] }
    pub fn incoming_contact_requests(&self) -> &[Arc<User>] { &[] }
    pub fn outgoing_contact_requests(&self) -> &[Arc<User>] { &[] }
    pub fn participant_indices(&self) -> &std::collections::HashMap<u64, ParticipantIndex> {
        static EMPTY: std::sync::LazyLock<std::collections::HashMap<u64, ParticipantIndex>> =
            std::sync::LazyLock::new(std::collections::HashMap::new);
        &EMPTY
    }
    #[cfg(feature = "test-support")]
    pub fn clear_cache(&mut self) {}
}

#[derive(Debug, PartialEq)]
pub struct Contact { pub user: Arc<User>, pub online: bool, pub busy: bool }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactRequestStatus { None, RequestSent, RequestReceived, RequestAccepted }

#[derive(Debug, Clone)]
pub struct Organization { pub id: OrganizationId, pub is_personal: bool }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationId(pub Arc<str>);

#[derive(Debug, Clone)]
pub struct OrganizationConfiguration;

#[derive(Debug, Clone, Copy, Default)]
pub struct EditPredictionUsage { pub over_limit: bool }

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Subscription { #[default] Free }

#[derive(Copy, Clone, Deserialize, Debug, RegisterSetting)]
pub struct TelemetrySettings { pub diagnostics: bool, pub metrics: bool }

impl settings::Settings for TelemetrySettings {
    fn from_settings(content: &SettingsContent) -> Self {
        Self {
            diagnostics: content.telemetry.as_ref().unwrap().diagnostics.unwrap(),
            metrics: content.telemetry.as_ref().unwrap().metrics.unwrap(),
        }
    }
}

pub trait NeedsLlmTokenRefresh {
    fn needs_llm_token_refresh(&self) -> bool { false }
}
impl NeedsLlmTokenRefresh for http_client::Response<http_client::AsyncBody> {}

pub fn global_llm_token(_cx: &App) -> () {}

pub struct RefreshLlmTokenListener;
impl gpui::EventEmitter<()> for RefreshLlmTokenListener {}
impl RefreshLlmTokenListener {
    pub fn register(_client: Arc<Client>, _user_store: Entity<UserStore>, _cx: &mut App) {}
}
#[derive(Debug, Clone, Default)]
pub struct LlmApiToken;

pub const XENOMORPHIC_URL_SCHEME: &str = "xenomorphic";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XenomorphicLink {
    AgentShared { session_id: String },
}

pub fn parse_zed_link(link: &str, _cx: &App) -> Option<XenomorphicLink> {
    let path = link.strip_prefix(XENOMORPHIC_URL_SCHEME).and_then(|r| r.strip_prefix("://"))?;
    let mut parts = path.split('/');
    match parts.next()? {
        "agent" => {
            if parts.next()? == "shared" {
                Some(XenomorphicLink::AgentShared { session_id: parts.next()?.to_string() })
            } else { None }
        }
        _ => None,
    }
}

pub fn shared_agent_thread_url(session_id: &str) -> String {
    format!("xenomorphic://agent/shared/{}", session_id)
}

pub mod xenomorphic_urls {
    use super::ClientSettings;
    use gpui::App;
    use settings::Settings;
    fn server_url(cx: &App) -> &str { &ClientSettings::get_global(cx).server_url }
    pub fn account_url(cx: &App) -> String { format!("{}/account", server_url(cx)) }
    pub fn start_trial_url(cx: &App) -> String { format!("{}/account/start-trial", server_url(cx)) }
    pub fn upgrade_to_xenomorphic_pro_url(cx: &App) -> String { format!("{}/account/upgrade", server_url(cx)) }
    pub fn terms_of_service(cx: &App) -> String { format!("{}/terms-of-service", server_url(cx)) }
    pub fn ai_privacy_and_security(cx: &App) -> String { format!("{}/docs/ai/privacy-and-security", server_url(cx)) }
    pub fn edit_prediction_docs(cx: &App) -> String { format!("{}/docs/ai/edit-prediction", server_url(cx)) }
    pub fn parallel_agents_blog(cx: &App) -> String { format!("{}/blog", server_url(cx)) }
    pub fn shared_agent_thread_url(session_id: &str) -> String { super::shared_agent_thread_url(session_id) }
}
