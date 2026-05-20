use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// What an extension provides to Xenomorphic.
///
/// Previously came from the `cloud_api_types` crate, now defined locally
/// since that crate was removed.
#[derive(
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    strum::EnumString,
    strum::Display,
    strum::EnumIter,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ExtensionProvides {
    Themes,
    IconThemes,
    Languages,
    Grammars,
    LanguageServers,
    ContextServers,
    AgentServers,
    Snippets,
    DebugAdapters,
}

/// Extension API manifest, used for the extension marketplace.
///
/// Previously came from the `cloud_api_types` crate, now defined locally
/// since that crate was removed.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ExtensionApiManifest {
    pub name: String,
    pub version: Arc<str>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub repository: String,
    pub schema_version: Option<i32>,
    pub wasm_api_version: Option<String>,
    #[serde(default)]
    pub provides: BTreeSet<ExtensionProvides>,
}
