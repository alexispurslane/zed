//! Schema types for the agent thread protocol.
//!
//! These types are used internally by the native agent and its UI.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Trait for builder methods that accept `T`, `Option<T>`, or `&str` (when T=String).
pub trait IntoOption<T> {
    fn into_option(self) -> Option<T>;
}

impl<T> IntoOption<T> for Option<T> {
    fn into_option(self) -> Option<T> {
        self
    }
}

impl<T> IntoOption<T> for T {
    fn into_option(self) -> Option<T> {
        Some(self)
    }
}

impl IntoOption<String> for &str {
    fn into_option(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoOption<String> for &String {
    fn into_option(self) -> Option<String> {
        Some(self.clone())
    }
}

impl IntoOption<serde_json::Value> for &str {
    fn into_option(self) -> Option<serde_json::Value> {
        Some(serde_json::Value::String(self.to_string()))
    }
}

impl IntoOption<i32> for u32 {
    fn into_option(self) -> Option<i32> {
        Some(self as i32)
    }
}

// ---------------------------------------------------------------------------
// Newtype IDs
// ---------------------------------------------------------------------------

/// A unique identifier for a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, schemars::JsonSchema)]
#[serde(transparent)]
#[from(Arc<str>, String, &'static str)]
pub struct SessionId(pub Arc<str>);

impl SessionId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Unique identifier for a tool call within a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, schemars::JsonSchema)]
#[serde(transparent)]
#[from(Arc<str>, String, &'static str)]
pub struct ToolCallId(pub Arc<str>);

impl ToolCallId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Unique identifier for a terminal within a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, schemars::JsonSchema)]
#[serde(transparent)]
#[from(Arc<str>, String, &'static str)]
pub struct TerminalId(pub Arc<str>);

impl TerminalId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Identifier for a language model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, schemars::JsonSchema)]
#[serde(transparent)]
#[from(Arc<str>, String, &'static str)]
pub struct ModelId(pub Arc<str>);

impl ModelId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Identifier for a session mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, schemars::JsonSchema)]
#[serde(transparent)]
#[from(Arc<str>, String, &'static str)]
pub struct SessionModeId(pub Arc<str>);

impl SessionModeId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Unique identifier for a permission option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, schemars::JsonSchema)]
#[serde(transparent)]
#[from(Arc<str>, String, &'static str)]
pub struct PermissionOptionId(pub Arc<str>);

impl PermissionOptionId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Identifier for a session config option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, schemars::JsonSchema)]
#[serde(transparent)]
#[from(Arc<str>, String, &'static str)]
pub struct SessionConfigId(pub Arc<str>);

impl SessionConfigId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Identifier for a session config value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, schemars::JsonSchema)]
#[serde(transparent)]
#[from(Arc<str>, String, &'static str)]
pub struct SessionConfigValueId(pub Arc<str>);

impl SessionConfigValueId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

// ---------------------------------------------------------------------------
// Meta type
// ---------------------------------------------------------------------------

/// Extension metadata map (`_meta` field).
pub type Meta = serde_json::Map<String, serde_json::Value>;

// ---------------------------------------------------------------------------
// Content types
// ---------------------------------------------------------------------------

/// Content blocks represent displayable information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextContent),
    Image(ImageContent),
    Audio(AudioContent),
    ResourceLink(ResourceLink),
    Resource(EmbeddedResource),
}

impl<T: Into<String>> From<T> for ContentBlock {
    fn from(value: T) -> Self {
        Self::Text(TextContent::new(value))
    }
}

/// Text content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TextContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl TextContent {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            annotations: None,
            text: text.into(),
            meta: None,
        }
    }
}

/// Image content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ImageContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    pub data: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ImageContent {
    pub fn new(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            annotations: None,
            data: data.into(),
            mime_type: mime_type.into(),
            uri: None,
            meta: None,
        }
    }

    pub fn uri(mut self, uri: Option<String>) -> Self {
        self.uri = uri;
        self
    }
}

