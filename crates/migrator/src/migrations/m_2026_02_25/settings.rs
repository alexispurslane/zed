use anyhow::Result;
use serde_json::Value;

use crate::migrations::migrate_settings;

const AGENT_SERVERS_KEY: &str = "agent_servers";

/// Old builtin agent keys that used the ACP registry (now removed).
/// Any entries under these keys are stale and should be dropped.
const STALE_BUILTIN_KEYS: &[&str] = &["claude", "codex", "gemini"];

pub fn migrate_builtin_agent_servers_to_registry(value: &mut Value) -> Result<()> {
    migrate_settings(value, &mut migrate_one)
}

fn migrate_one(obj: &mut serde_json::Map<String, Value>) -> Result<()> {
    let Some(agent_servers) = obj.get_mut(AGENT_SERVERS_KEY) else {
        return Ok(());
    };
    let Some(servers_map) = agent_servers.as_object_mut() else {
        return Ok(());
    };

    for &key in STALE_BUILTIN_KEYS {
        servers_map.remove(key);
    }

    Ok(())
}
