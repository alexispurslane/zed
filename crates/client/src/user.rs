use super::{Client, cloud_types, proto};
use cloud_types::{UsageLimit,
    EDIT_PREDICTIONS_USAGE_AMOUNT_HEADER_NAME, EDIT_PREDICTIONS_USAGE_LIMIT_HEADER_NAME};
use collections::HashMap;
use derive_more::Deref;
use gpui::{Context, EventEmitter, SharedString, SharedUri, Task};
use http_client::http::{HeaderMap, HeaderValue};
use postage::watch;
use std::sync::Arc;
use text::ReplicaId;

pub type LegacyUserId = u64;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ProjectId(pub u64);

impl ProjectId {
    pub fn to_proto(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticipantIndex(pub u32);

#[derive(Default, Debug)]
pub struct User {
    pub legacy_id: LegacyUserId,
    pub github_login: SharedString,
    pub avatar_uri: SharedUri,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collaborator {
    pub peer_id: proto::PeerId,
    pub replica_id: ReplicaId,
    pub user_id: LegacyUserId,
    pub is_host: bool,
    pub committer_name: Option<String>,
    pub committer_email: Option<String>,
}

impl PartialOrd for User {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for User {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.github_login.cmp(&other.github_login)
    }
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.legacy_id == other.legacy_id && self.github_login == other.github_login
    }
}

impl Eq for User {}

pub struct UserStore {
    users: HashMap<u64, Arc<User>>,
    by_github_login: HashMap<SharedString, u64>,
    participant_indices: HashMap<u64, ParticipantIndex>,
    edit_prediction_usage: Option<EditPredictionUsage>,
    current_user: watch::Receiver<Option<Arc<User>>>,
}

pub enum Event {
    ParticipantIndicesChanged,
}

impl EventEmitter<Event> for UserStore {}

#[derive(Debug, Clone, Copy, Deref)]
pub struct EditPredictionUsage(pub RequestUsage);

#[derive(Debug, Clone, Copy)]
pub struct RequestUsage {
    pub limit: UsageLimit,
    pub amount: i32,
}

impl UserStore {
    pub fn new(_client: Arc<Client>, _cx: &Context<Self>) -> Self {
        let (_current_user_tx, current_user_rx) = watch::channel();

        Self {
            users: Default::default(),
            by_github_login: Default::default(),
            current_user: current_user_rx,
            edit_prediction_usage: None,
            participant_indices: Default::default(),
        }
    }

    #[cfg(feature = "test-support")]
    pub fn clear_cache(&mut self) {
        self.users.clear();
        self.by_github_login.clear();
    }

    pub fn get_cached_user(&self, user_id: u64) -> Option<Arc<User>> {
        self.users.get(&user_id).cloned()
    }

    pub fn get_users(
        &self,
        user_ids: Vec<u64>,
        _cx: &Context<Self>,
    ) -> Task<anyhow::Result<Vec<Arc<User>>>> {
        let users: anyhow::Result<Vec<Arc<User>>> = user_ids
            .iter()
            .map(|id| {
                self.users
                    .get(id)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("user {id} not found"))
            })
            .collect();
        Task::ready(users)
    }

    pub fn get_user(&self, user_id: u64, _cx: &Context<Self>) -> Task<anyhow::Result<Arc<User>>> {
        if let Some(user) = self.users.get(&user_id).cloned() {
            return Task::ready(Ok(user));
        }

        Task::ready(Err(anyhow::anyhow!("user {user_id} not found")))
    }

    pub fn cached_user_by_github_login(&self, github_login: &str) -> Option<Arc<User>> {
        self.by_github_login
            .get(github_login)
            .and_then(|id| self.users.get(id).cloned())
    }

    pub fn current_user(&self) -> Option<Arc<User>> {
        self.current_user.borrow().clone()
    }

    pub fn edit_prediction_usage(&self) -> Option<EditPredictionUsage> {
        self.edit_prediction_usage
    }

    pub fn update_edit_prediction_usage(
        &mut self,
        usage: EditPredictionUsage,
        cx: &mut Context<Self>,
    ) {
        self.edit_prediction_usage = Some(usage);
        cx.notify();
    }

    pub fn watch_current_user(&self) -> watch::Receiver<Option<Arc<User>>> {
        self.current_user.clone()
    }

    pub fn insert(&mut self, users: Vec<proto::User>) -> Vec<Arc<User>> {
        let mut ret = Vec::with_capacity(users.len());
        for user in users {
            let user = User::new(user);
            if let Some(old) = self.users.insert(user.legacy_id, user.clone())
                && old.github_login != user.github_login
            {
                self.by_github_login.remove(&old.github_login);
            }
            self.by_github_login
                .insert(user.github_login.clone(), user.legacy_id);
            ret.push(user)
        }
        ret
    }

    pub fn set_participant_indices(
        &mut self,
        participant_indices: HashMap<u64, ParticipantIndex>,
        cx: &mut Context<Self>,
    ) {
        if participant_indices != self.participant_indices {
            self.participant_indices = participant_indices;
            cx.emit(Event::ParticipantIndicesChanged);
        }
    }

    pub fn participant_indices(&self) -> &HashMap<u64, ParticipantIndex> {
        &self.participant_indices
    }

    pub fn participant_names(
        &self,
        user_ids: impl Iterator<Item = u64>,
        _cx: &gpui::App,
    ) -> HashMap<u64, SharedString> {
        let mut ret = HashMap::default();
        for id in user_ids {
            if let Some(github_login) = self.get_cached_user(id).map(|u| u.github_login.clone()) {
                ret.insert(id, github_login);
            }
        }
        ret
    }
}

impl User {
    fn new(message: proto::User) -> Arc<Self> {
        Arc::new(User {
            legacy_id: message.id,
            github_login: message.github_login.into(),
            avatar_uri: message.avatar_url.into(),
            name: message.name,
        })
    }
}

impl Collaborator {
    pub fn from_proto(message: proto::Collaborator) -> anyhow::Result<Self> {
        Ok(Self {
            peer_id: message.peer_id.ok_or_else(|| anyhow::anyhow!("invalid peer id"))?,
            replica_id: ReplicaId::new(message.replica_id as u16),
            user_id: message.user_id as LegacyUserId,
            is_host: message.is_host,
            committer_name: message.committer_name,
            committer_email: message.committer_email,
        })
    }
}

impl RequestUsage {
    pub fn over_limit(&self) -> bool {
        match self.limit {
            UsageLimit::Limited(limit) => self.amount >= limit,
            UsageLimit::Unlimited => false,
        }
    }

    fn from_headers(
        limit_name: &str,
        amount_name: &str,
        headers: &HeaderMap<HeaderValue>,
    ) -> anyhow::Result<Self> {
        let limit = headers
            .get(limit_name)
            .ok_or_else(|| anyhow::anyhow!("missing {limit_name:?} header"))?;
        let limit = UsageLimit::from_str(limit.to_str()?)?;

        let amount = headers
            .get(amount_name)
            .ok_or_else(|| anyhow::anyhow!("missing {amount_name:?} header"))?;
        let amount = amount.to_str()?.parse::<i32>()?;

        Ok(Self { limit, amount })
    }
}

impl EditPredictionUsage {
    pub fn from_headers(headers: &HeaderMap<HeaderValue>) -> anyhow::Result<Self> {
        Ok(Self(RequestUsage::from_headers(
            EDIT_PREDICTIONS_USAGE_LIMIT_HEADER_NAME,
            EDIT_PREDICTIONS_USAGE_AMOUNT_HEADER_NAME,
            headers,
        )?))
    }
}