/// Audio content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AudioContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    pub data: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl AudioContent {
    pub fn new(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            annotations: None,
            data: data.into(),
            mime_type: mime_type.into(),
            meta: None,
        }
    }
}

/// A resource that the server is capable of reading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ResourceLink {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ResourceLink {
    pub fn new(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            annotations: None,
            description: None,
            mime_type: None,
            name: name.into(),
            size: None,
            title: None,
            uri: uri.into(),
            meta: None,
        }
    }

    pub fn title(mut self, title: impl IntoOption<String>) -> Self {
        self.title = title.into_option();
        self
    }

    pub fn mime_type(mut self, mime_type: impl IntoOption<String>) -> Self {
        self.mime_type = mime_type.into_option();
        self
    }

    pub fn description(mut self, description: impl IntoOption<String>) -> Self {
        self.description = description.into_option();
        self
    }

    pub fn annotations(mut self, annotations: impl IntoOption<Annotations>) -> Self {
        self.annotations = annotations.into_option();
        self
    }

    pub fn size(mut self, size: impl IntoOption<i64>) -> Self {
        self.size = size.into_option();
        self
    }

    pub fn meta(mut self, meta: impl IntoOption<Meta>) -> Self {
        self.meta = meta.into_option();
        self
    }
}

/// The contents of a resource, embedded into a prompt or tool call result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbeddedResource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    pub resource: EmbeddedResourceResource,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl EmbeddedResource {
    pub fn new(resource: EmbeddedResourceResource) -> Self {
        Self {
            annotations: None,
            resource,
            meta: None,
        }
    }
}

/// Resource content that can be embedded in a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum EmbeddedResourceResource {
    TextResourceContents(TextResourceContents),
    BlobResourceContents(BlobResourceContents),
}

/// Text-based resource contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TextResourceContents {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub text: String,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl TextResourceContents {
    pub fn new(text: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            mime_type: None,
            text: text.into(),
            uri: uri.into(),
            meta: None,
        }
    }
}

/// Binary resource contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BlobResourceContents {
    pub blob: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl BlobResourceContents {
    pub fn new(blob: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            blob: blob.into(),
            mime_type: None,
            uri: uri.into(),
            meta: None,
        }
    }
}

/// Optional annotations for content.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Annotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

/// The sender or recipient of messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Assistant,
    User,
}

// ---------------------------------------------------------------------------
// Content chunk
// ---------------------------------------------------------------------------

