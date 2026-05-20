//! Stubs for types previously provided by `cloud_api_types` and `cloud_api_client`.
//!
//! These types are preserved locally so that the `client` crate can compile
//! without the cloud infrastructure crates. In a future pass, `UserStore`
//! and related cloud-gated code will be removed entirely.

use std::collections::BTreeMap;
use std::sync::Arc;

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp(pub i64);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
