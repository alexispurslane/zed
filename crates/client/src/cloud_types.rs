//! Local type definitions for the edit prediction and LLM token infrastructure.
//!
//! These types were previously provided by cloud infrastructure crates that have
//! been removed. Only types still referenced by remaining code are preserved.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

// ---- Usage limits (used by edit prediction) ----

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

// ---- LlmApiToken ----

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

/// Checks whether an HTTP response indicates that the LLM token needs refresh.
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

// ---- Organization (minimal, used by edit prediction feedback) ----

use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize, Deserialize)]
pub struct OrganizationId(pub Arc<str>);

// ---- Edit prediction types (used by edit_prediction crates) ----

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PredictEditsMode {
    V3,
    RawCompletion,
    #[default]
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

impl AsRef<str> for PredictEditsMode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

pub const PREDICT_EDITS_MODE_HEADER_NAME: &str = "x-predict-edits-mode";
pub const MINIMUM_REQUIRED_VERSION_HEADER_NAME: &str = "x-minimum-required-version";
pub const PREFERRED_EXPERIMENT_HEADER_NAME: &str = "x-preferred-experiment";
pub const XENOMORPHIC_VERSION_HEADER_NAME: &str = "x-xenomorphic-version";
pub const MAX_EDIT_PREDICTION_REJECTIONS_PER_REQUEST: usize = 10;

// ---- predict_edits_v3 types ----

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

impl PredictEditsV3Response {
    pub fn request_id(&self) -> &str {
        &self.raw_completion.request_id
    }

    pub fn output(&self) -> &str {
        &self.raw_completion.output
    }

    pub fn editable_range(&self) -> Option<&std::ops::Range<usize>> {
        self.raw_completion.editable_range.as_ref()
    }

    pub fn cursor_offset(&self) -> Option<usize> {
        self.raw_completion.cursor_offset
    }

    pub fn model_version(&self) -> Option<&str> {
        self.model_version.as_deref()
    }
}

// ---- Accept/Reject edit prediction bodies (used by edit prediction crates) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptEditPredictionBody {
    pub request_id: String,
    pub model_version: Option<String>,
    pub e2e_latency_ms: Option<u128>,
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

// ---- Usage data (used by edit prediction) ----

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub used: i32,
    pub limit: UsageLimit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUsage {
    pub edit_predictions: UsageData,
}