/// A streamed item of content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ContentChunk {
    /// A single item of content.
    pub content: ContentBlock,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ContentChunk {
    pub fn new(content: ContentBlock) -> Self {
        Self {
            content,
            meta: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tool call types
// ---------------------------------------------------------------------------

/// Categories of tools that can be invoked.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Read,
    Edit,
    Delete,
    Move,
    Search,
    Execute,
    Think,
    Fetch,
    SwitchMode,
    #[default]
    #[serde(other)]
    Other,
}

impl ToolKind {
    pub fn is_default(&self) -> bool {
        matches!(self, ToolKind::Other)
    }
}

/// Execution status of a tool call.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl ToolCallStatus {
    pub fn is_default(&self) -> bool {
        matches!(self, ToolCallStatus::Pending)
    }
}

/// Represents a tool call that the language model has requested.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ToolCall {
    pub tool_call_id: ToolCallId,
    pub title: String,
    #[serde(default, skip_serializing_if = "ToolKind::is_default")]
    pub kind: ToolKind,
    #[serde(default, skip_serializing_if = "ToolCallStatus::is_default")]
    pub status: ToolCallStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ToolCallContent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<ToolCallLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ToolCall {
    pub fn new(tool_call_id: impl Into<ToolCallId>, title: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            title: title.into(),
            kind: ToolKind::default(),
            status: ToolCallStatus::default(),
            content: Vec::default(),
            locations: Vec::default(),
            raw_input: None,
            raw_output: None,
            meta: None,
        }
    }

    pub fn kind(mut self, kind: ToolKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn status(mut self, status: ToolCallStatus) -> Self {
        self.status = status;
        self
    }

    pub fn content(mut self, content: Vec<ToolCallContent>) -> Self {
        self.content = content;
        self
    }

    pub fn locations(mut self, locations: Vec<ToolCallLocation>) -> Self {
        self.locations = locations;
        self
    }

    pub fn raw_input(mut self, raw_input: impl IntoOption<serde_json::Value>) -> Self {
        self.raw_input = raw_input.into_option();
        self
    }

    pub fn raw_output(mut self, raw_output: impl IntoOption<serde_json::Value>) -> Self {
        self.raw_output = raw_output.into_option();
        self
    }

    pub fn meta(mut self, meta: impl IntoOption<Meta>) -> Self {
        self.meta = meta.into_option();
        self
    }

    pub fn update(&mut self, fields: ToolCallUpdateFields) {
        if let Some(title) = fields.title {
            self.title = title;
        }
        if let Some(kind) = fields.kind {
            self.kind = kind;
        }
        if let Some(status) = fields.status {
            self.status = status;
        }
        if let Some(content) = fields.content {
            self.content = content;
        }
        if let Some(locations) = fields.locations {
            self.locations = locations;
        }
        if let Some(raw_input) = fields.raw_input {
            self.raw_input = Some(raw_input);
        }
        if let Some(raw_output) = fields.raw_output {
            self.raw_output = Some(raw_output);
        }
    }
}

/// Content produced by a tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolCallContent {
    Content(Content),
    Diff(Diff),
    Terminal(Terminal),
}

impl From<Content> for ToolCallContent {
    fn from(content: Content) -> Self {
        ToolCallContent::Content(content)
    }
}

impl From<Diff> for ToolCallContent {
    fn from(diff: Diff) -> Self {
        ToolCallContent::Diff(diff)
    }
}

impl From<String> for ToolCallContent {
    fn from(text: String) -> Self {
        ToolCallContent::Content(Content::new(ContentBlock::Text(TextContent::new(text))))
    }
}

impl From<&str> for ToolCallContent {
    fn from(text: &str) -> Self {
        ToolCallContent::Content(Content::new(ContentBlock::Text(TextContent::new(text.to_string()))))
    }
}

/// Standard content block wrapper for tool call content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Content {
    pub content: ContentBlock,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl Content {
    pub fn new(content: impl Into<ContentBlock>) -> Self {
        Self {
            content: content.into(),
            meta: None,
        }
    }
}

/// A terminal reference within a tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Terminal {
    pub terminal_id: TerminalId,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl Terminal {
    pub fn new(terminal_id: impl Into<TerminalId>) -> Self {
        Self {
            terminal_id: terminal_id.into(),
            meta: None,
        }
    }
}

/// A diff representing file modifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Diff {
    pub path: PathBuf,
    pub old_text: Option<String>,
    pub new_text: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl Diff {
    pub fn new(path: impl Into<PathBuf>, new_text: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            old_text: None,
            new_text: new_text.into(),
            meta: None,
        }
    }

    pub fn old_text(mut self, old_text: impl IntoOption<String>) -> Self {
        self.old_text = old_text.into_option();
        self
    }
}

/// A file location being accessed or modified by a tool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ToolCallLocation {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ToolCallLocation {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            line: None,
            meta: None,
        }
    }

    pub fn line(mut self, line: impl IntoOption<u32>) -> Self {
        self.line = line.into_option();
        self
    }
}

/// An update to an existing tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ToolCallUpdate {
    pub tool_call_id: ToolCallId,
    #[serde(flatten)]
    pub fields: ToolCallUpdateFields,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ToolCallUpdate {
    pub fn new(tool_call_id: impl Into<ToolCallId>, fields: ToolCallUpdateFields) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            fields,
            meta: None,
        }
    }

    pub fn meta(mut self, meta: impl IntoOption<Meta>) -> Self {
        self.meta = meta.into_option();
        self
    }
}

