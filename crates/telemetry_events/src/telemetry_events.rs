//! Telemetry events have been removed. Types are kept as minimal stubs for compilation.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, time::Duration};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EventRequestBody {
    pub system_id: Option<String>,
    pub installation_id: Option<String>,
    pub session_id: Option<String>,
    pub metrics_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_staff: Option<bool>,
    pub app_version: String,
    pub os_name: String,
    pub os_version: Option<String>,
    pub architecture: String,
    pub release_channel: Option<String>,
    pub events: Vec<EventWrapper>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventWrapper {
    pub signed_in: bool,
    pub milliseconds_since_first_event: i64,
    #[serde(flatten)]
    pub event: Event,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantKind {
    Panel,
    Inline,
    InlineTerminal,
}

impl Display for AssistantKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Panel => write!(f, "panel"),
            Self::Inline => write!(f, "inline"),
            Self::InlineTerminal => write!(f, "inline_terminal"),
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantPhase {
    #[default]
    Response,
    Invoked,
    Accepted,
    Rejected,
}

impl Display for AssistantPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Response => write!(f, "response"),
            Self::Invoked => write!(f, "invoked"),
            Self::Accepted => write!(f, "accepted"),
            Self::Rejected => write!(f, "rejected"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    Flexible(FlexibleEvent),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlexibleEvent {
    pub event_type: String,
    pub event_properties: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EditPredictionRating {
    Positive,
    Negative,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssistantEventData {
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub kind: AssistantKind,
    #[serde(default)]
    pub phase: AssistantPhase,
    pub model: String,
    pub model_provider: String,
    pub response_latency: Option<Duration>,
    pub error_message: Option<String>,
    pub language_name: Option<String>,
}
