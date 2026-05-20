//! Stubs for types previously provided by `cloud_api_types` and `cloud_api_client`.
//!
//! These types are preserved locally so that the `client` crate can compile
//! without the cloud infrastructure crates. In a future pass, `UserStore`
//! and related cloud-gated code will be removed entirely.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

// ---- From cloud_api_types ----

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    #[default]
    XenomorphicFree,
    XenomorphicPro,
    XenomorphicProTrial,
    XenomorphicBusiness,
    XenomorphicStudent,
}

impl Plan {
    pub fn is_pro(&self) -> bool {
        matches!(self, Self::XenomorphicPro | Self::XenomorphicProTrial)
    }

    pub fn is_free(&self) -> bool {
        matches!(self, Self::XenomorphicFree)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
pub struct OrganizationId(pub Arc<str>);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: Arc<str>,
    pub is_personal: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OrganizationConfiguration {
    pub is_zed_model_provider_enabled: bool,
    pub is_agent_thread_feedback_enabled: bool,
    pub is_collaboration_enabled: bool,
    pub edit_prediction: OrganizationEditPredictionConfiguration,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OrganizationEditPredictionConfiguration {
    pub is_enabled: bool,
    pub is_feedback_enabled: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GetAuthenticatedUserResponse {
    pub user: AuthenticatedUser,
    pub feature_flags: Vec<String>,
    #[serde(default)]
    pub organizations: Vec<Organization>,
    #[serde(default)]
    pub default_organization_id: Option<OrganizationId>,
    #[serde(default)]
    pub plans_by_organization: BTreeMap<OrganizationId, KnownOrUnknown<Plan, String>>,
    #[serde(default)]
    pub configuration_by_organization: BTreeMap<OrganizationId, OrganizationConfiguration>,
    pub plan: PlanInfo,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: i32,
    pub metrics_id: String,
    pub avatar_url: String,
    pub github_login: String,
    pub name: Option<String>,
    pub is_staff: bool,
    pub accepted_tos_at: Option<Timestamp>,
}

/// A timestamp with a serialized representation in RFC 3339 format.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Timestamp(pub chrono::DateTime<chrono::Utc>);

impl Timestamp {
    pub fn new(datetime: chrono::DateTime<chrono::Utc>) -> Self {
        Self(datetime)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for Timestamp {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        Self(value)
    }
}

impl Serialize for Timestamp {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        chrono::DateTime::parse_from_rfc3339(&s)
            .map(|dt| Timestamp(dt.with_timezone(&chrono::Utc)))
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SubscriptionPeriod {
    pub started_at: Timestamp,
    pub ended_at: Timestamp,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanInfo {
    #[serde(rename = "plan_v3")]
    pub plan: KnownOrUnknown<Plan, String>,
    pub subscription_period: Option<SubscriptionPeriod>,
    pub usage: CurrentUsage,
    pub trial_started_at: Option<Timestamp>,
    pub is_account_too_young: bool,
    pub has_overdue_invoices: bool,
}

impl PlanInfo {
    pub fn plan(&self) -> Plan {
        match &self.plan {
            KnownOrUnknown::Known(plan) => *plan,
            KnownOrUnknown::Unknown(_) => Plan::XenomorphicFree,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum KnownOrUnknown<K, V> {
    Known(K),
    Unknown(V),
}

// ---- From cloud_api_client::websocket_protocol ----

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageToClient {
    UserUpdated,
}

// ---- From cloud_llm_client ----

pub const EDIT_PREDICTIONS_USAGE_AMOUNT_HEADER_NAME: &str = "x-zed-edit-predictions-usage-amount";
pub const EDIT_PREDICTIONS_USAGE_LIMIT_HEADER_NAME: &str = "x-zed-edit-predictions-usage-limit";
pub const EXPIRED_LLM_TOKEN_HEADER_NAME: &str = "x-zed-expired-llm-token";
pub const OUTDATED_LLM_TOKEN_HEADER_NAME: &str = "x-zed-outdated-llm-token";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageLimit {
    Limited(i32),
    Unlimited,
}

impl UsageLimit {
    pub fn is_unlimited(&self) -> bool {
        matches!(self, Self::Unlimited)
    }

    pub fn to_str(&self) -> Option<&str> {
        match self {
            Self::Unlimited => Some("unlimited"),
            Self::Limited(_) => None,
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "unlimited" => Ok(Self::Unlimited),
            n => Ok(Self::Limited(n.parse().with_context(|| format!("invalid usage limit: {n:?}"))?)),
        }
    }
}

impl std::str::FromStr for UsageLimit {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unlimited" => Ok(Self::Unlimited),
            n => Ok(Self::Limited(n.parse()?)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UsageData {
    pub used: i32,
    pub limit: UsageLimit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUsage {
    pub edit_predictions: UsageData,
}

// ---- LlmApiToken stub ----

/// Stub for the LlmApiToken type previously from cloud_api_client.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmApiToken(pub String);

impl LlmApiToken {
    pub fn new(user_id: u32, token: String) -> Self {
        let _ = user_id;
        Self(token)
    }
}

impl std::fmt::Display for LlmApiToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---- Edit prediction types from cloud_llm_client ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EditPredictionRejectReason {
    #[default]
    Other,
    Empty,
    InterpolatedEmpty,
    Replaced,
    CurrentPreferred,
    Canceled,
    Discarded,
    Rejected,
    QuotaLimitReached,
    TrialExpired,
}

/// Stub for NeedsLlmTokenRefresh trait
pub trait NeedsLlmTokenRefresh {
    fn needs_llm_token_refresh(&self) -> bool;
}

impl NeedsLlmTokenRefresh for http_client::Response<http_client::AsyncBody> {
    fn needs_llm_token_refresh(&self) -> bool {
        self.headers().get(EXPIRED_LLM_TOKEN_HEADER_NAME).is_some()
            || self.headers().get(OUTDATED_LLM_TOKEN_HEADER_NAME).is_some()
    }
}

/// Returns a stub LLM token (cloud tokens no longer available)
pub fn global_llm_token(_cx: &gpui::App) -> LlmApiToken {
    LlmApiToken::default()
}

// ---- predict_edits_v3 types from cloud_llm_client ----

pub mod predict_edits_v3 {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct RawCompletionRequest {
        #[serde(default)]
        pub model: String,
        #[serde(default)]
        pub inputs: Option<String>,
        #[serde(default)]
        pub snapshot: Option<String>,
        #[serde(default)]
        pub prefix: Option<String>,
        #[serde(default)]
        pub suffix: Option<String>,
        #[serde(default)]
        pub indent: Option<String>,
        #[serde(default)]
        pub prompt: String,
        #[serde(default)]
        pub stop: Vec<std::borrow::Cow<'static, str>>,
        #[serde(default)]
        pub environment: Option<String>,
        pub max_tokens: Option<u32>,
        pub temperature: Option<f32>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RawCompletionChoice {
        pub text: String,
        #[serde(default)]
        pub index: Option<u32>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct RawCompletionResponse {
        #[serde(default)]
        pub completion: String,
        #[serde(default)]
        pub stop_reason: Option<String>,
        #[serde(default)]
        pub id: String,
        #[serde(default)]
        pub choices: Vec<RawCompletionChoice>,
        #[serde(default)]
        pub request_id: String,
        #[serde(default)]
        pub model_version: Option<String>,
        #[serde(default)]
        pub output: String,
        #[serde(default)]
        pub editable_range: Option<std::ops::Range<usize>>,
        #[serde(default)]
        pub cursor_offset: Option<usize>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum PredictEditsRequestTrigger {
        #[default]
        Typing,
        Cli,
        Eager,
        Diagnostics,
        Other,
    }

    // Re-export types that are defined at the parent module level
    // for backward compatibility with code that imports from predict_edits_v3
    pub use crate::{PredictEditsV3Request, PredictEditsV3Response, PredictEditsMode};
}

// ---- AcceptEditPredictionBody from cloud_llm_client ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptEditPredictionBody {
    pub request_id: String,
    pub model_version: Option<String>,
    pub e2e_latency_ms: Option<u128>,
}

// ---- predict_edits_v3 related constants and types ----

pub const PREDICT_EDITS_MODE_HEADER_NAME: &str = "x-predict-edits-mode";
pub const MINIMUM_REQUIRED_VERSION_HEADER_NAME: &str = "x-minimum-required-version";
pub const PREFERRED_EXPERIMENT_HEADER_NAME: &str = "x-preferred-experiment";
pub const XENOMORPHIC_VERSION_HEADER_NAME: &str = "x-xenomorphic-version";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictEditsMode {
    V3,
    RawCompletion,
    Default,
    Eager,
    Subtle,
}

impl PredictEditsMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V3 => "v3",
            Self::RawCompletion => "raw_completion",
            Self::Default => "default",
            Self::Eager => "eager",
            Self::Subtle => "subtle",
        }
    }
}

impl Default for PredictEditsMode {
    fn default() -> Self {
        Self::Default
    }
}

impl AsRef<str> for PredictEditsMode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PredictEditsV3Request {
    #[serde(flatten, default)]
    pub raw_completion: predict_edits_v3::RawCompletionRequest,
    #[serde(default)]
    pub mode: PredictEditsMode,
    pub trigger: predict_edits_v3::PredictEditsRequestTrigger,
    #[serde(default)]
    pub experiment: Option<String>,
    #[serde(default)]
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictEditsV3Response {
    #[serde(default)]
    pub mode: Option<PredictEditsMode>,
    #[serde(flatten, default)]
    pub raw_completion: predict_edits_v3::RawCompletionResponse,
    #[serde(default)]
    pub model_version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditPredictionRejection {
    #[serde(default)]
    pub id: String,
    pub reason: Option<EditPredictionRejectReason>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub was_shown: bool,
    #[serde(default)]
    pub model_version: Option<String>,
    #[serde(default)]
    pub e2e_latency_ms: Option<u64>,
}

pub const MAX_EDIT_PREDICTION_REJECTIONS_PER_REQUEST: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectEditPredictionsBodyRef<'a> {
    pub rejections: Vec<EditPredictionRejection>,
    #[serde(borrow)]
    pub installation_id: Option<&'a str>,
}

// Also support the owned version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectEditPredictionsBody {
    pub rejections: Vec<EditPredictionRejection>,
    pub installation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitEditPredictionFeedbackBody {
    #[serde(default)]
    pub organization_id: Option<OrganizationId>,
    pub request_id: String,
    pub rating: String,
    #[serde(default)]
    pub inputs: Option<String>,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub feedback: Option<String>,
}

// ---- RefreshLlmTokenListener stub from llm_token.rs ----

/// Stub: Cloud LLM token refresh is no longer available.
/// This type is preserved for API compatibility but does nothing.
pub struct RefreshLlmTokenListener;

impl RefreshLlmTokenListener {
    pub fn register(_client: std::sync::Arc<crate::Client>, _user_store: gpui::Entity<crate::UserStore>, _cx: &mut gpui::App) {
        // No-op: cloud token refresh removed
    }
}