/// Optional fields that can be updated in a tool call.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ToolCallUpdateFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ToolKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ToolCallContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<ToolCallLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<serde_json::Value>,
}

impl ToolCallUpdateFields {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kind(mut self, kind: impl IntoOption<ToolKind>) -> Self {
        self.kind = kind.into_option();
        self
    }

    pub fn status(mut self, status: impl IntoOption<ToolCallStatus>) -> Self {
        self.status = status.into_option();
        self
    }

    pub fn title(mut self, title: impl IntoOption<String>) -> Self {
        self.title = title.into_option();
        self
    }

    pub fn content(mut self, content: impl IntoOption<Vec<ToolCallContent>>) -> Self {
        self.content = content.into_option();
        self
    }

    pub fn locations(mut self, locations: impl IntoOption<Vec<ToolCallLocation>>) -> Self {
        self.locations = locations.into_option();
        self
    }

    pub fn raw_input(mut self, raw_input: impl IntoOption<serde_json::Value>) -> Self {
        self.raw_input = raw_input.into_option();
        self
    }

    pub fn raw_output(mut self, raw_output: impl IntoOption<serde_json::Value>) -> Self {
        self.raw_output = raw_output.into_option();
        self
    }
}

impl TryFrom<ToolCallUpdate> for ToolCall {
    type Error = Error;

    fn try_from(update: ToolCallUpdate) -> Result<Self, Self::Error> {
        let ToolCallUpdate {
            tool_call_id,
            fields:
                ToolCallUpdateFields {
                    kind,
                    status,
                    title,
                    content,
                    locations,
                    raw_input,
                    raw_output,
                },
            meta,
        } = update;

        Ok(Self {
            tool_call_id,
            title: title.ok_or_else(|| {
                Error::invalid_params()
            })?,
            kind: kind.unwrap_or_default(),
            status: status.unwrap_or_default(),
            content: content.unwrap_or_default(),
            locations: locations.unwrap_or_default(),
            raw_input,
            raw_output,
            meta,
        })
    }
}

impl From<ToolCall> for ToolCallUpdate {
    fn from(value: ToolCall) -> Self {
        let ToolCall {
            tool_call_id,
            title,
            kind,
            status,
            content,
            locations,
            raw_input,
            raw_output,
            meta,
        } = value;
        Self {
            tool_call_id,
            fields: ToolCallUpdateFields {
                kind: Some(kind),
                status: Some(status),
                title: Some(title),
                content: Some(content),
                locations: Some(locations),
                raw_input,
                raw_output,
            },
            meta,
        }
    }
}

// ---------------------------------------------------------------------------
// Permission types
// ---------------------------------------------------------------------------

/// An option presented to the user when requesting permission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PermissionOption {
    pub option_id: PermissionOptionId,
    pub name: String,
    pub kind: PermissionOptionKind,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl PermissionOption {
    pub fn new(
        option_id: impl Into<PermissionOptionId>,
        name: impl Into<String>,
        kind: PermissionOptionKind,
    ) -> Self {
        Self {
            option_id: option_id.into(),
            name: name.into(),
            kind,
            meta: None,
        }
    }
}

/// The type of permission option.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

/// The outcome of a permission request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RequestPermissionOutcome {
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Selected(SelectedPermissionOutcome),
}

/// The user selected one of the provided options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SelectedPermissionOutcome {
    pub option_id: PermissionOptionId,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl SelectedPermissionOutcome {
    pub fn new(option_id: impl Into<PermissionOptionId>) -> Self {
        Self {
            option_id: option_id.into(),
            meta: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Session update types
// ---------------------------------------------------------------------------

/// Different types of updates during session processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "sessionUpdate", rename_all = "snake_case")]
pub enum SessionUpdate {
    UserMessageChunk(ContentChunk),
    AgentMessageChunk(ContentChunk),
    AgentThoughtChunk(ContentChunk),
    ToolCall(ToolCall),
    ToolCallUpdate(ToolCallUpdate),
    Plan(Plan),
    AvailableCommandsUpdate(AvailableCommandsUpdate),
    CurrentModeUpdate(CurrentModeUpdate),
    ConfigOptionUpdate(ConfigOptionUpdate),
    SessionInfoUpdate(SessionInfoUpdate),
    UsageUpdate(UsageUpdate),
}

/// The current mode of the session has changed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CurrentModeUpdate {
    pub current_mode_id: SessionModeId,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl CurrentModeUpdate {
    pub fn new(current_mode_id: impl Into<SessionModeId>) -> Self {
        Self {
            current_mode_id: current_mode_id.into(),
            meta: None,
        }
    }
}

/// Session configuration options have been updated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ConfigOptionUpdate {
    pub config_options: Vec<SessionConfigOption>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ConfigOptionUpdate {
    pub fn new(config_options: Vec<SessionConfigOption>) -> Self {
        Self {
            config_options,
            meta: None,
        }
    }
}

/// Update to session metadata.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SessionInfoUpdate {
    #[serde(default, skip_serializing_if = "MaybeUndefined::is_undefined")]
    pub title: MaybeUndefined<String>,
    #[serde(default, skip_serializing_if = "MaybeUndefined::is_undefined")]
    pub updated_at: MaybeUndefined<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl SessionInfoUpdate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<MaybeUndefined<String>>) -> Self {
        self.title = title.into();
        self
    }
}

/// Context window and cost update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UsageUpdate {
    pub used: u64,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<Cost>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl UsageUpdate {
    pub fn new(used: u64, size: u64) -> Self {
        Self {
            used,
            size,
            cost: None,
            meta: None,
        }
    }

    pub fn cost(mut self, cost: impl IntoOption<Cost>) -> Self {
        self.cost = cost.into_option();
        self
    }
}

/// Cost information for a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Cost {
    pub amount: f64,
    pub currency: String,
}

impl Cost {
    pub fn new(amount: f64, currency: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into(),
        }
    }
}

/// Available commands update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AvailableCommandsUpdate {
    pub available_commands: Vec<AvailableCommand>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl AvailableCommandsUpdate {
    pub fn new(available_commands: Vec<AvailableCommand>) -> Self {
        Self {
            available_commands,
            meta: None,
        }
    }
}

/// Information about a command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AvailableCommand {
    pub name: String,
    pub description: String,
    pub input: Option<AvailableCommandInput>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl AvailableCommand {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input: None,
            meta: None,
        }
    }

    pub fn input(mut self, input: impl IntoOption<AvailableCommandInput>) -> Self {
        self.input = input.into_option();
        self
    }
}

/// The input specification for a command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged, rename_all = "camelCase")]
pub enum AvailableCommandInput {
    Unstructured(UnstructuredCommandInput),
}

/// Unstructured command input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UnstructuredCommandInput {
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl UnstructuredCommandInput {
    pub fn new(hint: impl Into<String>) -> Self {
        Self {
            hint: hint.into(),
            meta: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Prompt types
// ---------------------------------------------------------------------------

/// Prompt capabilities supported by the agent.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PromptCapabilities {
    #[serde(default)]
    pub image: bool,
    #[serde(default)]
    pub audio: bool,
    #[serde(default)]
    pub embedded_context: bool,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl PromptCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn image(mut self, image: bool) -> Self {
        self.image = image;
        self
    }

    pub fn audio(mut self, audio: bool) -> Self {
        self.audio = audio;
        self
    }

    pub fn embedded_context(mut self, embedded_context: bool) -> Self {
        self.embedded_context = embedded_context;
        self
    }
}

/// Contains the user's message and any additional context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PromptRequest {
    pub session_id: SessionId,
    pub prompt: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl PromptRequest {
    pub fn new(session_id: impl Into<SessionId>, prompt: Vec<ContentBlock>) -> Self {
        Self {
            session_id: session_id.into(),
            prompt,
            meta: None,
        }
    }
}

/// Response from processing a user prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PromptResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    pub stop_reason: StopReason,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl PromptResponse {
    pub fn new(stop_reason: StopReason) -> Self {
        Self {
            stop_reason,
            usage: None,
            meta: None,
        }
    }

    pub fn usage(mut self, usage: impl IntoOption<Usage>) -> Self {
        self.usage = usage.into_option();
        self
    }
}

/// Reasons why an agent stops processing a prompt turn.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    MaxTurnRequests,
    Refusal,
    Cancelled,
}

// ---------------------------------------------------------------------------
// Usage (for PromptResponse)
// ---------------------------------------------------------------------------

/// Token usage information for a prompt turn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Usage {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_write_tokens: Option<u64>,
}

impl Usage {
    pub fn new(total_tokens: u64, input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            total_tokens,
            input_tokens,
            output_tokens,
            thought_tokens: None,
            cached_read_tokens: None,
            cached_write_tokens: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Session config types
// ---------------------------------------------------------------------------

/// A session mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SessionMode {
    pub id: SessionModeId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl SessionMode {
    pub fn new(id: impl Into<SessionModeId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            meta: None,
        }
    }
}

/// Category for a session config option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionConfigOptionCategory {
    Mode,
    Model,
    ThoughtLevel,
    #[serde(untagged)]
    Other(String),
}

/// Kind of session config option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionConfigKind {
    Select(SessionConfigSelect),
    #[serde(rename = "boolean")]
    Boolean(SessionConfigBoolean),
}

/// A select-type session config option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SessionConfigSelect {
    pub current_value: SessionConfigValueId,
    pub options: SessionConfigSelectOptions,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

/// A select option for a session config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SessionConfigSelectOption {
    pub value: SessionConfigValueId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl SessionConfigSelectOption {
    pub fn new(value: impl Into<SessionConfigValueId>, name: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            name: name.into(),
            description: None,
            meta: None,
        }
    }
}

/// Select options that may include grouped items.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SessionConfigSelectOptions {
    Ungrouped(Vec<SessionConfigSelectOption>),
    Grouped(Vec<SessionConfigSelectGroup>),
}

/// A group of select options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SessionConfigSelectGroup {
    pub name: String,
    pub options: Vec<SessionConfigSelectOption>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

/// A boolean session config option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SessionConfigBoolean {
    pub current_value: bool,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

/// A session configuration option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SessionConfigOption {
    pub id: SessionConfigId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<SessionConfigOptionCategory>,
    #[serde(flatten)]
    pub kind: SessionConfigKind,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl SessionConfigOption {
    pub fn select(
        id: impl Into<SessionConfigId>,
        name: impl Into<String>,
        current_value: impl Into<SessionConfigValueId>,
        options: Vec<SessionConfigSelectOption>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            category: None,
            kind: SessionConfigKind::Select(SessionConfigSelect {
                current_value: current_value.into(),
                options: SessionConfigSelectOptions::Ungrouped(options),
                meta: None,
            }),
            meta: None,
        }
    }

    pub fn boolean(
        id: impl Into<SessionConfigId>,
        name: impl Into<String>,
        current_value: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            category: None,
            kind: SessionConfigKind::Boolean(SessionConfigBoolean {
                current_value,
                meta: None,
            }),
            meta: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Plan types
// ---------------------------------------------------------------------------

/// An execution plan for accomplishing complex tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Plan {
    pub entries: Vec<PlanEntry>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl Plan {
    pub fn new(entries: Vec<PlanEntry>) -> Self {
        Self {
            entries,
            meta: None,
        }
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// A single entry in the execution plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PlanEntry {
    pub content: String,
    pub priority: PlanEntryPriority,
    pub status: PlanEntryStatus,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl PlanEntry {
    pub fn new(
        content: impl Into<String>,
        priority: PlanEntryPriority,
        status: PlanEntryStatus,
    ) -> Self {
        Self {
            content: content.into(),
            priority,
            status,
            meta: None,
        }
    }
}

/// Priority levels for plan entries.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanEntryPriority {
    High,
    Medium,
    Low,
}

impl Default for PlanEntryPriority {
    fn default() -> Self {
        Self::Medium
    }
}

/// Status of a plan entry.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanEntryStatus {
    Pending,
    InProgress,
    Completed,
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Terminal output types
// ---------------------------------------------------------------------------

/// Terminal exit status.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TerminalExitStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
}

impl TerminalExitStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn exit_code(mut self, code: impl IntoOption<i32>) -> Self {
        self.exit_code = code.into_option();
        self
    }

    pub fn signal(mut self, signal: impl IntoOption<String>) -> Self {
        self.signal = signal.into_option();
        self
    }
}

/// Terminal output response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TerminalOutputResponse {
    pub output: String,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_status: Option<TerminalExitStatus>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl TerminalOutputResponse {
    pub fn new(output: impl Into<String>, truncated: bool) -> Self {
        Self {
            output: output.into(),
            truncated,
            exit_status: None,
            meta: None,
        }
    }

    pub fn exit_status(mut self, status: TerminalExitStatus) -> Self {
        self.exit_status = Some(status);
        self
    }
}

// ---------------------------------------------------------------------------
// MaybeUndefined
// ---------------------------------------------------------------------------

/// Similar to `Option`, but with three states: `Undefined`, `Null`, and `Value`.
#[allow(missing_docs)]
#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum MaybeUndefined<T> {
    #[default]
    Undefined,
    Null,
    Value(T),
}

impl<T> MaybeUndefined<T> {
    pub const fn is_undefined(&self) -> bool {
        matches!(self, MaybeUndefined::Undefined)
    }

    pub const fn is_null(&self) -> bool {
        matches!(self, MaybeUndefined::Null)
    }

    pub const fn is_value(&self) -> bool {
        matches!(self, MaybeUndefined::Value(_))
    }

    pub const fn value(&self) -> Option<&T> {
        match self {
            MaybeUndefined::Value(value) => Some(value),
            _ => None,
        }
    }

    pub fn take(self) -> Option<T> {
        match self {
            MaybeUndefined::Value(value) => Some(value),
            _ => None,
        }
    }
}

impl<T: Serialize> Serialize for MaybeUndefined<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MaybeUndefined::Value(value) => value.serialize(serializer),
            MaybeUndefined::Null => serializer.serialize_none(),
            MaybeUndefined::Undefined => serializer.serialize_unit(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for MaybeUndefined<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => MaybeUndefined::Value(value),
            None => MaybeUndefined::Null,
        })
    }
}

impl<T> From<MaybeUndefined<T>> for Option<Option<T>> {
    fn from(maybe_undefined: MaybeUndefined<T>) -> Self {
        match maybe_undefined {
            MaybeUndefined::Undefined => None,
            MaybeUndefined::Null => Some(None),
            MaybeUndefined::Value(value) => Some(Some(value)),
        }
    }
}

impl<T> From<Option<Option<T>>> for MaybeUndefined<T> {
    fn from(value: Option<Option<T>>) -> Self {
        match value {
            Some(Some(value)) => Self::Value(value),
            Some(None) => Self::Null,
            None => Self::Undefined,
        }
    }
}

impl From<&str> for MaybeUndefined<String> {
    fn from(value: &str) -> Self {
        MaybeUndefined::Value(value.to_string())
    }
}

impl From<String> for MaybeUndefined<String> {
    fn from(value: String) -> Self {
        MaybeUndefined::Value(value)
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// JSON-RPC error object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Error {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Error {
            code: code.into(),
            message: message.into(),
            data: None,
        }
    }

    pub fn data(mut self, data: impl IntoOption<serde_json::Value>) -> Self {
        self.data = data.into_option();
        self
    }

    pub fn invalid_params() -> Self {
        ErrorCode::InvalidParams.into()
    }

    pub fn invalid_params_msg(msg: impl Into<String>) -> Self {
        Self::new(i32::from(ErrorCode::InvalidParams), format!("Invalid params: {}", msg.into()))
    }

    pub fn method_not_found() -> Self {
        ErrorCode::MethodNotFound.into()
    }

    pub fn resource_not_found(uri: Option<String>) -> Self {
        let err: Self = ErrorCode::ResourceNotFound.into();
        if let Some(uri) = uri {
            err.data(serde_json::json!({ "uri": uri }))
        } else {
            err
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.message.is_empty() {
            write!(f, "{}", i32::from(self.code))?;
        } else {
            write!(f, "{}", self.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::new(-32603, error.to_string())
    }
}

/// Error codes.
#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "i32", into = "i32")]
pub enum ErrorCode {
    ParseError,      // -32700
    InvalidRequest,  // -32600
    MethodNotFound,  // -32601
    InvalidParams,   // -32602
    InternalError,   // -32603
    ResourceNotFound, // -32002
    Other(i32),
}

impl From<i32> for ErrorCode {
    fn from(value: i32) -> Self {
        match value {
            -32700 => ErrorCode::ParseError,
            -32600 => ErrorCode::InvalidRequest,
            -32601 => ErrorCode::MethodNotFound,
            -32602 => ErrorCode::InvalidParams,
            -32603 => ErrorCode::InternalError,
            -32002 => ErrorCode::ResourceNotFound,
            _ => ErrorCode::Other(value),
        }
    }
}

impl From<ErrorCode> for i32 {
    fn from(value: ErrorCode) -> Self {
        match value {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::ResourceNotFound => -32002,
            ErrorCode::Other(v) => v,
        }
    }
}

impl From<ErrorCode> for Error {
    fn from(error_code: ErrorCode) -> Self {
        let message = match error_code {
            ErrorCode::ParseError => "Parse error",
            ErrorCode::InvalidRequest => "Invalid request",
            ErrorCode::MethodNotFound => "Method not found",
            ErrorCode::InvalidParams => "Invalid params",
            ErrorCode::InternalError => "Internal error",
            ErrorCode::ResourceNotFound => "Resource not found",
            ErrorCode::Other(_) => "Unknown error",
        };
        Error::new(error_code.into(), message)
    }
}

impl std::fmt::Debug for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", i32::from(*self), self)
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::ParseError => write!(f, "Parse error"),
            ErrorCode::InvalidRequest => write!(f, "Invalid request"),
            ErrorCode::MethodNotFound => write!(f, "Method not found"),
            ErrorCode::InvalidParams => write!(f, "Invalid params"),
            ErrorCode::InternalError => write!(f, "Internal error"),
            ErrorCode::ResourceNotFound => write!(f, "Resource not found"),
            ErrorCode::Other(_) => write!(f, "Unknown error"),
        }
    }
}

// ---------------------------------------------------------------------------
// Model info
// ---------------------------------------------------------------------------

/// Information about a language model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelInfo {
    pub model_id: ModelId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ModelInfo {
    pub fn new(model_id: impl Into<ModelId>, name: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            name: name.into(),
            description: None,
            meta: None,
        }
    }
}

// ---------------------------------------------------------------------------
// EnvVariable (used in terminal requests)
// ---------------------------------------------------------------------------

/// An environment variable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct EnvVariable {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
}

fn default_true() -> bool {
    true
}
fn is_true(v: &bool) -> bool {
    *v
}
fn is_false(v: &bool) -> bool {
    !*v
}

impl EnvVariable {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            label: None,
            secret: true,
            optional: false,
        }
    }
}
